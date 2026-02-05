//! OpeniBank Playground - Central Hub with Maple AI Framework
//!
//! The Playground is the central hub for the entire OpeniBank ecosystem.
//! It owns the SystemState (Maple runtime, Ledger, Issuer, Agents) and
//! serves both the web dashboard and the HTTP API that the CLI connects to.
//!
//! ## Architecture
//!
//! ```text
//! Playground (port 8080) ←── owns SystemState
//!     ├─ Web Dashboard (/)
//!     ├─ REST API (/api/*)
//!     └─ SSE Events (/api/events)
//!         ↑
//! CLI ────┘  (connects via HTTP)
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, Sse},
        Html, IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use openibank_agents::{AgentBrain, Service};
use openibank_core::{Amount, AssetId, ResonatorId};
use openibank_issuer::MintIntent;
use openibank_llm::LLMRouter;
use openibank_maple::{
    MapleResonatorAgent,
    bridge::{AgentPresenceState, ResonatorAgentRole},
    attention::AttentionManager,
    IdentityRef,
};
use openibank_state::{
    SystemState, SystemEvent,
    ReceiptRecord, TransactionRecord, TransactionStatus,
};
use openibank_receipts::{Receipt, VerificationResult, verify_receipt_json};
use serde::Deserialize;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting OpeniBank Playground with Maple AI Framework...");

    // Bootstrap SystemState (includes Maple runtime with iBank config)
    let state = match SystemState::new().await {
        Ok(s) => {
            tracing::info!("Maple iBank runtime bootstrapped successfully");
            Arc::new(s)
        }
        Err(e) => {
            tracing::error!("Failed to bootstrap system state: {}", e);
            std::process::exit(1);
        }
    };

    // Check LLM availability
    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;
    tracing::info!(
        "LLM Status: {}",
        if llm_available { "Available" } else { "Not available (deterministic mode)" }
    );

    // Log system start
    state.log_activity(
        openibank_state::activity::ActivityEntry::system_started()
    ).await;

    state.emit_event(SystemEvent::MapleRuntimeEvent {
        event_type: "started".to_string(),
        description: "Maple iBank runtime started with 8 canonical invariants".to_string(),
        timestamp: Utc::now(),
    });

    // Build router with all API endpoints
    let app = Router::new()
        // Web UI
        .route("/", get(index_page))
        // System status
        .route("/api/status", get(get_status))
        // Agent management
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{id}", get(get_agent))
        .route("/api/agents/{id}/activity", get(get_agent_activity))
        .route("/api/agents/{id}/kernel-trace", get(get_agent_kernel_trace))
        .route("/api/agents/{id}/fund", post(fund_agent))
        .route("/api/agents/{id}/presence", post(set_agent_presence))
        .route("/api/agents/buyer", post(create_buyer))
        .route("/api/agents/seller", post(create_seller))
        // Trading
        .route("/api/trade", post(execute_trade))
        .route("/api/trade/auto", post(start_auto_trading))
        .route("/api/simulate", post(simulate_marketplace))
        // Issuer / Supply
        .route("/api/issuer/supply", get(get_supply))
        .route("/api/issuer/receipts", get(get_issuer_receipts))
        // Ledger
        .route("/api/ledger/accounts", get(get_ledger_accounts))
        // Transactions & Receipts
        .route("/api/transactions", get(get_transactions))
        .route("/api/receipts", get(get_receipts))
        .route("/api/receipts/export", get(export_receipts))
        .route("/api/receipts/verify", post(verify_receipt))
        .route("/api/receipts/replay", post(replay_receipts))
        .route("/api/receipts/{id}", get(get_receipt))
        // Resonators (Maple)
        .route("/api/resonators", get(get_resonators))
        // System log
        .route("/api/system/log", get(get_system_log))
        // SSE stream
        .route("/api/events", get(event_stream))
        // UAL command endpoint
        .route("/api/ual", post(execute_ual))
        // Info
        .route("/api/info", get(get_info))
        // Reset
        .route("/api/reset", post(reset_playground))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("Playground running at http://localhost:8080");
    tracing::info!("API available at http://localhost:8080/api/status");
    tracing::info!("Dashboard at http://localhost:8080");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ============================================================================
// Web UI
// ============================================================================

async fn index_page() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

// ============================================================================
// Status
// ============================================================================

async fn get_status(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let summary = state.status_summary().await;
    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;

    Json(serde_json::json!({
        "name": "OpeniBank Playground",
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
            "total_supply_display": format!("${:.2}", summary.total_supply as f64 / 100.0),
            "remaining_supply_display": format!("${:.2}", summary.remaining_supply as f64 / 100.0)
        },
        "maple_accountability": summary.maple_accountability,
        "maple_couplings": summary.maple_couplings,
        "maple_commitments": summary.maple_commitments,
        "uptime_seconds": summary.uptime_seconds,
        "started_at": summary.started_at
    }))
}

// ============================================================================
// Agent Management
// ============================================================================

async fn list_agents(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let registry = state.agents.read().await;

    let agents: Vec<serde_json::Value> = registry.agents.values()
        .map(|a| serde_json::to_value(a.to_api_info()).unwrap_or_default())
        .collect();

    Json(serde_json::json!({
        "agents": agents,
        "count": agents.len(),
        "trade_count": registry.trade_count,
        "total_volume": registry.total_volume
    }))
}

async fn get_agent(
    State(state): State<Arc<SystemState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.agents.get(&id)
        .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

    Ok(Json(serde_json::to_value(agent.to_api_info()).unwrap_or_default()))
}

async fn get_agent_activity(
    State(state): State<Arc<SystemState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.agents.get(&id)
        .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

    let entries = agent.activity_log().entries();
    Ok(Json(serde_json::json!({
        "agent_id": id,
        "entries": entries,
        "count": entries.len()
    })))
}

#[derive(Deserialize)]
struct KernelTraceQuery {
    download: Option<bool>,
}

async fn get_agent_kernel_trace(
    State(state): State<Arc<SystemState>>,
    Path(id): Path<String>,
    Query(query): Query<KernelTraceQuery>,
) -> Result<axum::response::Response, AppError> {
    let registry = state.agents.read().await;
    let agent = registry.agents.get(&id)
        .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

    let trace = agent
        .kernel_trace()
        .ok_or_else(|| AppError::Internal("Kernel trace not available".to_string()))?;

    if query.download.unwrap_or(false) {
        let body = serde_json::to_string_pretty(trace)
            .map_err(|e| AppError::Internal(format!("Trace serialization failed: {}", e)))?;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let filename = format!("attachment; filename=\"kernel-trace-{}.json\"", id);
        headers.insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&filename).unwrap_or_else(|_| HeaderValue::from_static("attachment")),
        );
        return Ok((headers, body).into_response());
    }

    Ok(Json(trace).into_response())
}

#[derive(Deserialize)]
struct FundAgentRequest {
    amount: u64,
}

