//! Core authentication types
//!
//! Shared types used across all authentication components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

// =============================================================================
// User Authentication Types
// =============================================================================

/// Authenticated user information extracted from tokens/sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    /// User ID
    pub user_id: Uuid,
    /// User email
    pub email: String,
    /// User role
    pub role: UserRole,
    /// User permissions
    pub permissions: HashSet<Permission>,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Session ID (if using session auth)
    pub session_id: Option<Uuid>,
    /// API key ID (if using API key auth)
    pub api_key_id: Option<Uuid>,
    /// Whether 2FA was verified for this session
    pub two_factor_verified: bool,
    /// Fee tier for trading
    pub fee_tier: FeeTier,
    /// IP address of the request
    pub ip_address: Option<String>,
    /// User agent of the request
    pub user_agent: Option<String>,
}

impl AuthenticatedUser {
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission) || self.permissions.contains(&Permission::Admin)
    }

    /// Check if user has any of the given permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Check if user has all of the given permissions
    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }

    /// Check if user is an admin
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin || self.has_permission(&Permission::Admin)
    }
}

/// User roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// Regular user
    User,
    /// Verified/KYC'd user
    Verified,
    /// Market maker with special privileges
    MarketMaker,
    /// Support staff
    Support,
    /// Administrator
    Admin,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Verified => write!(f, "verified"),
            Self::MarketMaker => write!(f, "market_maker"),
            Self::Support => write!(f, "support"),
            Self::Admin => write!(f, "admin"),
        }
    }
}

/// User permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Trading permissions
    SpotTrade,
    MarginTrade,
    FuturesTrade,

    // Wallet permissions
    Deposit,
    Withdraw,
    InternalTransfer,

    // Account permissions
    ReadAccount,
    UpdateAccount,
    ManageApiKeys,

    // Market data permissions
    ReadMarketData,
    ReadOrderBook,

    // Admin permissions
    Admin,
    ManageUsers,
    ManageMarkets,
    ViewAuditLogs,
    ManageWithdrawals,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Fee tier levels (Binance-compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeeTier {
    #[default]
    Standard,
    Bronze,
    Silver,
    Gold,
    Diamond,
    Vip,
    MarketMaker,
}

impl FeeTier {
    /// Get maker fee rate (in basis points)
    pub fn maker_fee_bps(&self) -> u32 {
        match self {
            Self::Standard => 10,      // 0.10%
            Self::Bronze => 9,         // 0.09%
            Self::Silver => 8,         // 0.08%
            Self::Gold => 6,           // 0.06%
            Self::Diamond => 4,        // 0.04%
            Self::Vip => 2,            // 0.02%
            Self::MarketMaker => 0,    // 0.00%
        }
    }

    /// Get taker fee rate (in basis points)
    pub fn taker_fee_bps(&self) -> u32 {
        match self {
            Self::Standard => 10,      // 0.10%
            Self::Bronze => 10,        // 0.10%
            Self::Silver => 9,         // 0.09%
            Self::Gold => 7,           // 0.07%
            Self::Diamond => 5,        // 0.05%
            Self::Vip => 3,            // 0.03%
            Self::MarketMaker => 1,    // 0.01%
        }
    }
}

/// Authentication method used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// JWT token (access token)
    Jwt,
    /// API key + signature
    ApiKey,
    /// Session token
    Session,
}

// =============================================================================
// Token Types
// =============================================================================

/// JWT token pair (access + refresh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: String,
    /// Access token expiry (Unix timestamp)
    pub access_expires_at: i64,
    /// Refresh token expiry (Unix timestamp)
    pub refresh_expires_at: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
}

impl TokenPair {
    pub fn new(
        access_token: String,
        refresh_token: String,
        access_expires_at: i64,
        refresh_expires_at: i64,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            access_expires_at,
            refresh_expires_at,
            token_type: "Bearer".to_string(),
        }
    }
}

/// Token type enum for JWT claims
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Token type
    pub token_type: TokenType,
    /// User email
    pub email: String,
    /// User role
    pub role: UserRole,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Not before (Unix timestamp)
    pub nbf: i64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// JWT ID (unique identifier)
    pub jti: String,
    /// Session ID (for refresh tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    /// 2FA verified
    #[serde(default)]
    pub two_factor_verified: bool,
}

// =============================================================================
// API Key Types
// =============================================================================

/// API key credentials for signing requests
#[derive(Debug, Clone)]
pub struct ApiKeyCredentials {
    /// Public API key
    pub api_key: String,
    /// Secret key (for HMAC signing)
    pub api_secret: String,
}

/// API key permissions (what the key can do)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPermissions {
    /// Can read account information
    pub can_read: bool,
    /// Can place spot trades
    pub can_trade: bool,
    /// Can withdraw funds
    pub can_withdraw: bool,
    /// Can access margin trading
    pub can_margin: bool,
    /// Can access futures trading
    pub can_futures: bool,
    /// Can manage internal transfers
    pub can_transfer: bool,
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self {
            can_read: true,
            can_trade: false,
            can_withdraw: false,
            can_margin: false,
            can_futures: false,
            can_transfer: false,
        }
    }
}

