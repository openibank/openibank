//! Custom Axum Extractors
//!
//! Request extractors for authentication, pagination, and validation.

use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use uuid::Uuid;

use crate::error::{ApiError, ErrorResponse};

// =============================================================================
// Re-export auth types
// =============================================================================

pub use openibank_auth::types::{Permission, UserRole, FeeTier};

// =============================================================================
// Authenticated User Extractor
// =============================================================================

/// Authenticated user information extracted from request
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// User ID
    pub user_id: Uuid,
    /// User email
    pub email: String,
    /// User role
    pub role: UserRole,
    /// Permissions
    pub permissions: HashSet<Permission>,
    /// Session ID (if using session auth)
    pub session_id: Option<Uuid>,
    /// API key ID (if using API key auth)
    pub api_key_id: Option<Uuid>,
    /// Whether 2FA was verified for this request
    pub two_factor_verified: bool,
    /// User's fee tier
    pub fee_tier: FeeTier,
}

impl AuthenticatedUser {
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        // Admin has all permissions
        if self.permissions.contains(&Permission::Admin) {
            return true;
        }
        self.permissions.contains(permission)
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin || self.permissions.contains(&Permission::Admin)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get authenticated user from extensions (set by auth middleware)
        parts
            .extensions
            .get::<openibank_auth::types::AuthenticatedUser>()
            .cloned()
            .map(|u| AuthenticatedUser {
                user_id: u.user_id,
                email: u.email,
                role: u.role,
                permissions: u.permissions,
                session_id: u.session_id,
                api_key_id: u.api_key_id,
                two_factor_verified: u.two_factor_verified,
                fee_tier: u.fee_tier,
            })
            .ok_or_else(|| {
                let error = ApiError::Unauthorized;
                error_response(error)
            })
    }
}

// =============================================================================
// Optional User Extractor
// =============================================================================

/// Optional authenticated user (doesn't fail if not authenticated)
pub struct OptionalUser(pub Option<AuthenticatedUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<openibank_auth::types::AuthenticatedUser>()
            .cloned()
            .map(|u| AuthenticatedUser {
                user_id: u.user_id,
                email: u.email,
                role: u.role,
                permissions: u.permissions,
                session_id: u.session_id,
                api_key_id: u.api_key_id,
                two_factor_verified: u.two_factor_verified,
                fee_tier: u.fee_tier,
            });

        Ok(OptionalUser(user))
    }
}

// =============================================================================
// 2FA Required Extractor
// =============================================================================

/// Extractor that requires 2FA verification
pub struct Require2FA(pub AuthenticatedUser);

#[async_trait]
impl<S> FromRequestParts<S> for Require2FA
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<openibank_auth::types::AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| error_response(ApiError::Unauthorized))?;

        if !user.two_factor_verified {
            return Err(error_response(ApiError::TwoFactorRequired));
        }

        Ok(Require2FA(AuthenticatedUser {
            user_id: user.user_id,
            email: user.email,
            role: user.role,
            permissions: user.permissions,
            session_id: user.session_id,
            api_key_id: user.api_key_id,
            two_factor_verified: user.two_factor_verified,
            fee_tier: user.fee_tier,
        }))
    }
}

// =============================================================================
// Admin Required Extractor
// =============================================================================

/// Extractor that requires admin role
pub struct RequireAdmin(pub AuthenticatedUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<openibank_auth::types::AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| error_response(ApiError::Unauthorized))?;

        if user.role != UserRole::Admin && !user.permissions.contains(&Permission::Admin) {
            return Err(error_response(ApiError::Forbidden));
        }

        Ok(RequireAdmin(AuthenticatedUser {
            user_id: user.user_id,
            email: user.email,
            role: user.role,
            permissions: user.permissions,
            session_id: user.session_id,
            api_key_id: user.api_key_id,
            two_factor_verified: user.two_factor_verified,
            fee_tier: user.fee_tier,
        }))
    }
}

// =============================================================================
// Permission Required Extractor
// =============================================================================

