//! OpeniBank Playground - Interactive Web Demo
//!
//! A live web playground that demonstrates AI agent trading with real-time updates.
//!
//! ## Features
//!
//! - **Live Agent Trading**: Watch AI agents trade in real-time
//! - **LLM Reasoning Display**: See agent decision-making with visible reasoning
//! - **Interactive Controls**: Create agents, fund wallets, trigger trades
//! - **Real-time Updates**: Server-Sent Events for live state updates
//!
//! ## Running
//!
//! ```bash
//! # Start the playground
//! cargo run -p openibank-playground
//!
//! # Open in browser
//! open http://localhost:8080
//! ```

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        Html, IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use openibank_agents::{AgentBrain, BuyerAgent, SellerAgent, Service};
use openibank_core::{Amount, AssetId, ResonatorId};
use openibank_issuer::{Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use openibank_llm::LLMRouter;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state shared across handlers
struct AppState {
    ledger: Arc<Ledger>,
    issuer: Issuer,
    agents: RwLock<AgentRegistry>,
    events: broadcast::Sender<PlaygroundEvent>,
    llm_router: LLMRouter,
}

/// Registry of active agents
#[derive(Default)]
struct AgentRegistry {
    buyers: Vec<BuyerAgentState>,
    sellers: Vec<SellerAgentState>,
    trade_count: u32,
    total_volume: u64,
}

struct BuyerAgentState {
    id: String,
    agent: BuyerAgent,
    name: String,
}

struct SellerAgentState {
    id: String,
    agent: SellerAgent,
    name: String,
    service: Service,
}

/// Events sent to the frontend via SSE
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
enum PlaygroundEvent {
    #[serde(rename = "agent_created")]
    AgentCreated {
        agent_type: String,
        id: String,
        name: String,
    },
    #[serde(rename = "balance_updated")]
    BalanceUpdated { agent_id: String, balance: u64 },
    #[serde(rename = "trade_started")]
    TradeStarted {
        buyer_id: String,
        seller_id: String,
        service: String,
        amount: u64,
    },
    #[serde(rename = "llm_reasoning")]
    LLMReasoning {
        agent_id: String,
        reasoning: String,
        decision: String,
    },
    #[serde(rename = "trade_completed")]
    TradeCompleted {
        buyer_id: String,
        seller_id: String,
        amount: u64,
        receipt_id: String,
    },
    #[serde(rename = "trade_failed")]
    TradeFailed {
        buyer_id: String,
        seller_id: String,
        reason: String,
    },
    #[serde(rename = "state_sync")]
    StateSync {
        buyers: Vec<AgentSummary>,
        sellers: Vec<AgentSummary>,
        trade_count: u32,
        total_volume: u64,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Clone, Serialize)]
struct AgentSummary {
    id: String,
    name: String,
    balance: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<u64>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("🎮 Starting OpeniBank Playground...");

    // Create shared state
    let ledger = Arc::new(Ledger::new());
    let issuer = Issuer::new(
        IssuerConfig::default(),
        Amount::new(100_000_000_00), // $1M reserve
        ledger.clone(),
    );

    let (event_tx, _) = broadcast::channel(1000);
    let llm_router = LLMRouter::from_env();

    let llm_available = llm_router.is_available().await;
    tracing::info!(
        "LLM Status: {}",
        if llm_available {
            "Available ✓"
        } else {
            "Not available (deterministic mode)"
        }
    );

    let state = Arc::new(AppState {
        ledger,
        issuer,
        agents: RwLock::new(AgentRegistry::default()),
        events: event_tx,
        llm_router,
    });

    // Build router
    let app = Router::new()
        // Web UI
        .route("/", get(index_page))
        // API endpoints
        .route("/api/status", get(get_status))
        .route("/api/agents", get(list_agents))
        .route("/api/agents/buyer", post(create_buyer))
        .route("/api/agents/seller", post(create_seller))
        .route("/api/trade", post(execute_trade))
        .route("/api/trade/auto", post(start_auto_trading))
        .route("/api/reset", post(reset_playground))
        // SSE stream
        .route("/api/events", get(event_stream))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("🌐 Playground running at http://localhost:8080");
    tracing::info!("📡 API available at http://localhost:8080/api/status");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ============================================================================
// Web UI Handler
// ============================================================================

async fn index_page() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

// ============================================================================
// API Handlers
// ============================================================================

#[derive(Serialize)]
struct StatusResponse {
    name: String,
    version: String,
    llm_available: bool,
    llm_provider: String,
    total_supply: u64,
    agents: AgentCounts,
}

#[derive(Serialize)]
struct AgentCounts {
    buyers: usize,
    sellers: usize,
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let agents = state.agents.read().await;
    let llm_available = state.llm_router.is_available().await;

    Json(StatusResponse {
        name: "OpeniBank Playground".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        llm_available,
        llm_provider: std::env::var("OPENIBANK_LLM_PROVIDER").unwrap_or_else(|_| "none".to_string()),
        total_supply: state.issuer.total_supply().await.0,
        agents: AgentCounts {
            buyers: agents.buyers.len(),
            sellers: agents.sellers.len(),
        },
    })
}

async fn list_agents(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let agents = state.agents.read().await;

    let buyers: Vec<AgentSummary> = agents
        .buyers
        .iter()
        .map(|b| AgentSummary {
            id: b.id.clone(),
            name: b.name.clone(),
            balance: b.agent.balance().0,
            service: None,
            price: None,
        })
        .collect();

    let sellers: Vec<AgentSummary> = agents
        .sellers
        .iter()
        .map(|s| AgentSummary {
            id: s.id.clone(),
            name: s.name.clone(),
            balance: s.agent.balance().0,
            service: Some(s.service.name.clone()),
            price: Some(s.service.price.0),
        })
        .collect();

    Json(serde_json::json!({
        "buyers": buyers,
        "sellers": sellers,
        "trade_count": agents.trade_count,
        "total_volume": agents.total_volume
    }))
}

#[derive(Deserialize)]
struct CreateBuyerRequest {
    name: String,
    funding: Option<u64>,
}

async fn create_buyer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBuyerRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let funding = req.funding.unwrap_or(500_00); // Default $500
    let id = format!("buyer_{}", req.name.to_lowercase().replace(' ', "_"));
    let resonator_id = ResonatorId::from_string(&id);

    // Create brain with LLM if available
    let brain = if state.llm_router.is_available().await {
        AgentBrain::with_llm(LLMRouter::from_env())
    } else {
        AgentBrain::deterministic()
    };

    let mut buyer = BuyerAgent::with_brain(resonator_id.clone(), state.ledger.clone(), brain);

    // Fund the buyer
    let mint = MintIntent::new(resonator_id, Amount::new(funding), "Playground funding");
    state
        .issuer
        .mint(mint)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    buyer
        .setup(Amount::new(funding), Amount::new(funding / 2))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut agents = state.agents.write().await;
    agents.buyers.push(BuyerAgentState {
        id: id.clone(),
        agent: buyer,
        name: req.name.clone(),
    });

    // Broadcast event
    let _ = state.events.send(PlaygroundEvent::AgentCreated {
        agent_type: "buyer".to_string(),
        id: id.clone(),
        name: req.name.clone(),
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "id": id,
        "name": req.name,
        "balance": funding,
        "budget": funding / 2
    })))
}

