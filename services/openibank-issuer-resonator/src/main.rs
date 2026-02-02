//! OpeniBank Issuer Resonator Service
//!
//! A runnable HTTP service for the mock IUSD issuer with full ledger access.
//!
//! ## Endpoints
//!
//! ### Issuer Operations
//! - `POST /v1/issuer/init` - Initialize the issuer
//! - `POST /v1/issuer/mint` - Mint new IUSD
//! - `POST /v1/issuer/burn` - Burn IUSD
//! - `POST /v1/issuer/attest_reserve` - Attest reserve backing
//! - `GET /v1/issuer/supply` - Get current supply info
//! - `GET /v1/issuer/receipts` - Get recent receipts
//! - `POST /v1/issuer/halt` - Halt the issuer (emergency stop)
//! - `POST /v1/issuer/resume` - Resume the issuer
//! - `GET /v1/issuer/policy` - Get issuance policy
//! - `POST /v1/issuer/policy` - Update issuance policy
//!
//! ### Ledger Operations
//! - `GET /v1/ledger/balance/:account` - Get account balance
//! - `GET /v1/ledger/entries/:account` - Get account history
//! - `GET /v1/ledger/accounts` - List all accounts
//! - `GET /v1/ledger/recent` - Get recent ledger entries
//! - `POST /v1/ledger/transfer` - Transfer between accounts (commitment-gated)
//!
//! ### Receipt Operations
//! - `POST /v1/receipts/verify` - Verify a receipt signature
//!
//! ## Usage
//!
//! ```bash
//! cargo run
//! curl http://localhost:3000/v1/issuer/supply
//! ```

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use openibank_core::{
    crypto::Keypair, Amount, AssetClass, AssetId, BudgetPolicy, CommitmentGate, ConsequenceRef,
    CounterpartyConstraint, PaymentIntent, PermitId, ResonatorId, SpendPermit, SpendPurpose,
};
use openibank_issuer::{BurnIntent, IssuancePolicy, Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state
struct AppState {
    issuer: RwLock<Option<Issuer>>,
    ledger: Arc<Ledger>,
    commitment_gate: CommitmentGate,
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
    let keypair = Keypair::generate();
    let commitment_gate = CommitmentGate::new(keypair);

    let state = Arc::new(AppState {
        issuer: RwLock::new(None),
        ledger,
        commitment_gate,
    });

    // Build router
    let app = Router::new()
        // Root and health
        .route("/", get(root))
        .route("/health", get(health))
        // Issuer endpoints
        .route("/v1/issuer/init", post(init_issuer))
        .route("/v1/issuer/mint", post(mint))
        .route("/v1/issuer/burn", post(burn))
        .route("/v1/issuer/attest_reserve", post(attest_reserve))
        .route("/v1/issuer/supply", get(supply))
        .route("/v1/issuer/receipts", get(receipts))
        .route("/v1/issuer/halt", post(halt))
        .route("/v1/issuer/resume", post(resume))
        .route("/v1/issuer/policy", get(get_policy))
        .route("/v1/issuer/policy", post(update_policy))
        // Ledger endpoints
        .route("/v1/ledger/balance/:account", get(get_balance))
        .route("/v1/ledger/entries/:account", get(get_entries))
        .route("/v1/ledger/accounts", get(list_accounts))
        .route("/v1/ledger/recent", get(recent_entries))
        .route("/v1/ledger/transfer", post(transfer))
        // Receipt endpoints
        .route("/v1/receipts/verify", post(verify_receipt))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ============================================================================
// Root Handlers
// ============================================================================

async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "OpeniBank Issuer Resonator",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Mock IUSD stablecoin issuer for AI agent banking",
        "endpoints": {
            "issuer": {
                "init": "POST /v1/issuer/init",
                "mint": "POST /v1/issuer/mint",
                "burn": "POST /v1/issuer/burn",
                "attest_reserve": "POST /v1/issuer/attest_reserve",
                "supply": "GET /v1/issuer/supply",
                "receipts": "GET /v1/issuer/receipts",
                "halt": "POST /v1/issuer/halt",
                "resume": "POST /v1/issuer/resume",
                "policy": "GET/POST /v1/issuer/policy"
            },
            "ledger": {
                "balance": "GET /v1/ledger/balance/:account",
                "entries": "GET /v1/ledger/entries/:account",
                "accounts": "GET /v1/ledger/accounts",
                "recent": "GET /v1/ledger/recent",
                "transfer": "POST /v1/ledger/transfer"
            },
            "receipts": {
                "verify": "POST /v1/receipts/verify"
            }
        }
    }))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "healthy"}))
}

// ============================================================================
// Issuer Handlers
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
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

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
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

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
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

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

