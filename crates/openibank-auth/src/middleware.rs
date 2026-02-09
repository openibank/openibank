//! Authentication Middleware for Axum
//!
//! Production-grade middleware that:
//! - Extracts and validates authentication from requests
//! - Supports multiple auth methods (JWT, API key, Session)
//! - Provides authenticated user context to handlers
//! - Integrates with rate limiting

use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashSet;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use uuid::Uuid;

use crate::api_key::{extract_api_key_from_headers, extract_signature, extract_timestamp, extract_recv_window, ApiKeyService};
use crate::error::{AuthError, ErrorResponse};
use crate::jwt::JwtService;
use crate::session::SessionService;
use crate::types::{AuthenticatedUser, AuthMethod, FeeTier, Permission, SignedRequest, UserRole};

/// Authentication middleware layer
#[derive(Clone)]
pub struct AuthLayer {
    jwt: Arc<JwtService>,
    api_key: Arc<ApiKeyService>,
    session: Arc<SessionService>,
}

impl AuthLayer {
    /// Create a new authentication layer
    pub fn new(
        jwt: Arc<JwtService>,
        api_key: Arc<ApiKeyService>,
        session: Arc<SessionService>,
    ) -> Self {
        Self {
            jwt,
            api_key,
            session,
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            jwt: self.jwt.clone(),
            api_key: self.api_key.clone(),
            session: self.session.clone(),
        }
    }
}

/// Authentication middleware service
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    jwt: Arc<JwtService>,
    api_key: Arc<ApiKeyService>,
    session: Arc<SessionService>,
}

impl<S> Service<Request> for AuthMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let jwt = self.jwt.clone();
        let api_key = self.api_key.clone();
        let session = self.session.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Try to authenticate
            let auth_result = authenticate_request(
                req.headers(),
                req.uri().query().unwrap_or(""),
                &jwt,
                &api_key,
                &session,
            )
            .await;

            match auth_result {
                Ok(user) => {
                    // Add authenticated user to request extensions
                    let (mut parts, body) = req.into_parts();
                    parts.extensions.insert(user);
                    let req = Request::from_parts(parts, body);
                    inner.call(req).await
                }
                Err(AuthError::Unauthenticated) => {
                    // No auth provided - let the request through without user context
                    // Handler can decide if auth is required
                    inner.call(req).await
                }
                Err(e) => {
                    // Auth was provided but invalid
                    Ok(auth_error_response(e))
                }
            }
        })
    }
}

/// Authenticate a request using available methods
async fn authenticate_request(
    headers: &axum::http::HeaderMap,
    query: &str,
    jwt: &JwtService,
    api_key: &ApiKeyService,
    session: &SessionService,
) -> Result<AuthenticatedUser, AuthError> {
    // Try API key authentication first (for trading endpoints)
    if let Some(key) = extract_api_key_from_headers(headers) {
        return authenticate_api_key(&key, headers, query, api_key).await;
    }

    // Try JWT authentication
    if let Some(auth_header) = headers.get("Authorization") {
        let auth_str = auth_header.to_str().map_err(|_| AuthError::InvalidToken)?;
        if auth_str.starts_with("Bearer ") {
            let token = &auth_str[7..];
            return authenticate_jwt(token, jwt).await;
        }
    }

    // Try session token (from cookie or header)
    if let Some(session_token) = extract_session_token(headers) {
        return authenticate_session(&session_token, session).await;
    }

    Err(AuthError::Unauthenticated)
}

/// Authenticate using JWT
async fn authenticate_jwt(token: &str, jwt: &JwtService) -> Result<AuthenticatedUser, AuthError> {
    let claims = jwt.validate_access_token(token).await?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
    let session_id = claims.sid.as_ref().and_then(|s| Uuid::parse_str(s).ok());

    Ok(AuthenticatedUser {
        user_id,
        email: claims.email,
        role: claims.role,
        permissions: role_to_permissions(claims.role),
        auth_method: AuthMethod::Jwt,
        session_id,
        api_key_id: None,
        two_factor_verified: claims.two_factor_verified,
        fee_tier: FeeTier::Standard, // Would be loaded from DB
        ip_address: None,
        user_agent: None,
    })
}