#[derive(Deserialize)]
struct CreateSellerRequest {
    name: String,
    service_name: String,
    price: u64,
}

async fn create_seller(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSellerRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = format!("seller_{}", req.name.to_lowercase().replace(' ', "_"));
    let resonator_id = ResonatorId::from_string(&id);

    let mut seller = SellerAgent::new(resonator_id, state.ledger.clone());

    let service = Service {
        name: req.service_name.clone(),
        description: format!("AI service: {}", req.service_name),
        price: Amount::new(req.price),
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Service completion".to_string()],
    };

    seller.publish_service(service.clone());

    let mut agents = state.agents.write().await;
    agents.sellers.push(SellerAgentState {
        id: id.clone(),
        agent: seller,
        name: req.name.clone(),
        service: service.clone(),
    });

    // Broadcast event
    let _ = state.events.send(PlaygroundEvent::AgentCreated {
        agent_type: "seller".to_string(),
        id: id.clone(),
        name: req.name.clone(),
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "id": id,
        "name": req.name,
        "service": req.service_name,
        "price": req.price
    })))
}

#[derive(Deserialize)]
struct TradeRequest {
    buyer_id: String,
    seller_id: String,
}

async fn execute_trade(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TradeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut agents = state.agents.write().await;

    // Find buyer and seller indices
    let buyer_idx = agents
        .buyers
        .iter()
        .position(|b| b.id == req.buyer_id)
        .ok_or_else(|| AppError::NotFound(format!("Buyer {} not found", req.buyer_id)))?;

    let seller_idx = agents
        .sellers
        .iter()
        .position(|s| s.id == req.seller_id)
        .ok_or_else(|| AppError::NotFound(format!("Seller {} not found", req.seller_id)))?;

    // Get service info
    let service_name = agents.sellers[seller_idx].service.name.clone();
    let service_price = agents.sellers[seller_idx].service.price.0;

    // Broadcast trade started
    let _ = state.events.send(PlaygroundEvent::TradeStarted {
        buyer_id: req.buyer_id.clone(),
        seller_id: req.seller_id.clone(),
        service: service_name.clone(),
        amount: service_price,
    });

    // Get offer
    let offer = {
        let seller = &agents.sellers[seller_idx];
        seller.agent.get_offer(&service_name)
    };

    let offer = match offer {
        Some(o) => o,
        None => {
            let _ = state.events.send(PlaygroundEvent::TradeFailed {
                buyer_id: req.buyer_id.clone(),
                seller_id: req.seller_id.clone(),
                reason: "No offer available".to_string(),
            });
            return Err(AppError::Internal("No offer available".to_string()));
        }
    };

    // Buyer evaluates offer
    let can_afford = agents.buyers[buyer_idx].agent.evaluate_offer(&offer).await;

    // Send reasoning event if LLM is used
    if state.llm_router.is_available().await {
        let _ = state.events.send(PlaygroundEvent::LLMReasoning {
            agent_id: req.buyer_id.clone(),
            reasoning: format!(
                "Evaluating {} service at ${:.2}. Checking budget and value proposition.",
                service_name,
                service_price as f64 / 100.0
            ),
            decision: if can_afford {
                "Accept offer".to_string()
            } else {
                "Decline offer".to_string()
            },
        });
    }

    if !can_afford {
        let _ = state.events.send(PlaygroundEvent::TradeFailed {
            buyer_id: req.buyer_id.clone(),
            seller_id: req.seller_id.clone(),
            reason: "Buyer cannot afford or declined".to_string(),
        });
        return Err(AppError::Internal("Buyer cannot afford".to_string()));
    }

    // Get buyer ID for invoice
    let buyer_resonator_id = agents.buyers[buyer_idx].agent.id().clone();

    // Issue invoice
    let invoice = agents.sellers[seller_idx]
        .agent
        .issue_invoice(buyer_resonator_id, &service_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let invoice_id = invoice.invoice_id.clone();

    // Buyer accepts invoice
    agents.buyers[buyer_idx]
        .agent
        .accept_invoice(invoice)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Pay invoice (creates escrow)
    let (_, escrow) = agents.buyers[buyer_idx]
        .agent
        .pay_invoice(&invoice_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let escrow_id = escrow.escrow_id.clone();

    // Seller delivers
    agents.sellers[seller_idx]
        .agent
        .deliver_service(&invoice_id, "Service delivered successfully".to_string())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Buyer confirms delivery
    let amount = agents.buyers[buyer_idx]
        .agent
        .confirm_delivery(&escrow_id)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Seller receives payment
    agents.sellers[seller_idx]
        .agent
        .receive_payment(amount)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Update stats
    agents.trade_count += 1;
    agents.total_volume += amount.0;

    // Broadcast completion
    let _ = state.events.send(PlaygroundEvent::TradeCompleted {
        buyer_id: req.buyer_id.clone(),
        seller_id: req.seller_id.clone(),
        amount: amount.0,
        receipt_id: escrow_id.0,
    });

    // Send balance updates
    let _ = state.events.send(PlaygroundEvent::BalanceUpdated {
        agent_id: req.buyer_id.clone(),
        balance: agents.buyers[buyer_idx].agent.balance().0,
    });

    let _ = state.events.send(PlaygroundEvent::BalanceUpdated {
        agent_id: req.seller_id.clone(),
        balance: agents.sellers[seller_idx].agent.balance().0,
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "amount": amount.0,
        "buyer_balance": agents.buyers[buyer_idx].agent.balance().0,
        "seller_balance": agents.sellers[seller_idx].agent.balance().0
    })))
}

