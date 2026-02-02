//! OpeniBank Issuer Resonator Service
//!
//! A runnable HTTP service for the mock IUSD issuer.
//!
//! ## Endpoints
//!
//! - `POST /v1/issuer/init` - Initialize the issuer
//! - `POST /v1/issuer/mint` - Mint new IUSD
//! - `POST /v1/issuer/burn` - Burn IUSD
//! - `POST /v1/issuer/attest_reserve` - Attest reserve backing
//! - `GET /v1/issuer/supply` - Get current supply info
//! - `GET /v1/issuer/receipts` - Get recent receipts
//!
//! ## Usage
//!
//! ```bash
//! cargo run
//! curl http://localhost:3000/v1/issuer/supply
//! ```

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use openibank_core::{Amount, ResonatorId};
use openibank_issuer::{BurnIntent, Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state
struct AppState {
    issuer: RwLock<Option<Issuer>>,
    ledger: Arc<Ledger>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting OpeniBank Issuer Resonator...");

    // Create shared state
    let ledger = Arc::new(Ledger::new());
    let state = Arc::new(AppState {
        issuer: RwLock::new(None),
        ledger,
    });

    // Build router
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v1/issuer/init", post(init_issuer))
        .route("/v1/issuer/mint", post(mint))
        .route("/v1/issuer/burn", post(burn))
        .route("/v1/issuer/attest_reserve", post(attest_reserve))
        .route("/v1/issuer/supply", get(supply))
        .route("/v1/issuer/receipts", get(receipts))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ============================================================================
// Handlers
// ============================================================================

async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "OpeniBank Issuer Resonator",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Mock IUSD stablecoin issuer for AI agent banking",
        "endpoints": {
            "init": "POST /v1/issuer/init",
            "mint": "POST /v1/issuer/mint",
            "burn": "POST /v1/issuer/burn",
            "attest_reserve": "POST /v1/issuer/attest_reserve",
            "supply": "GET /v1/issuer/supply",
            "receipts": "GET /v1/issuer/receipts"
        }
    }))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "healthy"}))
}

// ============================================================================
// Init
// ============================================================================

#[derive(Debug, Deserialize)]
struct InitRequest {
    reserve_cap: u64,
}

#[derive(Debug, Serialize)]
struct InitResponse {
    success: bool,
    issuer_id: String,
    public_key: String,
    reserve_cap: u64,
}

async fn init_issuer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InitRequest>,
) -> Result<Json<InitResponse>, AppError> {
    let config = IssuerConfig::default();
    let issuer = Issuer::new(config.clone(), Amount::new(req.reserve_cap), state.ledger.clone());

    let public_key = issuer.public_key();

    let mut issuer_lock = state.issuer.write().await;
    *issuer_lock = Some(issuer);

    Ok(Json(InitResponse {
        success: true,
        issuer_id: config.issuer_id,
        public_key,
        reserve_cap: req.reserve_cap,
    }))
}

// ============================================================================
// Mint
// ============================================================================

#[derive(Debug, Deserialize)]
struct MintRequest {
    to: String,
    amount: u64,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct MintResponse {
    success: bool,
    receipt_id: String,
    amount: u64,
    new_supply: u64,
    signature: String,
}

async fn mint(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MintRequest>,
) -> Result<Json<MintResponse>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or_else(|| AppError::NotInitialized)?;

    let intent = MintIntent::new(
        ResonatorId::from_string(&req.to),
        Amount::new(req.amount),
        req.reason.unwrap_or_else(|| "Mint via API".to_string()),
    );

    let receipt = issuer.mint(intent).await.map_err(|e| AppError::IssuerError(e.to_string()))?;
    let new_supply = issuer.total_supply().await;

    Ok(Json(MintResponse {
        success: true,
        receipt_id: receipt.receipt_id,
        amount: req.amount,
        new_supply: new_supply.0,
        signature: receipt.signature,
    }))
}

// ============================================================================
// Burn
// ============================================================================

#[derive(Debug, Deserialize)]
struct BurnRequest {
    from: String,
    amount: u64,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct BurnResponse {
    success: bool,
    receipt_id: String,
    amount: u64,
    new_supply: u64,
    signature: String,
}

async fn burn(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BurnRequest>,
) -> Result<Json<BurnResponse>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or_else(|| AppError::NotInitialized)?;

    let intent = BurnIntent::new(
        ResonatorId::from_string(&req.from),
        Amount::new(req.amount),
        req.reason.unwrap_or_else(|| "Burn via API".to_string()),
    );

    let receipt = issuer.burn(intent).await.map_err(|e| AppError::IssuerError(e.to_string()))?;
    let new_supply = issuer.total_supply().await;

    Ok(Json(BurnResponse {
        success: true,
        receipt_id: receipt.receipt_id,
        amount: req.amount,
        new_supply: new_supply.0,
        signature: receipt.signature,
    }))
}

// ============================================================================
// Attest Reserve
// ============================================================================

#[derive(Debug, Deserialize)]
struct AttestReserveRequest {
    reserve_amount: u64,
    attestor: String,
}

#[derive(Debug, Serialize)]
struct AttestReserveResponse {
    success: bool,
    attestation_id: String,
    reserve_amount: u64,
    attestor: String,
}

async fn attest_reserve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AttestReserveRequest>,
) -> Result<Json<AttestReserveResponse>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or_else(|| AppError::NotInitialized)?;

    let attestation = issuer
        .attest_reserve(Amount::new(req.reserve_amount), req.attestor.clone())
        .await
        .map_err(|e| AppError::IssuerError(e.to_string()))?;

    Ok(Json(AttestReserveResponse {
        success: true,
        attestation_id: attestation.attestation_id,
        reserve_amount: req.reserve_amount,
        attestor: req.attestor,
    }))
}

// ============================================================================
// Supply
// ============================================================================

#[derive(Debug, Serialize)]
struct SupplyResponse {
    total_supply: u64,
    remaining_mintable: u64,
    is_halted: bool,
}

async fn supply(State(state): State<Arc<AppState>>) -> Result<Json<SupplyResponse>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or_else(|| AppError::NotInitialized)?;

    let total_supply = issuer.total_supply().await;
    let remaining = issuer.remaining_supply().await;
    let is_halted = issuer.is_halted().await;

    Ok(Json(SupplyResponse {
        total_supply: total_supply.0,
        remaining_mintable: remaining.0,
        is_halted,
    }))
}

// ============================================================================
// Receipts
// ============================================================================

#[derive(Debug, Deserialize)]
struct ReceiptsQuery {
    limit: Option<usize>,
}

async fn receipts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReceiptsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or_else(|| AppError::NotInitialized)?;

    let limit = query.limit.unwrap_or(10);
    let receipts = issuer.recent_receipts(limit).await;

    Ok(Json(serde_json::json!({
        "count": receipts.len(),
        "receipts": receipts
    })))
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
enum AppError {
    NotInitialized,
    IssuerError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotInitialized => (
                StatusCode::BAD_REQUEST,
                "Issuer not initialized. Call POST /v1/issuer/init first.".to_string(),
            ),
            AppError::IssuerError(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(serde_json::json!({
            "error": true,
            "message": message
        }));

        (status, body).into_response()
    }
}