/// Authenticate using API key
async fn authenticate_api_key(
    key: &str,
    headers: &axum::http::HeaderMap,
    query: &str,
    api_key_service: &ApiKeyService,
) -> Result<AuthenticatedUser, AuthError> {
    // Extract signature and timestamp
    let signature = extract_signature(query, headers).ok_or(AuthError::InvalidSignature)?;
    let timestamp = extract_timestamp(query).ok_or(AuthError::InvalidTimestamp)?;
    let recv_window = extract_recv_window(query);

    // Build signed request
    let signed_request = SignedRequest {
        api_key: key.to_string(),
        signature,
        timestamp,
        recv_window,
        query_string: query.to_string(),
        body: None, // Would be populated from request body for POST
    };

    // In production, we would:
    // 1. Look up the API key in the database
    // 2. Get the associated secret and user
    // 3. Verify the signature
    // 4. Check permissions and IP whitelist

    // For now, return an error since we don't have the secret
    // This would be completed when integrated with the database
    Err(AuthError::InvalidApiKey)
}

/// Authenticate using session token
async fn authenticate_session(
    token: &str,
    session: &SessionService,
) -> Result<AuthenticatedUser, AuthError> {
    let session_data = session.validate_session(token).await?;

    Ok(AuthenticatedUser {
        user_id: session_data.user_id,
        email: String::new(), // Would be loaded from DB
        role: UserRole::User, // Would be loaded from DB
        permissions: role_to_permissions(UserRole::User),
        auth_method: AuthMethod::Session,
        session_id: Some(session_data.id),
        api_key_id: None,
        two_factor_verified: session_data.two_factor_verified,
        fee_tier: FeeTier::Standard,
        ip_address: Some(session_data.ip_address),
        user_agent: Some(session_data.device.user_agent),
    })
}

/// Extract session token from headers/cookies
fn extract_session_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try X-Session-Token header first
    if let Some(token) = headers.get("X-Session-Token") {
        return token.to_str().ok().map(String::from);
    }

    // Try cookie
    if let Some(cookie_header) = headers.get("Cookie") {
        if let Ok(cookies) = cookie_header.to_str() {
            for cookie in cookies.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("session_token=") {
                    return Some(cookie[14..].to_string());
                }
            }
        }
    }

    None
}

/// Convert role to default permissions
fn role_to_permissions(role: UserRole) -> HashSet<Permission> {
    let mut permissions = HashSet::new();

    match role {
        UserRole::User => {
            permissions.insert(Permission::ReadAccount);
            permissions.insert(Permission::ReadMarketData);
            permissions.insert(Permission::ReadOrderBook);
        }
        UserRole::Verified => {
            permissions.insert(Permission::ReadAccount);
            permissions.insert(Permission::UpdateAccount);
            permissions.insert(Permission::ReadMarketData);
            permissions.insert(Permission::ReadOrderBook);
            permissions.insert(Permission::SpotTrade);
            permissions.insert(Permission::Deposit);
            permissions.insert(Permission::Withdraw);
            permissions.insert(Permission::ManageApiKeys);
        }
        UserRole::MarketMaker => {
            permissions.insert(Permission::ReadAccount);
            permissions.insert(Permission::UpdateAccount);
            permissions.insert(Permission::ReadMarketData);
            permissions.insert(Permission::ReadOrderBook);
            permissions.insert(Permission::SpotTrade);
            permissions.insert(Permission::MarginTrade);
            permissions.insert(Permission::Deposit);
            permissions.insert(Permission::Withdraw);
            permissions.insert(Permission::ManageApiKeys);
        }
        UserRole::Support => {
            permissions.insert(Permission::ReadAccount);
            permissions.insert(Permission::ReadMarketData);
            permissions.insert(Permission::ReadOrderBook);
            permissions.insert(Permission::ViewAuditLogs);
        }
        UserRole::Admin => {
            permissions.insert(Permission::Admin);
            // Admin has all permissions through Admin check
        }
    }

    permissions
}