async fn fund_agent(
    State(state): State<Arc<SystemState>>,
    Path(id): Path<String>,
    Json(req): Json<FundAgentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.amount == 0 {
        return Err(AppError::Internal("Funding amount must be > 0".to_string()));
    }

    // Validate target agent exists and supports wallets.
    {
        let registry = state.agents.read().await;
        let Some(agent) = registry.agents.get(&id) else {
            return Err(AppError::NotFound(format!("Agent {} not found", id)));
        };
        if agent.role() == ResonatorAgentRole::Arbiter || agent.role() == ResonatorAgentRole::Issuer {
            return Err(AppError::Internal(
                "This agent role does not support wallet funding".to_string()
            ));
        }
    }

    // Mint against issuer supply controls first.
    let mint = MintIntent::new(
        ResonatorId::from_string(&id),
        Amount::new(req.amount),
        "Manual treasury top-up",
    );
    let mint_receipt = {
        let issuer = state.issuer.read().await;
        issuer
            .mint(mint)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    let mint_record = build_receipt_record(
        &Receipt::Issuer(mint_receipt.clone()),
        Some("issuer".to_string()),
        format!("Minted ${:.2} to {}", req.amount as f64 / 100.0, id),
    );
    store_receipt_records(&state, vec![mint_record]).await;

    state.emit_event(SystemEvent::Minted {
        receipt_id: mint_receipt.receipt_id.clone(),
        account: id.clone(),
        amount: mint_receipt.amount.0,
        asset: mint_receipt.asset.0.clone(),
        total_supply: None,
        timestamp: mint_receipt.issued_at,
    });

    // Then credit the local wallet view for this agent.
    let (agent_name, old_balance, new_balance, info) = {
        let mut registry = state.agents.write().await;
        let agent = registry
            .agents
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?;

        let old_balance = agent.balance().map(|a| a.0).unwrap_or(0);
        let iusd = AssetId::iusd();

        match agent.role() {
            ResonatorAgentRole::Buyer => {
                agent
                    .as_buyer_mut()
                    .ok_or_else(|| AppError::Internal("Buyer agent unavailable".to_string()))?
                    .wallet_mut()
                    .credit(&iusd, Amount::new(req.amount))
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            ResonatorAgentRole::Seller => {
                agent
                    .as_seller_mut()
                    .ok_or_else(|| AppError::Internal("Seller agent unavailable".to_string()))?
                    .wallet_mut()
                    .credit(&iusd, Amount::new(req.amount))
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            ResonatorAgentRole::Arbiter => {
                return Err(AppError::Internal(
                    "Arbiter agents do not support wallet funding".to_string()
                ));
            }
            ResonatorAgentRole::Issuer => {
                return Err(AppError::Internal(
                    "Issuer agents do not support wallet funding".to_string()
                ));
            }
        }

        let new_balance = agent.balance().map(|a| a.0).unwrap_or(0);
        agent.log_balance_change(format!(
            "Manual top-up ${:.2}",
            req.amount as f64 / 100.0
        ));

        (
            agent.name.clone(),
            old_balance,
            new_balance,
            agent.to_api_info(),
        )
    };

    state.emit_event(SystemEvent::BalanceUpdated {
        agent_id: id.clone(),
        agent_name: agent_name.clone(),
        old_balance,
        new_balance,
        reason: format!("Manual top-up ${:.2}", req.amount as f64 / 100.0),
        timestamp: Utc::now(),
    });

    state.log_activity(
        openibank_state::activity::ActivityEntry::iusd_minted(&id, req.amount)
    ).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "agent": info,
        "funded_amount": req.amount,
        "funded_display": format!("${:.2}", req.amount as f64 / 100.0)
    })))
}

#[derive(Deserialize)]
struct SetAgentPresenceRequest {
    state: String,
}

fn parse_presence_state(value: &str) -> Option<AgentPresenceState> {
    match value.trim().to_ascii_lowercase().as_str() {
        "idle" | "resume" | "active" => Some(AgentPresenceState::Idle),
        "trading" => Some(AgentPresenceState::Trading),
        "thinkingllm" | "thinking_llm" | "thinking" => Some(AgentPresenceState::ThinkingLLM),
        "waitingescrow" | "waiting_escrow" | "escrow" => Some(AgentPresenceState::WaitingEscrow),
        "resolvingdispute" | "resolving_dispute" | "dispute" => Some(AgentPresenceState::ResolvingDispute),
        "suspended" | "suspend" => Some(AgentPresenceState::Suspended),
        _ => None,
    }
}

async fn set_agent_presence(
    State(state): State<Arc<SystemState>>,
    Path(id): Path<String>,
    Json(req): Json<SetAgentPresenceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let new_state = parse_presence_state(&req.state)
        .ok_or_else(|| AppError::Internal(format!("Unsupported presence state '{}'", req.state)))?;

    // Remove the agent temporarily so we don't hold the write lock across await.
    let mut agent = {
        let mut registry = state.agents.write().await;
        registry
            .agents
            .remove(&id)
            .ok_or_else(|| AppError::NotFound(format!("Agent {} not found", id)))?
    };

    let old_state = agent.presence.clone();
    agent.set_presence(new_state.clone()).await;
    let info = agent.to_api_info();
    let agent_name = agent.name.clone();

    {
        let mut registry = state.agents.write().await;
        registry.agents.insert(id.clone(), agent);
    }

    state.emit_event(SystemEvent::AgentStateChanged {
        agent_id: id.clone(),
        old_state: format!("{:?}", old_state),
        new_state: format!("{:?}", new_state),
        timestamp: Utc::now(),
    });

    state.log_activity(
        openibank_state::activity::ActivityEntry::info(
            agent_name.clone(),
            openibank_state::activity::ActivityCategory::AgentLifecycle,
            format!("Presence updated: {:?} -> {:?}", old_state, new_state),
        ),
    ).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "agent": info
    })))
}

#[derive(Deserialize)]
struct CreateBuyerRequest {
    name: String,
    funding: Option<u64>,
}

async fn create_buyer(
    State(state): State<Arc<SystemState>>,
    Json(req): Json<CreateBuyerRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let funding = req.funding.unwrap_or(500_00); // Default $500
    let agent_id = format!("res_{}", req.name.to_lowercase().replace(' ', "_"));

    // Check if already exists
    {
        let registry = state.agents.read().await;
        if registry.agents.contains_key(&agent_id) {
            return Err(AppError::Internal(format!("Agent {} already exists", agent_id)));
        }
    }

    // Create brain with LLM if available
    let llm_router = LLMRouter::from_env();
    let brain = if llm_router.is_available().await {
        AgentBrain::with_llm(LLMRouter::from_env())
    } else {
        AgentBrain::deterministic()
    };

    // Register with Maple runtime
    let resonator_handle = match state.runtime.register_agent(&req.name, ResonatorAgentRole::Buyer).await {
        Ok(h) => Some(h),
        Err(e) => {
            tracing::warn!("Could not register Maple resonator for {}: {}", req.name, e);
            None
        }
    };

    // Create the MapleResonatorAgent
    let mut agent = MapleResonatorAgent::new_buyer(
        &req.name,
        state.ledger.clone(),
        brain,
        resonator_handle,
    );

    // Fund the buyer via issuer
    let resonator_id = ResonatorId::from_string(&agent_id);
    let mint = MintIntent::new(resonator_id, Amount::new(funding), "Playground funding");
    let mint_receipt = {
        let issuer = state.issuer.read().await;
        issuer.mint(mint).await
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    let mint_record = build_receipt_record(
        &Receipt::Issuer(mint_receipt.clone()),
        Some("issuer".to_string()),
        format!("Minted ${:.2} to {}", funding as f64 / 100.0, agent_id),
    );
    store_receipt_records(&state, vec![mint_record]).await;

    state.emit_event(SystemEvent::Minted {
        receipt_id: mint_receipt.receipt_id.clone(),
        account: agent_id.clone(),
        amount: mint_receipt.amount.0,
        asset: mint_receipt.asset.0.clone(),
        total_supply: None,
        timestamp: mint_receipt.issued_at,
    });

    // Setup buyer wallet
    agent.as_buyer_mut().unwrap()
        .setup(Amount::new(funding), Amount::new(funding / 2))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    agent.log_balance_change(format!("Funded ${:.2}", funding as f64 / 100.0));

    // Register identity with AAS and grant capabilities
    let aas_info = {
        let role = ResonatorAgentRole::Buyer;
        let registered = state.accountability
            .register_agent_identity(&req.name, &role);
        match registered {
            Ok(registered_agent) => {
                let agent_aas_id = registered_agent.agent_id.clone();
                // Grant role-based capabilities
                match state.accountability.grant_role_capabilities(&agent_aas_id, &role) {
                    Ok(grants) => {
                        for grant in &grants {
                            state.emit_event(SystemEvent::CapabilityGranted {
                                agent_id: agent_id.clone(),
                                agent_name: req.name.clone(),
                                capability: format!("{:?}", grant.capability.scope.operations),
                                domain: "Finance".to_string(),
                                timestamp: Utc::now(),
                            });
                        }
                        Some((agent_aas_id, grants.len()))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to grant capabilities for {}: {}", req.name, e);
                        Some((agent_aas_id, 0))
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to register AAS identity for {}: {}", req.name, e);
                None
            }
        }
    };

    let info = agent.to_api_info();

    // Register in state
    {
        let mut registry = state.agents.write().await;
        registry.agents.insert(agent_id.clone(), agent);
    }

    // Emit events
    state.emit_event(SystemEvent::AgentCreated {
        agent_id: agent_id.clone(),
        name: req.name.clone(),
        role: "Buyer".to_string(),
        has_resonator: info.has_resonator,
        timestamp: Utc::now(),
    });

    if let Some((_, cap_count)) = &aas_info {
        state.emit_event(SystemEvent::MapleRuntimeEvent {
            event_type: "aas_registered".to_string(),
            description: format!("{} registered with AAS ({} capabilities granted)", req.name, cap_count),
            timestamp: Utc::now(),
        });
    }

    state.log_activity(
        openibank_state::activity::ActivityEntry::agent_created(&req.name, "Buyer")
    ).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "agent": info
    })))
}

