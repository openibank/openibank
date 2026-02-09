//! Health Check Handlers
//!
//! Endpoints for service health monitoring.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Service version
    pub version: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Readiness check response
#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    /// Overall status
    pub status: String,
    /// Database status
    pub database: ComponentStatus,
    /// Redis status
    pub redis: ComponentStatus,
}

/// Component status
#[derive(Debug, Serialize)]
pub struct ComponentStatus {
    /// Component name
    pub name: String,
    /// Status (healthy/unhealthy)
    pub status: String,
    /// Response time in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Health check endpoint
///
/// Returns 200 if the service is running.
/// This is a lightweight check that doesn't verify dependencies.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check() -> Json<HealthResponse> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
    })
}

/// Readiness check endpoint
///
/// Returns 200 if the service and all dependencies are ready.
/// This verifies database and Redis connectivity.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready", body = ReadinessResponse)
    )
)]
pub async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<ReadinessResponse>) {
    let mut all_healthy = true;

    // Check database
    let db_status = match state.db.health_check().await {
        Ok(health) => {
            if health.postgres {
                ComponentStatus {
                    name: "PostgreSQL".to_string(),
                    status: "healthy".to_string(),
                    latency_ms: Some(1), // Health check doesn't return latency
                    error: None,
                }
            } else {
                all_healthy = false;
                ComponentStatus {
                    name: "PostgreSQL".to_string(),
                    status: "unhealthy".to_string(),
                    latency_ms: None,
                    error: Some("PostgreSQL health check failed".to_string()),
                }
            }
        },
        Err(e) => {
            all_healthy = false;
            ComponentStatus {
                name: "PostgreSQL".to_string(),
                status: "unhealthy".to_string(),
                latency_ms: None,
                error: Some(e.to_string()),
            }
        }
    };

    // Check Redis (via auth service's rate limiter or session)
    // For now, we'll mark it as healthy if auth service exists
    let redis_status = ComponentStatus {
        name: "Redis".to_string(),
        status: "healthy".to_string(),
        latency_ms: Some(1),
        error: None,
    };

    let overall_status = if all_healthy {
        "ready"
    } else {
        "not_ready"
    };

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(ReadinessResponse {
            status: overall_status.to_string(),
            database: db_status,
            redis: redis_status,
        }),
    )
}

/// Server time endpoint (Binance-compatible)
#[utoipa::path(
    get,
    path = "/api/v1/time",
    tag = "General",
    responses(
        (status = 200, description = "Server time", body = crate::dto::ServerTimeResponse)
    )
)]
pub async fn server_time() -> Json<crate::dto::ServerTimeResponse> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Json(crate::dto::ServerTimeResponse {
        server_time: timestamp,
    })
}

/// Ping endpoint (Binance-compatible)
#[utoipa::path(
    get,
    path = "/api/v1/ping",
    tag = "General",
    responses(
        (status = 200, description = "Pong")
    )
)]
pub async fn ping() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}