#[derive(Deserialize)]
struct AutoTradeRequest {
    rounds: Option<u32>,
    delay_ms: Option<u64>,
}

async fn start_auto_trading(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AutoTradeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rounds = req.rounds.unwrap_or(10);
    let delay_ms = req.delay_ms.unwrap_or(1000);

    // Spawn background task
    let state_clone = state.clone();
    tokio::spawn(async move {
        for round in 0..rounds {
            let agents = state_clone.agents.read().await;
            if agents.buyers.is_empty() || agents.sellers.is_empty() {
                break;
            }

            let buyer_idx = (round as usize) % agents.buyers.len();
            let seller_idx = (round as usize) % agents.sellers.len();

            let buyer_id = agents.buyers[buyer_idx].id.clone();
            let seller_id = agents.sellers[seller_idx].id.clone();
            drop(agents);

            // Execute trade (ignore errors in auto mode)
            let _ = execute_trade_internal(&state_clone, &buyer_id, &seller_id).await;

            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Started auto trading for {} rounds", rounds)
    })))
}

async fn execute_trade_internal(state: &Arc<AppState>, buyer_id: &str, seller_id: &str) -> Result<(), String> {
    let mut agents = state.agents.write().await;

    let buyer_idx = agents
        .buyers
        .iter()
        .position(|b| b.id == buyer_id)
        .ok_or("Buyer not found")?;

    let seller_idx = agents
        .sellers
        .iter()
        .position(|s| s.id == seller_id)
        .ok_or("Seller not found")?;

    let service_name = agents.sellers[seller_idx].service.name.clone();
    let service_price = agents.sellers[seller_idx].service.price.0;

    let _ = state.events.send(PlaygroundEvent::TradeStarted {
        buyer_id: buyer_id.to_string(),
        seller_id: seller_id.to_string(),
        service: service_name.clone(),
        amount: service_price,
    });

    let offer = agents.sellers[seller_idx]
        .agent
        .get_offer(&service_name)
        .ok_or("No offer")?;

    let can_afford = agents.buyers[buyer_idx].agent.evaluate_offer(&offer).await;

    if !can_afford {
        let _ = state.events.send(PlaygroundEvent::TradeFailed {
            buyer_id: buyer_id.to_string(),
            seller_id: seller_id.to_string(),
            reason: "Cannot afford".to_string(),
        });
        return Err("Cannot afford".to_string());
    }

    let buyer_resonator_id = agents.buyers[buyer_idx].agent.id().clone();

    let invoice = agents.sellers[seller_idx]
        .agent
        .issue_invoice(buyer_resonator_id, &service_name)
        .await
        .map_err(|e| e.to_string())?;

    let invoice_id = invoice.invoice_id.clone();

    agents.buyers[buyer_idx]
        .agent
        .accept_invoice(invoice)
        .map_err(|e| e.to_string())?;

    let (_, escrow) = agents.buyers[buyer_idx]
        .agent
        .pay_invoice(&invoice_id)
        .await
        .map_err(|e| e.to_string())?;

    let escrow_id = escrow.escrow_id.clone();

    agents.sellers[seller_idx]
        .agent
        .deliver_service(&invoice_id, "Auto-delivered".to_string())
        .map_err(|e| e.to_string())?;

    let amount = agents.buyers[buyer_idx]
        .agent
        .confirm_delivery(&escrow_id)
        .map_err(|e| e.to_string())?;

    agents.sellers[seller_idx]
        .agent
        .receive_payment(amount)
        .map_err(|e| e.to_string())?;

    agents.trade_count += 1;
    agents.total_volume += amount.0;

    let _ = state.events.send(PlaygroundEvent::TradeCompleted {
        buyer_id: buyer_id.to_string(),
        seller_id: seller_id.to_string(),
        amount: amount.0,
        receipt_id: escrow_id.0,
    });

    Ok(())
}