impl ApiKeyPermissions {
    /// Create permissions with all enabled
    pub fn all() -> Self {
        Self {
            can_read: true,
            can_trade: true,
            can_withdraw: true,
            can_margin: true,
            can_futures: true,
            can_transfer: true,
        }
    }

    /// Create read-only permissions
    pub fn read_only() -> Self {
        Self::default()
    }

    /// Create trading permissions (no withdraw)
    pub fn trading() -> Self {
        Self {
            can_read: true,
            can_trade: true,
            can_withdraw: false,
            can_margin: false,
            can_futures: false,
            can_transfer: false,
        }
    }
}

/// Signed API request (Binance-compatible format)
#[derive(Debug, Clone)]
pub struct SignedRequest {
    /// API key
    pub api_key: String,
    /// Request signature (HMAC-SHA256)
    pub signature: String,
    /// Request timestamp (milliseconds)
    pub timestamp: i64,
    /// Receive window (milliseconds)
    pub recv_window: Option<i64>,
    /// Query string to verify
    pub query_string: String,
    /// Request body (for POST requests)
    pub body: Option<String>,
}

// =============================================================================
// Session Types
// =============================================================================

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Session token (hashed in storage)
    pub token_hash: String,
    /// Device information
    pub device: DeviceInfo,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Last activity
    pub last_activity: DateTime<Utc>,
    /// Expires at
    pub expires_at: DateTime<Utc>,
    /// IP address
    pub ip_address: String,
    /// Whether 2FA was verified
    pub two_factor_verified: bool,
    /// Is this session active
    pub is_active: bool,
}

/// Device information for session tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID (fingerprint)
    pub device_id: Option<String>,
    /// Device name (user-defined or auto-detected)
    pub device_name: Option<String>,
    /// Device type
    pub device_type: DeviceType,
    /// User agent string
    pub user_agent: String,
    /// Operating system
    pub os: Option<String>,
    /// Browser
    pub browser: Option<String>,
}

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    #[default]
    Unknown,
    Desktop,
    Mobile,
    Tablet,
    Api,
}

// =============================================================================
// 2FA Types
// =============================================================================

/// TOTP setup response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSetup {
    /// Secret key (base32 encoded)
    pub secret: String,
    /// QR code URL (otpauth://)
    pub qr_url: String,
    /// Backup codes
    pub backup_codes: Vec<String>,
}

/// 2FA verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorResult {
    /// Whether verification succeeded
    pub success: bool,
    /// Whether a backup code was used
    pub backup_code_used: bool,
    /// Number of remaining backup codes
    pub remaining_backup_codes: Option<u32>,
}

// =============================================================================
// Login/Auth Request Types
// =============================================================================

/// Login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// 2FA code (if enabled)
    pub two_factor_code: Option<String>,
    /// Device information
    pub device_info: Option<DeviceInfo>,
    /// Remember me (longer session)
    #[serde(default)]
    pub remember_me: bool,
}

/// Login response
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    /// Token pair
    pub tokens: TokenPair,
    /// User info
    pub user: UserInfo,
    /// Whether 2FA is required
    pub two_factor_required: bool,
    /// Session ID
    pub session_id: Uuid,
}

/// Basic user info for responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User ID
    pub id: Uuid,
    /// Email
    pub email: String,
    /// Display name
    pub name: Option<String>,
    /// Role
    pub role: UserRole,
    /// Fee tier
    pub fee_tier: FeeTier,
    /// Whether 2FA is enabled
    pub two_factor_enabled: bool,
    /// Whether email is verified
    pub email_verified: bool,
    /// KYC level
    pub kyc_level: u8,
}

/// Refresh token request
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenRequest {
    /// Refresh token
    pub refresh_token: String,
}

/// Change password request
#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    /// Current password
    pub current_password: String,
    /// New password
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_has_permission() {
        let mut user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            role: UserRole::User,
            permissions: HashSet::from([Permission::SpotTrade, Permission::ReadAccount]),
            auth_method: AuthMethod::Jwt,
            session_id: None,
            api_key_id: None,
            two_factor_verified: false,
            fee_tier: FeeTier::Standard,
            ip_address: None,
            user_agent: None,
        };

        assert!(user.has_permission(&Permission::SpotTrade));
        assert!(!user.has_permission(&Permission::Admin));

        // Admin permission grants all
        user.permissions.insert(Permission::Admin);
        assert!(user.has_permission(&Permission::ManageUsers));
    }

    #[test]
    fn test_fee_tier_rates() {
        assert_eq!(FeeTier::Standard.maker_fee_bps(), 10);
        assert_eq!(FeeTier::Standard.taker_fee_bps(), 10);
        assert_eq!(FeeTier::MarketMaker.maker_fee_bps(), 0);
        assert_eq!(FeeTier::MarketMaker.taker_fee_bps(), 1);
    }

    #[test]
    fn test_api_key_permissions() {
        let read_only = ApiKeyPermissions::read_only();
        assert!(read_only.can_read);
        assert!(!read_only.can_trade);
        assert!(!read_only.can_withdraw);

        let all = ApiKeyPermissions::all();
        assert!(all.can_read);
        assert!(all.can_trade);
        assert!(all.can_withdraw);
    }
}