#[derive(Debug, Serialize)]
struct SupplyResponse {
    total_supply: u64,
    remaining_mintable: u64,
    is_halted: bool,
}

async fn supply(State(state): State<Arc<AppState>>) -> Result<Json<SupplyResponse>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

    let total_supply = issuer.total_supply().await;
    let remaining = issuer.remaining_supply().await;
    let is_halted = issuer.is_halted().await;

    Ok(Json(SupplyResponse {
        total_supply: total_supply.0,
        remaining_mintable: remaining.0,
        is_halted,
    }))
}

#[derive(Debug, Deserialize)]
struct ReceiptsQuery {
    limit: Option<usize>,
}

async fn receipts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReceiptsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

    let limit = query.limit.unwrap_or(10);
    let receipts = issuer.recent_receipts(limit).await;

    Ok(Json(serde_json::json!({
        "count": receipts.len(),
        "receipts": receipts
    })))
}

#[derive(Debug, Deserialize)]
struct HaltRequest {
    reason: String,
}

async fn halt(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HaltRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

    issuer.halt(&req.reason).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Issuer halted: {}", req.reason)
    })))
}

async fn resume(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

    issuer.resume().await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Issuer resumed"
    })))
}

async fn get_policy(State(state): State<Arc<AppState>>) -> Result<Json<IssuancePolicy>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

    let policy = issuer.policy().await;
    Ok(Json(policy))
}

#[derive(Debug, Deserialize)]
struct UpdatePolicyRequest {
    minting_enabled: Option<bool>,
    burning_enabled: Option<bool>,
    max_single_mint: Option<u64>,
    max_single_burn: Option<u64>,
}

async fn update_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdatePolicyRequest>,
) -> Result<Json<IssuancePolicy>, AppError> {
    let issuer_lock = state.issuer.read().await;
    let issuer = issuer_lock.as_ref().ok_or(AppError::NotInitialized)?;

    let mut policy = issuer.policy().await;

    if let Some(enabled) = req.minting_enabled {
        policy.minting_enabled = enabled;
    }
    if let Some(enabled) = req.burning_enabled {
        policy.burning_enabled = enabled;
    }
    if let Some(max) = req.max_single_mint {
        policy.max_single_mint = Amount::new(max);
    }
    if let Some(max) = req.max_single_burn {
        policy.max_single_burn = Amount::new(max);
    }

    issuer.update_policy(policy.clone()).await;

    Ok(Json(policy))
}

// ============================================================================
// Ledger Handlers
// ============================================================================

#[derive(Debug, Serialize)]
struct BalanceResponse {
    account: String,
    asset: String,
    balance: u64,
    formatted: String,
}

async fn get_balance(
    State(state): State<Arc<AppState>>,
    Path(account): Path<String>,
) -> Result<Json<BalanceResponse>, AppError> {
    let account_id = ResonatorId::from_string(&account);
    let asset = AssetId::iusd();
    let balance = state.ledger.balance(&account_id, &asset).await;

    Ok(Json(BalanceResponse {
        account,
        asset: "IUSD".to_string(),
        balance: balance.0,
        formatted: format!("{}", balance),
    }))
}

#[derive(Debug, Deserialize)]
struct EntriesQuery {
    limit: Option<usize>,
}

async fn get_entries(
    State(state): State<Arc<AppState>>,
    Path(account): Path<String>,
    Query(query): Query<EntriesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let account_id = ResonatorId::from_string(&account);
    let entries = state.ledger.account_entries(&account_id).await;

    let limit = query.limit.unwrap_or(50);
    let entries: Vec<_> = entries.into_iter().rev().take(limit).collect();

    Ok(Json(serde_json::json!({
        "account": account,
        "count": entries.len(),
        "entries": entries
    })))
}

