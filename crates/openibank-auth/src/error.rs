//! Authentication error types
//!
//! Comprehensive error handling for all authentication operations.
//! Errors are designed to be:
//! - Informative for logging/debugging
//! - Safe for external exposure (no sensitive data leakage)
//! - Convertible to HTTP status codes

use thiserror::Error;

/// Result type alias for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication error types
#[derive(Debug, Error)]
pub enum AuthError {
    // =========================================================================
    // Token Errors
    // =========================================================================
    /// Token has expired
    #[error("Token has expired")]
    TokenExpired,

    /// Token is invalid (malformed, wrong signature, etc.)
    #[error("Invalid token")]
    InvalidToken,

    /// Token has been revoked
    #[error("Token has been revoked")]
    TokenRevoked,

    /// Refresh token is invalid or expired
    #[error("Invalid refresh token")]
    InvalidRefreshToken,

    /// Token type mismatch (expected access, got refresh, etc.)
    #[error("Invalid token type")]
    InvalidTokenType,

    // =========================================================================
    // Credential Errors
    // =========================================================================
    /// Invalid credentials (username/password)
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Invalid API signature
    #[error("Invalid signature")]
    InvalidSignature,

    /// Request timestamp is too old or in the future
    #[error("Invalid timestamp")]
    InvalidTimestamp,

    /// Nonce has been used before (replay attack prevention)
    #[error("Nonce already used")]
    NonceReused,

    // =========================================================================
    // Password Errors
    // =========================================================================
    /// Password does not meet requirements
    #[error("Password does not meet requirements: {0}")]
    WeakPassword(String),

    /// Password hash verification failed
    #[error("Password verification failed")]
    PasswordVerificationFailed,

    /// Password hashing failed
    #[error("Password hashing failed")]
    PasswordHashingFailed,

    // =========================================================================
    // 2FA Errors
    // =========================================================================
    /// Two-factor authentication is required
    #[error("Two-factor authentication required")]
    TwoFactorRequired,

    /// Invalid 2FA code
    #[error("Invalid 2FA code")]
    InvalidTwoFactorCode,

    /// 2FA is not enabled for this account
    #[error("2FA not enabled")]
    TwoFactorNotEnabled,

    /// 2FA is already enabled
    #[error("2FA already enabled")]
    TwoFactorAlreadyEnabled,

    /// Invalid backup code
    #[error("Invalid backup code")]
    InvalidBackupCode,

    // =========================================================================
    // Session Errors
    // =========================================================================
    /// Session not found
    #[error("Session not found")]
    SessionNotFound,

    /// Session has expired
    #[error("Session has expired")]
    SessionExpired,

    /// Maximum sessions exceeded
    #[error("Maximum sessions exceeded")]
    MaxSessionsExceeded,

    /// Session invalidated (e.g., password change)
    #[error("Session has been invalidated")]
    SessionInvalidated,

    // =========================================================================
    // Rate Limiting Errors
    // =========================================================================
    /// Rate limit exceeded
    #[error("Rate limit exceeded, try again in {retry_after} seconds")]
    RateLimitExceeded {
        /// Seconds until the rate limit resets
        retry_after: u64,
    },

    /// Account is locked due to too many failed attempts
    #[error("Account is locked, try again in {retry_after} seconds")]
    AccountLocked {
        /// Seconds until the account is unlocked
        retry_after: u64,
    },

    // =========================================================================
    // Permission Errors
    // =========================================================================
    /// User is not authenticated
    #[error("Authentication required")]
    Unauthenticated,

    /// User does not have required permissions
    #[error("Insufficient permissions")]
    InsufficientPermissions,

    /// API key does not have required permissions
    #[error("API key lacks required permissions")]
    ApiKeyPermissionDenied,

    /// IP address is not whitelisted
    #[error("IP address not allowed")]
    IpNotWhitelisted,

    // =========================================================================
    // User State Errors
    // =========================================================================
    /// User account is disabled
    #[error("Account is disabled")]
    AccountDisabled,

    /// User account is not verified
    #[error("Account not verified")]
    AccountNotVerified,

