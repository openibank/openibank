//! API routes

use axum::{routing::get, Router};

/// V1 API routes
pub fn v1_routes() -> Router {
    Router::new()
        .route("/health", get(health))
        // Wallets
        .route("/wallets", get(list_wallets))
        .route("/wallets/:id", get(get_wallet))
        // Transactions
        .route("/transactions", get(list_transactions))
        .route("/transactions/:id", get(get_transaction))
        // Permits
        .route("/permits", get(list_permits))
        .route("/permits/:id", get(get_permit))
        // Clearing
        .route("/clearing/batches", get(list_batches))
        // Marketplace
        .route("/marketplace/listings", get(list_listings))
        // Arena
        .route("/arena/challenges", get(list_challenges))
        // Receipts
        .route("/receipts/:id", get(get_receipt))
}

/// WebSocket routes
pub fn ws_routes() -> Router {
    Router::new()
        .route("/events", get(ws_events))
}

// Placeholder handlers
async fn health() -> &'static str { "ok" }
async fn list_wallets() -> &'static str { "[]" }
async fn get_wallet() -> &'static str { "{}" }
async fn list_transactions() -> &'static str { "[]" }
async fn get_transaction() -> &'static str { "{}" }
async fn list_permits() -> &'static str { "[]" }
async fn get_permit() -> &'static str { "{}" }
async fn list_batches() -> &'static str { "[]" }
async fn list_listings() -> &'static str { "[]" }
async fn list_challenges() -> &'static str { "[]" }
async fn get_receipt() -> &'static str { "{}" }
async fn ws_events() -> &'static str { "ws" }
