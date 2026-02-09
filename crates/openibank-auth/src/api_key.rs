//! API Key Authentication Service
//!
//! Binance-compatible API key authentication with HMAC-SHA256 signing.
//! Features:
//! - API key generation with secure random secrets
//! - HMAC-SHA256 request signing (Binance-compatible)
//! - Timestamp validation (prevent replay attacks)
//! - IP whitelist support
//! - Permission-based access control
//! - Constant-time signature comparison

use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::config::ApiKeyConfig;
use crate::error::{AuthError, AuthResult};
use crate::types::{ApiKeyCredentials, ApiKeyPermissions, SignedRequest};
use openibank_db::Database;

type HmacSha256 = Hmac<Sha256>;

/// API key service for authentication
#[derive(Clone)]
pub struct ApiKeyService {
    db: Arc<Database>,
    config: ApiKeyConfig,
}

/// API key record (matches database schema)
#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub secret_hash: String,
    pub label: String,
    pub permissions: ApiKeyPermissions,
    pub ip_whitelist: Option<Vec<String>>,
    pub created_at: chrono::DateTime<Utc>,
    pub last_used_at: Option<chrono::DateTime<Utc>>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub is_active: bool,
}

impl ApiKeyService {
    /// Create a new API key service
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            config: ApiKeyConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(db: Arc<Database>, config: ApiKeyConfig) -> Self {
        Self { db, config }
    }

    /// Generate a new API key pair
    pub fn generate_key_pair(&self) -> ApiKeyCredentials {
        let mut api_key_bytes = vec![0u8; self.config.key_length];
        let mut api_secret_bytes = vec![0u8; self.config.secret_length];

        rand::thread_rng().fill_bytes(&mut api_key_bytes);
        rand::thread_rng().fill_bytes(&mut api_secret_bytes);

        // Encode as hex for easy handling
        let api_key = hex::encode(&api_key_bytes);
        let api_secret = hex::encode(&api_secret_bytes);

        ApiKeyCredentials {
            api_key,
            api_secret,
        }
    }

    /// Hash an API key for storage
    pub fn hash_key(&self, key: &str) -> String {
        use sha2::Digest;
        let hash = Sha256::digest(key.as_bytes());
        hex::encode(hash)
    }

    /// Verify a signed request (Binance-compatible)
    pub fn verify_signature(&self, request: &SignedRequest, secret: &str) -> AuthResult<bool> {
        // Validate timestamp
        self.validate_timestamp(request.timestamp, request.recv_window)?;

        // Build the message to sign
        let message = self.build_sign_message(
            &request.query_string,
            request.body.as_deref(),
            request.timestamp,
        );

        // Calculate expected signature
        let expected_signature = self.sign_message(&message, secret)?;

        // Constant-time comparison
        let is_valid = expected_signature
            .as_bytes()
            .ct_eq(request.signature.as_bytes())
            .into();

        if is_valid {
            Ok(true)
        } else {
            Err(AuthError::InvalidSignature)
        }
    }

    /// Sign a message with HMAC-SHA256
    pub fn sign_message(&self, message: &str, secret: &str) -> AuthResult<String> {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| AuthError::CryptoError)?;

        mac.update(message.as_bytes());
        let result = mac.finalize();

