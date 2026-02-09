//! Authentication DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// =============================================================================
// Login
// =============================================================================

/// Login request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    /// Email address
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    /// Password
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    /// 2FA code (if enabled)
    #[serde(default)]
    pub two_factor_code: Option<String>,
    /// Remember me (longer session)
    #[serde(default)]
    pub remember_me: bool,
}

/// Login response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Access token expiry (seconds)
    pub expires_in: i64,
    /// Whether 2FA verification is required
    pub requires_2fa: bool,
    /// User ID
    pub user_id: String,
}

// =============================================================================
// Registration
// =============================================================================

/// Registration request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// Email address
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    /// Password
    #[validate(length(min = 12, message = "Password must be at least 12 characters"))]
    pub password: String,
    /// Accept terms of service
    #[serde(default)]
    pub accept_terms: bool,
    /// Referral code (optional)
    #[serde(default)]
    pub referral_code: Option<String>,
}

/// Registration response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    /// User ID
    pub user_id: String,
    /// Email
    pub email: String,
    /// Created timestamp
    pub created_at: i64,
}

// =============================================================================
// Token Refresh
// =============================================================================

/// Refresh token request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// Refresh token response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenResponse {
    /// New access token
    pub access_token: String,
    /// New refresh token
    pub refresh_token: String,
    /// Token type
    pub token_type: String,
    /// Expires in (seconds)
    pub expires_in: i64,
}

// =============================================================================
// Logout
// =============================================================================

/// Logout request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LogoutRequest {
    /// Refresh token to revoke
    pub refresh_token: String,
}

/// Logout response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LogoutResponse {
    /// Success
    pub success: bool,
}

// =============================================================================
// 2FA
// =============================================================================

/// 2FA setup response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorSetupResponse {
    /// TOTP secret (base32 encoded)
    pub secret: String,
    /// QR code URL (otpauth://)
    pub qr_code_url: String,
    /// Backup codes
    pub backup_codes: Vec<String>,
}

/// Verify 2FA request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TwoFactorVerifyRequest {
    /// TOTP code from authenticator app
    pub code: String,
}

/// Verify 2FA response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorVerifyResponse {
    /// Whether 2FA is now enabled
    pub enabled: bool,
    /// Timestamp of verification
    pub verified_at: i64,
}

// =============================================================================
// Password
// =============================================================================

/// Change password request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    /// Current password
    pub current_password: String,
    /// New password
    #[validate(length(min = 12, message = "Password must be at least 12 characters"))]
    pub new_password: String,
}

// =============================================================================
// API Key
// =============================================================================

/// Create API key request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateApiKeyRequest {
    /// Label for the API key
    #[validate(length(min = 1, max = 50, message = "Label must be 1-50 characters"))]
    pub label: String,
    /// Permissions
    pub permissions: Vec<String>,
    /// IP whitelist (optional)
    #[serde(default)]
    pub ip_whitelist: Option<Vec<String>>,
    /// Expiry timestamp (optional)
    #[serde(default)]
    pub expires_at: Option<i64>,
}

/// Create API key response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyResponse {
    /// API key ID
    pub id: String,
    /// API key (only shown once!)
    pub api_key: String,
    /// API secret (only shown once!)
    pub secret_key: String,
    /// Label
    pub label: String,
    /// Permissions
    pub permissions: Vec<String>,
    /// Created timestamp
    pub created_at: i64,
}

/// API key info (without secret)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfo {
    /// API key ID
    pub id: String,
    /// Masked API key
    pub api_key: String,
    /// Label
    pub label: String,
    /// Permissions
    pub permissions: Vec<String>,
    /// IP whitelist
    pub ip_whitelist: Option<Vec<String>>,
    /// Created timestamp
    pub created_at: i64,
    /// Last used timestamp
    pub last_used_at: Option<i64>,
    /// Expires timestamp
    pub expires_at: Option<i64>,
}

/// Delete API key request
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteApiKeyRequest {
    /// API key ID to delete
    pub key_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_validation() {
        let request = LoginRequest {
            email: "invalid-email".to_string(),
            password: "short".to_string(),
            two_factor_code: None,
            remember_me: false,
        };

        let result = request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_login_request() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "securepassword123".to_string(),
            two_factor_code: None,
            remember_me: false,
        };

        let result = request.validate();
        assert!(result.is_ok());
    }
}