async fn reset_playground(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut agents = state.agents.write().await;
    agents.buyers.clear();
    agents.sellers.clear();
    agents.trade_count = 0;
    agents.total_volume = 0;

    Json(serde_json::json!({
        "success": true,
        "message": "Playground reset"
    }))
}

// ============================================================================
// SSE Event Stream
// ============================================================================

async fn event_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.events.subscribe();

    // Send initial state
    let agents = state.agents.read().await;
    let initial_state = PlaygroundEvent::StateSync {
        buyers: agents
            .buyers
            .iter()
            .map(|b| AgentSummary {
                id: b.id.clone(),
                name: b.name.clone(),
                balance: b.agent.balance().0,
                service: None,
                price: None,
            })
            .collect(),
        sellers: agents
            .sellers
            .iter()
            .map(|s| AgentSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                balance: s.agent.balance().0,
                service: Some(s.service.name.clone()),
                price: Some(s.service.price.0),
            })
            .collect(),
        trade_count: agents.trade_count,
        total_volume: agents.total_volume,
    };
    drop(agents);

    let initial_event = Event::default()
        .event("state_sync")
        .data(serde_json::to_string(&initial_state).unwrap_or_default());

    let stream = async_stream::stream! {
        yield Ok(initial_event);

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let event_type = match &event {
                        PlaygroundEvent::AgentCreated { .. } => "agent_created",
                        PlaygroundEvent::BalanceUpdated { .. } => "balance_updated",
                        PlaygroundEvent::TradeStarted { .. } => "trade_started",
                        PlaygroundEvent::LLMReasoning { .. } => "llm_reasoning",
                        PlaygroundEvent::TradeCompleted { .. } => "trade_completed",
                        PlaygroundEvent::TradeFailed { .. } => "trade_failed",
                        PlaygroundEvent::StateSync { .. } => "state_sync",
                        PlaygroundEvent::Error { .. } => "error",
                    };

                    yield Ok(Event::default()
                        .event(event_type)
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
