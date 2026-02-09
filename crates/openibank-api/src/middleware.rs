//! API Middleware
//!
//! Middleware components for the API layer.

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

use crate::error::{ApiError, ErrorResponse};
use crate::extractors::ClientIp;
use crate::state::AppState;

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    // Check IP rate limit
    match state.auth.rate_limiter.check_ip_limit(&ip).await {
        Ok(_) => Ok(next.run(req).await),
        Err(e) => {
            let error = ApiError::from(e);
            let status = error.status_code();
            let response = ErrorResponse::from(&error);

            let mut res = Response::builder()
                .status(status)
                .header("Content-Type", "application/json");

            // Add Retry-After header for rate limit errors
            if let ApiError::TooManyRequests { retry_after } = &error {
                res = res.header("Retry-After", retry_after.to_string());
            }

            Err(res.body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
                .unwrap_or_else(|_| Response::new(Body::empty())))
        }
    }
}

/// Request timing middleware
pub async fn timing_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    let elapsed = start.elapsed();

    // Log slow requests
    if elapsed.as_millis() > 1000 {
        tracing::warn!(
            method = %method,
            uri = %uri,
            elapsed_ms = elapsed.as_millis(),
            "Slow request detected"
        );
    } else {
        tracing::debug!(
            method = %method,
            uri = %uri,
            elapsed_ms = elapsed.as_millis(),
            status = response.status().as_u16(),
            "Request completed"
        );
    }

    response
}

/// Authentication requirement middleware
pub async fn require_auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    // Check if user is authenticated (set by AuthLayer)
    if req.extensions().get::<openibank_auth::types::AuthenticatedUser>().is_none() {
        let error = ApiError::Unauthorized;
        let status = StatusCode::UNAUTHORIZED;
        let response = ErrorResponse::from(&error);

        return Err(Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
            .unwrap_or_else(|_| Response::new(Body::empty())));
    }

    Ok(next.run(req).await)
}

/// 2FA requirement middleware
pub async fn require_2fa_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let user = req
        .extensions()
        .get::<openibank_auth::types::AuthenticatedUser>()
        .ok_or_else(|| {
            let error = ApiError::Unauthorized;
            let response = ErrorResponse::from(&error);
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        })?;

    if !user.two_factor_verified {
        let error = ApiError::TwoFactorRequired;
        let response = ErrorResponse::from(&error);
        return Err(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
            .unwrap_or_else(|_| Response::new(Body::empty())));
    }

    Ok(next.run(req).await)
}

/// Trading permission middleware
pub async fn require_trading_middleware(
    State(_state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let user = req
        .extensions()
        .get::<openibank_auth::types::AuthenticatedUser>()
        .ok_or_else(|| {
            let error = ApiError::Unauthorized;
            let response = ErrorResponse::from(&error);
            Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        })?;

    // Check if user can trade
    if !user.has_permission(&openibank_auth::types::Permission::SpotTrade) && !user.is_admin() {
        let error = ApiError::TradingDisabled;
        let response = ErrorResponse::from(&error);
        return Err(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
            .unwrap_or_else(|_| Response::new(Body::empty())));
    }

    // Note: Additional database verification would require proper user repository
    // This is a stub - full implementation would check user.is_verified from DB

    Ok(next.run(req).await)
}

/// Request body size limit middleware
pub async fn body_limit_middleware(
    req: Request,
    next: Next,
    max_size: usize,
) -> Result<Response, Response> {
    // Check Content-Length header if present
    if let Some(content_length) = req.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > max_size {
                    let error = ApiError::BadRequest(format!(
                        "Request body too large. Maximum size is {} bytes",
                        max_size
                    ));
                    let response = ErrorResponse::from(&error);
                    return Err(Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .header("Content-Type", "application/json")
                        .body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
                        .unwrap_or_else(|_| Response::new(Body::empty())));
                }
            }
        }
    }

    Ok(next.run(req).await)
}

/// Security headers middleware
pub async fn security_headers_middleware(
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();

    // Security headers
    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().unwrap(),
    );
    headers.insert(
        "X-Frame-Options",
        "DENY".parse().unwrap(),
    );
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap(),
    );
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "Cache-Control",
        "no-store, no-cache, must-revalidate".parse().unwrap(),
    );
    headers.insert(
        "Pragma",
        "no-cache".parse().unwrap(),
    );

    response
}

/// CORS preflight handler
pub async fn cors_preflight() -> Response {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization, X-MBX-APIKEY, X-Request-ID")
        .header("Access-Control-Max-Age", "86400")
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}
