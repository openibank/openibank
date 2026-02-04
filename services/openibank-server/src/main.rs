//! OpeniBank Server - One-command AI Agent Banking Server
//!
//! The unified server that combines all OpeniBank services into a single binary:
//! - Maple iBank Runtime (Resonance Architecture)
//! - PALM Fleet Orchestration
//! - UAL Command Interface
//! - REST API + SSE Events
//! - Web Dashboard
//! - IUSD Issuer
//!
//! # Quick Start
//!
//! ```bash
//! # Start with defaults (localhost:8080)
//! openibank-server
//!
//! # Custom port and host
//! openibank-server --port 9090 --host 0.0.0.0
//!
//! # With LLM backend
//! OPENIBANK_LLM_PROVIDER=ollama openibank-server
//!
//! # With Anthropic Claude
//! OPENIBANK_LLM_PROVIDER=anthropic ANTHROPIC_API_KEY=sk-... openibank-server
//! ```

use std::sync::Arc;

use axum::{
    extract::State,
    response::{Html, Json as AxumJson},
    routing::{get, post},
    Router,
};
use clap::Parser;
use openibank_llm::LLMRouter;
use openibank_palm::{IBankFleetManager, FleetConfig, FinancialAgentType};
use openibank_state::SystemState;
use openibank_ual::{parse_input, compile_statements, ParsedInput, ExecutionResult, BankingCommand};
use palm_registry::AgentRegistry;
use serde::Deserialize;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// OpeniBank Server - AI Agent Banking Infrastructure
#[derive(Parser, Debug)]
#[command(
    name = "openibank-server",
    about = "OpeniBank - The Open AI Agent Banking Server",
    long_about = "Run your own AI Agent Bank. Powered by Maple AI Framework's Resonance Architecture.\n\nVisit https://www.openibank.com for documentation and community.",
    version
)]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0", env = "OPENIBANK_HOST")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080", env = "OPENIBANK_PORT")]
    port: u16,

    /// Enable auto-trading simulation
    #[arg(long, default_value = "false")]
    auto_trade: bool,

    /// Number of default buyer agents to create on startup
    #[arg(long, default_value = "0")]
    buyers: u32,

    /// Number of default seller agents to create on startup
    #[arg(long, default_value = "0")]
    sellers: u32,

    /// Initial IUSD supply to mint (in cents, e.g. 100000 = $1000)
    #[arg(long, default_value = "0")]
    initial_supply: u64,
}

/// Shared application state
struct AppState {
    system: SystemState,
    fleet: IBankFleetManager,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    print_banner();

