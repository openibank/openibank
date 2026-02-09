//! OpeniBank REST API
//!
//! Production-grade REST API for the OpeniBank trading platform.
//! Binance-compatible endpoints for seamless integration with existing tools.
//!
//! # API Structure
//!
//! ```text
//! /api/v1/
//! ├── /auth          - Authentication (login, register, 2FA)
//! ├── /account       - Account management
//! ├── /wallet        - Wallet and balances
//! ├── /order         - Order management
//! ├── /trade         - Trade history
//! ├── /deposit       - Deposit operations
//! ├── /withdraw      - Withdrawal operations
//! ├── /market        - Market data
//! └── /ws            - WebSocket streams
//! ```
//!
//! # Authentication Methods
//!
//! - **Bearer Token**: JWT access token in Authorization header
//! - **API Key**: X-API-KEY header with HMAC-SHA256 signature
//! - **Session**: Cookie-based session authentication

pub mod error;
pub mod state;
pub mod routes;
pub mod handlers;
pub mod dto;
pub mod extractors;
pub mod middleware;
pub mod websocket;
pub mod openapi;

use axum::Router;
use axum::http::HeaderName;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub use error::{ApiError, ApiResult};
pub use state::AppState;

/// API configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Enable CORS for browser clients
    pub enable_cors: bool,
    /// Allowed origins for CORS
    pub cors_origins: Vec<String>,
    /// Enable request compression
    pub enable_compression: bool,
    /// Enable request tracing
    pub enable_tracing: bool,
    /// API rate limit (requests per minute)
    pub rate_limit: u32,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            enable_compression: true,
            enable_tracing: true,
            rate_limit: 1200,
            max_body_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Create the main API router with all middleware
pub fn create_router(state: Arc<AppState>, config: ApiConfig) -> Router {
    let mut router = Router::new()
        // API v1 routes
        .nest("/api/v1", routes::api_v1_routes())
        // Health check at root
        .route("/health", axum::routing::get(handlers::health::health_check))
        .route("/ready", axum::routing::get(handlers::health::readiness_check))
        // OpenAPI documentation
        .merge(routes::swagger_routes())
        // Shared state
        .with_state(state);

    // Add request ID middleware
    let x_request_id = HeaderName::from_static("x-request-id");
    router = router
        .layer(SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(x_request_id));

    // Add tracing
    if config.enable_tracing {
        router = router.layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");

                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id,
                    )
                }),
        );
    }

    // Add compression
    if config.enable_compression {
        router = router.layer(CompressionLayer::new());
    }

    // Add CORS
    if config.enable_cors {
        let cors = if config.cors_origins.contains(&"*".to_string()) {
            CorsLayer::permissive()
        } else {
            CorsLayer::new()
                .allow_origin(
                    config
                        .cors_origins
                        .iter()
                        .filter_map(|o| o.parse().ok())
                        .collect::<Vec<_>>(),
                )
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers(Any)
        };
        router = router.layer(cors);
    }

    router
}

/// Create a minimal router for testing
pub fn create_test_router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api/v1", routes::api_v1_routes())
        .route("/health", axum::routing::get(handlers::health::health_check))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert!(config.enable_cors);
        assert!(config.enable_compression);
        assert_eq!(config.rate_limit, 1200);
    }
}