    /// User not found
    #[error("User not found")]
    UserNotFound,

    // =========================================================================
    // API Key Errors
    // =========================================================================
    /// Maximum API keys reached
    #[error("Maximum API keys limit reached")]
    MaxApiKeysExceeded,

    /// API key not found
    #[error("API key not found")]
    ApiKeyNotFound,

    /// API key has expired
    #[error("API key has expired")]
    ApiKeyExpired,

    // =========================================================================
    // Internal Errors
    // =========================================================================
    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Redis/cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Cryptographic operation failed
    #[error("Cryptographic error")]
    CryptoError,

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal error (should not be exposed to clients)
    #[error("Internal error")]
    Internal(String),
}

impl AuthError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            // 400 Bad Request
            Self::WeakPassword(_) | Self::InvalidTimestamp | Self::InvalidTokenType => 400,

            // 401 Unauthorized
            Self::TokenExpired
            | Self::InvalidToken
            | Self::TokenRevoked
            | Self::InvalidRefreshToken
            | Self::InvalidCredentials
            | Self::InvalidApiKey
            | Self::InvalidSignature
            | Self::PasswordVerificationFailed
            | Self::Unauthenticated
            | Self::SessionNotFound
            | Self::SessionExpired
            | Self::SessionInvalidated
            | Self::ApiKeyExpired => 401,

            // 403 Forbidden
            Self::TwoFactorRequired
            | Self::InvalidTwoFactorCode
            | Self::InvalidBackupCode
            | Self::InsufficientPermissions
            | Self::ApiKeyPermissionDenied
            | Self::IpNotWhitelisted
            | Self::AccountDisabled
            | Self::AccountNotVerified => 403,

            // 404 Not Found
            Self::UserNotFound | Self::ApiKeyNotFound | Self::TwoFactorNotEnabled => 404,

            // 409 Conflict
            Self::TwoFactorAlreadyEnabled | Self::MaxSessionsExceeded | Self::MaxApiKeysExceeded | Self::NonceReused => {
                409
            }

            // 429 Too Many Requests
            Self::RateLimitExceeded { .. } | Self::AccountLocked { .. } => 429,

            // 500 Internal Server Error
            Self::Database(_)
            | Self::Cache(_)
            | Self::CryptoError
            | Self::PasswordHashingFailed
            | Self::Config(_)
            | Self::Internal(_) => 500,
        }
    }

    /// Get an error code for the client (safe to expose)
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::TokenRevoked => "TOKEN_REVOKED",
            Self::InvalidRefreshToken => "INVALID_REFRESH_TOKEN",
            Self::InvalidTokenType => "INVALID_TOKEN_TYPE",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::InvalidApiKey => "INVALID_API_KEY",
            Self::InvalidSignature => "INVALID_SIGNATURE",
            Self::InvalidTimestamp => "INVALID_TIMESTAMP",
            Self::NonceReused => "NONCE_REUSED",
            Self::WeakPassword(_) => "WEAK_PASSWORD",
            Self::PasswordVerificationFailed => "INVALID_CREDENTIALS",
            Self::PasswordHashingFailed => "INTERNAL_ERROR",
            Self::TwoFactorRequired => "2FA_REQUIRED",
            Self::InvalidTwoFactorCode => "INVALID_2FA_CODE",
            Self::TwoFactorNotEnabled => "2FA_NOT_ENABLED",
            Self::TwoFactorAlreadyEnabled => "2FA_ALREADY_ENABLED",
            Self::InvalidBackupCode => "INVALID_BACKUP_CODE",
            Self::SessionNotFound => "SESSION_NOT_FOUND",
            Self::SessionExpired => "SESSION_EXPIRED",
            Self::MaxSessionsExceeded => "MAX_SESSIONS_EXCEEDED",
            Self::SessionInvalidated => "SESSION_INVALIDATED",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            Self::AccountLocked { .. } => "ACCOUNT_LOCKED",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::InsufficientPermissions => "INSUFFICIENT_PERMISSIONS",
            Self::ApiKeyPermissionDenied => "API_KEY_PERMISSION_DENIED",
            Self::IpNotWhitelisted => "IP_NOT_WHITELISTED",
            Self::AccountDisabled => "ACCOUNT_DISABLED",
            Self::AccountNotVerified => "ACCOUNT_NOT_VERIFIED",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::MaxApiKeysExceeded => "MAX_API_KEYS_EXCEEDED",
            Self::ApiKeyNotFound => "API_KEY_NOT_FOUND",
            Self::ApiKeyExpired => "API_KEY_EXPIRED",
            Self::Database(_) => "INTERNAL_ERROR",
            Self::Cache(_) => "INTERNAL_ERROR",
            Self::CryptoError => "INTERNAL_ERROR",
            Self::Config(_) => "INTERNAL_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Check if this error should be logged at error level
    pub fn is_server_error(&self) -> bool {
        self.status_code() >= 500
    }

    /// Get safe message for client (doesn't leak internal details)
    pub fn client_message(&self) -> String {
        match self {
            // Don't leak internal details
            Self::Database(_) | Self::Cache(_) | Self::Internal(_) | Self::Config(_) => {
                "An internal error occurred".to_string()
            }
            // Safe to show
            _ => self.to_string(),
        }
    }

    /// Create a rate limit error with retry-after duration
    pub fn rate_limited(duration: std::time::Duration) -> Self {
        Self::RateLimitExceeded {
            retry_after: duration.as_secs(),
        }
    }

    /// Create an account locked error with retry-after duration
    pub fn account_locked(duration: std::time::Duration) -> Self {
        Self::AccountLocked {
            retry_after: duration.as_secs(),
        }
    }
}