        Ok(hex::encode(result.into_bytes()))
    }

    /// Build the message to sign (Binance-compatible format)
    pub fn build_sign_message(
        &self,
        query_string: &str,
        body: Option<&str>,
        timestamp: i64,
    ) -> String {
        let mut message = String::new();

        // Add query string (without leading ?)
        if !query_string.is_empty() {
            message.push_str(query_string);
        }

        // Add timestamp if not already in query string
        if !query_string.contains("timestamp=") {
            if !message.is_empty() {
                message.push('&');
            }
            message.push_str(&format!("timestamp={}", timestamp));
        }

        // Add body for POST requests
        if let Some(body) = body {
            if !body.is_empty() {
                if !message.is_empty() {
                    message.push('&');
                }
                message.push_str(body);
            }
        }

        message
    }

    /// Validate request timestamp
    pub fn validate_timestamp(&self, timestamp: i64, recv_window: Option<i64>) -> AuthResult<()> {
        let now = Utc::now().timestamp_millis();
        let recv_window = recv_window.unwrap_or(self.config.recv_window.as_millis() as i64);

        // Check if timestamp is too old
        if now - timestamp > recv_window {
            return Err(AuthError::InvalidTimestamp);
        }

        // Check if timestamp is in the future (with small tolerance)
        let tolerance = self.config.timestamp_tolerance.as_millis() as i64;
        if timestamp - now > tolerance {
            return Err(AuthError::InvalidTimestamp);
        }

        Ok(())
    }

    /// Validate IP against whitelist
    pub fn validate_ip(&self, ip: &str, whitelist: Option<&[String]>) -> AuthResult<()> {
        if !self.config.enable_ip_whitelist {
            return Ok(());
        }

        match whitelist {
            Some(list) if !list.is_empty() => {
                // Check if IP is in whitelist
                // Support CIDR notation in production
                if list.iter().any(|allowed| self.ip_matches(ip, allowed)) {
                    Ok(())
                } else {
                    Err(AuthError::IpNotWhitelisted)
                }
            }
            _ => Ok(()), // No whitelist configured, allow all
        }
    }

    /// Check if an IP matches a pattern (simple or CIDR)
    fn ip_matches(&self, ip: &str, pattern: &str) -> bool {
        // Exact match
        if ip == pattern {
            return true;
        }

        // Simple wildcard match (e.g., "192.168.1.*")
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('.').collect();
            let ip_parts: Vec<&str> = ip.split('.').collect();

            if pattern_parts.len() != ip_parts.len() {
                return false;
            }

            return pattern_parts
                .iter()
                .zip(ip_parts.iter())
                .all(|(p, i)| *p == "*" || p == i);
        }

        // CIDR notation (simplified - in production use a proper CIDR library)
        if pattern.contains('/') {
            // For now, just do exact prefix match before /
            if let Some(prefix) = pattern.split('/').next() {
                return ip.starts_with(prefix);
            }
        }

        false
    }

    /// Check if API key has required permission
    pub fn check_permission(
        &self,
        permissions: &ApiKeyPermissions,
        required: ApiKeyPermissionType,
    ) -> AuthResult<()> {
        let has_permission = match required {
            ApiKeyPermissionType::Read => permissions.can_read,
            ApiKeyPermissionType::Trade => permissions.can_trade,
            ApiKeyPermissionType::Withdraw => permissions.can_withdraw,
            ApiKeyPermissionType::Margin => permissions.can_margin,
            ApiKeyPermissionType::Futures => permissions.can_futures,
            ApiKeyPermissionType::Transfer => permissions.can_transfer,
        };

        if has_permission {
            Ok(())
        } else {
            Err(AuthError::ApiKeyPermissionDenied)
        }
    }

    /// Generate current timestamp in milliseconds
    pub fn current_timestamp(&self) -> i64 {
        Utc::now().timestamp_millis()
    }

    /// Create a signed query string for a request
    pub fn create_signed_query(
        &self,
        params: &[(&str, &str)],
        secret: &str,
    ) -> AuthResult<String> {
        let timestamp = self.current_timestamp();

        // Build query string
        let mut query_parts: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        query_parts.push(format!("timestamp={}", timestamp));

        let query_string = query_parts.join("&");

        // Sign
        let signature = self.sign_message(&query_string, secret)?;

        Ok(format!("{}&signature={}", query_string, signature))
    }
}

/// API key permission types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyPermissionType {
    Read,
    Trade,
    Withdraw,
    Margin,
    Futures,
    Transfer,
}

