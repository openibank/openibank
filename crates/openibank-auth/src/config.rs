//! Authentication configuration
//!
//! Centralized configuration for all authentication components with
//! secure defaults following OWASP recommendations.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Password hashing configuration
    pub password: PasswordConfig,
    /// Session configuration
    pub session: SessionConfig,
    /// TOTP (2FA) configuration
    pub totp: TotpConfig,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// API key configuration
    pub api_key: ApiKeyConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt: JwtConfig::default(),
            password: PasswordConfig::default(),
            session: SessionConfig::default(),
            totp: TotpConfig::default(),
            rate_limit: RateLimitConfig::default(),
            api_key: ApiKeyConfig::default(),
        }
    }
}

/// JWT token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Secret key for signing tokens (should be at least 256 bits)
    pub secret: String,
    /// Access token lifetime
    #[serde(with = "humantime_serde")]
    pub access_token_lifetime: Duration,
    /// Refresh token lifetime
    #[serde(with = "humantime_serde")]
    pub refresh_token_lifetime: Duration,
    /// Token issuer claim
    pub issuer: String,
    /// Token audience claim
    pub audience: String,
    /// Algorithm to use (HS256, HS384, HS512)
    pub algorithm: String,
    /// Enable refresh token rotation
    pub rotate_refresh_tokens: bool,
    /// Grace period for refresh token rotation (allows using old token briefly)
    #[serde(with = "humantime_serde")]
    pub refresh_grace_period: Duration,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: String::new(), // Must be set in production
            access_token_lifetime: Duration::from_secs(15 * 60), // 15 minutes
            refresh_token_lifetime: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            issuer: "openibank".to_string(),
            audience: "openibank-api".to_string(),
            algorithm: "HS256".to_string(),
            rotate_refresh_tokens: true,
            refresh_grace_period: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Password hashing configuration (Argon2id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordConfig {
    /// Memory cost in KiB (OWASP recommends 19456 KiB = 19 MiB minimum)
    pub memory_cost: u32,
    /// Time cost (iterations) - OWASP recommends 2 minimum
    pub time_cost: u32,
    /// Parallelism factor
    pub parallelism: u32,
    /// Output hash length in bytes
    pub hash_length: u32,
    /// Salt length in bytes
    pub salt_length: usize,
    /// Pepper (additional secret, optional)
    pub pepper: Option<String>,
    /// Minimum password length
    pub min_password_length: usize,
    /// Maximum password length (to prevent DoS)
    pub max_password_length: usize,
    /// Require at least one uppercase letter
    pub require_uppercase: bool,
    /// Require at least one lowercase letter
    pub require_lowercase: bool,
    /// Require at least one digit
    pub require_digit: bool,
    /// Require at least one special character
    pub require_special: bool,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            // OWASP recommended values for Argon2id
            memory_cost: 19456, // 19 MiB
            time_cost: 2,
            parallelism: 1,
            hash_length: 32,
            salt_length: 16,
            pepper: None,
            min_password_length: 12, // NIST recommends 8 minimum, we use 12
            max_password_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: false, // NIST doesn't require special chars
        }
    }
}

/// Session management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session token length in bytes
    pub token_length: usize,
    /// Session lifetime (absolute timeout)
    #[serde(with = "humantime_serde")]
    pub lifetime: Duration,
    /// Idle timeout (sliding expiration)
    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
    /// Maximum concurrent sessions per user
    pub max_sessions_per_user: usize,
    /// Enable device tracking
    pub track_devices: bool,
    /// Require re-authentication for sensitive operations
    pub require_reauth_for_sensitive: bool,
    /// Time after which re-authentication is required for sensitive ops
    #[serde(with = "humantime_serde")]
    pub reauth_timeout: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            token_length: 32, // 256 bits
            lifetime: Duration::from_secs(24 * 60 * 60), // 24 hours
            idle_timeout: Duration::from_secs(30 * 60), // 30 minutes
            max_sessions_per_user: 5,
            track_devices: true,
            require_reauth_for_sensitive: true,
            reauth_timeout: Duration::from_secs(5 * 60), // 5 minutes
        }
    }
}

/// TOTP (Time-based One-Time Password) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpConfig {
    /// TOTP issuer name (shown in authenticator apps)
    pub issuer: String,
    /// Number of digits in OTP (6 or 8)
    pub digits: u32,
    /// Time step in seconds (usually 30)
    pub step: u64,
    /// Number of backup codes to generate
    pub backup_codes_count: usize,
    /// Backup code length
    pub backup_code_length: usize,
    /// Algorithm (SHA1, SHA256, SHA512)
    pub algorithm: String,
    /// Allow time skew (number of periods before/after current)
    pub skew: u8,
}

