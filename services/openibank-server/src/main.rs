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
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::{
        sse::{Event, Sse},
        Html, IntoResponse, Json as AxumJson,
    },
    routing::{get, post},
    Router,
};
use chrono::Utc;
use clap::Parser;
use futures::stream::Stream;
use openibank_agents::{AgentBrain, Service};
use openibank_core::{Amount, AssetId, ResonatorId};
use openibank_issuer::MintIntent;
use openibank_maple::{bridge::ResonatorAgentRole, MapleResonatorAgent};
use openibank_palm::{FinancialAgentType, FleetConfig, IBankFleetManager};
use openibank_receipts::{verify_receipt_json, Receipt, VerificationResult};
use openibank_state::{
    ReceiptRecord, SystemEvent, SystemState, TransactionRecord, TransactionStatus,
};
use openibank_ual::{
    compile_statements, parse_input, BankingCommand, ExecutionResult, ParsedInput,
};
use palm_registry::AgentRegistry;
use serde::Deserialize;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

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

fn configured_llm_provider() -> String {
    std::env::var("OPENIBANK_LLM_PROVIDER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "none".to_string())
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

    // Check LLM availability without creating network clients at startup
    let llm_provider = configured_llm_provider();
    let llm_available = llm_provider != "none";
    tracing::info!(
        "LLM: {}",
        if llm_available {
            format!("Configured ({})", llm_provider)
        } else {
            "Not available (deterministic mode)".to_string()
        }
    );

    let state = Arc::new(AppState { system, fleet });

    // Build router
    let app = Router::new()
        // Web dashboard
        .route("/", get(dashboard))
        .route("/palm", get(palm_dashboard))
        // System APIs
        .route("/api/status", get(api_status))
        .route("/api/health", get(api_health))
        .route("/api/events", get(api_events))
        // UAL command endpoint
        .route("/api/ual", post(api_ual_execute))
        // Demo endpoint
        .route("/api/demo/run", post(api_demo_run))
        // Fleet management
        .route("/api/fleet/status", get(api_fleet_status))
        .route("/api/fleet/specs", get(api_fleet_specs))
        .route("/api/fleet/deploy", post(api_fleet_deploy))
        // Agent management (delegate to system state)
        .route("/api/agents", get(api_list_agents))
        .route("/api/agents/{id}", get(api_agent_detail))
        .route("/api/transactions", get(api_transactions))
        .route("/api/ledger/accounts", get(api_ledger_accounts))
        .route("/api/receipts", get(api_receipts))
        .route("/api/receipts/verify", post(api_receipts_verify))
        .route("/api/receipts/export", get(api_receipts_export))
        .route("/api/receipts/{id}", get(api_receipt_by_id))
        // Issuer
        .route("/api/issuer/supply", get(api_supply))
        // Info
        .route("/api/info", get(api_info))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", args.host, args.port);
    tracing::info!(
        "OpeniBank Server running at http://{}:{}",
        args.host,
        args.port
    );
    tracing::info!("Dashboard:  http://localhost:{}", args.port);
    tracing::info!("API:        http://localhost:{}/api/status", args.port);
    tracing::info!("UAL:        POST http://localhost:{}/api/ual", args.port);
    tracing::info!(
        "Fleet:      http://localhost:{}/api/fleet/status",
        args.port
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn print_banner() {
    eprintln!(
        r#"
 ╔══════════════════════════════════════════════════════╗
 ║                                                      ║
 ║   ██████╗ ██████╗ ███████╗███╗   ██╗                 ║
 ║  ██╔═══██╗██╔══██╗██╔════╝████╗  ██║                 ║
 ║  ██║   ██║██████╔╝█████╗  ██╔██╗ ██║                 ║
 ║  ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║                 ║
 ║  ╚██████╔╝██║     ███████╗██║ ╚████║                 ║
 ║   ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝                 ║
 ║       ██╗██████╗  █████╗ ███╗   ██╗██╗  ██╗          ║
 ║       ██║██╔══██╗██╔══██╗████╗  ██║██║ ██╔╝          ║
 ║       ██║██████╔╝███████║██╔██╗ ██║█████╔╝           ║
 ║       ██║██╔══██╗██╔══██║██║╚██╗██║██╔═██╗           ║
 ║       ██║██████╔╝██║  ██║██║ ╚████║██║  ██╗          ║
 ║       ╚═╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝          ║
 ║                                                      ║
 ║  The Open AI Agent Banking Server                    ║
 ║  Powered by Maple Resonance Architecture             ║
 ║  https://www.openibank.com                           ║
 ║                                                      ║
 ╚══════════════════════════════════════════════════════╝
"#
    );
}

async fn register_default_specs(fleet: &IBankFleetManager) {
    let specs = [
        (
            "buyer-agent",
            "1.0.0",
            FinancialAgentType::Buyer,
            "Standard buyer agent with budget management and spend permits",
        ),
        (
            "seller-agent",
            "1.0.0",
            FinancialAgentType::Seller,
            "Standard seller agent with service publishing and invoice issuance",
        ),
        (
            "arbiter-agent",
            "1.0.0",
            FinancialAgentType::Arbiter,
            "Dispute resolution agent with escrow release/refund authority",
        ),
        (
            "issuer-agent",
            "1.0.0",
            FinancialAgentType::Issuer,
            "IUSD stablecoin issuer with reserve management",
        ),
        (
            "auditor-agent",
            "1.0.0",
            FinancialAgentType::Auditor,
            "Ledger audit and receipt verification agent",
        ),
        (
            "compliance-agent",
            "1.0.0",
            FinancialAgentType::Compliance,
            "Policy enforcement and risk assessment agent",
        ),
    ];

    for (name, version, agent_type, description) in specs {
        match fleet
            .register_agent_spec(name, version, agent_type, description)
            .await
        {
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

async fn palm_dashboard() -> Html<String> {
    Html(PALM_DASHBOARD_HTML.to_string())
}

// ============================================================================
// API Endpoints
// ============================================================================

async fn api_status(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let summary = state.system.status_summary().await;
    let llm_provider = configured_llm_provider();
    let llm_available = llm_provider != "none";
    let fleet_status = state.fleet.fleet_status().await.ok();

    AxumJson(serde_json::json!({
        "name": "OpeniBank Server",
        "version": env!("CARGO_PKG_VERSION"),
        "maple_runtime": summary.runtime,
        "llm_available": llm_available,
        "llm_provider": llm_provider,
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
        Ok(ParsedInput::Ual(statements)) => match compile_statements(&statements) {
            Ok(compiled) => {
                let artifacts: Vec<serde_json::Value> = compiled
                    .iter()
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
        },
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
                format!(
                    "System: {} agents, {} trades, ${:.2} volume",
                    summary.agent_count,
                    summary.trade_count,
                    summary.total_volume as f64 / 100.0
                ),
                serde_json::to_value(&summary).unwrap_or_default(),
            )
        }
        BankingCommand::ListAgents => {
            let registry = state.system.agents.read().await;
            let agents: Vec<serde_json::Value> = registry
                .agents
                .values()
                .map(|a| serde_json::to_value(a.to_api_info()).unwrap_or_default())
                .collect();
            ExecutionResult::ok_with_data(
                format!("{} agents registered", agents.len()),
                serde_json::json!({ "agents": agents }),
            )
        }
        BankingCommand::FleetStatus => match state.fleet.fleet_status().await {
            Ok(status) => ExecutionResult::ok_with_data(
                format!(
                    "{} specs, {} instances ({} healthy)",
                    status.total_specs, status.total_instances, status.healthy_instances
                ),
                serde_json::to_value(&status).unwrap_or_default(),
            ),
            Err(e) => ExecutionResult::err(format!("Fleet status error: {}", e)),
        },
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
            let _fin_type = match agent_type.as_str() {
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
                AxumJson(
                    serde_json::json!({ "error": format!("Agent spec '{}' not found", spec_name) }),
                )
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
    let agents: Vec<serde_json::Value> = registry
        .agents
        .values()
        .map(|a| serde_json::to_value(a.to_api_info()).unwrap_or_default())
        .collect();
    AxumJson(serde_json::json!({
        "agents": agents,
        "count": agents.len(),
    }))
}

async fn api_agent_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AxumJson<serde_json::Value> {
    let registry = state.system.agents.read().await;

    // Find agent by ID (could be full ID or partial match)
    let agent = registry.agents.iter().find(|(key, _)| {
        key.contains(&id) || *key == &id
    });

    match agent {
        Some((agent_id, agent)) => {
            let info = agent.to_api_info();
            let res_id = ResonatorId::from_string(agent_id);
            let iusd = AssetId::iusd();
            let balance = state.system.ledger.balance(&res_id, &iusd).await;

            // Get agent's transactions
            let transactions: Vec<_> = registry
                .transactions
                .iter()
                .filter(|tx| tx.buyer_id == *agent_id || tx.seller_id == *agent_id)
                .cloned()
                .collect();

            // Get agent's receipts
            let receipts: Vec<_> = registry
                .receipts
                .iter()
                .filter(|r| r.actor == *agent_id)
                .cloned()
                .collect();

            AxumJson(serde_json::json!({
                "success": true,
                "agent": {
                    "id": agent_id,
                    "info": info,
                    "balance": balance.0,
                    "balance_display": format!("${:.2}", balance.0 as f64 / 100.0),
                },
                "transactions": transactions,
                "transaction_count": transactions.len(),
                "receipts": receipts,
                "receipt_count": receipts.len(),
            }))
        }
        None => AxumJson(serde_json::json!({
            "success": false,
            "error": format!("Agent '{}' not found", id),
        })),
    }
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

async fn api_transactions(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let registry = state.system.agents.read().await;
    AxumJson(serde_json::json!({
        "transactions": registry.transactions,
        "count": registry.transactions.len(),
    }))
}

async fn api_ledger_accounts(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let account_ids = state.system.ledger.all_accounts().await;
    let iusd = AssetId::iusd();
    let mut accounts = Vec::new();
    for account_id in &account_ids {
        let balance = state.system.ledger.balance(account_id, &iusd).await;
        accounts.push(serde_json::json!({
            "account_id": account_id.0,
            "balance": balance.0,
            "balance_display": format!("${:.2}", balance.0 as f64 / 100.0),
        }));
    }

    AxumJson(serde_json::json!({
        "accounts": accounts,
        "count": account_ids.len(),
    }))
}

async fn api_receipts(State(state): State<Arc<AppState>>) -> AxumJson<serde_json::Value> {
    let registry = state.system.agents.read().await;
    AxumJson(serde_json::json!({
        "receipts": registry.receipts,
        "count": registry.receipts.len(),
    }))
}

async fn api_receipt_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AxumJson<serde_json::Value> {
    let registry = state.system.agents.read().await;
    if let Some(receipt) = registry.receipts.iter().find(|r| r.receipt_id == id) {
        return AxumJson(serde_json::json!({ "success": true, "receipt": receipt }));
    }
    AxumJson(serde_json::json!({ "success": false, "error": format!("Receipt {} not found", id) }))
}

#[derive(Deserialize)]
struct VerifyReceiptRequest {
    receipt_id: Option<String>,
    receipt: Option<serde_json::Value>,
}

async fn api_receipts_verify(
    State(state): State<Arc<AppState>>,
    AxumJson(req): AxumJson<VerifyReceiptRequest>,
) -> AxumJson<serde_json::Value> {
    let payload = if let Some(receipt) = req.receipt {
        match serde_json::to_string(&receipt) {
            Ok(v) => v,
            Err(e) => {
                return AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to serialize receipt payload: {}", e),
                }))
            }
        }
    } else if let Some(receipt_id) = req.receipt_id.clone() {
        let registry = state.system.agents.read().await;
        let Some(record) = registry
            .receipts
            .iter()
            .find(|r| r.receipt_id == receipt_id)
        else {
            return AxumJson(serde_json::json!({
                "success": false,
                "error": format!("Receipt {} not found", receipt_id),
            }));
        };
        match serde_json::to_string(&record.data) {
            Ok(v) => v,
            Err(e) => {
                return AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to serialize stored receipt: {}", e),
                }))
            }
        }
    } else {
        return AxumJson(serde_json::json!({
            "success": false,
            "error": "receipt_id or receipt payload is required",
        }));
    };

    let result: VerificationResult = verify_receipt_json(&payload);
    state.system.emit_event(SystemEvent::ReceiptVerified {
        receipt_id: result.receipt_id.clone(),
        valid: result.valid,
        errors: result.errors.clone(),
        timestamp: Utc::now(),
    });

    AxumJson(serde_json::json!({
        "success": true,
        "result": result,
    }))
}

async fn api_receipts_export(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let registry = state.system.agents.read().await;
    let mut rows = registry.receipts.clone();
    rows.sort_by_key(|r| r.timestamp);
    let mut body = rows
        .iter()
        .filter_map(|r| serde_json::to_string(&r.data).ok())
        .collect::<Vec<_>>()
        .join("\n");
    if !body.is_empty() {
        body.push('\n');
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"openibank-demo-receipts.jsonl\""),
    );
    (headers, body)
}

async fn api_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.system.subscribe();

    let initial_data = serde_json::json!({
        "type": "state_sync",
        "data": state.system.status_summary().await,
    });
    let initial = Event::default()
        .event("state_sync")
        .data(serde_json::to_string(&initial_data).unwrap_or_default());

    let stream = async_stream::stream! {
        yield Ok(initial);
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let event_name = match &event {
                        SystemEvent::AgentCreated { .. } => "agent_created",
                        SystemEvent::BalanceUpdated { .. } => "balance_updated",
                        SystemEvent::TradeStarted { .. } => "trade_started",
                        SystemEvent::TradeCompleted { .. } => "trade_completed",
                        SystemEvent::TradeFailed { .. } => "trade_failed",
                        SystemEvent::CommitmentDeclared { .. } => "commitment_declared",
                        SystemEvent::CommitmentApproved { .. } => "commitment_approved",
                        SystemEvent::CommitmentRejected { .. } => "commitment_rejected",
                        SystemEvent::TransferProposed { .. } => "transfer_proposed",
                        SystemEvent::TransferPosted { .. } => "transfer_posted",
                        SystemEvent::EscrowOpened { .. } => "escrow_opened",
                        SystemEvent::EscrowReleased { .. } => "escrow_released",
                        SystemEvent::ReceiptIssued { .. } => "receipt_issued",
                        SystemEvent::ReceiptVerified { .. } => "receipt_verified",
                        SystemEvent::Minted { .. } => "minted",
                        SystemEvent::Burned { .. } => "burned",
                        SystemEvent::MapleRuntimeEvent { .. } => "maple_runtime",
                        _ => "system_event",
                    };

                    yield Ok(
                        Event::default()
                            .event(event_name)
                            .data(serde_json::to_string(&event).unwrap_or_default())
                    );
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

fn receipt_timestamp(receipt: &Receipt) -> chrono::DateTime<Utc> {
    match receipt {
        Receipt::Commitment(r) => r.committed_at,
        Receipt::Issuer(r) => r.issued_at,
    }
}

fn receipt_type_label(receipt: &Receipt) -> &'static str {
    match receipt {
        Receipt::Commitment(_) => "commitment",
        Receipt::Issuer(_) => "issuer",
    }
}

fn receipt_actor_default(receipt: &Receipt) -> String {
    match receipt {
        Receipt::Commitment(r) => r.actor.0.clone(),
        Receipt::Issuer(_) => "issuer".to_string(),
    }
}

fn build_receipt_record(
    receipt: &Receipt,
    actor_override: Option<String>,
    description: String,
) -> ReceiptRecord {
    ReceiptRecord {
        receipt_id: receipt.id().to_string(),
        receipt_type: receipt_type_label(receipt).to_string(),
        actor: actor_override.unwrap_or_else(|| receipt_actor_default(receipt)),
        description,
        data: serde_json::to_value(receipt)
            .unwrap_or_else(|_| serde_json::json!({"error": "receipt_serialization_failed"})),
        timestamp: receipt_timestamp(receipt),
    }
}

async fn store_receipt_records(system: &SystemState, records: Vec<ReceiptRecord>) {
    if records.is_empty() {
        return;
    }

    {
        let mut registry = system.agents.write().await;
        registry.receipts.extend(records.clone());
    }

    for record in records {
        system.emit_event(SystemEvent::ReceiptIssued {
            receipt_id: record.receipt_id.clone(),
            receipt_type: record.receipt_type.clone(),
            actor: record.actor.clone(),
            description: record.description.clone(),
            timestamp: record.timestamp,
        });
    }
}

#[derive(Deserialize)]
struct DemoRunRequest {
    #[serde(default)]
    commit: bool,
}

async fn api_demo_run(
    State(state): State<Arc<AppState>>,
    AxumJson(req): AxumJson<DemoRunRequest>,
) -> AxumJson<serde_json::Value> {
    if !req.commit {
        let rejected_id = format!("commit_required_{}", Uuid::new_v4());
        state.system.emit_event(SystemEvent::CommitmentRejected {
            commitment_id: rejected_id.clone(),
            reason: "Demo requires explicit COMMIT=true".to_string(),
            timestamp: Utc::now(),
        });
        return AxumJson(serde_json::json!({
            "success": false,
            "error": "Explicit COMMIT required. Send {\"commit\": true}.",
            "commitment_id": rejected_id,
        }));
    }

    let suffix = Uuid::new_v4().to_string()[..8].to_string();
    let demo_id = format!("demo_{}", suffix);
    let bundle_id = format!("bundle_{}", Uuid::new_v4());
    let buyer_name = format!("DemoBuyer{}", suffix);
    let seller_name = format!("DemoSeller{}", suffix);
    let arbiter_name = format!("DemoArbiter{}", suffix);
    let buyer_id = format!("res_{}", buyer_name.to_lowercase());
    let seller_id = format!("res_{}", seller_name.to_lowercase());
    let arbiter_id = format!("res_{}", arbiter_name.to_lowercase());
    let service_name = "OpeniBank Instant Demo Service".to_string();
    let trade_amount = 25_00_u64;
    let funding = 100_00_u64;

    state.system.emit_event(SystemEvent::MapleRuntimeEvent {
        event_type: "demo_started".to_string(),
        description: format!("Running deterministic demo {}", demo_id),
        timestamp: Utc::now(),
    });

    // 1) Init reserve/mint path
    let mint_receipt = {
        let issuer = state.system.issuer.read().await;
        let mint = MintIntent::new(
            ResonatorId::from_string(&buyer_id),
            Amount::new(funding),
            "Demo bootstrap mint",
        );
        match issuer.mint(mint).await {
            Ok(receipt) => receipt,
            Err(e) => {
                return AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Mint failed: {}", e),
                }))
            }
        }
    };

    store_receipt_records(
        &state.system,
        vec![build_receipt_record(
            &Receipt::Issuer(mint_receipt.clone()),
            Some("issuer".to_string()),
            format!("Demo mint ${:.2} to {}", funding as f64 / 100.0, buyer_id),
        )],
    )
    .await;

    state.system.emit_event(SystemEvent::Minted {
        receipt_id: mint_receipt.receipt_id.clone(),
        account: buyer_id.clone(),
        amount: mint_receipt.amount.0,
        asset: mint_receipt.asset.0.clone(),
        total_supply: None,
        timestamp: mint_receipt.issued_at,
    });

    // 2) Create Buyer/Seller/Arbiter agents in deterministic mode
    let buyer_handle = state
        .system
        .runtime
        .register_agent(&buyer_name, ResonatorAgentRole::Buyer)
        .await
        .ok();
    let seller_handle = state
        .system
        .runtime
        .register_agent(&seller_name, ResonatorAgentRole::Seller)
        .await
        .ok();
    let arbiter_handle = state
        .system
        .runtime
        .register_agent(&arbiter_name, ResonatorAgentRole::Arbiter)
        .await
        .ok();

    let mut buyer = MapleResonatorAgent::new_buyer(
        &buyer_name,
        state.system.ledger.clone(),
        AgentBrain::deterministic(),
        buyer_handle,
    );
    if let Some(agent) = buyer.as_buyer_mut() {
        if let Err(e) = agent.setup(Amount::new(funding), Amount::new(funding / 2)) {
            return AxumJson(serde_json::json!({
                "success": false,
                "error": format!("Buyer setup failed: {}", e),
            }));
        }
    }

    let mut seller = MapleResonatorAgent::new_seller(
        &seller_name,
        state.system.ledger.clone(),
        AgentBrain::deterministic(),
        seller_handle,
    );
    if let Some(agent) = seller.as_seller_mut() {
        agent.publish_service(Service {
            name: service_name.clone(),
            description: "Scenario service for viral demo".to_string(),
            price: Amount::new(trade_amount),
            asset: AssetId::iusd(),
            delivery_conditions: vec!["instant_demo_delivery".to_string()],
        });
    }

    let arbiter = MapleResonatorAgent::new_arbiter(
        &arbiter_name,
        state.system.ledger.clone(),
        AgentBrain::deterministic(),
        arbiter_handle,
    );

    {
        let mut registry = state.system.agents.write().await;
        registry.agents.insert(buyer_id.clone(), buyer);
        registry.agents.insert(seller_id.clone(), seller);
        registry.agents.insert(arbiter_id.clone(), arbiter);
    }

    state.system.emit_event(SystemEvent::AgentCreated {
        agent_id: buyer_id.clone(),
        name: buyer_name.clone(),
        role: "Buyer".to_string(),
        has_resonator: true,
        timestamp: Utc::now(),
    });
    state.system.emit_event(SystemEvent::AgentCreated {
        agent_id: seller_id.clone(),
        name: seller_name.clone(),
        role: "Seller".to_string(),
        has_resonator: true,
        timestamp: Utc::now(),
    });
    state.system.emit_event(SystemEvent::AgentCreated {
        agent_id: arbiter_id.clone(),
        name: arbiter_name.clone(),
        role: "Arbiter".to_string(),
        has_resonator: true,
        timestamp: Utc::now(),
    });

    // 3) Explicit COMMIT gate (fail-closed already enforced above)
    let commitment_id = format!("commit_demo_{}", Uuid::new_v4());
    state.system.emit_event(SystemEvent::CommitmentDeclared {
        commitment_id: commitment_id.clone(),
        buyer_id: buyer_id.clone(),
        seller_id: seller_id.clone(),
        amount: trade_amount,
        service_name: service_name.clone(),
        timestamp: Utc::now(),
    });
    state.system.emit_event(SystemEvent::CommitmentApproved {
        commitment_id: commitment_id.clone(),
        decision: "Explicit COMMIT accepted".to_string(),
        timestamp: Utc::now(),
    });

    // 4) Open escrow trade and release
    let mut receipt_records = Vec::new();
    let invoice_id;
    let escrow_id;
    let amount;
    let escrow_receipt_id;
    let release_receipt_id;
    let buyer_balance;
    let seller_balance;

    {
        let mut registry = state.system.agents.write().await;
        if let Some(agent) = registry.agents.get_mut(&buyer_id) {
            agent.set_active_commitment(commitment_id.clone(), true);
        }
        if let Some(agent) = registry.agents.get_mut(&seller_id) {
            agent.set_active_commitment(commitment_id.clone(), true);
        }
        if let Some(agent) = registry.agents.get_mut(&arbiter_id) {
            agent.set_active_commitment(commitment_id.clone(), true);
        }

        let buyer_resonator = ResonatorId::from_string(&buyer_id);
        let issued_invoice = {
            let Some(seller_agent) = registry.agents.get_mut(&seller_id) else {
                return AxumJson(serde_json::json!({"success": false, "error": "Seller missing"}));
            };
            let Some(seller) = seller_agent.as_seller_mut() else {
                return AxumJson(
                    serde_json::json!({"success": false, "error": "Seller role mismatch"}),
                );
            };
            match seller.issue_invoice(buyer_resonator, &service_name).await {
                Ok(v) => v,
                Err(e) => {
                    return AxumJson(serde_json::json!({
                        "success": false,
                        "error": format!("Issue invoice failed: {}", e),
                    }))
                }
            }
        };

        invoice_id = issued_invoice.invoice_id.clone();

        {
            let Some(buyer_agent) = registry.agents.get_mut(&buyer_id) else {
                return AxumJson(serde_json::json!({"success": false, "error": "Buyer missing"}));
            };
            let Some(buyer) = buyer_agent.as_buyer_mut() else {
                return AxumJson(
                    serde_json::json!({"success": false, "error": "Buyer role mismatch"}),
                );
            };
            if let Err(e) = buyer.accept_invoice(issued_invoice) {
                return AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Accept invoice failed: {}", e),
                }));
            }
        }

        let (escrow, escrow_receipt) = {
            let Some(buyer_agent) = registry.agents.get_mut(&buyer_id) else {
                return AxumJson(serde_json::json!({"success": false, "error": "Buyer missing"}));
            };
            let Some(buyer) = buyer_agent.as_buyer_mut() else {
                return AxumJson(
                    serde_json::json!({"success": false, "error": "Buyer role mismatch"}),
                );
            };
            match buyer.pay_invoice_with_receipt(&invoice_id).await {
                Ok((_, escrow, receipt)) => (escrow, receipt),
                Err(e) => {
                    return AxumJson(serde_json::json!({
                        "success": false,
                        "error": format!("Escrow open failed: {}", e),
                    }))
                }
            }
        };

        escrow_id = escrow.escrow_id.clone();
        escrow_receipt_id = escrow_receipt.commitment_id.0.clone();
        state.system.emit_event(SystemEvent::EscrowOpened {
            escrow_id: escrow_id.0.clone(),
            payer: buyer_id.clone(),
            payee: seller_id.clone(),
            amount: escrow.amount.0,
            asset: escrow.asset.0.clone(),
            receipt_id: Some(escrow_receipt_id.clone()),
            timestamp: Utc::now(),
        });
        state.system.emit_event(SystemEvent::TransferProposed {
            transfer_id: escrow_receipt_id.clone(),
            from: buyer_id.clone(),
            to: seller_id.clone(),
            amount: escrow.amount.0,
            asset: escrow.asset.0.clone(),
            receipt_id: Some(escrow_receipt_id.clone()),
            timestamp: Utc::now(),
        });
        receipt_records.push(build_receipt_record(
            &Receipt::Commitment(escrow_receipt),
            Some(buyer_id.clone()),
            "Escrow opened".to_string(),
        ));

        {
            let Some(seller_agent) = registry.agents.get_mut(&seller_id) else {
                return AxumJson(serde_json::json!({"success": false, "error": "Seller missing"}));
            };
            let Some(seller) = seller_agent.as_seller_mut() else {
                return AxumJson(
                    serde_json::json!({"success": false, "error": "Seller role mismatch"}),
                );
            };
            if let Err(e) = seller.deliver_service(&invoice_id, "demo_delivery_proof".to_string()) {
                return AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Deliver service failed: {}", e),
                }));
            }
        }

        let release_receipt = {
            let Some(buyer_agent) = registry.agents.get_mut(&buyer_id) else {
                return AxumJson(serde_json::json!({"success": false, "error": "Buyer missing"}));
            };
            let Some(buyer) = buyer_agent.as_buyer_mut() else {
                return AxumJson(
                    serde_json::json!({"success": false, "error": "Buyer role mismatch"}),
                );
            };
            match buyer.confirm_delivery_with_receipt(&escrow_id) {
                Ok((release_amount, receipt)) => {
                    amount = release_amount;
                    receipt
                }
                Err(e) => {
                    return AxumJson(serde_json::json!({
                        "success": false,
                        "error": format!("Escrow release failed: {}", e),
                    }))
                }
            }
        };

        release_receipt_id = release_receipt.commitment_id.0.clone();
        receipt_records.push(build_receipt_record(
            &Receipt::Commitment(release_receipt.clone()),
            Some(buyer_id.clone()),
            "Escrow released".to_string(),
        ));

        {
            let Some(seller_agent) = registry.agents.get_mut(&seller_id) else {
                return AxumJson(serde_json::json!({"success": false, "error": "Seller missing"}));
            };
            let Some(seller) = seller_agent.as_seller_mut() else {
                return AxumJson(
                    serde_json::json!({"success": false, "error": "Seller role mismatch"}),
                );
            };
            if let Err(e) = seller.receive_payment(amount) {
                return AxumJson(serde_json::json!({
                    "success": false,
                    "error": format!("Receive payment failed: {}", e),
                }));
            }
        }

        if let Err(e) = state
            .system
            .ledger
            .transfer(
                &ResonatorId::from_string(&buyer_id),
                &ResonatorId::from_string(&seller_id),
                &AssetId::iusd(),
                amount,
                &release_receipt,
            )
            .await
        {
            return AxumJson(serde_json::json!({
                "success": false,
                "error": format!("Ledger transfer failed: {}", e),
            }));
        }

        state.system.emit_event(SystemEvent::EscrowReleased {
            escrow_id: escrow_id.0.clone(),
            payer: buyer_id.clone(),
            payee: seller_id.clone(),
            amount: amount.0,
            asset: AssetId::iusd().0,
            receipt_id: Some(release_receipt_id.clone()),
            timestamp: Utc::now(),
        });
        state.system.emit_event(SystemEvent::TransferPosted {
            transfer_id: release_receipt_id.clone(),
            from: buyer_id.clone(),
            to: seller_id.clone(),
            amount: amount.0,
            asset: AssetId::iusd().0,
            receipt_id: Some(release_receipt_id.clone()),
            timestamp: Utc::now(),
        });

        registry.transactions.push(TransactionRecord {
            tx_id: format!("tx_{}", Uuid::new_v4()),
            buyer_id: buyer_id.clone(),
            seller_id: seller_id.clone(),
            service_name: service_name.clone(),
            amount: amount.0,
            status: TransactionStatus::Completed,
            receipt_id: Some(release_receipt_id.clone()),
            timestamp: Utc::now(),
        });
        registry.trade_count += 1;
        registry.total_volume += amount.0;

        if let Some(agent) = registry.agents.get_mut(&buyer_id) {
            agent.clear_active_commitment();
            buyer_balance = agent.balance().map(|v| v.0).unwrap_or(0);
        } else {
            buyer_balance = 0;
        }
        if let Some(agent) = registry.agents.get_mut(&seller_id) {
            agent.clear_active_commitment();
            seller_balance = agent.balance().map(|v| v.0).unwrap_or(0);
        } else {
            seller_balance = 0;
        }
        if let Some(agent) = registry.agents.get_mut(&arbiter_id) {
            agent.clear_active_commitment();
        }
    }

    store_receipt_records(&state.system, receipt_records).await;

    state.system.emit_event(SystemEvent::TradeCompleted {
        trade_id: format!("trade_{}", Uuid::new_v4()),
        buyer_id: buyer_id.clone(),
        seller_id: seller_id.clone(),
        service_name: service_name.clone(),
        amount: amount.0,
        receipt_id: Some(release_receipt_id.clone()),
        timestamp: Utc::now(),
    });

    state.system.emit_event(SystemEvent::MapleRuntimeEvent {
        event_type: "demo_bundle_ready".to_string(),
        description: format!("Receipt bundle {} ready for export", bundle_id),
        timestamp: Utc::now(),
    });

    AxumJson(serde_json::json!({
        "success": true,
        "demo_id": demo_id,
        "scenario": "deterministic_escrow_commit",
        "llm_mode": "deterministic",
        "agents": {
            "buyer_id": buyer_id,
            "seller_id": seller_id,
            "arbiter_id": arbiter_id,
        },
        "commitment_ids": [
            commitment_id,
            escrow_receipt_id,
            release_receipt_id,
        ],
        "receipt_bundle_id": bundle_id,
        "share_export_url": "/api/receipts/export",
        "balances": {
            "buyer": buyer_balance,
            "seller": seller_balance,
        }
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
  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .table th, .table td {
    border-bottom: 1px solid #1a1a2a;
    padding: 0.45rem 0.35rem;
    text-align: left;
  }
  .table th {
    color: #8ba0c8;
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .mono { font-family: inherit; color: #86e1ff; }
  .timeline {
    max-height: 260px;
    overflow-y: auto;
    border: 1px solid #1a1a2a;
    border-radius: 6px;
    padding: 0.6rem;
    background: #0a0a12;
  }
  .timeline-item {
    border-bottom: 1px solid #1a1a2a;
    padding: 0.45rem 0;
    font-size: 0.8rem;
  }
  .timeline-item:last-child { border-bottom: none; }
  .pill {
    display: inline-block;
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    border: 1px solid #2a2a3a;
    color: #8ba0c8;
    font-size: 0.68rem;
    margin-right: 0.35rem;
  }
  .btn-soft {
    background: #1a2236;
    color: #86e1ff;
    border: 1px solid #2d3c5d;
    padding: 0.6rem 1rem;
    font-family: inherit;
    font-weight: bold;
    cursor: pointer;
    border-radius: 4px;
    margin-top: 0.8rem;
  }
  .btn-soft:hover { background: #22304d; }
  .btn-row { display: flex; gap: 0.6rem; flex-wrap: wrap; }
  .small-btn {
    background: #1a2236;
    color: #86e1ff;
    border: 1px solid #2d3c5d;
    border-radius: 4px;
    padding: 0.18rem 0.45rem;
    font-family: inherit;
    font-size: 0.72rem;
    cursor: pointer;
  }
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

  <div class="card">
    <h2>VIRAL DEMO</h2>
    <div class="stat"><span class="stat-label">Mode</span><span class="stat-value">Deterministic</span></div>
    <div class="stat"><span class="stat-label">Scenario</span><span class="stat-value">Escrow + COMMIT</span></div>
    <div class="btn-row">
      <button class="btn" onclick="runDemo()">RUN DEMO</button>
      <button class="btn-soft" onclick="exportReceipts()">SHARE / EXPORT</button>
    </div>
    <div class="output" id="demo-output">Press RUN DEMO to start a complete scenario.</div>
  </div>

  <div class="card">
    <h2>LIVE BALANCES</h2>
    <table class="table">
      <thead>
        <tr><th>Agent</th><th>Role</th><th>Wallet</th></tr>
      </thead>
      <tbody id="balances-table">
        <tr><td colspan="3">No agents yet.</td></tr>
      </tbody>
    </table>
  </div>

  <div class="card">
    <h2>COMMITMENTS TIMELINE</h2>
    <div class="timeline" id="timeline">
      <div class="timeline-item">Waiting for events...</div>
    </div>
  </div>

  <div class="card">
    <h2>RECEIPT VIEWER</h2>
    <table class="table">
      <thead>
        <tr><th>ID</th><th>Type</th><th>Actor</th><th>Verify</th></tr>
      </thead>
      <tbody id="receipts-table">
        <tr><td colspan="4">No receipts yet.</td></tr>
      </tbody>
    </table>
    <div class="output" id="verify-output">Verification output appears here.</div>
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
const demoEvents = [];
let eventSource = null;

function escapeHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function shortId(value, n = 14) {
  if (!value) return '-';
  const text = String(value);
  return text.length <= n ? text : `${text.slice(0, n)}...`;
}

function money(cents) {
  return `$${(Number(cents || 0) / 100).toFixed(2)}`;
}

async function loadStatus() {
  try {
    const res = await fetch('/api/status');
    const data = await res.json();
    document.getElementById('status').innerHTML = `
      <div class="stat"><span class="stat-label">Runtime</span><span class="stat-value">Active</span></div>
      <div class="stat"><span class="stat-label">Agents</span><span class="stat-value">${data.agents?.total || 0}</span></div>
      <div class="stat"><span class="stat-label">Trades</span><span class="stat-value">${data.trading?.trade_count || 0}</span></div>
      <div class="stat"><span class="stat-label">Volume</span><span class="stat-value">${data.trading?.total_volume_display || '$0.00'}</span></div>
      <div class="stat"><span class="stat-label">LLM</span><span class="stat-value">${data.llm_available ? 'Available (optional)' : 'Deterministic default'}</span></div>
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

async function loadBalances() {
  try {
    const res = await fetch('/api/agents');
    const data = await res.json();
    const rows = data.agents || [];
    const table = document.getElementById('balances-table');
    if (!rows.length) {
      table.innerHTML = '<tr><td colspan="3">No agents yet.</td></tr>';
      return;
    }

    table.innerHTML = rows.map((a) => `
      <tr>
        <td class="mono">${escapeHtml(shortId(a.id, 22))}</td>
        <td>${escapeHtml(String(a.role || '-'))}</td>
        <td>${escapeHtml(a.balance != null ? money(a.balance) : '-')}</td>
      </tr>
    `).join('');
  } catch (e) {
    document.getElementById('balances-table').innerHTML = '<tr><td colspan="3">Error loading balances.</td></tr>';
  }
}

async function loadReceipts() {
  try {
    const res = await fetch('/api/receipts');
    const data = await res.json();
    const receipts = data.receipts || [];
    const table = document.getElementById('receipts-table');

    if (!receipts.length) {
      table.innerHTML = '<tr><td colspan="4">No receipts yet.</td></tr>';
      return;
    }

    table.innerHTML = receipts.slice().reverse().slice(0, 40).map((r) => `
      <tr>
        <td class="mono">${escapeHtml(shortId(r.receipt_id, 16))}</td>
        <td>${escapeHtml(String(r.receipt_type || '-'))}</td>
        <td class="mono">${escapeHtml(shortId(r.actor || '-', 16))}</td>
        <td><button class="small-btn verify-btn" data-id="${escapeHtml(r.receipt_id)}">verify</button></td>
      </tr>
    `).join('');

    table.querySelectorAll('.verify-btn').forEach((btn) => {
      btn.addEventListener('click', () => verifyReceipt(btn.dataset.id));
    });
  } catch (e) {
    document.getElementById('receipts-table').innerHTML = '<tr><td colspan="4">Error loading receipts.</td></tr>';
  }
}

function summarizeEvent(name, payload) {
  switch (name) {
    case 'commitment_declared':
      return `Commitment declared ${shortId(payload.commitment_id, 12)} for ${money(payload.amount)}`;
    case 'commitment_approved':
      return `Commitment approved ${shortId(payload.commitment_id, 12)}`;
    case 'escrow_opened':
      return `Escrow opened ${shortId(payload.escrow_id, 12)} (${money(payload.amount)})`;
    case 'escrow_released':
      return `Escrow released ${shortId(payload.escrow_id, 12)} (${money(payload.amount)})`;
    case 'transfer_proposed':
      return `Transfer proposed ${money(payload.amount)} ${shortId(payload.from, 10)} -> ${shortId(payload.to, 10)}`;
    case 'transfer_posted':
      return `Transfer posted ${money(payload.amount)} ${shortId(payload.from, 10)} -> ${shortId(payload.to, 10)}`;
    case 'receipt_issued':
      return `Receipt issued ${shortId(payload.receipt_id, 12)} (${payload.receipt_type})`;
    case 'receipt_verified':
      return `Receipt ${shortId(payload.receipt_id, 12)} ${payload.valid ? 'valid' : 'invalid'}`;
    case 'minted':
      return `Minted ${money(payload.amount)} to ${shortId(payload.account, 12)}`;
    case 'trade_completed':
      return `Trade completed ${money(payload.amount)} (${payload.service_name || 'service'})`;
    default:
      return `${name}: ${payload.description || payload.reason || 'event'}`;
  }
}

function renderTimeline() {
  const root = document.getElementById('timeline');
  if (!demoEvents.length) {
    root.innerHTML = '<div class="timeline-item">Waiting for events...</div>';
    return;
  }

  root.innerHTML = demoEvents.slice(0, 60).map((entry) => `
    <div class="timeline-item">
      <span class="pill">${escapeHtml(entry.type)}</span>
      ${escapeHtml(entry.message)}
    </div>
  `).join('');
}

function connectEvents() {
  if (eventSource) eventSource.close();
  eventSource = new EventSource('/api/events');

  const names = [
    'state_sync',
    'commitment_declared',
    'commitment_approved',
    'commitment_rejected',
    'escrow_opened',
    'escrow_released',
    'transfer_proposed',
    'transfer_posted',
    'receipt_issued',
    'receipt_verified',
    'minted',
    'trade_completed',
    'maple_runtime',
  ];

  names.forEach((name) => {
    eventSource.addEventListener(name, (ev) => {
      let payload = {};
      try { payload = JSON.parse(ev.data || '{}'); } catch (_) { payload = {}; }
      if (name === 'state_sync') return;
      demoEvents.unshift({
        type: name,
        message: summarizeEvent(name, payload),
        timestamp: payload.timestamp || new Date().toISOString(),
      });
      if (demoEvents.length > 300) demoEvents.length = 300;
      renderTimeline();
      if (['trade_completed', 'minted', 'transfer_posted'].includes(name)) {
        loadStatus();
        loadBalances();
        loadReceipts();
      }
    });
  });
}

async function runDemo() {
  const output = document.getElementById('demo-output');
  output.textContent = 'Running deterministic demo...';
  try {
    const res = await fetch('/api/demo/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ commit: true }),
    });
    const data = await res.json();
    if (!res.ok || data.success === false) {
      output.textContent = JSON.stringify(data, null, 2);
      return;
    }
    output.textContent = [
      `Demo ID: ${data.demo_id}`,
      `Scenario: ${data.scenario}`,
      `Commitments: ${(data.commitment_ids || []).join(', ')}`,
      `Receipt bundle: ${data.receipt_bundle_id}`,
      `Buyer balance: ${money(data.balances?.buyer || 0)}`,
      `Seller balance: ${money(data.balances?.seller || 0)}`,
      `Export: ${data.share_export_url}`,
    ].join('\n');
    await Promise.all([loadStatus(), loadBalances(), loadReceipts()]);
  } catch (e) {
    output.textContent = `Error: ${e.message}`;
  }
}

function exportReceipts() {
  window.open('/api/receipts/export', '_blank');
}

async function verifyReceipt(receiptId) {
  const output = document.getElementById('verify-output');
  output.textContent = `Verifying ${receiptId}...`;
  try {
    const res = await fetch('/api/receipts/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ receipt_id: receiptId }),
    });
    const data = await res.json();
    output.textContent = JSON.stringify(data, null, 2);
  } catch (e) {
    output.textContent = `Error: ${e.message}`;
  }
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
loadBalances();
loadReceipts();
connectEvents();
setInterval(loadStatus, 10000);
setInterval(loadFleet, 15000);
setInterval(loadBalances, 5000);
setInterval(loadReceipts, 8000);
</script>
</body>
</html>"##;

// ============================================================================
// PALM Fleet Dashboard HTML
// ============================================================================

const PALM_DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>PALM Fleet Dashboard - OpeniBank</title>
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
    border-bottom: 2px solid #ff6b35;
    padding: 1.5rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .header h1 { color: #ff6b35; font-size: 1.8rem; letter-spacing: 3px; }
  .header .subtitle { color: #888; font-size: 0.85rem; }
  .header-right { display: flex; gap: 1rem; align-items: center; }
  .nav-link {
    color: #00d4ff;
    text-decoration: none;
    padding: 0.5rem 1rem;
    border: 1px solid #2a2a3a;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .nav-link:hover { background: #1a1a2e; }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
    gap: 1.5rem;
    padding: 2rem;
    max-width: 1600px;
    margin: 0 auto;
  }
  .card {
    background: #12121a;
    border: 1px solid #2a2a3a;
    border-radius: 8px;
    padding: 1.5rem;
  }
  .card.full-width { grid-column: 1 / -1; }
  .card h2 {
    color: #ff6b35;
    font-size: 1rem;
    letter-spacing: 2px;
    margin-bottom: 1rem;
    border-bottom: 1px solid #2a2a3a;
    padding-bottom: 0.5rem;
  }
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
  }
  .stat-box {
    background: #0a0a12;
    border: 1px solid #2a2a3a;
    border-radius: 6px;
    padding: 1rem;
    text-align: center;
  }
  .stat-box .value { font-size: 2rem; color: #00ff88; font-weight: bold; }
  .stat-box .label { color: #888; font-size: 0.75rem; margin-top: 0.3rem; }
  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .table th, .table td {
    border-bottom: 1px solid #1a1a2a;
    padding: 0.6rem 0.5rem;
    text-align: left;
  }
  .table th {
    color: #8ba0c8;
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .mono { font-family: inherit; color: #86e1ff; }
  .status-badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: bold;
  }
  .status-healthy { background: #0a3d2a; color: #00ff88; }
  .status-unhealthy { background: #3d1a1a; color: #ff4444; }
  .status-pending { background: #3d3a1a; color: #ffcc00; }
  .btn {
    background: #ff6b35;
    color: #0a0a0f;
    border: none;
    padding: 0.5rem 1rem;
    font-family: inherit;
    font-weight: bold;
    cursor: pointer;
    border-radius: 4px;
    margin-right: 0.5rem;
    font-size: 0.8rem;
  }
  .btn:hover { background: #ff8855; }
  .btn-secondary {
    background: #1a2236;
    color: #86e1ff;
    border: 1px solid #2d3c5d;
  }
  .btn-secondary:hover { background: #22304d; }
  .deploy-form {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
    flex-wrap: wrap;
  }
  .deploy-form select, .deploy-form input {
    background: #0a0a12;
    border: 1px solid #2a2a3a;
    color: #e0e0e0;
    padding: 0.5rem;
    border-radius: 4px;
    font-family: inherit;
  }
  .agent-card {
    background: #0a0a12;
    border: 1px solid #2a2a3a;
    border-radius: 6px;
    padding: 1rem;
    margin-bottom: 0.5rem;
    cursor: pointer;
    transition: border-color 0.2s;
  }
  .agent-card:hover { border-color: #ff6b35; }
  .agent-card .agent-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .agent-card .agent-name { color: #00d4ff; font-weight: bold; }
  .agent-card .agent-role { color: #888; font-size: 0.8rem; }
  .agent-card .agent-balance { color: #00ff88; font-size: 0.9rem; }
  .live-dot {
    display: inline-block;
    width: 8px; height: 8px;
    background: #00ff88;
    border-radius: 50%;
    margin-right: 0.5rem;
    animation: pulse 2s infinite;
  }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
  .footer {
    text-align: center;
    padding: 2rem;
    color: #444;
    font-size: 0.8rem;
    border-top: 1px solid #1a1a2a;
  }
  .footer a { color: #ff6b35; text-decoration: none; }
</style>
</head>
<body>
<div class="header">
  <div>
    <h1>PALM FLEET</h1>
    <div class="subtitle"><span class="live-dot"></span>Persistent Agent Lifecycle Manager</div>
  </div>
  <div class="header-right">
    <a href="/" class="nav-link">← Main Dashboard</a>
    <a href="/api/fleet/status" class="nav-link">API</a>
  </div>
</div>

<div class="grid">
  <div class="card full-width">
    <h2>FLEET OVERVIEW</h2>
    <div class="stat-grid" id="fleet-stats">
      <div class="stat-box"><div class="value" id="stat-specs">-</div><div class="label">Agent Specs</div></div>
      <div class="stat-box"><div class="value" id="stat-instances">-</div><div class="label">Total Instances</div></div>
      <div class="stat-box"><div class="value" id="stat-healthy">-</div><div class="label">Healthy</div></div>
      <div class="stat-box"><div class="value" id="stat-unhealthy">-</div><div class="label">Unhealthy</div></div>
    </div>
  </div>

  <div class="card">
    <h2>AGENT SPECIFICATIONS</h2>
    <table class="table" id="specs-table">
      <thead>
        <tr><th>Name</th><th>Version</th><th>Autonomy</th><th>Capabilities</th></tr>
      </thead>
      <tbody id="specs-body">
        <tr><td colspan="4">Loading...</td></tr>
      </tbody>
    </table>
  </div>

  <div class="card">
    <h2>DEPLOY AGENTS</h2>
    <p style="color: #888; font-size: 0.8rem; margin-bottom: 1rem;">Deploy new agent instances from registered specifications.</p>
    <div class="deploy-form">
      <select id="deploy-type">
        <option value="buyer">Buyer Agent</option>
        <option value="seller">Seller Agent</option>
        <option value="arbiter">Arbiter Agent</option>
        <option value="issuer">Issuer Agent</option>
        <option value="auditor">Auditor Agent</option>
        <option value="compliance">Compliance Agent</option>
      </select>
      <input type="number" id="deploy-count" value="1" min="1" max="10" style="width: 60px;">
      <button class="btn" onclick="deployAgents()">DEPLOY</button>
    </div>
    <div id="deploy-result" style="margin-top: 1rem; color: #888; font-size: 0.8rem;"></div>
  </div>

  <div class="card full-width">
    <h2>ACTIVE AGENTS</h2>
    <div id="agents-list">Loading agents...</div>
  </div>

  <div class="card full-width">
    <h2>FLEET HEALTH THRESHOLDS</h2>
    <table class="table">
      <thead>
        <tr><th>Metric</th><th>Warning</th><th>Critical</th><th>Current</th><th>Status</th></tr>
      </thead>
      <tbody id="health-body">
        <tr>
          <td>Memory Usage</td><td>70%</td><td>90%</td><td class="mono">45%</td>
          <td><span class="status-badge status-healthy">OK</span></td>
        </tr>
        <tr>
          <td>CPU Usage</td><td>80%</td><td>95%</td><td class="mono">23%</td>
          <td><span class="status-badge status-healthy">OK</span></td>
        </tr>
        <tr>
          <td>Active Connections</td><td>1000</td><td>5000</td><td class="mono">42</td>
          <td><span class="status-badge status-healthy">OK</span></td>
        </tr>
        <tr>
          <td>Error Rate</td><td>1%</td><td>5%</td><td class="mono">0.0%</td>
          <td><span class="status-badge status-healthy">OK</span></td>
        </tr>
      </tbody>
    </table>
  </div>
</div>

<div class="footer">
  <a href="https://www.openibank.com">openibank.com</a> &middot;
  PALM - Persistent Agent Lifecycle Manager &middot;
  Part of the Maple AI Framework
</div>

<script>
function escapeHtml(value) {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function money(cents) {
  return `$${(Number(cents || 0) / 100).toFixed(2)}`;
}

async function loadFleetStatus() {
  try {
    const res = await fetch('/api/fleet/status');
    const data = await res.json();
    document.getElementById('stat-specs').textContent = data.total_specs || 0;
    document.getElementById('stat-instances').textContent = data.total_instances || 0;
    document.getElementById('stat-healthy').textContent = data.healthy_instances || 0;
    document.getElementById('stat-unhealthy').textContent = data.unhealthy_instances || 0;
  } catch (e) {
    console.error('Failed to load fleet status:', e);
  }
}

async function loadSpecs() {
  try {
    const res = await fetch('/api/fleet/specs');
    const data = await res.json();
    const tbody = document.getElementById('specs-body');
    const specs = data.specs || [];

    if (!specs.length) {
      tbody.innerHTML = '<tr><td colspan="4">No specs registered</td></tr>';
      return;
    }

    tbody.innerHTML = specs.map(s => `
      <tr>
        <td class="mono">${escapeHtml(s.name)}</td>
        <td>${escapeHtml(s.version)}</td>
        <td>${escapeHtml(s.autonomy)}</td>
        <td style="font-size: 0.7rem; color: #666;">${escapeHtml((s.capabilities || []).join(', '))}</td>
      </tr>
    `).join('');
  } catch (e) {
    console.error('Failed to load specs:', e);
  }
}

async function loadAgents() {
  try {
    const res = await fetch('/api/agents');
    const data = await res.json();
    const container = document.getElementById('agents-list');
    const agents = data.agents || [];

    if (!agents.length) {
      container.innerHTML = '<p style="color: #666;">No active agents. Deploy some using the form above.</p>';
      return;
    }

    container.innerHTML = agents.map(a => `
      <div class="agent-card" onclick="viewAgent('${escapeHtml(a.id)}')">
        <div class="agent-header">
          <div>
            <span class="agent-name">${escapeHtml(a.name || a.id)}</span>
            <span class="agent-role">${escapeHtml(a.role || 'Unknown')}</span>
          </div>
          <div class="agent-balance">${a.balance != null ? money(a.balance) : '-'}</div>
        </div>
      </div>
    `).join('');
  } catch (e) {
    container.innerHTML = '<p style="color: #ff4444;">Error loading agents</p>';
  }
}

async function deployAgents() {
  const agentType = document.getElementById('deploy-type').value;
  const count = parseInt(document.getElementById('deploy-count').value) || 1;
  const resultEl = document.getElementById('deploy-result');

  resultEl.textContent = 'Deploying...';

  try {
    const res = await fetch('/api/fleet/deploy', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent_type: agentType, count }),
    });
    const data = await res.json();

    if (data.success) {
      resultEl.innerHTML = `<span style="color: #00ff88;">✓ Deployed ${data.instances_deployed} ${agentType} agent(s)</span>`;
      loadFleetStatus();
      loadAgents();
    } else {
      resultEl.innerHTML = `<span style="color: #ff4444;">✗ ${data.error || 'Deployment failed'}</span>`;
    }
  } catch (e) {
    resultEl.innerHTML = `<span style="color: #ff4444;">✗ Error: ${e.message}</span>`;
  }
}

function viewAgent(id) {
  window.location.href = `/api/agents/${id}`;
}

// Initial load
loadFleetStatus();
loadSpecs();
loadAgents();

// Auto-refresh
setInterval(loadFleetStatus, 10000);
setInterval(loadAgents, 5000);
</script>
</body>
</html>"##;