/// Macro to create permission-specific extractors
#[macro_export]
macro_rules! require_permission {
    ($name:ident, $permission:expr) => {
        pub struct $name(pub AuthenticatedUser);

        #[async_trait::async_trait]
        impl<S> axum::extract::FromRequestParts<S> for $name
        where
            S: Send + Sync,
        {
            type Rejection = axum::response::Response;

            async fn from_request_parts(
                parts: &mut axum::http::request::Parts,
                _state: &S,
            ) -> Result<Self, Self::Rejection> {
                let user = parts
                    .extensions
                    .get::<openibank_auth::types::AuthenticatedUser>()
                    .cloned()
                    .ok_or_else(|| $crate::extractors::error_response($crate::error::ApiError::Unauthorized))?;

                if !user.has_permission(&$permission) && !user.is_admin() {
                    return Err($crate::extractors::error_response($crate::error::ApiError::Forbidden));
                }

                Ok($name(AuthenticatedUser {
                    user_id: user.user_id,
                    email: user.email,
                    role: user.role,
                    permissions: user.permissions,
                    session_id: user.session_id,
                    api_key_id: user.api_key_id,
                    two_factor_verified: user.two_factor_verified,
                    fee_tier: user.fee_tier,
                }))
            }
        }
    };
}

// Pre-defined permission extractors
require_permission!(RequireSpotTrade, Permission::SpotTrade);
require_permission!(RequireWithdraw, Permission::Withdraw);
require_permission!(RequireDeposit, Permission::Deposit);
require_permission!(RequireManageApiKeys, Permission::ManageApiKeys);

// =============================================================================
// Validated Query Extractor
// =============================================================================

/// Query extractor with validation
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + validator::Validate,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                error_response(ApiError::BadRequest(e.to_string()))
            })?;

        value.validate().map_err(|e| {
            error_response(ApiError::ValidationError(format_validation_errors(&e)))
        })?;

        Ok(ValidatedQuery(value))
    }
}

// =============================================================================
// Validated JSON Extractor
// =============================================================================

/// JSON extractor with validation
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + validator::Validate,
{
    type Rejection = Response;

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                error_response(ApiError::BadRequest(e.to_string()))
            })?;

        value.validate().map_err(|e| {
            error_response(ApiError::ValidationError(format_validation_errors(&e)))
        })?;

        Ok(ValidatedJson(value))
    }
}

// =============================================================================
// Client IP Extractor
// =============================================================================

/// Extract client IP from request
pub struct ClientIp(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        // Try common proxy headers
        let ip = headers
            .get("CF-Connecting-IP")
            .or_else(|| headers.get("X-Real-IP"))
            .or_else(|| headers.get("X-Forwarded-For"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(ClientIp(ip))
    }
}

// =============================================================================
// Request ID Extractor
// =============================================================================

/// Extract request ID from headers
pub struct RequestId(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        Ok(RequestId(request_id))
    }
}

// =============================================================================
// Pagination Extractor
// =============================================================================

/// Pagination parameters
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    50
}

impl PaginationParams {
    /// Get offset for database query
    pub fn offset(&self) -> i64 {
        ((self.page.saturating_sub(1)) * self.limit) as i64
    }

    /// Get limit clamped to maximum
    pub fn limit(&self, max: u32) -> i64 {
        self.limit.min(max) as i64
    }
}

/// Pagination extractor
pub struct Pagination(pub PaginationParams);

#[async_trait]
impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(params) = Query::<PaginationParams>::from_request_parts(parts, state)
            .await
            .map_err(|e| error_response(ApiError::BadRequest(e.to_string())))?;

        // Validate
        if params.page == 0 {
            return Err(error_response(ApiError::BadRequest(
                "Page must be >= 1".to_string(),
            )));
        }
        if params.limit == 0 || params.limit > 1000 {
            return Err(error_response(ApiError::BadRequest(
                "Limit must be between 1 and 1000".to_string(),
            )));
        }

        Ok(Pagination(params))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create error response from ApiError
pub fn error_response(error: ApiError) -> Response {
    let status = error.status_code();
    let response = ErrorResponse::from(&error);

    (status, Json(response)).into_response()
}

/// Format validation errors into a readable string
fn format_validation_errors(errors: &validator::ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, errs)| {
            errs.iter().map(move |e| {
                let message = e.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| {
                    format!("{}: validation failed", field)
                });
                message
            })
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams { page: 1, limit: 50 };
        assert_eq!(params.offset(), 0);

        let params = PaginationParams { page: 2, limit: 50 };
        assert_eq!(params.offset(), 50);

        let params = PaginationParams { page: 3, limit: 20 };
        assert_eq!(params.offset(), 40);
    }

    #[test]
    fn test_pagination_limit_clamped() {
        let params = PaginationParams { page: 1, limit: 500 };
        assert_eq!(params.limit(100), 100);

        let params = PaginationParams { page: 1, limit: 50 };
        assert_eq!(params.limit(100), 50);
    }
}
