//! API error handling
//!
//! Comprehensive error handling with Binance-compatible error codes.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// API result type
pub type ApiResult<T> = Result<T, ApiError>;

/// API error with Binance-compatible error codes
#[derive(Debug, Error)]
pub enum ApiError {
    // =========================================================================
    // Authentication Errors (-1000 to -1099)
    // =========================================================================
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Timestamp outside recv window")]
    TimestampOutsideRecvWindow,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Two-factor authentication required")]
    TwoFactorRequired,

    #[error("Invalid 2FA code")]
    InvalidTwoFactorCode,

    #[error("Session expired")]
    SessionExpired,

    // =========================================================================
    // Request Errors (-1100 to -1199)
    // =========================================================================
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid request body")]
    InvalidRequestBody,

    #[error("Request too large")]
    RequestTooLarge,

    #[error("Too many requests")]
    TooManyRequests { retry_after: u64 },

    // =========================================================================
    // Trading Errors (-2000 to -2099)
    // =========================================================================
    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Order not found")]
    OrderNotFound,

    #[error("Order already filled")]
    OrderAlreadyFilled,

    #[error("Order already cancelled")]
    OrderAlreadyCancelled,

    #[error("Invalid order type")]
    InvalidOrderType,

    #[error("Invalid order side")]
    InvalidOrderSide,

    #[error("Invalid quantity")]
    InvalidQuantity,

    #[error("Invalid price")]
    InvalidPrice,

    #[error("Market not found")]
    MarketNotFound,

    #[error("Market not trading")]
    MarketNotTrading,

    #[error("Min notional not met")]
    MinNotionalNotMet,

    // =========================================================================
    // Account Errors (-3000 to -3099)
    // =========================================================================
    #[error("Account not found")]
    AccountNotFound,

    #[error("Account disabled")]
    AccountDisabled,

    #[error("Account not verified")]
    AccountNotVerified,

    #[error("Email already registered")]
    EmailAlreadyRegistered,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Password too weak")]
    PasswordTooWeak,

    // =========================================================================
    // Wallet Errors (-4000 to -4099)
    // =========================================================================
    #[error("Wallet not found")]
    WalletNotFound,

    #[error("Asset not found")]
    AssetNotFound,

    #[error("Withdrawal limit exceeded")]
    WithdrawalLimitExceeded,

    #[error("Withdrawal not allowed")]
    WithdrawalNotAllowed,

    #[error("Invalid address")]
    InvalidAddress,

    #[error("Deposit not found")]
    DepositNotFound,

    #[error("Withdrawal not found")]
    WithdrawalNotFound,

    // =========================================================================
    // Internal Errors (-5000 to -5099)
    // =========================================================================
    #[error("Internal server error")]
    InternalError,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Database error")]
    DatabaseError,

    // =========================================================================
    // Resource Errors
    // =========================================================================
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Forbidden")]
    Forbidden,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("Invalid interval: {0}")]
    InvalidInterval(String),

    #[error("Market closed")]
    MarketClosed,

    #[error("Trading disabled")]
    TradingDisabled,
}

impl ApiError {
    /// Get the Binance-compatible error code
    pub fn error_code(&self) -> i32 {
        match self {
            // Authentication (-1000 to -1099)
            Self::InvalidApiKey => -1002,
            Self::InvalidSignature => -1022,
            Self::TimestampOutsideRecvWindow => -1021,
            Self::Unauthorized => -1010,
            Self::TwoFactorRequired => -1050,
            Self::InvalidTwoFactorCode => -1051,
            Self::SessionExpired => -1052,

            // Request (-1100 to -1199)
            Self::InvalidParameter(_) => -1102,
            Self::MissingParameter(_) => -1102,
            Self::InvalidRequestBody => -1100,
            Self::RequestTooLarge => -1103,
            Self::TooManyRequests { .. } => -1015,

            // Trading (-2000 to -2099)
            Self::InsufficientBalance => -2010,
            Self::OrderNotFound => -2013,
            Self::OrderAlreadyFilled => -2021,
            Self::OrderAlreadyCancelled => -2022,
            Self::InvalidOrderType => -2011,
            Self::InvalidOrderSide => -2012,
            Self::InvalidQuantity => -2014,
            Self::InvalidPrice => -2015,
            Self::MarketNotFound => -2001,
            Self::MarketNotTrading => -2002,
            Self::MinNotionalNotMet => -2016,

            // Account (-3000 to -3099)
            Self::AccountNotFound => -3001,
            Self::AccountDisabled => -3002,
            Self::AccountNotVerified => -3003,
            Self::EmailAlreadyRegistered => -3004,
            Self::InvalidCredentials => -3005,
            Self::PasswordTooWeak => -3006,

            // Wallet (-4000 to -4099)
            Self::WalletNotFound => -4001,
            Self::AssetNotFound => -4002,
            Self::WithdrawalLimitExceeded => -4003,
            Self::WithdrawalNotAllowed => -4004,
            Self::InvalidAddress => -4005,
            Self::DepositNotFound => -4006,
            Self::WithdrawalNotFound => -4007,

            // Internal (-5000 to -5099)
            Self::InternalError => -5000,
            Self::ServiceUnavailable => -5001,
            Self::DatabaseError => -5002,

            // Resource
            Self::NotFound(_) => -4000,
            Self::Conflict(_) => -4010,
            Self::BadRequest(_) => -1100,
            Self::Forbidden => -1010,
            Self::Internal(_) => -5000,
            Self::ValidationError(_) => -1102,
            Self::EmailAlreadyExists => -3004,
            Self::InvalidSymbol(_) => -2001,
            Self::InvalidInterval(_) => -2002,
            Self::MarketClosed => -2002,
            Self::TradingDisabled => -2003,
        }
    }