    // Bootstrap system state
    tracing::info!("Bootstrapping Maple iBank Runtime...");
    let system = match SystemState::new().await {
        Ok(s) => {
            tracing::info!("Maple iBank runtime bootstrapped with 8 canonical invariants");
            s
        }
        Err(e) => {
            tracing::error!("Failed to bootstrap system: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize PALM fleet manager
    tracing::info!("Initializing PALM fleet orchestration...");
    let fleet = IBankFleetManager::new(FleetConfig::default());

    // Register default agent specs
    register_default_specs(&fleet).await;

    // Check LLM availability
    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;
    tracing::info!(
        "LLM: {}",
        if llm_available {
            format!("Available ({})", std::env::var("OPENIBANK_LLM_PROVIDER").unwrap_or_else(|_| "auto".to_string()))
        } else {
            "Not available (deterministic mode)".to_string()
        }
    );

    let state = Arc::new(AppState { system, fleet });

    // Build router
    let app = Router::new()
        // Web dashboard
        .route("/", get(dashboard))
        // System APIs
        .route("/api/status", get(api_status))
        .route("/api/health", get(api_health))
        // UAL command endpoint
        .route("/api/ual", post(api_ual_execute))
        // Fleet management
        .route("/api/fleet/status", get(api_fleet_status))
        .route("/api/fleet/specs", get(api_fleet_specs))
        .route("/api/fleet/deploy", post(api_fleet_deploy))
        // Agent management (delegate to system state)
        .route("/api/agents", get(api_list_agents))
        // Issuer
        .route("/api/issuer/supply", get(api_supply))
        // Info
        .route("/api/info", get(api_info))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", args.host, args.port);
    tracing::info!("OpeniBank Server running at http://{}:{}", args.host, args.port);
    tracing::info!("Dashboard:  http://localhost:{}", args.port);
    tracing::info!("API:        http://localhost:{}/api/status", args.port);
    tracing::info!("UAL:        POST http://localhost:{}/api/ual", args.port);
    tracing::info!("Fleet:      http://localhost:{}/api/fleet/status", args.port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn print_banner() {
    eprintln!(r#"
 ╔══════════════════════════════════════════════════════╗
 ║                                                      ║
 ║   ██████╗ ██████╗ ███████╗███╗   ██╗██╗              ║
 ║  ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██║              ║
 ║  ██║   ██║██████╔╝█████╗  ██╔██╗ ██║██║              ║
 ║  ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██║              ║
 ║  ╚██████╔╝██║     ███████╗██║ ╚████║██║              ║
 ║   ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝╚═╝              ║
 ║          ██████╗  █████╗ ███╗   ██╗██╗  ██╗           ║
 ║          ██╔══██╗██╔══██╗████╗  ██║██║ ██╔╝           ║
 ║          ██████╔╝███████║██╔██╗ ██║█████╔╝            ║
 ║          ██╔══██╗██╔══██║██║╚██╗██║██╔═██╗            ║
 ║          ██████╔╝██║  ██║██║ ╚████║██║  ██╗           ║
 ║          ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝           ║
 ║                                                      ║
 ║  The Open AI Agent Banking Server                    ║
 ║  Powered by Maple Resonance Architecture             ║
 ║  https://www.openibank.com                           ║
 ║                                                      ║
 ╚══════════════════════════════════════════════════════╝
"#);
}

async fn register_default_specs(fleet: &IBankFleetManager) {
    let specs = [
        ("buyer-agent", "1.0.0", FinancialAgentType::Buyer, "Standard buyer agent with budget management and spend permits"),
        ("seller-agent", "1.0.0", FinancialAgentType::Seller, "Standard seller agent with service publishing and invoice issuance"),
        ("arbiter-agent", "1.0.0", FinancialAgentType::Arbiter, "Dispute resolution agent with escrow release/refund authority"),
        ("issuer-agent", "1.0.0", FinancialAgentType::Issuer, "IUSD stablecoin issuer with reserve management"),
        ("auditor-agent", "1.0.0", FinancialAgentType::Auditor, "Ledger audit and receipt verification agent"),
        ("compliance-agent", "1.0.0", FinancialAgentType::Compliance, "Policy enforcement and risk assessment agent"),
    ];

    for (name, version, agent_type, description) in specs {
        match fleet.register_agent_spec(name, version, agent_type, description).await {
            Ok(id) => tracing::debug!(name = name, "Registered agent spec: {}", id),
            Err(e) => tracing::warn!(name = name, "Failed to register spec: {}", e),
        }
    }

    tracing::info!("Registered {} default financial agent specs", specs.len());
}

// ============================================================================
// Dashboard
// ============================================================================

async fn dashboard() -> Html<String> {
    Html(DASHBOARD_HTML.to_string())
}

// ============================================================================
// API Endpoints
// ============================================================================

async fn api_status(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let summary = state.system.status_summary().await;
    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;
    let fleet_status = state.fleet.fleet_status().await.ok();

    AxumJson(serde_json::json!({
        "name": "OpeniBank Server",
        "version": env!("CARGO_PKG_VERSION"),
        "maple_runtime": summary.runtime,
        "llm_available": llm_available,
        "llm_provider": std::env::var("OPENIBANK_LLM_PROVIDER").unwrap_or_else(|_| "none".to_string()),
        "agents": {
            "total": summary.agent_count,
            "buyers": summary.buyer_count,
            "sellers": summary.seller_count,
            "arbiters": summary.arbiter_count
        },
        "trading": {
            "trade_count": summary.trade_count,
            "total_volume": summary.total_volume,
            "total_volume_display": format!("${:.2}", summary.total_volume as f64 / 100.0)
        },
        "issuer": {
            "total_supply": summary.total_supply,
            "remaining_supply": summary.remaining_supply,
        },
        "fleet": fleet_status,
        "maple": {
            "accountability": summary.maple_accountability,
            "couplings": summary.maple_couplings,
            "commitments": summary.maple_commitments,
        },
        "uptime_seconds": summary.uptime_seconds,
        "started_at": summary.started_at,
    }))
}

async fn api_health(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let summary = state.system.status_summary().await;
    AxumJson(serde_json::json!({
        "status": "healthy",
        "uptime_seconds": summary.uptime_seconds,
        "maple_runtime": "active",
        "palm_fleet": "active",
    }))
}

async fn api_info() -> AxumJson<serde_json::Value> {
    AxumJson(serde_json::json!({
        "name": "OpeniBank",
        "description": "The Open AI Agent Banking Server",
        "version": env!("CARGO_PKG_VERSION"),
        "homepage": "https://www.openibank.com",
        "repository": "https://github.com/openibank/openibank",
        "license": "Apache-2.0",
        "architecture": {
            "runtime": "Maple Resonance Architecture",
            "fleet": "PALM (Persistent Agent Lifecycle Manager)",
            "language": "UAL (Universal Agent Language)",
            "accountability": "AAS (Authority & Accountability Service)",
            "commitments": "RCF (Resonance Commitment Framework)"
        },
        "capabilities": [
            "AI-agent-only banking (no human accounts)",
            "Commitment-gated escrow settlement",
            "Multi-provider LLM integration (Ollama, OpenAI, Anthropic, Gemini, Grok)",
            "IUSD stablecoin issuance with reserve management",
            "Ed25519 cryptographic receipts",
            "Double-entry immutable ledger",
            "PALM fleet orchestration with IBank health thresholds",
            "UAL SQL-like banking commands",
            "Real-time SSE event streaming",
            "MCP server for Claude Desktop integration"
        ],
        "invariants": [
            "Presence precedes meaning",
            "Meaning precedes intent",
            "Intent precedes commitment",
            "Commitment precedes consequence",
            "Coupling bounded by attention",
            "Safety overrides optimization",
            "Human agency cannot be bypassed",
            "Failure must be explicit"
        ]
    }))
}

// ============================================================================
// UAL Command Endpoint
// ============================================================================

#[derive(Deserialize)]
struct UalRequest {
    command: String,
}

async fn api_ual_execute(
    State(state): State<Arc<AppState>>,
    AxumJson(req): AxumJson<UalRequest>,
) -> AxumJson<serde_json::Value> {
    match parse_input(&req.command) {
        Ok(ParsedInput::Ual(statements)) => {
            match compile_statements(&statements) {
                Ok(compiled) => {
                    let artifacts: Vec<serde_json::Value> = compiled.iter()
                        .map(|c| serde_json::to_value(c).unwrap_or_default())
                        .collect();

                    AxumJson(serde_json::json!({
                        "success": true,
                        "type": "ual",
                        "statement_count": statements.len(),
                        "compiled": artifacts,
                        "message": format!("Compiled {} UAL statement(s) into formal artifacts", statements.len()),
                    }))
                }
                Err(e) => AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Compilation error: {}", e),
                })),
            }
        }
        Ok(ParsedInput::Banking(cmd)) => {
            let result = execute_banking_command(&state, cmd).await;
            AxumJson(serde_json::to_value(&result).unwrap_or_default())
        }
        Err(e) => AxumJson(serde_json::json!({
            "success": false,
            "error": format!("Parse error: {}", e),
            "hint": "Try: STATUS, BALANCE <agent>, MINT 10000 IUSD TO <agent>, or UAL COMMIT statements",
        })),
    }
}

async fn execute_banking_command(state: &AppState, cmd: BankingCommand) -> ExecutionResult {
    match cmd {
        BankingCommand::Status => {
            let summary = state.system.status_summary().await;
            ExecutionResult::ok_with_data(
                format!("System: {} agents, {} trades, ${:.2} volume",
                    summary.agent_count, summary.trade_count,
                    summary.total_volume as f64 / 100.0),
                serde_json::to_value(&summary).unwrap_or_default(),
            )
        }
        BankingCommand::ListAgents => {
            let registry = state.system.agents.read().await;
            let agents: Vec<serde_json::Value> = registry.agents.values()
                .map(|a| serde_json::to_value(a.to_api_info()).unwrap_or_default())
                .collect();
            ExecutionResult::ok_with_data(
                format!("{} agents registered", agents.len()),
                serde_json::json!({ "agents": agents }),
            )
        }
        BankingCommand::FleetStatus => {
            match state.fleet.fleet_status().await {
                Ok(status) => ExecutionResult::ok_with_data(
                    format!("{} specs, {} instances ({} healthy)",
                        status.total_specs, status.total_instances, status.healthy_instances),
                    serde_json::to_value(&status).unwrap_or_default(),
                ),
                Err(e) => ExecutionResult::err(format!("Fleet status error: {}", e)),
            }
        }
        BankingCommand::Balance { account } => {
            let res_id = openibank_core::ResonatorId::from_string(&account);
            let asset = openibank_core::AssetId::iusd();
            let balance = state.system.ledger.balance(&res_id, &asset).await;
            ExecutionResult::ok_with_data(
                format!("{}: ${:.2}", account, balance.0 as f64 / 100.0),
                serde_json::json!({ "account": account, "balance": balance.0 }),
            )
        }
        BankingCommand::DeployFleet { agent_type, count } => {
            let fin_type = match agent_type.as_str() {
                "buyer" => FinancialAgentType::Buyer,
                "seller" => FinancialAgentType::Seller,
                "arbiter" => FinancialAgentType::Arbiter,
                "issuer" => FinancialAgentType::Issuer,
                "auditor" => FinancialAgentType::Auditor,
                "compliance" => FinancialAgentType::Compliance,
                _ => return ExecutionResult::err(format!("Unknown agent type: {}", agent_type)),
            };
            let spec_name = format!("{}-agent", agent_type);
            // Find the spec by name
            match state.fleet.agent_registry().list().await {
                Ok(specs) => {
                    if let Some(spec) = specs.iter().find(|s| s.name == spec_name) {
                        match state.fleet.deploy_instances(&spec.id, count).await {
                            Ok(ids) => ExecutionResult::ok_with_data(
                                format!("Deployed {} {} instance(s)", ids.len(), agent_type),
                                serde_json::json!({ "instances": ids.len(), "type": agent_type }),
                            ),
                            Err(e) => ExecutionResult::err(format!("Deploy failed: {}", e)),
                        }
                    } else {
                        ExecutionResult::err(format!("Spec '{}' not found", spec_name))
                    }
                }
                Err(e) => ExecutionResult::err(format!("Registry error: {}", e)),
            }
        }
        _ => ExecutionResult::ok("Command acknowledged (execution pending full integration)"),
    }
}

// ============================================================================
// Fleet API
// ============================================================================

async fn api_fleet_status(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    match state.fleet.fleet_status().await {
        Ok(status) => AxumJson(serde_json::to_value(&status).unwrap_or_default()),
        Err(e) => AxumJson(serde_json::json!({ "error": format!("{}", e) })),
    }
}

async fn api_fleet_specs(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    match state.fleet.agent_registry().list().await {
        Ok(specs) => {
            let infos: Vec<serde_json::Value> = specs.iter()
                .map(|s| serde_json::json!({
                    "id": s.id.to_string(),
                    "name": s.name,
                    "version": s.version.to_string(),
                    "autonomy": format!("{:?}", s.resonator_profile.autonomy_level),
                    "risk_tolerance": format!("{:?}", s.resonator_profile.risk_tolerance),
                    "capabilities": s.capabilities.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
                }))
                .collect();
            AxumJson(serde_json::json!({ "specs": infos, "count": infos.len() }))
        }
        Err(e) => AxumJson(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct DeployRequest {
    agent_type: String,
    count: Option<u32>,
}

async fn api_fleet_deploy(
    State(state): State<Arc<AppState>>,
    AxumJson(req): AxumJson<DeployRequest>,
) -> AxumJson<serde_json::Value> {
    let count = req.count.unwrap_or(1);
    let spec_name = format!("{}-agent", req.agent_type);

    match state.fleet.agent_registry().list().await {
        Ok(specs) => {
            if let Some(spec) = specs.iter().find(|s| s.name == spec_name) {
                match state.fleet.deploy_instances(&spec.id, count).await {
                    Ok(ids) => AxumJson(serde_json::json!({
                        "success": true,
                        "instances_deployed": ids.len(),
                        "agent_type": req.agent_type,
                        "instance_ids": ids.iter().map(|i| i.to_string()).collect::<Vec<_>>(),
                    })),
                    Err(e) => AxumJson(serde_json::json!({ "error": format!("{}", e) })),
                }
            } else {
                AxumJson(serde_json::json!({ "error": format!("Agent spec '{}' not found", spec_name) }))
            }
        }
        Err(e) => AxumJson(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ============================================================================
// Agent & Issuer APIs (delegate to system state)
// ============================================================================

async fn api_list_agents(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let registry = state.system.agents.read().await;
    let agents: Vec<serde_json::Value> = registry.agents.values()
        .map(|a| serde_json::to_value(a.to_api_info()).unwrap_or_default())
        .collect();
    AxumJson(serde_json::json!({
        "agents": agents,
        "count": agents.len(),
    }))
}

async fn api_supply(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let issuer = state.system.issuer.read().await;
    let total = issuer.total_supply().await;
    let remaining = issuer.remaining_supply().await;
    AxumJson(serde_json::json!({
        "total_supply": total.0,
        "remaining_supply": remaining.0,
        "total_display": format!("${:.2}", total.0 as f64 / 100.0),
        "remaining_display": format!("${:.2}", remaining.0 as f64 / 100.0),
        "currency": "IUSD",
    }))
}

// ============================================================================
// Dashboard HTML
// ============================================================================

const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OpeniBank - AI Agent Banking Server</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
    background: #0a0a0f;
    color: #e0e0e0;
    min-height: 100vh;
  }
  .header {
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
    border-bottom: 2px solid #00d4ff;
    padding: 2rem;
    text-align: center;
  }
  .header h1 { color: #00d4ff; font-size: 2rem; letter-spacing: 3px; }
  .header .subtitle { color: #888; font-size: 0.9rem; margin-top: 0.5rem; }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1.5rem;
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }
  .card {
    background: #12121a;
    border: 1px solid #2a2a3a;
    border-radius: 8px;
    padding: 1.5rem;
  }
  .card h2 {
    color: #00d4ff;
    font-size: 1rem;
    letter-spacing: 2px;
    margin-bottom: 1rem;
    border-bottom: 1px solid #2a2a3a;
    padding-bottom: 0.5rem;
  }
  .stat { display: flex; justify-content: space-between; padding: 0.4rem 0; }
  .stat-label { color: #888; }
  .stat-value { color: #00ff88; font-weight: bold; }
  .ual-input {
    width: 100%;
    background: #0a0a12;
    border: 1px solid #2a2a3a;
    color: #e0e0e0;
    padding: 0.8rem;
    font-family: inherit;
    font-size: 0.9rem;
    border-radius: 4px;
    resize: vertical;
    min-height: 80px;
  }
  .btn {
    background: #00d4ff;
    color: #0a0a0f;
    border: none;
    padding: 0.6rem 1.5rem;
    font-family: inherit;
    font-weight: bold;
    cursor: pointer;
    border-radius: 4px;
    margin-top: 0.8rem;
  }
  .btn:hover { background: #00b8e6; }
  .output {
    background: #0a0a12;
    border: 1px solid #2a2a3a;
    padding: 0.8rem;
    margin-top: 0.8rem;
    border-radius: 4px;
    white-space: pre-wrap;
    font-size: 0.85rem;
    max-height: 300px;
    overflow-y: auto;
    color: #00ff88;
  }
  .spec-list { list-style: none; }
  .spec-list li {
    padding: 0.4rem 0;
    border-bottom: 1px solid #1a1a2a;
    font-size: 0.85rem;
  }
  .spec-name { color: #00d4ff; }
  .spec-caps { color: #666; font-size: 0.75rem; }
  .footer {
    text-align: center;
    padding: 2rem;
    color: #444;
    font-size: 0.8rem;
    border-top: 1px solid #1a1a2a;
  }
  .footer a { color: #00d4ff; text-decoration: none; }
  .live-dot {
    display: inline-block;
    width: 8px; height: 8px;
    background: #00ff88;
    border-radius: 50%;
    margin-right: 0.5rem;
    animation: pulse 2s infinite;
  }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
</style>
</head>
<body>
<div class="header">
  <h1>OPENIBANK</h1>
  <div class="subtitle"><span class="live-dot"></span>AI Agent Banking Server &middot; Powered by Maple Resonance Architecture</div>
</div>

<div class="grid">
  <div class="card">
    <h2>SYSTEM STATUS</h2>
    <div id="status">Loading...</div>
  </div>

  <div class="card">
    <h2>FLEET ORCHESTRATION</h2>
    <div id="fleet">Loading...</div>
  </div>

  <div class="card">
    <h2>AGENT SPECS</h2>
    <ul class="spec-list" id="specs">Loading...</ul>
  </div>

  <div class="card" style="grid-column: 1 / -1;">
    <h2>UAL COMMAND CONSOLE</h2>
    <textarea class="ual-input" id="ual-input" placeholder="Enter UAL commands or banking operations...&#10;&#10;Examples:&#10;  STATUS&#10;  FLEET STATUS&#10;  DEPLOY buyer COUNT 3&#10;  BALANCE buyer-001&#10;  COMMIT BY &quot;agent-001&quot; DOMAIN Finance OUTCOME &quot;Test&quot; SCOPE GLOBAL REVERSIBLE;"></textarea>
    <button class="btn" onclick="executeUal()">EXECUTE</button>
    <div class="output" id="ual-output">Ready for commands...</div>
  </div>
</div>

<div class="footer">
  <a href="https://www.openibank.com">openibank.com</a> &middot;
  <a href="https://github.com/openibank/openibank">GitHub</a> &middot;
  Apache-2.0 License
</div>

<script>
async function loadStatus() {
  try {
    const res = await fetch('/api/status');
    const data = await res.json();
    document.getElementById('status').innerHTML = `
      <div class="stat"><span class="stat-label">Runtime</span><span class="stat-value">Active</span></div>
      <div class="stat"><span class="stat-label">Agents</span><span class="stat-value">${data.agents?.total || 0}</span></div>
      <div class="stat"><span class="stat-label">Trades</span><span class="stat-value">${data.trading?.trade_count || 0}</span></div>
      <div class="stat"><span class="stat-label">Volume</span><span class="stat-value">${data.trading?.total_volume_display || '$0.00'}</span></div>
      <div class="stat"><span class="stat-label">LLM</span><span class="stat-value">${data.llm_available ? 'Available' : 'Deterministic'}</span></div>
      <div class="stat"><span class="stat-label">Uptime</span><span class="stat-value">${data.uptime_seconds || 0}s</span></div>
    `;
  } catch (e) { document.getElementById('status').textContent = 'Error loading status'; }
}

async function loadFleet() {
  try {
    const res = await fetch('/api/fleet/status');
    const data = await res.json();
    document.getElementById('fleet').innerHTML = `
      <div class="stat"><span class="stat-label">Specs</span><span class="stat-value">${data.total_specs || 0}</span></div>
      <div class="stat"><span class="stat-label">Instances</span><span class="stat-value">${data.total_instances || 0}</span></div>
      <div class="stat"><span class="stat-label">Healthy</span><span class="stat-value">${data.healthy_instances || 0}</span></div>
      <div class="stat"><span class="stat-label">Unhealthy</span><span class="stat-value">${data.unhealthy_instances || 0}</span></div>
    `;
  } catch (e) { document.getElementById('fleet').textContent = 'Error loading fleet'; }
}

async function loadSpecs() {
  try {
    const res = await fetch('/api/fleet/specs');
    const data = await res.json();
    const list = document.getElementById('specs');
    list.innerHTML = (data.specs || []).map(s =>
      `<li><span class="spec-name">${s.name}</span> v${s.version} [${s.autonomy}]<br><span class="spec-caps">${(s.capabilities || []).join(', ')}</span></li>`
    ).join('') || '<li>No specs registered</li>';
  } catch (e) { document.getElementById('specs').textContent = 'Error loading specs'; }
}

async function executeUal() {
  const input = document.getElementById('ual-input').value.trim();
  if (!input) return;
  const output = document.getElementById('ual-output');
  output.textContent = 'Executing...';
  try {
    const res = await fetch('/api/ual', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ command: input }),
    });
    const data = await res.json();
    output.textContent = JSON.stringify(data, null, 2);
    loadStatus();
    loadFleet();
  } catch (e) { output.textContent = 'Error: ' + e.message; }
}

document.getElementById('ual-input').addEventListener('keydown', function(e) {
  if (e.ctrlKey && e.key === 'Enter') executeUal();
});

loadStatus();
loadFleet();
loadSpecs();
setInterval(loadStatus, 10000);
setInterval(loadFleet, 15000);
</script>
</body>
</html>"##;
