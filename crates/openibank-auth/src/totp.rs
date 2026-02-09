//! TOTP (Time-based One-Time Password) Service
//!
//! Two-factor authentication implementation with:
//! - TOTP generation and verification (RFC 6238)
//! - QR code URL generation for authenticator apps
//! - Backup code generation and management
//! - Time skew tolerance

use rand::RngCore;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use hmac::{Hmac, Mac};
use std::time::{SystemTime, UNIX_EPOCH};
use base32::{Alphabet, encode as base32_encode, decode as base32_decode};

use crate::config::TotpConfig;
use crate::error::{AuthError, AuthResult};
use crate::types::TotpSetup;

/// TOTP service for two-factor authentication
#[derive(Clone)]
pub struct TotpService {
    config: TotpConfig,
}

impl TotpService {
    /// Create a new TOTP service
    pub fn new(config: TotpConfig) -> Self {
        Self { config }
    }

    /// Generate a new TOTP secret and setup information
    pub fn generate_setup(&self, account_name: &str) -> AuthResult<TotpSetup> {
        // Generate random secret (20 bytes for SHA1, 32 for SHA256)
        let secret_len = match self.config.algorithm.as_str() {
            "SHA256" => 32,
            "SHA512" => 64,
            _ => 20, // SHA1 default
        };

        let mut secret_bytes = vec![0u8; secret_len];
        rand::thread_rng().fill_bytes(&mut secret_bytes);

        // Encode as base32 (standard for TOTP)
        let secret = base32_encode(Alphabet::RFC4648 { padding: false }, &secret_bytes);

        // Generate QR code URL (otpauth://)
        let qr_url = self.generate_otpauth_url(&secret, account_name);

        // Generate backup codes
        let backup_codes = self.generate_backup_codes();

        Ok(TotpSetup {
            secret,
            qr_url,
            backup_codes,
        })
    }

    /// Verify a TOTP code
    pub fn verify_code(&self, secret: &str, code: &str) -> AuthResult<bool> {
        // Decode the secret
        let secret_bytes = base32_decode(Alphabet::RFC4648 { padding: false }, secret)
            .ok_or(AuthError::Internal("Invalid TOTP secret".to_string()))?;

        // Get current time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .as_secs();

        let counter = now / self.config.step;

        // Check current period and skew periods
        for i in 0..=self.config.skew {
            // Check past periods
            if i > 0 {
                let past_code = self.generate_code_for_counter(&secret_bytes, counter - i as u64)?;
                if constant_time_compare(code, &past_code) {
                    return Ok(true);
                }
            }

            // Check current/future periods
            let future_code = self.generate_code_for_counter(&secret_bytes, counter + i as u64)?;
            if constant_time_compare(code, &future_code) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Generate the current TOTP code (for testing/display)
    pub fn generate_current_code(&self, secret: &str) -> AuthResult<String> {
        let secret_bytes = base32_decode(Alphabet::RFC4648 { padding: false }, secret)
            .ok_or(AuthError::Internal("Invalid TOTP secret".to_string()))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .as_secs();

        let counter = now / self.config.step;

        self.generate_code_for_counter(&secret_bytes, counter)
    }

    /// Get seconds remaining until next code
    pub fn seconds_remaining(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.config.step - (now % self.config.step)
    }

    /// Generate backup codes
    pub fn generate_backup_codes(&self) -> Vec<String> {
        let mut codes = Vec::with_capacity(self.config.backup_codes_count);

        for _ in 0..self.config.backup_codes_count {
            let mut bytes = vec![0u8; self.config.backup_code_length];
            rand::thread_rng().fill_bytes(&mut bytes);

            // Convert to alphanumeric code
            let code: String = bytes
                .iter()
                .map(|b| {
                    let chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No 0, O, 1, I for clarity
                    let idx = (*b as usize) % chars.len();
                    chars.chars().nth(idx).unwrap()
                })
                .collect();

            // Format as XXXX-XXXX for readability
            let formatted = if code.len() >= 8 {
                format!("{}-{}", &code[..4], &code[4..8])
            } else {
                code
            };

            codes.push(formatted);
        }

        codes
    }

    /// Verify a backup code (and mark as used)
    pub fn verify_backup_code(&self, code: &str, stored_codes: &[String]) -> Option<usize> {
        let normalized = code.replace('-', "").to_uppercase();

        for (idx, stored) in stored_codes.iter().enumerate() {
            let stored_normalized = stored.replace('-', "").to_uppercase();
            if constant_time_compare(&normalized, &stored_normalized) {
                return Some(idx);
            }
        }

        None
    }

    /// Hash a backup code for storage
    pub fn hash_backup_code(&self, code: &str) -> String {
        use sha2::Digest;
        let normalized = code.replace('-', "").to_uppercase();
        let hash = Sha256::digest(normalized.as_bytes());
        hex::encode(hash)
    }

    // =========================================================================
    // Internal Methods
    // =========================================================================

    /// Generate otpauth:// URL for QR codes
    fn generate_otpauth_url(&self, secret: &str, account_name: &str) -> String {
        let issuer_encoded = urlencoding::encode(&self.config.issuer);
        let account_encoded = urlencoding::encode(account_name);

        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}&period={}",
            issuer_encoded,
            account_encoded,
            secret,
            issuer_encoded,
            self.config.algorithm,
            self.config.digits,
            self.config.step,
        )
    }

