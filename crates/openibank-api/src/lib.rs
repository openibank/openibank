//! OpeniBank API - REST, gRPC, and WebSocket API
//!
//! Provides the API surface for OpeniBank:
//! - REST API at /api/v1/*
//! - WebSocket at /api/ws/*
//! - (Future) gRPC

pub mod routes;
pub mod handlers;
pub mod error;

use axum::Router;

/// Create the API router
pub fn create_router() -> Router {
    Router::new()
        .nest("/api/v1", routes::v1_routes())
        .nest("/api/ws", routes::ws_routes())
}
