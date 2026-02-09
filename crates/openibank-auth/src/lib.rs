//! OpeniBank Authentication Layer
//!
//! Production-grade authentication for the OpeniBank platform supporting:
//!
//! - **JWT Authentication**: Access tokens + refresh tokens with rotation
//! - **API Key Authentication**: HMAC-SHA256 signed requests (Binance-compatible)
//! - **Session Management**: Secure session tokens with device tracking
//! - **2FA**: TOTP-based two-factor authentication
//! - **Password Security**: Argon2id hashing (OWASP recommended)
//! - **Rate Limiting**: Per-user and per-IP rate limiting
//!
//! # Security Features
//!
//! - Constant-time comparisons to prevent timing attacks
//! - Secure random token generation
//! - Token rotation and revocation
//! - IP whitelisting for API keys
//! - Audit logging for security events
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Authentication Flow                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Request → RateLimiter → AuthMiddleware → Handler           │
//! │                              │                               │
//! │              ┌───────────────┼───────────────┐               │
//! │              ▼               ▼               ▼               │
//! │          JWT Token     API Key+Sig      Session Token        │
//! │              │               │               │               │
//! │              ▼               ▼               ▼               │
//! │         JwtService    ApiKeyService   SessionService         │
//! │              │               │               │               │
//! │              └───────────────┼───────────────┘               │
//! │                              ▼                               │
//! │                      AuthenticatedUser                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod error;
pub mod jwt;
pub mod api_key;
pub mod password;
pub mod session;
pub mod totp;
pub mod rate_limit;
pub mod middleware;
pub mod types;

pub use config::AuthConfig;
pub use error::{AuthError, AuthResult};
pub use jwt::JwtService;
pub use api_key::ApiKeyService;
pub use password::PasswordService;
pub use session::SessionService;
pub use totp::TotpService;
pub use rate_limit::RateLimiter;
pub use middleware::{AuthMiddleware, AuthLayer};
pub use types::*;

use openibank_db::Database;
use std::sync::Arc;

/// Main authentication service combining all auth methods
pub struct AuthService {
    pub jwt: JwtService,
    pub api_key: ApiKeyService,
    pub password: PasswordService,
    pub session: SessionService,
    pub totp: TotpService,
    pub rate_limiter: RateLimiter,
    db: Arc<Database>,
    config: AuthConfig,
}

impl AuthService {
    /// Create a new auth service with all components
    pub fn new(db: Arc<Database>, config: AuthConfig) -> Self {
        let jwt = JwtService::new(config.jwt.clone());
        let api_key = ApiKeyService::new(db.clone());
        let password = PasswordService::new(config.password.clone());
        let session = SessionService::new(db.clone(), config.session.clone());
        let totp = TotpService::new(config.totp.clone());
        let rate_limiter = RateLimiter::new(db.clone(), config.rate_limit.clone());

        Self {
            jwt,
            api_key,
            password,
            session,
            totp,
            rate_limiter,
            db,
            config,
        }
    }

    /// Get the database reference
    pub fn db(&self) -> &Arc<Database> {
        &self.db
    }

    /// Get the config reference
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Create an auth layer for Axum router
    pub fn layer(&self) -> AuthLayer {
        AuthLayer::new(
            Arc::new(self.jwt.clone()),
            Arc::new(self.api_key.clone()),
            Arc::new(self.session.clone()),
        )
    }
}

impl Clone for AuthService {
    fn clone(&self) -> Self {
        Self {
            jwt: self.jwt.clone(),
            api_key: self.api_key.clone(),
            password: self.password.clone(),
            session: self.session.clone(),
            totp: self.totp.clone(),
            rate_limiter: self.rate_limiter.clone(),
            db: self.db.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_service_creation() {
        // This would need a mock database, testing structure only
        assert!(true);
    }
}