impl Default for TotpConfig {
    fn default() -> Self {
        Self {
            issuer: "OpeniBank".to_string(),
            digits: 6,
            step: 30,
            backup_codes_count: 10,
            backup_code_length: 8,
            algorithm: "SHA1".to_string(), // Most compatible with authenticator apps
            skew: 1, // Allow 1 period before/after
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Login attempts per window
    pub login_attempts: u32,
    /// Login window duration
    #[serde(with = "humantime_serde")]
    pub login_window: Duration,
    /// API requests per window (for authenticated users)
    pub api_requests_per_window: u32,
    /// API window duration
    #[serde(with = "humantime_serde")]
    pub api_window: Duration,
    /// IP-based rate limit (unauthenticated)
    pub ip_requests_per_window: u32,
    /// IP window duration
    #[serde(with = "humantime_serde")]
    pub ip_window: Duration,
    /// Lockout duration after exceeding login attempts
    #[serde(with = "humantime_serde")]
    pub lockout_duration: Duration,
    /// Progressive lockout multiplier
    pub lockout_multiplier: f64,
    /// Maximum lockout duration
    #[serde(with = "humantime_serde")]
    pub max_lockout_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            login_attempts: 5,
            login_window: Duration::from_secs(15 * 60), // 15 minutes
            api_requests_per_window: 1200, // 1200 per minute = 20/sec
            api_window: Duration::from_secs(60),
            ip_requests_per_window: 100,
            ip_window: Duration::from_secs(60),
            lockout_duration: Duration::from_secs(15 * 60), // 15 minutes
            lockout_multiplier: 2.0, // Double lockout on each failure
            max_lockout_duration: Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API key length in bytes
    pub key_length: usize,
    /// API secret length in bytes
    pub secret_length: usize,
    /// Maximum API keys per user
    pub max_keys_per_user: usize,
    /// Request timestamp tolerance (to prevent replay attacks)
    #[serde(with = "humantime_serde")]
    pub timestamp_tolerance: Duration,
    /// Enable IP whitelisting
    pub enable_ip_whitelist: bool,
    /// Signature algorithm (HMAC-SHA256, HMAC-SHA512)
    pub signature_algorithm: String,
    /// Recv window parameter (Binance-compatible)
    #[serde(with = "humantime_serde")]
    pub recv_window: Duration,
}

impl Default for ApiKeyConfig {
    fn default() -> Self {
        Self {
            key_length: 32, // 256 bits
            secret_length: 64, // 512 bits for HMAC key
            max_keys_per_user: 10,
            timestamp_tolerance: Duration::from_secs(5000 / 1000), // 5000ms Binance default
            enable_ip_whitelist: true,
            signature_algorithm: "HMAC-SHA256".to_string(),
            recv_window: Duration::from_secs(5), // 5 seconds
        }
    }
}

impl AuthConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let mut config = Self::default();

        // JWT configuration
        if let Ok(secret) = std::env::var("JWT_SECRET") {
            config.jwt.secret = secret;
        }
        if let Ok(issuer) = std::env::var("JWT_ISSUER") {
            config.jwt.issuer = issuer;
        }
        if let Ok(audience) = std::env::var("JWT_AUDIENCE") {
            config.jwt.audience = audience;
        }

        // Password pepper
        if let Ok(pepper) = std::env::var("PASSWORD_PEPPER") {
            config.password.pepper = Some(pepper);
        }

        // TOTP issuer
        if let Ok(issuer) = std::env::var("TOTP_ISSUER") {
            config.totp.issuer = issuer;
        }

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // JWT validation
        if self.jwt.secret.is_empty() {
            errors.push("JWT secret must be set".to_string());
        } else if self.jwt.secret.len() < 32 {
            errors.push("JWT secret should be at least 256 bits (32 bytes)".to_string());
        }

        // Password validation
        if self.password.memory_cost < 19456 {
            errors.push("Argon2 memory cost should be at least 19456 KiB (OWASP recommendation)".to_string());
        }
        if self.password.time_cost < 2 {
            errors.push("Argon2 time cost should be at least 2 (OWASP recommendation)".to_string());
        }

        // Session validation
        if self.session.token_length < 16 {
            errors.push("Session token length should be at least 128 bits (16 bytes)".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt.access_token_lifetime, Duration::from_secs(15 * 60));
        assert_eq!(config.password.memory_cost, 19456);
        assert_eq!(config.totp.digits, 6);
    }

    #[test]
    fn test_config_validation_missing_secret() {
        let config = AuthConfig::default();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let mut config = AuthConfig::default();
        config.jwt.secret = "a".repeat(32); // 32 bytes
        let result = config.validate();
        assert!(result.is_ok());
    }
}