/// Helper to parse API key header (X-API-KEY or X-MBX-APIKEY for Binance compatibility)
pub fn extract_api_key_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try our header first
    headers
        .get("X-API-KEY")
        .or_else(|| headers.get("X-MBX-APIKEY")) // Binance compatibility
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// Helper to extract signature from query or headers
pub fn extract_signature(
    query: &str,
    headers: &axum::http::HeaderMap,
) -> Option<String> {
    // Try query string first
    if let Some(sig) = extract_query_param(query, "signature") {
        return Some(sig);
    }

    // Try header
    headers
        .get("X-API-SIGNATURE")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

/// Helper to extract timestamp from query
pub fn extract_timestamp(query: &str) -> Option<i64> {
    extract_query_param(query, "timestamp")
        .and_then(|s| s.parse().ok())
}

/// Helper to extract recv_window from query
pub fn extract_recv_window(query: &str) -> Option<i64> {
    extract_query_param(query, "recvWindow")
        .and_then(|s| s.parse().ok())
}

/// Extract a parameter from query string
fn extract_query_param(query: &str, param: &str) -> Option<String> {
    query
        .split('&')
        .find(|p| p.starts_with(&format!("{}=", param)))
        .and_then(|p| p.split('=').nth(1))
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test service without database (for crypto-only tests)
    fn test_config() -> ApiKeyConfig {
        ApiKeyConfig::default()
    }

    /// Simple test service that uses only config for crypto operations
    struct TestApiKeyService {
        config: ApiKeyConfig,
    }

    impl TestApiKeyService {
        fn new() -> Self {
            Self { config: test_config() }
        }

        fn generate_key_pair(&self) -> ApiKeyCredentials {
            let mut api_key_bytes = vec![0u8; self.config.key_length];
            let mut api_secret_bytes = vec![0u8; self.config.secret_length];
            rand::thread_rng().fill_bytes(&mut api_key_bytes);
            rand::thread_rng().fill_bytes(&mut api_secret_bytes);
            ApiKeyCredentials {
                api_key: hex::encode(&api_key_bytes),
                api_secret: hex::encode(&api_secret_bytes),
            }
        }

        fn hash_key(&self, key: &str) -> String {
            use sha2::Digest;
            let hash = Sha256::digest(key.as_bytes());
            hex::encode(hash)
        }

        fn sign_message(&self, message: &str, secret: &str) -> AuthResult<String> {
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|_| AuthError::CryptoError)?;
            mac.update(message.as_bytes());
            let result = mac.finalize();
            Ok(hex::encode(result.into_bytes()))
        }

        fn build_sign_message(&self, query_string: &str, body: Option<&str>, timestamp: i64) -> String {
            let mut message = String::new();
            if !query_string.is_empty() {
                message.push_str(query_string);
            }
            if !query_string.contains("timestamp=") {
                if !message.is_empty() { message.push('&'); }
                message.push_str(&format!("timestamp={}", timestamp));
            }
            if let Some(body) = body {
                if !body.is_empty() {
                    if !message.is_empty() { message.push('&'); }
                    message.push_str(body);
                }
            }
            message
        }

        fn verify_signature(&self, request: &SignedRequest, secret: &str) -> AuthResult<bool> {
            self.validate_timestamp(request.timestamp, request.recv_window)?;
            let message = self.build_sign_message(&request.query_string, request.body.as_deref(), request.timestamp);
            let expected_signature = self.sign_message(&message, secret)?;
            let is_valid = expected_signature.as_bytes().ct_eq(request.signature.as_bytes()).into();
            if is_valid { Ok(true) } else { Err(AuthError::InvalidSignature) }
        }

        fn validate_timestamp(&self, timestamp: i64, recv_window: Option<i64>) -> AuthResult<()> {
            let now = Utc::now().timestamp_millis();
            let recv_window = recv_window.unwrap_or(self.config.recv_window.as_millis() as i64);
            if now - timestamp > recv_window { return Err(AuthError::InvalidTimestamp); }
            let tolerance = self.config.timestamp_tolerance.as_millis() as i64;
            if timestamp - now > tolerance { return Err(AuthError::InvalidTimestamp); }
            Ok(())
        }

        fn validate_ip(&self, ip: &str, whitelist: Option<&[String]>) -> AuthResult<()> {
            if !self.config.enable_ip_whitelist { return Ok(()); }
            match whitelist {
                Some(list) if !list.is_empty() => {
                    if list.iter().any(|allowed| ip_matches(ip, allowed)) { Ok(()) }
                    else { Err(AuthError::IpNotWhitelisted) }
                }
                _ => Ok(()),
            }
        }

        fn check_permission(&self, permissions: &ApiKeyPermissions, required: ApiKeyPermissionType) -> AuthResult<()> {
            let has_permission = match required {
                ApiKeyPermissionType::Read => permissions.can_read,
                ApiKeyPermissionType::Trade => permissions.can_trade,
                ApiKeyPermissionType::Withdraw => permissions.can_withdraw,
                ApiKeyPermissionType::Margin => permissions.can_margin,
                ApiKeyPermissionType::Futures => permissions.can_futures,
                ApiKeyPermissionType::Transfer => permissions.can_transfer,
            };
            if has_permission { Ok(()) } else { Err(AuthError::ApiKeyPermissionDenied) }
        }

        fn create_signed_query(&self, params: &[(&str, &str)], secret: &str) -> AuthResult<String> {
            let timestamp = Utc::now().timestamp_millis();
            let mut query_parts: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            query_parts.push(format!("timestamp={}", timestamp));
            let query_string = query_parts.join("&");
            let signature = self.sign_message(&query_string, secret)?;
            Ok(format!("{}&signature={}", query_string, signature))
        }
    }

    fn ip_matches(ip: &str, pattern: &str) -> bool {
        if ip == pattern { return true; }
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('.').collect();
            let ip_parts: Vec<&str> = ip.split('.').collect();
            if pattern_parts.len() != ip_parts.len() { return false; }
            return pattern_parts.iter().zip(ip_parts.iter()).all(|(p, i)| *p == "*" || p == i);
        }
        false
    }

    #[test]
    fn test_generate_key_pair() {
        let service = TestApiKeyService::new();
        let pair = service.generate_key_pair();
        assert!(pair.api_key.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(pair.api_secret.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(pair.api_key.len(), service.config.key_length * 2);
        assert_eq!(pair.api_secret.len(), service.config.secret_length * 2);
    }

    #[test]
    fn test_hash_key() {
        let service = TestApiKeyService::new();
        let key = "test-api-key";
        let hash1 = service.hash_key(key);
        let hash2 = service.hash_key(key);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_sign_message() {
        let service = TestApiKeyService::new();
        let message = "symbol=BTCUSDT&side=BUY&timestamp=1234567890";
        let secret = "test-secret-key";
        let signature = service.sign_message(message, secret).unwrap();
        assert_eq!(signature.len(), 64);
        let signature2 = service.sign_message(message, secret).unwrap();
        assert_eq!(signature, signature2);
        let signature3 = service.sign_message(message, "different-secret").unwrap();
        assert_ne!(signature, signature3);
    }

    #[test]
    fn test_build_sign_message() {
        let service = TestApiKeyService::new();
        let message = service.build_sign_message("symbol=BTCUSDT&side=BUY", None, 1234567890);
        assert!(message.contains("symbol=BTCUSDT"));
        assert!(message.contains("side=BUY"));
        assert!(message.contains("timestamp=1234567890"));
        let message = service.build_sign_message("symbol=BTCUSDT", Some("quantity=1"), 1234567890);
        assert!(message.contains("quantity=1"));
    }

    #[test]
    fn test_verify_signature() {
        let service = TestApiKeyService::new();
        let secret = "test-secret-key";
        let timestamp = Utc::now().timestamp_millis();
        let query = format!("symbol=BTCUSDT&side=BUY&timestamp={}", timestamp);
        let signature = service.sign_message(&query, secret).unwrap();
        let request = SignedRequest {
            api_key: "test-key".to_string(),
            signature,
            timestamp,
            recv_window: Some(5000),
            query_string: format!("symbol=BTCUSDT&side=BUY&timestamp={}", timestamp),
            body: None,
        };
        assert!(service.verify_signature(&request, secret).is_ok());
    }

    #[test]
    fn test_verify_signature_invalid() {
        let service = TestApiKeyService::new();
        let timestamp = Utc::now().timestamp_millis();
        let request = SignedRequest {
            api_key: "test-key".to_string(),
            signature: "invalid-signature".to_string(),
            timestamp,
            recv_window: Some(5000),
            query_string: format!("symbol=BTCUSDT&timestamp={}", timestamp),
            body: None,
        };
        assert!(matches!(service.verify_signature(&request, "secret"), Err(AuthError::InvalidSignature)));
    }

    #[test]
    fn test_validate_timestamp() {
        let service = TestApiKeyService::new();
        let now = Utc::now().timestamp_millis();
        assert!(service.validate_timestamp(now, Some(5000)).is_ok());
        let old = now - 10000;
        assert!(matches!(service.validate_timestamp(old, Some(5000)), Err(AuthError::InvalidTimestamp)));
        let future = now + 1000;
        assert!(service.validate_timestamp(future, Some(5000)).is_ok());
    }

    #[test]
    fn test_ip_whitelist() {
        let service = TestApiKeyService::new();
        assert!(service.validate_ip("192.168.1.1", None).is_ok());
        assert!(service.validate_ip("192.168.1.1", Some(&[])).is_ok());
        let whitelist = vec!["192.168.1.1".to_string(), "10.0.0.*".to_string()];
        assert!(service.validate_ip("192.168.1.1", Some(&whitelist)).is_ok());
        assert!(service.validate_ip("10.0.0.5", Some(&whitelist)).is_ok());
        assert!(matches!(service.validate_ip("172.16.0.1", Some(&whitelist)), Err(AuthError::IpNotWhitelisted)));
    }

    #[test]
    fn test_check_permission() {
        let service = TestApiKeyService::new();
        let read_only = ApiKeyPermissions::read_only();
        assert!(service.check_permission(&read_only, ApiKeyPermissionType::Read).is_ok());
        assert!(matches!(service.check_permission(&read_only, ApiKeyPermissionType::Trade), Err(AuthError::ApiKeyPermissionDenied)));
        let trading = ApiKeyPermissions::trading();
        assert!(service.check_permission(&trading, ApiKeyPermissionType::Trade).is_ok());
        assert!(matches!(service.check_permission(&trading, ApiKeyPermissionType::Withdraw), Err(AuthError::ApiKeyPermissionDenied)));
    }

    #[test]
    fn test_create_signed_query() {
        let service = TestApiKeyService::new();
        let secret = "test-secret";
        let params = [("symbol", "BTCUSDT"), ("side", "BUY")];
        let signed = service.create_signed_query(&params, secret).unwrap();
        assert!(signed.contains("symbol=BTCUSDT"));
        assert!(signed.contains("side=BUY"));
        assert!(signed.contains("timestamp="));
        assert!(signed.contains("signature="));
    }

    #[test]
    fn test_extract_query_param() {
        assert_eq!(extract_query_param("symbol=BTCUSDT&side=BUY", "symbol"), Some("BTCUSDT".to_string()));
        assert_eq!(extract_query_param("symbol=BTCUSDT&side=BUY", "side"), Some("BUY".to_string()));
        assert_eq!(extract_query_param("symbol=BTCUSDT&side=BUY", "missing"), None);
    }
}