/// Error response for API clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code (machine-readable)
    pub code: String,
    /// Error message (human-readable)
    pub message: String,
    /// Retry-after in seconds (for rate limiting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

use serde::{Deserialize, Serialize};

impl From<&AuthError> for ErrorResponse {
    fn from(error: &AuthError) -> Self {
        let retry_after = match error {
            AuthError::RateLimitExceeded { retry_after } => Some(*retry_after),
            AuthError::AccountLocked { retry_after } => Some(*retry_after),
            _ => None,
        };

        Self {
            code: error.error_code().to_string(),
            message: error.client_message(),
            retry_after,
        }
    }
}

// Implement conversion from common error types
impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => Self::TokenExpired,
            ErrorKind::InvalidToken
            | ErrorKind::InvalidSignature
            | ErrorKind::InvalidAlgorithm
            | ErrorKind::InvalidKeyFormat => Self::InvalidToken,
            _ => Self::InvalidToken,
        }
    }
}

impl From<argon2::password_hash::Error> for AuthError {
    fn from(_: argon2::password_hash::Error) -> Self {
        Self::PasswordVerificationFailed
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_codes() {
        assert_eq!(AuthError::InvalidCredentials.status_code(), 401);
        assert_eq!(AuthError::InsufficientPermissions.status_code(), 403);
        assert_eq!(AuthError::UserNotFound.status_code(), 404);
        assert_eq!(
            AuthError::RateLimitExceeded { retry_after: 60 }.status_code(),
            429
        );
        assert_eq!(
            AuthError::Database("test".to_string()).status_code(),
            500
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(AuthError::TokenExpired.error_code(), "TOKEN_EXPIRED");
        assert_eq!(
            AuthError::Database("secret info".to_string()).error_code(),
            "INTERNAL_ERROR"
        );
    }

    #[test]
    fn test_client_message_hides_internal_details() {
        let err = AuthError::Database("connection string with password".to_string());
        assert!(!err.client_message().contains("password"));
        assert_eq!(err.client_message(), "An internal error occurred");
    }

    #[test]
    fn test_error_response() {
        let err = AuthError::RateLimitExceeded { retry_after: 60 };
        let response = ErrorResponse::from(&err);
        assert_eq!(response.code, "RATE_LIMIT_EXCEEDED");
        assert_eq!(response.retry_after, Some(60));
    }
}