#[derive(Deserialize)]
struct CreateSellerRequest {
    name: String,
    service_name: String,
    price: u64,
}

async fn create_seller(
    State(state): State<Arc<SystemState>>,
    Json(req): Json<CreateSellerRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let agent_id = format!("res_{}", req.name.to_lowercase().replace(' ', "_"));

    // Check if already exists
    {
        let registry = state.agents.read().await;
        if registry.agents.contains_key(&agent_id) {
            return Err(AppError::Internal(format!("Agent {} already exists", agent_id)));
        }
    }

    let brain = AgentBrain::deterministic();

    // Register with Maple runtime
    let resonator_handle = match state.runtime.register_agent(&req.name, ResonatorAgentRole::Seller).await {
        Ok(h) => Some(h),
        Err(e) => {
            tracing::warn!("Could not register Maple resonator for {}: {}", req.name, e);
            None
        }
    };

    let mut agent = MapleResonatorAgent::new_seller(
        &req.name,
        state.ledger.clone(),
        brain,
        resonator_handle,
    );

    // Publish service
    let service = Service {
        name: req.service_name.clone(),
        description: format!("AI service: {}", req.service_name),
        price: Amount::new(req.price),
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Service completion".to_string()],
    };

    agent.as_seller_mut().unwrap().publish_service(service);

    // Register identity with AAS and grant capabilities
    {
        let role = ResonatorAgentRole::Seller;
        match state.accountability.register_agent_identity(&req.name, &role) {
            Ok(registered_agent) => {
                let agent_aas_id = registered_agent.agent_id.clone();
                match state.accountability.grant_role_capabilities(&agent_aas_id, &role) {
                    Ok(grants) => {
                        for grant in &grants {
                            state.emit_event(SystemEvent::CapabilityGranted {
                                agent_id: agent_id.clone(),
                                agent_name: req.name.clone(),
                                capability: format!("{:?}", grant.capability.scope.operations),
                                domain: "Finance".to_string(),
                                timestamp: Utc::now(),
                            });
                        }
                    }
                    Err(e) => tracing::warn!("Failed to grant capabilities for {}: {}", req.name, e),
                }
            }
            Err(e) => tracing::warn!("Failed to register AAS identity for {}: {}", req.name, e),
        }
    }

    let info = agent.to_api_info();

    // Register in state
    {
        let mut registry = state.agents.write().await;
        registry.agents.insert(agent_id.clone(), agent);
    }

    // Emit events
    state.emit_event(SystemEvent::AgentCreated {
        agent_id: agent_id.clone(),
        name: req.name.clone(),
        role: "Seller".to_string(),
        has_resonator: info.has_resonator,
        timestamp: Utc::now(),
    });

    state.log_activity(
        openibank_state::activity::ActivityEntry::agent_created(&req.name, "Seller")
    ).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "agent": info
    })))
}

// ============================================================================
// Trading
// ============================================================================

#[derive(Deserialize)]
struct TradeRequest {
    buyer_id: String,
    seller_id: String,
}

async fn execute_trade(
    State(state): State<Arc<SystemState>>,
    Json(req): Json<TradeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    execute_trade_internal(&state, &req.buyer_id, &req.seller_id).await
}

