//! ResonanceX API - REST API Server
//!
//! This crate provides the REST API server for the ResonanceX exchange,
//! exposing endpoints for trading, market data, and account management.
//!
//! # Endpoints
//!
//! ## Market Data
//! - `GET /api/v1/markets` - List all markets
//! - `GET /api/v1/markets/:id` - Get market details
//! - `GET /api/v1/markets/:id/ticker` - Get ticker
//! - `GET /api/v1/markets/:id/depth` - Get orderbook depth
//! - `GET /api/v1/markets/:id/trades` - Get recent trades
//! - `GET /api/v1/markets/:id/candles` - Get OHLCV candles
//!
//! ## Trading
//! - `POST /api/v1/orders` - Submit an order
//! - `GET /api/v1/orders/:id` - Get order status
//! - `DELETE /api/v1/orders/:id` - Cancel an order
//! - `GET /api/v1/orders` - List open orders
//!
//! ## Account
//! - `GET /api/v1/account/balances` - Get account balances
//! - `GET /api/v1/account/trades` - Get trade history
//!
//! # Example
//!
//! ```ignore
//! use resonancex_api::{ApiServer, ApiConfig};
//!
//! let config = ApiConfig {
//!     bind_addr: "0.0.0.0:8080".parse().unwrap(),
//!     ..Default::default()
//! };
//!
//! let server = ApiServer::new(config, engine, market_data);
//! server.run().await?;
//! ```

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post, delete},
    extract::{Path, Query, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export core types
pub use resonancex_types::{
    MarketId, OrderId, Order, Trade, Side, OrderType, OrderStatus,
    Ticker, DepthSnapshot, Candle, CandleInterval,
};

/// API configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Address to bind to
    pub bind_addr: SocketAddr,
    /// Maximum request body size
    pub max_body_size: usize,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Rate limit (requests per minute)
    pub rate_limit: Option<u32>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            max_body_size: 1024 * 1024, // 1MB
            cors_enabled: true,
            rate_limit: Some(120),
        }
    }
}

/// API errors
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Rate limited")]
    RateLimited,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limited".into()),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = serde_json::json!({
            "error": true,
            "message": message,
        });

        (status, Json(body)).into_response()
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Order submission request
#[derive(Debug, Deserialize)]
pub struct SubmitOrderRequest {
    /// Market ID
    pub market: String,
    /// Order side (buy/sell)
    pub side: String,
    /// Order type (limit/market)
    pub order_type: String,
    /// Amount in base currency
    pub amount: Decimal,
    /// Price (required for limit orders)
    pub price: Option<Decimal>,
    /// Client order ID (optional)
    pub client_order_id: Option<String>,
    /// Time in force (GTC, IOC, FOK)
    pub time_in_force: Option<String>,
    /// Post-only flag
    pub post_only: Option<bool>,
}

/// Order response
#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub market: String,
    pub side: String,
    pub order_type: String,
    pub price: Option<Decimal>,
    pub amount: Decimal,
    pub filled: Decimal,
    pub remaining: Decimal,
    pub status: String,
    pub created_at: String,
}

/// Candle query parameters
#[derive(Debug, Deserialize)]
pub struct CandleQuery {
    /// Interval (1m, 5m, 15m, 1h, 4h, 1d)
    pub interval: String,
    /// Start timestamp (optional)
    pub start: Option<i64>,
    /// End timestamp (optional)
    pub end: Option<i64>,
    /// Limit (default 100, max 1000)
    pub limit: Option<usize>,
}

/// Depth query parameters
#[derive(Debug, Deserialize)]
pub struct DepthQuery {
    /// Number of levels (default 20, max 100)
    pub levels: Option<usize>,
}

/// Build the API router
pub fn build_router() -> Router {
    Router::new()
        // Market data endpoints
        .route("/api/v1/markets", get(list_markets))
        .route("/api/v1/markets/:id", get(get_market))
        .route("/api/v1/markets/:id/ticker", get(get_ticker))
        .route("/api/v1/markets/:id/depth", get(get_depth))
        .route("/api/v1/markets/:id/trades", get(get_trades))
        .route("/api/v1/markets/:id/candles", get(get_candles))
        // Trading endpoints
        .route("/api/v1/orders", post(submit_order))
        .route("/api/v1/orders/:id", get(get_order))
        .route("/api/v1/orders/:id", delete(cancel_order))
        // Health endpoint
        .route("/health", get(health_check))
}

// Handler functions (stubs that will be implemented with actual state)

async fn list_markets() -> impl IntoResponse {
    Json(ApiResponse::ok(Vec::<String>::new()))
}

async fn get_market(Path(id): Path<String>) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "id": id,
            "base": "ETH",
            "quote": "IUSD",
            "status": "active"
        }
    }))
}

async fn get_ticker(Path(id): Path<String>) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "market": id,
            "last_price": "3000.00",
            "bid": "2999.00",
            "ask": "3001.00",
            "volume_24h": "1000.00"
        }
    }))
}

async fn get_depth(
    Path(id): Path<String>,
    Query(params): Query<DepthQuery>,
) -> impl IntoResponse {
    let levels = params.levels.unwrap_or(20).min(100);
    Json(serde_json::json!({
        "success": true,
        "data": {
            "market": id,
            "bids": [],
            "asks": [],
            "levels": levels
        }
    }))
}

async fn get_trades(Path(id): Path<String>) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "market": id,
            "trades": []
        }
    }))
}

async fn get_candles(
    Path(id): Path<String>,
    Query(params): Query<CandleQuery>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "market": id,
            "interval": params.interval,
            "candles": []
        }
    }))
}

async fn submit_order(
    Json(req): Json<SubmitOrderRequest>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "id": uuid::Uuid::new_v4().to_string(),
            "market": req.market,
            "side": req.side,
            "order_type": req.order_type,
            "amount": req.amount.to_string(),
            "status": "pending"
        }
    }))
}

async fn get_order(Path(id): Path<String>) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "id": id,
            "status": "open"
        }
    }))
}

async fn cancel_order(Path(id): Path<String>) -> impl IntoResponse {
    Json(serde_json::json!({
        "success": true,
        "data": {
            "id": id,
            "cancelled": true
        }
    }))
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.bind_addr.port(), 8080);
        assert!(config.cors_enabled);
        assert_eq!(config.rate_limit, Some(120));
    }

    #[test]
    fn test_api_response() {
        let response = ApiResponse::ok(vec!["ETH_IUSD", "BTC_IUSD"]);
        assert!(response.success);
        assert_eq!(response.data.len(), 2);
    }

    #[test]
    fn test_submit_order_request() {
        let json = r#"{
            "market": "ETH_IUSD",
            "side": "buy",
            "order_type": "limit",
            "amount": "1.5",
            "price": "3000.00"
        }"#;

        let req: SubmitOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.market, "ETH_IUSD");
        assert_eq!(req.side, "buy");
        assert_eq!(req.amount, rust_decimal_macros::dec!(1.5));
    }

    #[test]
    fn test_build_router() {
        let router = build_router();
        // Router builds successfully
        assert!(true);
    }
}