    /// Generate TOTP code for a specific counter value
    fn generate_code_for_counter(&self, secret: &[u8], counter: u64) -> AuthResult<String> {
        // Convert counter to big-endian bytes
        let counter_bytes = counter.to_be_bytes();

        // Calculate HMAC based on configured algorithm
        let hash = match self.config.algorithm.as_str() {
            "SHA256" => {
                let mut mac = Hmac::<Sha256>::new_from_slice(secret)
                    .map_err(|_| AuthError::CryptoError)?;
                mac.update(&counter_bytes);
                mac.finalize().into_bytes().to_vec()
            }
            "SHA512" => {
                let mut mac = Hmac::<Sha512>::new_from_slice(secret)
                    .map_err(|_| AuthError::CryptoError)?;
                mac.update(&counter_bytes);
                mac.finalize().into_bytes().to_vec()
            }
            _ => {
                // Default to SHA1 (most compatible)
                let mut mac = Hmac::<Sha1>::new_from_slice(secret)
                    .map_err(|_| AuthError::CryptoError)?;
                mac.update(&counter_bytes);
                mac.finalize().into_bytes().to_vec()
            }
        };

        // Dynamic truncation (RFC 4226)
        let offset = (hash.last().unwrap_or(&0) & 0x0f) as usize;
        let binary = ((hash[offset] & 0x7f) as u32) << 24
            | (hash[offset + 1] as u32) << 16
            | (hash[offset + 2] as u32) << 8
            | (hash[offset + 3] as u32);

        // Generate digits
        let modulo = 10u32.pow(self.config.digits);
        let code = binary % modulo;

        Ok(format!("{:0width$}", code, width = self.config.digits as usize))
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;

    if a.len() != b.len() {
        return false;
    }

    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> TotpConfig {
        TotpConfig {
            issuer: "TestApp".to_string(),
            digits: 6,
            step: 30,
            backup_codes_count: 10,
            backup_code_length: 8,
            algorithm: "SHA1".to_string(),
            skew: 1,
        }
    }

    #[test]
    fn test_generate_setup() {
        let service = TotpService::new(test_config());
        let setup = service.generate_setup("test@example.com").unwrap();

        // Secret should be base32 encoded
        assert!(!setup.secret.is_empty());
        assert!(setup.secret.chars().all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(c)));

        // QR URL should be valid otpauth format
        assert!(setup.qr_url.starts_with("otpauth://totp/"));
        assert!(setup.qr_url.contains(&setup.secret));

        // Should have backup codes
        assert_eq!(setup.backup_codes.len(), 10);
    }

    #[test]
    fn test_verify_code() {
        let service = TotpService::new(test_config());
        let setup = service.generate_setup("test@example.com").unwrap();

        // Generate current code
        let current_code = service.generate_current_code(&setup.secret).unwrap();

        // Current code should verify
        assert!(service.verify_code(&setup.secret, &current_code).unwrap());

        // Wrong code should not verify
        assert!(!service.verify_code(&setup.secret, "000000").unwrap());
    }

    #[test]
    fn test_code_format() {
        let service = TotpService::new(test_config());
        let setup = service.generate_setup("test@example.com").unwrap();
        let code = service.generate_current_code(&setup.secret).unwrap();

        // Code should be exactly 6 digits
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_backup_codes() {
        let service = TotpService::new(test_config());
        let codes = service.generate_backup_codes();

        assert_eq!(codes.len(), 10);

        // Each code should be formatted as XXXX-XXXX
        for code in &codes {
            assert!(code.contains('-'));
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), 2);
        }
    }

    #[test]
    fn test_verify_backup_code() {
        let service = TotpService::new(test_config());
        let codes = service.generate_backup_codes();

        // First code should verify and return index 0
        let result = service.verify_backup_code(&codes[0], &codes);
        assert_eq!(result, Some(0));

        // Normalized code (without dash) should also work
        let normalized = codes[0].replace('-', "");
        let result = service.verify_backup_code(&normalized, &codes);
        assert_eq!(result, Some(0));

        // Invalid code should return None
        let result = service.verify_backup_code("INVALID-CODE", &codes);
        assert_eq!(result, None);
    }

    #[test]
    fn test_otpauth_url() {
        let service = TotpService::new(test_config());
        let setup = service.generate_setup("test@example.com").unwrap();

        assert!(setup.qr_url.contains("otpauth://totp/"));
        assert!(setup.qr_url.contains("TestApp"));
        assert!(setup.qr_url.contains("test%40example.com"));
        assert!(setup.qr_url.contains("digits=6"));
        assert!(setup.qr_url.contains("period=30"));
    }

    #[test]
    fn test_seconds_remaining() {
        let service = TotpService::new(test_config());
        let remaining = service.seconds_remaining();

        // Should be between 1 and 30
        assert!(remaining >= 1 && remaining <= 30);
    }

    #[test]
    fn test_sha256_algorithm() {
        let mut config = test_config();
        config.algorithm = "SHA256".to_string();
        let service = TotpService::new(config);

        let setup = service.generate_setup("test@example.com").unwrap();
        let code = service.generate_current_code(&setup.secret).unwrap();

        // Should verify with SHA256
        assert!(service.verify_code(&setup.secret, &code).unwrap());
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("123456", "123456"));
        assert!(!constant_time_compare("123456", "123457"));
        assert!(!constant_time_compare("123456", "12345"));
    }
}