async fn list_accounts(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, AppError> {
    let accounts = state.ledger.all_accounts().await;
    let asset = AssetId::iusd();

    let mut account_list = Vec::new();
    for account in accounts {
        let balance = state.ledger.balance(&account, &asset).await;
        account_list.push(serde_json::json!({
            "account": account.0,
            "balance": balance.0,
            "formatted": format!("{}", balance)
        }));
    }

    Ok(Json(serde_json::json!({
        "count": account_list.len(),
        "accounts": account_list
    })))
}

async fn recent_entries(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EntriesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = query.limit.unwrap_or(20);
    let entries = state.ledger.recent_entries(limit).await;

    Ok(Json(serde_json::json!({
        "count": entries.len(),
        "entries": entries
    })))
}

#[derive(Debug, Deserialize)]
struct TransferRequest {
    from: String,
    to: String,
    amount: u64,
    purpose: Option<String>,
}

#[derive(Debug, Serialize)]
struct TransferResponse {
    success: bool,
    commitment_id: String,
    from: String,
    to: String,
    amount: u64,
    signature: String,
}

async fn transfer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<TransferResponse>, AppError> {
    let from = ResonatorId::from_string(&req.from);
    let to = ResonatorId::from_string(&req.to);
    let amount = Amount::new(req.amount);
    let asset = AssetId::iusd();

    // Create a budget and permit for this transfer
    let budget = BudgetPolicy::new(from.clone(), Amount::new(u64::MAX));
    let permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: from.clone(),
        bound_budget: budget.budget_id.clone(),
        asset_class: AssetClass::Stablecoin,
        max_amount: amount,
        remaining: amount,
        counterparty: CounterpartyConstraint::Specific(to.clone()),
        purpose: SpendPurpose {
            category: "transfer".to_string(),
            description: req.purpose.clone().unwrap_or_else(|| "API transfer".to_string()),
        },
        issued_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        signature: "api_transfer".to_string(),
    };

    // Create payment intent
    let intent = PaymentIntent::new(
        from.clone(),
        permit.permit_id.clone(),
        to.clone(),
        amount,
        asset.clone(),
        SpendPurpose {
            category: "transfer".to_string(),
            description: req.purpose.unwrap_or_else(|| "API transfer".to_string()),
        },
    );

    // Create commitment
    let consequence = ConsequenceRef {
        consequence_type: "ledger_transfer".to_string(),
        reference_id: intent.intent_id.0.clone(),
        metadata: serde_json::json!({
            "from": req.from,
            "to": req.to,
            "amount": req.amount
        }),
    };

    let (receipt, _evidence) = state
        .commitment_gate
        .create_commitment(&intent, &permit, &budget, consequence)
        .map_err(|e| AppError::TransferError(e.to_string()))?;

    // Execute the transfer on the ledger
    state
        .ledger
        .transfer(&from, &to, &asset, amount, &receipt)
        .await
        .map_err(|e| AppError::TransferError(e.to_string()))?;

    Ok(Json(TransferResponse {
        success: true,
        commitment_id: receipt.commitment_id.0,
        from: req.from,
        to: req.to,
        amount: req.amount,
        signature: receipt.signature,
    }))
}

// ============================================================================
// Receipt Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
struct VerifyReceiptRequest {
    receipt: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct VerifyReceiptResponse {
    valid: bool,
    message: String,
    receipt_id: Option<String>,
}

async fn verify_receipt(
    Json(req): Json<VerifyReceiptRequest>,
) -> Result<Json<VerifyReceiptResponse>, AppError> {
    // Try to parse as IssuerReceipt
    if let Ok(issuer_receipt) = serde_json::from_value::<openibank_core::IssuerReceipt>(req.receipt.clone()) {
        match issuer_receipt.verify() {
            Ok(()) => {
                return Ok(Json(VerifyReceiptResponse {
                    valid: true,
                    message: "IssuerReceipt signature is valid".to_string(),
                    receipt_id: Some(issuer_receipt.receipt_id),
                }));
            }
            Err(e) => {
                return Ok(Json(VerifyReceiptResponse {
                    valid: false,
                    message: format!("IssuerReceipt signature verification failed: {}", e),
                    receipt_id: Some(issuer_receipt.receipt_id),
                }));
            }
        }
    }

    // Try to parse as CommitmentReceipt
    if let Ok(commitment_receipt) =
        serde_json::from_value::<openibank_core::CommitmentReceipt>(req.receipt.clone())
    {
        match commitment_receipt.verify() {
            Ok(()) => {
                return Ok(Json(VerifyReceiptResponse {
                    valid: true,
                    message: "CommitmentReceipt signature is valid".to_string(),
                    receipt_id: Some(commitment_receipt.commitment_id.0),
                }));
            }
            Err(e) => {
                return Ok(Json(VerifyReceiptResponse {
                    valid: false,
                    message: format!("CommitmentReceipt signature verification failed: {}", e),
                    receipt_id: Some(commitment_receipt.commitment_id.0),
                }));
            }
        }
    }

    Ok(Json(VerifyReceiptResponse {
        valid: false,
        message: "Unable to parse receipt as IssuerReceipt or CommitmentReceipt".to_string(),
        receipt_id: None,
    }))
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
enum AppError {
    NotInitialized,
    IssuerError(String),
    TransferError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotInitialized => (
                StatusCode::BAD_REQUEST,
                "Issuer not initialized. Call POST /v1/issuer/init first.".to_string(),
            ),
            AppError::IssuerError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::TransferError(msg) => (StatusCode::BAD_REQUEST, format!("Transfer failed: {}", msg)),
        };

        let body = Json(serde_json::json!({
            "error": true,
            "message": message
        }));

        (status, body).into_response()
    }
}
