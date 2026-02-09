//! API Routes
//!
//! Route definitions for all API endpoints.

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use crate::handlers;
use crate::state::AppState;
use crate::websocket;

/// Create API v1 routes
pub fn api_v1_routes() -> Router<Arc<AppState>> {
    Router::new()
        // General endpoints
        .route("/ping", get(handlers::health::ping))
        .route("/time", get(handlers::health::server_time))
        // Exchange info
        .route("/exchangeInfo", get(handlers::market::get_exchange_info))
        // Auth routes
        .nest("/auth", auth_routes())
        // Account routes (requires auth)
        .nest("/account", account_routes())
        // Wallet/capital routes
        .nest("/capital", wallet_routes())
        // Order routes
        .route("/order", post(handlers::order::create_order))
        .route("/order", get(handlers::order::query_order))
        .route("/order", delete(handlers::order::cancel_order))
        .route("/openOrders", get(handlers::order::get_open_orders))
        .route("/openOrders", delete(handlers::order::cancel_all_orders))
        .route("/allOrders", get(handlers::order::get_all_orders))
        .route("/myTrades", get(handlers::order::get_account_trades))
        // Market data routes (public)
        .route("/depth", get(handlers::market::get_order_book))
        .route("/trades", get(handlers::market::get_recent_trades))
        .route("/aggTrades", get(handlers::market::get_agg_trades))
        .route("/klines", get(handlers::market::get_klines))
        .route("/ticker/24hr", get(handlers::market::get_ticker_24hr))
        .route("/ticker/price", get(handlers::market::get_price_ticker))
        .route("/ticker/bookTicker", get(handlers::market::get_book_ticker))
        .route("/avgPrice", get(handlers::market::get_avg_price))
        // WebSocket streams
        .nest("/ws", ws_routes())
}

/// WebSocket routes
fn ws_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Single stream endpoint
        .route("/", get(websocket::ws_market_handler))
        // Combined streams endpoint
        .route("/stream", get(websocket::ws_combined_handler))
        // User data stream
        .route("/user", get(websocket::ws_user_data_handler))
}

/// Authentication routes
fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/refresh", post(handlers::auth::refresh_token))
        .route("/logout", post(handlers::auth::logout))
        // 2FA
        .route("/2fa/setup", post(handlers::auth::setup_two_factor))
        .route("/2fa/verify", post(handlers::auth::verify_two_factor))
        .route("/2fa/disable", post(handlers::auth::disable_two_factor))
        // API Keys
        .route("/api-keys", post(handlers::auth::create_api_key))
        .route("/api-keys", get(handlers::auth::list_api_keys))
        .route("/api-keys", delete(handlers::auth::delete_api_key))
}

/// Account routes
fn account_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::account::get_account_info))
        .route("/", put(handlers::account::update_account))
        .route("/status", get(handlers::account::get_account_status))
        .route("/tradeFee", get(handlers::account::get_trade_fees))
        .route("/password", post(handlers::account::change_password))
        .route("/balances", get(handlers::account::get_balances))
        .route("/balance/{asset}", get(handlers::account::get_balance))
}

/// Wallet/capital routes
fn wallet_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Deposit
        .route("/deposit/address", get(handlers::wallet::get_deposit_address))
        .route("/deposit/hisrec", get(handlers::wallet::get_deposit_history))
        // Withdraw
        .route("/withdraw/apply", post(handlers::wallet::submit_withdrawal))
        .route("/withdraw/history", get(handlers::wallet::get_withdrawal_history))
        // Config
        .route("/config/getall", get(handlers::wallet::get_all_coins_info))
        // Transfer
        .route("/transfer", post(handlers::wallet::internal_transfer))
}

/// Create Swagger UI routes
pub fn swagger_routes() -> Router<Arc<AppState>> {
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;
    use crate::openapi::ApiDoc;

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", ApiDoc::openapi()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_routes_compile() {
        // Basic compilation test
        assert!(true);
    }
}