/// Create error response for authentication errors
fn auth_error_response(error: AuthError) -> Response {
    let status = StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let response = ErrorResponse::from(&error);

    let mut res = Response::builder()
        .status(status)
        .header("Content-Type", "application/json");

    // Add Retry-After header for rate limiting
    if let Some(retry_after) = response.retry_after {
        res = res.header("Retry-After", retry_after.to_string());
    }

    res.body(Body::from(serde_json::to_string(&response).unwrap_or_default()))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// =============================================================================
// Axum Extractors
// =============================================================================

/// Extractor for authenticated user (optional)
/// Returns None if no valid authentication is present
pub struct OptionalUser(pub Option<AuthenticatedUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalUser(parts.extensions.get::<AuthenticatedUser>().cloned()))
    }
}

/// Extractor for required authenticated user
/// Returns 401 if not authenticated
pub struct RequireAuth(pub AuthenticatedUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .map(RequireAuth)
            .ok_or_else(|| auth_error_response(AuthError::Unauthenticated))
    }
}

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
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| auth_error_response(AuthError::Unauthenticated))?;

        if user.two_factor_verified {
            Ok(Require2FA(user))
        } else {
            Err(auth_error_response(AuthError::TwoFactorRequired))
        }
    }
}

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
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| auth_error_response(AuthError::Unauthenticated))?;

        if user.is_admin() {
            Ok(RequireAdmin(user))
        } else {
            Err(auth_error_response(AuthError::InsufficientPermissions))
        }
    }
}

/// Macro to create permission requirement extractors
#[macro_export]
macro_rules! require_permission {
    ($name:ident, $permission:expr) => {
        pub struct $name(pub AuthenticatedUser);

        #[async_trait]
        impl<S> FromRequestParts<S> for $name
        where
            S: Send + Sync,
        {
            type Rejection = Response;

            async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
                let user = parts
                    .extensions
                    .get::<AuthenticatedUser>()
                    .cloned()
                    .ok_or_else(|| auth_error_response(AuthError::Unauthenticated))?;

                if user.has_permission(&$permission) {
                    Ok($name(user))
                } else {
                    Err(auth_error_response(AuthError::InsufficientPermissions))
                }
            }
        }
    };
}

/// Helper middleware function for routes that require authentication
pub async fn require_auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    // Check if user is authenticated
    if req.extensions().get::<AuthenticatedUser>().is_none() {
        return Err(auth_error_response(AuthError::Unauthenticated));
    }
    Ok(next.run(req).await)
}

/// Helper middleware function for routes that require 2FA
pub async fn require_2fa_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let user = req
        .extensions()
        .get::<AuthenticatedUser>()
        .ok_or_else(|| auth_error_response(AuthError::Unauthenticated))?;

    if !user.two_factor_verified {
        return Err(auth_error_response(AuthError::TwoFactorRequired));
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_to_permissions() {
        let user_perms = role_to_permissions(UserRole::User);
        assert!(user_perms.contains(&Permission::ReadAccount));
        assert!(!user_perms.contains(&Permission::SpotTrade));

        let verified_perms = role_to_permissions(UserRole::Verified);
        assert!(verified_perms.contains(&Permission::SpotTrade));
        assert!(verified_perms.contains(&Permission::Withdraw));

        let admin_perms = role_to_permissions(UserRole::Admin);
        assert!(admin_perms.contains(&Permission::Admin));
    }

    #[test]
    fn test_extract_session_token_from_header() {
        use axum::http::HeaderMap;

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Token", "test-token-123".parse().unwrap());

        let token = extract_session_token(&headers);
        assert_eq!(token, Some("test-token-123".to_string()));
    }

    #[test]
    fn test_extract_session_token_from_cookie() {
        use axum::http::HeaderMap;

        let mut headers = HeaderMap::new();
        headers.insert("Cookie", "other=value; session_token=cookie-token; more=stuff".parse().unwrap());

        let token = extract_session_token(&headers);
        assert_eq!(token, Some("cookie-token".to_string()));
    }

    #[test]
    fn test_auth_error_response() {
        let response = auth_error_response(AuthError::InvalidToken);
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = auth_error_response(AuthError::InsufficientPermissions);
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = auth_error_response(AuthError::RateLimitExceeded { retry_after: 60 });
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