    /// Get the HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 400 Bad Request
            Self::InvalidParameter(_)
            | Self::MissingParameter(_)
            | Self::InvalidRequestBody
            | Self::InvalidOrderType
            | Self::InvalidOrderSide
            | Self::InvalidQuantity
            | Self::InvalidPrice
            | Self::MinNotionalNotMet
            | Self::PasswordTooWeak
            | Self::InvalidAddress
            | Self::BadRequest(_)
            | Self::ValidationError(_)
            | Self::InvalidSymbol(_)
            | Self::InvalidInterval(_) => StatusCode::BAD_REQUEST,

            // 401 Unauthorized
            Self::InvalidApiKey
            | Self::InvalidSignature
            | Self::TimestampOutsideRecvWindow
            | Self::Unauthorized
            | Self::SessionExpired
            | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,

            // 403 Forbidden
            Self::TwoFactorRequired
            | Self::InvalidTwoFactorCode
            | Self::AccountDisabled
            | Self::AccountNotVerified
            | Self::WithdrawalNotAllowed
            | Self::Forbidden
            | Self::TradingDisabled => StatusCode::FORBIDDEN,

            // 404 Not Found
            Self::NotFound(_)
            | Self::OrderNotFound
            | Self::MarketNotFound
            | Self::AccountNotFound
            | Self::WalletNotFound
            | Self::AssetNotFound
            | Self::DepositNotFound
            | Self::WithdrawalNotFound => StatusCode::NOT_FOUND,

            // 409 Conflict
            Self::OrderAlreadyFilled
            | Self::OrderAlreadyCancelled
            | Self::EmailAlreadyRegistered
            | Self::EmailAlreadyExists
            | Self::Conflict(_) => StatusCode::CONFLICT,

            // 413 Payload Too Large
            Self::RequestTooLarge => StatusCode::PAYLOAD_TOO_LARGE,

            // 422 Unprocessable Entity
            Self::InsufficientBalance
            | Self::MarketNotTrading
            | Self::MarketClosed
            | Self::WithdrawalLimitExceeded => StatusCode::UNPROCESSABLE_ENTITY,

            // 429 Too Many Requests
            Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,

            // 500 Internal Server Error
            Self::InternalError | Self::DatabaseError | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // 503 Service Unavailable
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

/// API error response (Binance-compatible format)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Binance-compatible error code
    pub code: i32,
    /// Human-readable error message
    pub msg: String,
    /// Request ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl From<&ApiError> for ErrorResponse {
    fn from(err: &ApiError) -> Self {
        Self {
            code: err.error_code(),
            msg: err.to_string(),
            request_id: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ErrorResponse::from(&self);

        let mut response = (status, Json(error_response)).into_response();

        // Add Retry-After header for rate limiting
        if let ApiError::TooManyRequests { retry_after } = self {
            response.headers_mut().insert(
                "Retry-After",
                retry_after.to_string().parse().unwrap(),
            );
        }

        response
    }
}

// Implement From conversions
impl From<openibank_auth::AuthError> for ApiError {
    fn from(err: openibank_auth::AuthError) -> Self {
        use openibank_auth::AuthError;
        match err {
            AuthError::InvalidToken | AuthError::TokenExpired | AuthError::TokenRevoked => {
                Self::Unauthorized
            }
            AuthError::InvalidApiKey => Self::InvalidApiKey,
            AuthError::InvalidSignature => Self::InvalidSignature,
            AuthError::InvalidTimestamp => Self::TimestampOutsideRecvWindow,
            AuthError::TwoFactorRequired => Self::TwoFactorRequired,
            AuthError::InvalidTwoFactorCode | AuthError::InvalidBackupCode => {
                Self::InvalidTwoFactorCode
            }
            AuthError::SessionExpired | AuthError::SessionNotFound => Self::SessionExpired,
            AuthError::InvalidCredentials | AuthError::PasswordVerificationFailed => {
                Self::InvalidCredentials
            }
            AuthError::WeakPassword(_) => Self::PasswordTooWeak,
            AuthError::AccountDisabled => Self::AccountDisabled,
            AuthError::AccountNotVerified => Self::AccountNotVerified,
            AuthError::RateLimitExceeded { retry_after } => Self::TooManyRequests { retry_after },
            AuthError::AccountLocked { retry_after } => Self::TooManyRequests { retry_after },
            AuthError::InsufficientPermissions | AuthError::ApiKeyPermissionDenied => {
                Self::Unauthorized
            }
            _ => Self::InternalError,
        }
    }
}

impl From<openibank_db::DbError> for ApiError {
    fn from(err: openibank_db::DbError) -> Self {
        tracing::error!(error = ?err, "Database error");
        match err {
            openibank_db::DbError::NotFound(msg) => Self::NotFound(msg),
            _ => Self::DatabaseError,
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        Self::InvalidRequestBody
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = err
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |e| {
                    format!("{}: {}", field, e.message.as_ref().map(|m| m.as_ref()).unwrap_or("invalid"))
                })
            })
            .collect();
        Self::InvalidParameter(messages.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ApiError::InvalidApiKey.error_code(), -1002);
        assert_eq!(ApiError::InsufficientBalance.error_code(), -2010);
        assert_eq!(ApiError::AccountNotFound.error_code(), -3001);
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(ApiError::Unauthorized.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::NotFound("test".to_string()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(
            ApiError::TooManyRequests { retry_after: 60 }.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }
}