async fn execute_trade_internal(
    state: &Arc<SystemState>,
    buyer_id: &str,
    seller_id: &str,
) -> Result<Json<serde_json::Value>, AppError> {
    // ================================================================
    // Phase 1: Validate agents and get service info
    // ================================================================
    let (service_name, service_price, buyer_name, seller_name, _buyer_has_handle) = {
        let registry = state.agents.read().await;

        if !registry.agents.contains_key(buyer_id) {
            return Err(AppError::NotFound(format!("Buyer {} not found", buyer_id)));
        }
        if !registry.agents.contains_key(seller_id) {
            return Err(AppError::NotFound(format!("Seller {} not found", seller_id)));
        }

        let seller = registry.agents.get(seller_id).unwrap();
        let services = seller.services();
        if services.is_empty() {
            return Err(AppError::Internal("Seller has no services".to_string()));
        }

        let buyer = registry.agents.get(buyer_id).unwrap();
        (
            services[0].name.clone(),
            services[0].price.0,
            buyer.name.clone(),
            seller.name.clone(),
            buyer.resonator_handle.is_some(),
        )
    };

    let trade_id = format!("trade_{}", uuid::Uuid::new_v4());

    // ================================================================
    // Phase 2: Maple Attention Budget Check
    // ================================================================
    {
        let registry = state.agents.read().await;
        let buyer = registry.agents.get(buyer_id).unwrap();
        if let Some(ref handle) = buyer.resonator_handle {
            let can_trade = AttentionManager::can_trade(handle).await;
            if !can_trade {
                state.emit_event(SystemEvent::AttentionExhausted {
                    agent_id: buyer_id.to_string(),
                    agent_name: buyer_name.clone(),
                    timestamp: Utc::now(),
                });
                state.emit_event(SystemEvent::TradeFailed {
                    trade_id: trade_id.clone(),
                    buyer_id: buyer_id.to_string(),
                    seller_id: seller_id.to_string(),
                    reason: "Buyer attention budget exhausted".to_string(),
                    timestamp: Utc::now(),
                });
                return Err(AppError::Internal("Buyer attention budget exhausted".to_string()));
            }

            // Get attention status for event
            if let Some(status) = AttentionManager::get_status(handle, &buyer_name).await {
                state.emit_event(SystemEvent::AttentionAllocated {
                    agent_id: buyer_id.to_string(),
                    agent_name: buyer_name.clone(),
                    amount: 50,
                    remaining: status.available,
                    timestamp: Utc::now(),
                });
            }
        }
    }

    // ================================================================
    // Phase 3: Maple Coupling — Establish buyer↔seller connection
    // ================================================================
    let coupling_id = {
        let registry = state.agents.read().await;
        let buyer = registry.agents.get(buyer_id).unwrap();
        if let Some(ref buyer_handle) = buyer.resonator_handle {
            let seller_agent = registry.agents.get(seller_id).unwrap();
            if let Some(ref seller_handle) = seller_agent.resonator_handle {
                match state.coupling_manager
                    .establish_trade_coupling(buyer_handle, seller_handle.id, Some(trade_id.clone()))
                    .await
                {
                    Ok(coupling_handle) => {
                        let cid = coupling_handle.id.to_string();
                        state.emit_event(SystemEvent::CouplingEstablished {
                            coupling_id: cid.clone(),
                            buyer_id: buyer_id.to_string(),
                            seller_id: seller_id.to_string(),
                            strength: 0.2,
                            timestamp: Utc::now(),
                        });
                        Some(cid)
                    }
                    Err(e) => {
                        tracing::warn!("Could not establish trade coupling: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    // ================================================================
    // Phase 4: Maple Commitment — Create and submit RcfCommitment
    // ================================================================
    let commitment_id = {
        let buyer_identity = IdentityRef::new(&buyer_name);
        match state.commitment_manager.create_trade_commitment(
            &buyer_identity,
            &buyer_name,
            &seller_name,
            service_price,
            &service_name,
        ) {
            Ok(commitment) => {
                let cid = commitment.commitment_id.0.clone();

                state.emit_event(SystemEvent::CommitmentSubmitted {
                    commitment_id: cid.clone(),
                    buyer_name: buyer_name.clone(),
                    seller_name: seller_name.clone(),
                    amount: service_price,
                    service_name: service_name.clone(),
                    timestamp: Utc::now(),
                });
                state.emit_event(SystemEvent::CommitmentDeclared {
                    commitment_id: cid.clone(),
                    buyer_id: buyer_id.to_string(),
                    seller_id: seller_id.to_string(),
                    amount: service_price,
                    service_name: service_name.clone(),
                    timestamp: Utc::now(),
                });

                // Submit to AAS pipeline
                match state.commitment_manager.submit_and_track(
                    &state.accountability,
                    commitment,
                ).await {
                    Ok(decision) => {
                        if decision.decision.allows_execution() {
                            state.emit_event(SystemEvent::CommitmentApproved {
                                commitment_id: cid.clone(),
                                decision: format!("{:?}", decision.decision),
                                timestamp: Utc::now(),
                            });
                        } else {
                            // Even if AAS requires human review, we proceed
                            // (in production, this would wait for approval)
                            state.emit_event(SystemEvent::MapleRuntimeEvent {
                                event_type: "commitment_pending_review".to_string(),
                                description: format!(
                                    "Commitment {} pending review (Finance domain default policy). Auto-proceeding in playground.",
                                    &cid[..8.min(cid.len())]
                                ),
                                timestamp: Utc::now(),
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!("AAS submission warning (proceeding): {}", e);
                    }
                }

                // Record execution started
                let _ = state.commitment_manager.record_execution_started(
                    &state.accountability,
                    &cid,
                ).await;

                Some(cid)
            }
            Err(e) => {
                tracing::warn!("Could not create trade commitment: {}", e);
                None
            }
        }
    };

    // ================================================================
    // Phase 5: Execute trade (existing flow: invoice → escrow → deliver → release)
    // ================================================================
    state.emit_event(SystemEvent::TradeStarted {
        trade_id: trade_id.clone(),
        buyer_id: buyer_id.to_string(),
        seller_id: seller_id.to_string(),
        service_name: service_name.clone(),
        amount: service_price,
        timestamp: Utc::now(),
    });

    let mut registry = state.agents.write().await;
    if let Some(ref cid) = commitment_id {
        if let Some(buyer) = registry.agents.get_mut(buyer_id) {
            buyer.set_active_commitment(cid.clone(), true);
        }
        if let Some(seller) = registry.agents.get_mut(seller_id) {
            seller.set_active_commitment(cid.clone(), true);
        }
    }

    // Get offer from seller
    let offer = {
        let seller = registry.agents.get(seller_id).unwrap();
        seller.as_seller().unwrap().get_offer(&service_name)
    };

    let offer = match offer {
        Some(o) => o,
        None => {
            // Record failure in Maple
            if let Some(ref cid) = commitment_id {
                let _ = state.commitment_manager.record_outcome(
                    &state.accountability, cid, false, "No offer available",
                ).await;
            }
            state.emit_event(SystemEvent::TradeFailed {
                trade_id: trade_id.clone(),
                buyer_id: buyer_id.to_string(),
                seller_id: seller_id.to_string(),
                reason: "No offer available".to_string(),
                timestamp: Utc::now(),
            });
            return Err(AppError::Internal("No offer available".to_string()));
        }
    };

    // Buyer evaluates offer
    let can_afford = {
        let buyer = registry.agents.get(buyer_id).unwrap();
        buyer.as_buyer().unwrap().evaluate_offer(&offer).await
    };

    if !can_afford {
        if let Some(ref cid) = commitment_id {
        let _ = state.commitment_manager.record_outcome(
            &state.accountability, cid, false, "Buyer cannot afford",
        ).await;
        }
        state.emit_event(SystemEvent::TradeFailed {
            trade_id: trade_id.clone(),
            buyer_id: buyer_id.to_string(),
            seller_id: seller_id.to_string(),
            reason: "Cannot afford or declined".to_string(),
            timestamp: Utc::now(),
        });
        return Err(AppError::Internal("Buyer cannot afford".to_string()));
    }

    // Get buyer resonator ID for invoice
    let buyer_resonator_id = registry.agents.get(buyer_id).unwrap().id().clone();

    // Issue invoice from seller
    let invoice = {
        let seller = registry.agents.get_mut(seller_id).unwrap();
        seller.as_seller_mut().unwrap()
            .issue_invoice(buyer_resonator_id, &service_name)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    let invoice_id = invoice.invoice_id.clone();

    // Buyer accepts and pays
    {
        let buyer = registry.agents.get_mut(buyer_id).unwrap();
        let buyer_agent = buyer.as_buyer_mut().unwrap();
        buyer_agent.accept_invoice(invoice)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let mut receipt_records: Vec<ReceiptRecord> = Vec::new();

    let (escrow, escrow_receipt) = {
        let buyer = registry.agents.get_mut(buyer_id).unwrap();
        let buyer_agent = buyer.as_buyer_mut().unwrap();
        let (_permit, escrow, receipt) = buyer_agent
            .pay_invoice_with_receipt(&invoice_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        (escrow, receipt)
    };

    let escrow_id = escrow.escrow_id.clone();
    let escrow_receipt_id = escrow_receipt.commitment_id.0.clone();
    let escrow_receipt_record = build_receipt_record(
        &Receipt::Commitment(escrow_receipt.clone()),
        Some(buyer_id.to_string()),
        format!("Escrow lock for {}", service_name),
    );
    receipt_records.push(escrow_receipt_record);

    state.emit_event(SystemEvent::EscrowOpened {
        escrow_id: escrow_id.0.clone(),
        payer: buyer_id.to_string(),
        payee: seller_id.to_string(),
        amount: escrow.amount.0,
        asset: escrow.asset.0.clone(),
        receipt_id: Some(escrow_receipt_id.clone()),
        timestamp: Utc::now(),
    });
    state.emit_event(SystemEvent::TransferProposed {
        transfer_id: escrow_receipt_id.clone(),
        from: buyer_id.to_string(),
        to: seller_id.to_string(),
        amount: escrow.amount.0,
        asset: escrow.asset.0.clone(),
        receipt_id: Some(escrow_receipt_id.clone()),
        timestamp: Utc::now(),
    });

    // Seller delivers
    {
        let seller = registry.agents.get_mut(seller_id).unwrap();
        seller.as_seller_mut().unwrap()
            .deliver_service(&invoice_id, "Service delivered successfully".to_string())
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    // Buyer confirms delivery
    let (amount, release_receipt) = {
        let buyer = registry.agents.get_mut(buyer_id).unwrap();
        buyer.as_buyer_mut().unwrap()
            .confirm_delivery_with_receipt(&escrow_id)
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    let release_receipt_id = release_receipt.commitment_id.0.clone();
    let release_receipt_record = build_receipt_record(
        &Receipt::Commitment(release_receipt.clone()),
        Some(buyer_id.to_string()),
        format!("Escrow release for {}", service_name),
    );
    receipt_records.push(release_receipt_record);

    // Seller receives payment
    {
        let seller = registry.agents.get_mut(seller_id).unwrap();
        seller.as_seller_mut().unwrap()
            .receive_payment(amount)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let (debit_entry, credit_entry) = state.ledger
        .transfer(
            &ResonatorId::from_string(buyer_id),
            &ResonatorId::from_string(seller_id),
            &AssetId::iusd(),
            amount,
            &release_receipt,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    state.emit_event(SystemEvent::LedgerEntry {
        entry_id: debit_entry.0.clone(),
        from: buyer_id.to_string(),
        to: seller_id.to_string(),
        amount: amount.0,
        memo: Some(format!("credit_entry={}", credit_entry.0)),
        timestamp: Utc::now(),
    });

    state.emit_event(SystemEvent::EscrowReleased {
        escrow_id: escrow_id.0.clone(),
        payer: buyer_id.to_string(),
        payee: seller_id.to_string(),
        amount: amount.0,
        asset: AssetId::iusd().0,
        receipt_id: Some(release_receipt_id.clone()),
        timestamp: Utc::now(),
    });
    state.emit_event(SystemEvent::TransferPosted {
        transfer_id: release_receipt_id.clone(),
        from: buyer_id.to_string(),
        to: seller_id.to_string(),
        amount: amount.0,
        asset: AssetId::iusd().0,
        receipt_id: Some(release_receipt_id.clone()),
        timestamp: Utc::now(),
    });

    // ================================================================
    // Phase 6: Record outcome in Maple and update couplings
    // ================================================================

    // Record successful outcome in AAS
    if let Some(ref cid) = commitment_id {
        let _ = state.commitment_manager.record_outcome(
            &state.accountability,
            cid,
            true,
            &format!("Trade completed: {} bought '{}' from {} for ${:.2}",
                buyer_name, service_name, seller_name, amount.0 as f64 / 100.0),
        ).await;
        state.emit_event(SystemEvent::CommitmentOutcomeRecorded {
            commitment_id: cid.clone(),
            success: true,
            details: format!("Trade completed for ${:.2}", amount.0 as f64 / 100.0),
            timestamp: Utc::now(),
        });
    }

    // Strengthen coupling on success, then decouple
    if let Some(ref cid) = coupling_id {
        state.emit_event(SystemEvent::CouplingStrengthened {
            coupling_id: cid.clone(),
            old_strength: 0.2,
            new_strength: 0.3,
            timestamp: Utc::now(),
        });
        state.emit_event(SystemEvent::Decoupled {
            coupling_id: cid.clone(),
            reason: "Trade completed successfully".to_string(),
            timestamp: Utc::now(),
        });
    }

    // ================================================================
    // Phase 7: Log trades and update registry
    // ================================================================

    // Log trades on agents
    {
        let buyer = registry.agents.get_mut(buyer_id).unwrap();
        buyer.log_trade(
            format!("Bought '{}' for ${:.2}", service_name, amount.0 as f64 / 100.0),
            None,
        );
        buyer.log_balance_change(format!("Balance: ${:.2}", buyer.balance().unwrap_or(Amount::zero()).0 as f64 / 100.0));
    }
    {
        let seller = registry.agents.get_mut(seller_id).unwrap();
        seller.log_trade(
            format!("Sold '{}' for ${:.2}", service_name, amount.0 as f64 / 100.0),
            None,
        );
        seller.log_balance_change(format!("Balance: ${:.2}", seller.balance().unwrap_or(Amount::zero()).0 as f64 / 100.0));
    }

    // Clear active commitment context after trade completion
    if let Some(buyer) = registry.agents.get_mut(buyer_id) {
        buyer.clear_active_commitment();
    }
    if let Some(seller) = registry.agents.get_mut(seller_id) {
        seller.clear_active_commitment();
    }

    // Update registry stats
    registry.trade_count += 1;
    registry.total_volume += amount.0;

    // Record transaction
    registry.transactions.push(TransactionRecord {
        tx_id: trade_id.clone(),
        buyer_id: buyer_id.to_string(),
        seller_id: seller_id.to_string(),
        service_name: service_name.clone(),
        amount: amount.0,
        status: TransactionStatus::Completed,
        receipt_id: Some(release_receipt_id.clone()),
        timestamp: Utc::now(),
    });

    let buyer_balance = registry.agents.get(buyer_id).unwrap().balance().unwrap_or(Amount::zero()).0;
    let seller_balance = registry.agents.get(seller_id).unwrap().balance().unwrap_or(Amount::zero()).0;

    // Emit events
    state.emit_event(SystemEvent::TradeCompleted {
        trade_id: trade_id.clone(),
        buyer_id: buyer_id.to_string(),
        seller_id: seller_id.to_string(),
        service_name: service_name.clone(),
        amount: amount.0,
        receipt_id: Some(release_receipt_id.clone()),
        timestamp: Utc::now(),
    });

    state.log_activity(
        openibank_state::activity::ActivityEntry::trade_completed(buyer_id, seller_id, &service_name, amount.0)
    ).await;

    drop(registry);
    store_receipt_records(&state, receipt_records).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "trade_id": trade_id,
        "amount": amount.0,
        "buyer_balance": buyer_balance,
        "seller_balance": seller_balance,
        "maple": {
            "commitment_id": commitment_id,
            "coupling_id": coupling_id,
        }
    })))
}

#[derive(Deserialize)]
struct AutoTradeRequest {
    rounds: Option<u32>,
    delay_ms: Option<u64>,
}

async fn start_auto_trading(
    State(state): State<Arc<SystemState>>,
    Json(req): Json<AutoTradeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rounds = req.rounds.unwrap_or(10);
    let delay_ms = req.delay_ms.unwrap_or(1000);

    let state_clone = state.clone();
    tokio::spawn(async move {
        for round in 0..rounds {
            let (buyer_id, seller_id) = {
                let registry = state_clone.agents.read().await;
                let buyers: Vec<String> = registry.agents.iter()
                    .filter(|(_, a)| a.role() == ResonatorAgentRole::Buyer)
                    .map(|(k, _)| k.clone())
                    .collect();
                let sellers: Vec<String> = registry.agents.iter()
                    .filter(|(_, a)| a.role() == ResonatorAgentRole::Seller)
                    .map(|(k, _)| k.clone())
                    .collect();

                if buyers.is_empty() || sellers.is_empty() {
                    break;
                }

                let b = buyers[round as usize % buyers.len()].clone();
                let s = sellers[round as usize % sellers.len()].clone();
                (b, s)
            };

            let _ = execute_trade_internal(&state_clone, &buyer_id, &seller_id).await;
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Started auto trading for {} rounds", rounds)
    })))
}

#[derive(Deserialize)]
struct SimulateRequest {
    buyers: Option<u32>,
    sellers: Option<u32>,
    rounds: Option<u32>,
    delay_ms: Option<u64>,
}

async fn simulate_marketplace(
    State(state): State<Arc<SystemState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let num_buyers = req.buyers.unwrap_or(3);
    let num_sellers = req.sellers.unwrap_or(2);
    let rounds = req.rounds.unwrap_or(10);
    let delay_ms = req.delay_ms.unwrap_or(500);

    let service_catalog = vec![
        ("Data Analysis", 100_00),
        ("Model Training", 200_00),
        ("API Integration", 150_00),
        ("Security Audit", 300_00),
        ("Cloud Setup", 250_00),
    ];

    let buyer_names = vec!["Alice", "Bob", "Carol", "Dave", "Eve"];
    let seller_names = vec!["DataCorp", "CloudAI", "SecureNet", "ModelHub", "APIForge"];

    // Create sellers
    for i in 0..num_sellers as usize {
        let name = seller_names[i % seller_names.len()];
        let (svc, price) = &service_catalog[i % service_catalog.len()];
        let req = CreateSellerRequest {
            name: name.to_string(),
            service_name: svc.to_string(),
            price: *price as u64,
        };
        let _ = create_seller_internal(&state, req).await;
    }

    // Create buyers
    for i in 0..num_buyers as usize {
        let name = buyer_names[i % buyer_names.len()];
        let req = CreateBuyerRequest {
            name: name.to_string(),
            funding: Some(10000_00), // $10,000
        };
        let _ = create_buyer_internal(&state, req).await;
    }

    // Start auto trading in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        for round in 0..rounds {
            let (buyer_id, seller_id) = {
                let registry = state_clone.agents.read().await;
                let buyers: Vec<String> = registry.agents.iter()
                    .filter(|(_, a)| a.role() == ResonatorAgentRole::Buyer)
                    .map(|(k, _)| k.clone())
                    .collect();
                let sellers: Vec<String> = registry.agents.iter()
                    .filter(|(_, a)| a.role() == ResonatorAgentRole::Seller)
                    .map(|(k, _)| k.clone())
                    .collect();

                if buyers.is_empty() || sellers.is_empty() {
                    break;
                }

                let b = buyers[round as usize % buyers.len()].clone();
                let s = sellers[round as usize % sellers.len()].clone();
                (b, s)
            };

            let _ = execute_trade_internal(&state_clone, &buyer_id, &seller_id).await;
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Simulation started: {} buyers, {} sellers, {} rounds", num_buyers, num_sellers, rounds)
    })))
}

// Helper functions for internal use (avoid duplicate code)
async fn create_buyer_internal(state: &Arc<SystemState>, req: CreateBuyerRequest) -> Result<(), String> {
    let funding = req.funding.unwrap_or(500_00);
    let agent_id = format!("res_{}", req.name.to_lowercase().replace(' ', "_"));

    let brain = {
        let llm = LLMRouter::from_env();
        if llm.is_available().await {
            AgentBrain::with_llm(LLMRouter::from_env())
        } else {
            AgentBrain::deterministic()
        }
    };

    let resonator_handle = state.runtime.register_agent(&req.name, ResonatorAgentRole::Buyer).await.ok();

    let mut agent = MapleResonatorAgent::new_buyer(&req.name, state.ledger.clone(), brain, resonator_handle);

    let resonator_id = ResonatorId::from_string(&agent_id);
    let mint = MintIntent::new(resonator_id, Amount::new(funding), "Simulation funding");
    let mint_receipt = {
        let issuer = state.issuer.read().await;
        issuer.mint(mint).await.map_err(|e| e.to_string())?
    };

    let mint_record = build_receipt_record(
        &Receipt::Issuer(mint_receipt.clone()),
        Some("issuer".to_string()),
        format!("Minted ${:.2} to {}", funding as f64 / 100.0, agent_id),
    );
    store_receipt_records(&state, vec![mint_record]).await;

    state.emit_event(SystemEvent::Minted {
        receipt_id: mint_receipt.receipt_id.clone(),
        account: agent_id.clone(),
        amount: mint_receipt.amount.0,
        asset: mint_receipt.asset.0.clone(),
        total_supply: None,
        timestamp: mint_receipt.issued_at,
    });

    agent.as_buyer_mut().unwrap()
        .setup(Amount::new(funding), Amount::new(funding / 2))
        .map_err(|e| e.to_string())?;

    let mut registry = state.agents.write().await;
    registry.agents.insert(agent_id, agent);
    Ok(())
}

async fn create_seller_internal(state: &Arc<SystemState>, req: CreateSellerRequest) -> Result<(), String> {
    let agent_id = format!("res_{}", req.name.to_lowercase().replace(' ', "_"));
    let brain = AgentBrain::deterministic();
    let resonator_handle = state.runtime.register_agent(&req.name, ResonatorAgentRole::Seller).await.ok();

    let mut agent = MapleResonatorAgent::new_seller(&req.name, state.ledger.clone(), brain, resonator_handle);

    let service = Service {
        name: req.service_name.clone(),
        description: format!("AI service: {}", req.service_name),
        price: Amount::new(req.price),
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Service completion".to_string()],
    };
    agent.as_seller_mut().unwrap().publish_service(service);

    let mut registry = state.agents.write().await;
    registry.agents.insert(agent_id, agent);
    Ok(())
}

// ============================================================================
// Issuer / Supply
// ============================================================================

async fn get_supply(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let issuer = state.issuer.read().await;
    let total = issuer.total_supply().await;
    let remaining = issuer.remaining_supply().await;
    let receipts = issuer.receipts().await;

    Json(serde_json::json!({
        "total_supply": total.0,
        "remaining_supply": remaining.0,
        "total_display": format!("${:.2}", total.0 as f64 / 100.0),
        "remaining_display": format!("${:.2}", remaining.0 as f64 / 100.0),
        "receipt_count": receipts.len()
    }))
}

async fn get_issuer_receipts(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let issuer = state.issuer.read().await;
    let receipts = issuer.recent_receipts(50).await;

    let receipt_list: Vec<serde_json::Value> = receipts.iter().map(|r| {
        serde_json::json!({
            "receipt_id": r.receipt_id,
            "operation": format!("{:?}", r.operation),
            "target": r.target.0,
            "amount": r.amount.0,
            "amount_display": format!("${:.2}", r.amount.0 as f64 / 100.0),
            "issued_at": r.issued_at
        })
    }).collect();

    Json(serde_json::json!({
        "receipts": receipt_list,
        "count": receipt_list.len()
    }))
}

// ============================================================================
// Ledger
// ============================================================================

async fn get_ledger_accounts(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let account_ids = state.ledger.all_accounts().await;
    let iusd = AssetId::iusd();

    let mut entries = Vec::new();
    for account_id in &account_ids {
        let balance = state.ledger.balance(account_id, &iusd).await;
        entries.push(serde_json::json!({
            "account_id": account_id.0,
            "balance": balance.0,
            "balance_display": format!("${:.2}", balance.0 as f64 / 100.0)
        }));
    }

    let entry_count = state.ledger.entry_count().await;
    let recent = state.ledger.recent_entries(20).await;
    let recent_entries: Vec<serde_json::Value> = recent.iter().map(|e| {
        serde_json::json!({
            "entry_id": e.entry_id.0,
            "account": e.account.0,
            "entry_type": format!("{:?}", e.entry_type),
            "amount": e.amount.0,
            "amount_display": format!("${:.2}", e.amount.0 as f64 / 100.0),
            "balance_after": e.balance_after.0,
            "reason": format!("{:?}", e.reason),
            "created_at": e.created_at
        })
    }).collect();

    Json(serde_json::json!({
        "accounts": entries,
        "account_count": account_ids.len(),
        "total_entries": entry_count,
        "recent_entries": recent_entries
    }))
}

// ============================================================================
// Transactions & Receipts
// ============================================================================

async fn get_transactions(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let registry = state.agents.read().await;
    Json(serde_json::json!({
        "transactions": registry.transactions,
        "count": registry.transactions.len()
    }))
}

async fn get_receipts(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let registry = state.agents.read().await;
    Json(serde_json::json!({
        "receipts": registry.receipts,
        "count": registry.receipts.len()
    }))
}

fn receipt_timestamp(receipt: &Receipt) -> DateTime<Utc> {
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
    let receipt_id = receipt.id().to_string();
    let actor = actor_override.unwrap_or_else(|| receipt_actor_default(receipt));
    let receipt_type = receipt_type_label(receipt).to_string();
    let timestamp = receipt_timestamp(receipt);
    let data = serde_json::to_value(receipt).unwrap_or_else(|_| serde_json::json!({"error": "receipt_serialization_failed"}));

    ReceiptRecord {
        receipt_id,
        receipt_type,
        actor,
        description,
        data,
        timestamp,
    }
}

async fn store_receipt_records(state: &Arc<SystemState>, records: Vec<ReceiptRecord>) {
    if records.is_empty() {
        return;
    }

    {
        let mut registry = state.agents.write().await;
        registry.receipts.extend(records.clone());
    }

    for record in records {
        state.emit_event(SystemEvent::ReceiptIssued {
            receipt_id: record.receipt_id.clone(),
            receipt_type: record.receipt_type.clone(),
            actor: record.actor.clone(),
            description: record.description.clone(),
            timestamp: record.timestamp,
        });
        state.emit_event(SystemEvent::ReceiptGenerated {
            receipt_id: record.receipt_id.clone(),
            receipt_type: record.receipt_type.clone(),
            actor: record.actor.clone(),
            description: record.description.clone(),
            timestamp: record.timestamp,
        });
        state.log_activity(
            openibank_state::activity::ActivityEntry::info(
                "receipt",
                openibank_state::activity::ActivityCategory::Receipt,
                format!("Receipt {} issued", record.receipt_id),
            )
            .with_data(record.data.clone()),
        ).await;
    }
}

async fn get_receipt(
    State(state): State<Arc<SystemState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let registry = state.agents.read().await;
    let receipt = registry.receipts.iter().find(|r| r.receipt_id == id);
    if let Some(record) = receipt {
        return Ok(Json(serde_json::json!({ "receipt": record })));
    }

    Err(AppError::NotFound(format!("Receipt {} not found", id)))
}

async fn export_receipts(State(state): State<Arc<SystemState>>) -> impl IntoResponse {
    let registry = state.agents.read().await;
    let mut receipts = registry.receipts.clone();
    receipts.sort_by_key(|r| r.timestamp);

    let mut lines = Vec::with_capacity(receipts.len());
    for record in receipts {
        if let Ok(line) = serde_json::to_string(&record.data) {
            lines.push(line);
        }
    }

    let body = lines.join("\n");
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/x-ndjson"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"openibank-receipts.jsonl\""),
    );
    (headers, body)
}

#[derive(Deserialize)]
struct VerifyReceiptRequest {
    receipt_id: Option<String>,
    receipt: Option<serde_json::Value>,
}

async fn verify_receipt(
    State(state): State<Arc<SystemState>>,
    Json(req): Json<VerifyReceiptRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let payload = if let Some(receipt_value) = req.receipt {
        serde_json::to_string(&receipt_value)
            .map_err(|e| AppError::Internal(format!("Receipt serialization failed: {}", e)))?
    } else if let Some(receipt_id) = req.receipt_id.as_ref() {
        let registry = state.agents.read().await;
        let record = registry
            .receipts
            .iter()
            .find(|r| &r.receipt_id == receipt_id)
            .ok_or_else(|| AppError::NotFound(format!("Receipt {} not found", receipt_id)))?;
        serde_json::to_string(&record.data)
            .map_err(|e| AppError::Internal(format!("Receipt serialization failed: {}", e)))?
    } else {
        return Err(AppError::Internal("receipt_id or receipt payload required".to_string()));
    };

    let result: VerificationResult = verify_receipt_json(&payload);
    state.emit_event(SystemEvent::ReceiptVerified {
        receipt_id: result.receipt_id.clone(),
        valid: result.valid,
        errors: result.errors.clone(),
        timestamp: Utc::now(),
    });

    Ok(Json(serde_json::json!({
        "result": result
    })))
}

async fn replay_receipts(
    State(_state): State<Arc<SystemState>>,
    body: String,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut receipts = Vec::new();
    let mut errors = Vec::new();

    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Receipt>(trimmed) {
            Ok(receipt) => receipts.push(receipt),
            Err(e) => errors.push(format!("Line {}: {}", index + 1, e)),
        }
    }

    receipts.sort_by_key(receipt_timestamp);

    let mut balances: HashMap<String, i128> = HashMap::new();
    let mut timeline: Vec<serde_json::Value> = Vec::new();

    for receipt in &receipts {
        match receipt {
            Receipt::Issuer(r) => {
                let amount = r.amount.0 as i128;
                let entry = balances.entry(r.target.0.clone()).or_insert(0);
                match r.operation {
                    openibank_core::IssuerOperation::Mint => {
                        *entry += amount;
                        timeline.push(serde_json::json!({
                            "timestamp": r.issued_at,
                            "event": "Minted",
                            "receipt_id": r.receipt_id,
                            "account": r.target.0,
                            "amount": r.amount.0,
                            "asset": r.asset.0,
                        }));
                    }
                    openibank_core::IssuerOperation::Burn => {
                        *entry -= amount;
                        timeline.push(serde_json::json!({
                            "timestamp": r.issued_at,
                            "event": "Burned",
                            "receipt_id": r.receipt_id,
                            "account": r.target.0,
                            "amount": r.amount.0,
                            "asset": r.asset.0,
                        }));
                    }
                }
            }
            Receipt::Commitment(r) => {
                let metadata = r.consequence_ref.metadata.as_object();
                let consequence_type = r.consequence_ref.consequence_type.as_str();
                let amount = metadata.and_then(|m| m.get("amount")).and_then(|v| v.as_u64());
                let from = metadata.and_then(|m| m.get("from")).and_then(|v| v.as_str());
                let to = metadata.and_then(|m| m.get("to")).and_then(|v| v.as_str());
                let asset = metadata
                    .and_then(|m| m.get("asset"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("iusd");

                let should_apply = matches!(consequence_type, "ledger_transfer" | "escrow_release");

                if let (Some(amount), Some(from), Some(to)) = (amount, from, to) {
                    if should_apply {
                        let entry_from = balances.entry(from.to_string()).or_insert(0);
                        *entry_from -= amount as i128;
                        let entry_to = balances.entry(to.to_string()).or_insert(0);
                        *entry_to += amount as i128;
                    }
                    timeline.push(serde_json::json!({
                        "timestamp": r.committed_at,
                        "event": if should_apply { "TransferPosted" } else { "TransferProposed" },
                        "receipt_id": r.commitment_id.0,
                        "from": from,
                        "to": to,
                        "amount": amount,
                        "asset": asset,
                        "consequence": consequence_type,
                    }));
                } else {
                    errors.push(format!(
                        "Commitment {} missing transfer metadata",
                        r.commitment_id.0
                    ));
                }
            }
        }
    }

    let balances_json: serde_json::Value = balances
        .into_iter()
        .map(|(k, v)| (k, serde_json::json!({ "iusd": v })))
        .collect::<serde_json::Map<_, _>>()
        .into();

    Ok(Json(serde_json::json!({
        "receipt_count": receipts.len(),
        "timeline": timeline,
        "balances": balances_json,
        "errors": errors,
    })))
}

// ============================================================================
// Maple Resonators
// ============================================================================

async fn get_resonators(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let registry = state.agents.read().await;
    let runtime_status = state.runtime.status().await;

    let resonators: Vec<serde_json::Value> = registry.agents.values().map(|a| {
        serde_json::json!({
            "id": a.id().0,
            "name": a.name,
            "role": a.role(),
            "has_resonator_handle": a.resonator_handle.is_some(),
            "maple_profile": serde_json::to_value(a.maple_profile()).ok(),
            "presence": a.presence,
            "trade_count": a.trade_count,
        })
    }).collect();

    Json(serde_json::json!({
        "runtime": runtime_status,
        "resonators": resonators,
        "count": resonators.len()
    }))
}

// ============================================================================
// System Log
// ============================================================================

async fn get_system_log(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let log = state.activity_log.read().await;
    let entries: Vec<&openibank_state::activity::ActivityEntry> = log.iter().take(100).collect();

    Json(serde_json::json!({
        "entries": entries,
        "count": log.len()
    }))
}

// ============================================================================
// Reset
// ============================================================================

async fn reset_playground(State(state): State<Arc<SystemState>>) -> Json<serde_json::Value> {
    let mut registry = state.agents.write().await;
    registry.agents.clear();
    registry.trade_count = 0;
    registry.total_volume = 0;
    registry.transactions.clear();
    registry.receipts.clear();

    state.emit_event(SystemEvent::SystemReset {
        timestamp: Utc::now(),
    });

    Json(serde_json::json!({
        "success": true,
        "message": "Playground reset"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;

    async fn setup_demo_state() -> Arc<SystemState> {
        Arc::new(SystemState::new().await.expect("system state"))
    }

    async fn create_buyer_test(state: &Arc<SystemState>, name: &str, funding: u64) -> String {
        let agent_id = format!("res_{}", name.to_lowercase().replace(' ', "_"));
        let brain = AgentBrain::deterministic();
        let resonator_handle = state.runtime.register_agent(name, ResonatorAgentRole::Buyer).await.ok();

        let mut agent = MapleResonatorAgent::new_buyer(
            name,
            state.ledger.clone(),
            brain,
            resonator_handle,
        );

        let resonator_id = ResonatorId::from_string(&agent_id);
        let mint = MintIntent::new(resonator_id, Amount::new(funding), "Test funding");
        let mint_receipt = {
            let issuer = state.issuer.read().await;
            issuer.mint(mint).await.expect("mint")
        };

        let mint_record = build_receipt_record(
            &Receipt::Issuer(mint_receipt.clone()),
            Some("issuer".to_string()),
            format!("Minted ${:.2} to {}", funding as f64 / 100.0, agent_id),
        );
        store_receipt_records(state, vec![mint_record]).await;

        state.emit_event(SystemEvent::Minted {
            receipt_id: mint_receipt.receipt_id.clone(),
            account: agent_id.clone(),
            amount: mint_receipt.amount.0,
            asset: mint_receipt.asset.0.clone(),
            total_supply: None,
            timestamp: mint_receipt.issued_at,
        });

        agent
            .as_buyer_mut()
            .unwrap()
            .setup(Amount::new(funding), Amount::new(funding / 2))
            .expect("setup buyer");

        let mut registry = state.agents.write().await;
        registry.agents.insert(agent_id.clone(), agent);
        agent_id
    }

    async fn create_seller_test(state: &Arc<SystemState>, name: &str, service_name: &str, price: u64) -> String {
        let agent_id = format!("res_{}", name.to_lowercase().replace(' ', "_"));
        let brain = AgentBrain::deterministic();
        let resonator_handle = state.runtime.register_agent(name, ResonatorAgentRole::Seller).await.ok();

        let mut agent = MapleResonatorAgent::new_seller(
            name,
            state.ledger.clone(),
            brain,
            resonator_handle,
        );

        let service = Service {
            name: service_name.to_string(),
            description: format!("AI service: {}", service_name),
            price: Amount::new(price),
            asset: AssetId::iusd(),
            delivery_conditions: vec!["Service completion".to_string()],
        };
        agent.as_seller_mut().unwrap().publish_service(service);

        let mut registry = state.agents.write().await;
        registry.agents.insert(agent_id.clone(), agent);
        agent_id
    }

    #[tokio::test]
    async fn test_receipt_verification_demo_flow() {
        let state = setup_demo_state().await;

        create_buyer_test(&state, "Alice", 50_000).await;
        create_seller_test(&state, "DataCorp", "Data Analysis", 10_000).await;

        let _ = execute_trade_internal(&state, "res_alice", "res_datacorp")
            .await
            .expect("execute trade");

        let registry = state.agents.read().await;
        let receipts = registry.receipts.clone();
        assert!(!receipts.is_empty(), "expected receipts from demo flow");

        let mut any_valid = false;
        for record in receipts {
            let json = serde_json::to_string(&record.data).expect("receipt json");
            let result = verify_receipt_json(&json);
            if result.valid {
                any_valid = true;
                break;
            }
        }

        assert!(any_valid, "expected at least one valid receipt");
    }

    #[tokio::test]
    async fn test_replay_reproduces_balances() {
        let state = setup_demo_state().await;

        create_buyer_test(&state, "Alice", 50_000).await;
        create_seller_test(&state, "DataCorp", "Data Analysis", 10_000).await;

        let _ = execute_trade_internal(&state, "res_alice", "res_datacorp")
            .await
            .expect("execute trade");

        let registry = state.agents.read().await;
        let receipts = registry.receipts.clone();
        drop(registry);

        let jsonl = receipts
            .iter()
            .filter_map(|record| serde_json::to_string(&record.data).ok())
            .collect::<Vec<_>>()
            .join("\n");

        let response = replay_receipts(State(state.clone()), jsonl)
            .await
            .expect("replay");
        let Json(payload) = response;
        let balances = payload
            .get("balances")
            .and_then(|b| b.as_object())
            .expect("balances map");

        let buyer_balance = state
            .ledger
            .balance(&ResonatorId::from_string("res_alice"), &AssetId::iusd())
            .await;
        let seller_balance = state
            .ledger
            .balance(&ResonatorId::from_string("res_datacorp"), &AssetId::iusd())
            .await;

        let buyer_replay = balances
            .get("res_alice")
            .and_then(|v| v.get("iusd"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let seller_replay = balances
            .get("res_datacorp")
            .and_then(|v| v.get("iusd"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        assert_eq!(buyer_replay as u64, buyer_balance.0);
        assert_eq!(seller_replay as u64, seller_balance.0);
    }
}

// ============================================================================
// SSE Event Stream
// ============================================================================

async fn event_stream(
    State(state): State<Arc<SystemState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.subscribe();

    // Send initial state sync
    let summary = state.status_summary().await;
    let initial_data = serde_json::json!({
        "type": "state_sync",
        "data": summary
    });

    let initial_event = Event::default()
        .event("state_sync")
        .data(serde_json::to_string(&initial_data).unwrap_or_default());

    let stream = async_stream::stream! {
        yield Ok(initial_event);

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let event_name = match &event {
                        SystemEvent::AgentCreated { .. } => "agent_created",
                        SystemEvent::AgentStateChanged { .. } => "agent_state_changed",
                        SystemEvent::BalanceUpdated { .. } => "balance_updated",
                        SystemEvent::TradeStarted { .. } => "trade_started",
                        SystemEvent::TradeCompleted { .. } => "trade_completed",
                        SystemEvent::TradeFailed { .. } => "trade_failed",
                        SystemEvent::CommitmentDeclared { .. } => "commitment_declared",
                        SystemEvent::TransferProposed { .. } => "transfer_proposed",
                        SystemEvent::TransferPosted { .. } => "transfer_posted",
                        SystemEvent::EscrowOpened { .. } => "escrow_opened",
                        SystemEvent::EscrowReleased { .. } => "escrow_released",
                        SystemEvent::Minted { .. } => "minted",
                        SystemEvent::Burned { .. } => "burned",
                        SystemEvent::LLMReasoning { .. } => "llm_reasoning",
                        SystemEvent::ReceiptGenerated { .. } => "receipt_generated",
                        SystemEvent::ReceiptIssued { .. } => "receipt_issued",
                        SystemEvent::ReceiptVerified { .. } => "receipt_verified",
                        SystemEvent::IssuerEvent { .. } => "issuer_event",
                        SystemEvent::LedgerEntry { .. } => "ledger_entry",
                        SystemEvent::EscrowEvent { .. } => "escrow_event",
                        SystemEvent::DisputeEvent { .. } => "dispute_event",
                        SystemEvent::SystemStatus { .. } => "system_status",
                        SystemEvent::SystemReset { .. } => "system_reset",
                        SystemEvent::MapleRuntimeEvent { .. } => "maple_runtime",
                        SystemEvent::CouplingEstablished { .. } => "coupling_established",
                        SystemEvent::CouplingStrengthened { .. } => "coupling_strengthened",
                        SystemEvent::CouplingWeakened { .. } => "coupling_weakened",
                        SystemEvent::Decoupled { .. } => "decoupled",
                        SystemEvent::CommitmentSubmitted { .. } => "commitment_submitted",
                        SystemEvent::CommitmentApproved { .. } => "commitment_approved",
                        SystemEvent::CommitmentRejected { .. } => "commitment_rejected",
                        SystemEvent::CommitmentOutcomeRecorded { .. } => "commitment_outcome",
                        SystemEvent::AttentionAllocated { .. } => "attention_allocated",
                        SystemEvent::AttentionExhausted { .. } => "attention_exhausted",
                        SystemEvent::CapabilityGranted { .. } => "capability_granted",
                        SystemEvent::CapabilityDenied { .. } => "capability_denied",
                    };

                    yield Ok(Event::default()
                        .event(event_name)
                        .data(serde_json::to_string(&event).unwrap_or_default()));
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

// ============================================================================
// UAL Command Endpoint
// ============================================================================

#[derive(Deserialize)]
struct UalRequest {
    command: String,
}

async fn execute_ual(
    State(_state): State<Arc<SystemState>>,
    Json(req): Json<UalRequest>,
) -> Json<serde_json::Value> {
    use openibank_ual::{parse_input, compile_statements, ParsedInput};

    match parse_input(&req.command) {
        Ok(ParsedInput::Ual(statements)) => {
            match compile_statements(&statements) {
                Ok(compiled) => {
                    let artifacts: Vec<serde_json::Value> = compiled.iter()
                        .map(|c| serde_json::to_value(c).unwrap_or_default())
                        .collect();
                    Json(serde_json::json!({
                        "success": true,
                        "type": "ual",
                        "statement_count": statements.len(),
                        "compiled": artifacts,
                        "message": format!("Compiled {} UAL statement(s)", statements.len()),
                    }))
                }
                Err(e) => Json(serde_json::json!({
                    "success": false,
                    "error": format!("Compilation error: {}", e),
                })),
            }
        }
        Ok(ParsedInput::Banking(cmd)) => {
            Json(serde_json::json!({
                "success": true,
                "type": "banking",
                "command": serde_json::to_value(&cmd).unwrap_or_default(),
                "message": "Banking command parsed (execute via openibank-server for full support)",
            }))
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": format!("Parse error: {}", e),
            "hint": "Try: STATUS, BALANCE <agent>, or UAL COMMIT statements",
        })),
    }
}

async fn get_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "OpeniBank",
        "version": env!("CARGO_PKG_VERSION"),
        "homepage": "https://www.openibank.com",
        "repository": "https://github.com/openibank/openibank",
        "architecture": "Maple Resonance Architecture + PALM Fleet + UAL Commands",
    }))
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
enum AppError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": true,
            "message": message
        }));

        (status, body).into_response()
    }
}
