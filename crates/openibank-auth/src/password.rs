//! Password Service
//!
//! Production-grade password hashing using Argon2id (OWASP recommended).
//! Features:
//! - Argon2id hashing (resistant to side-channel and GPU attacks)
//! - Configurable parameters following OWASP guidelines
//! - Password strength validation
//! - Optional pepper for additional security
//! - Constant-time comparison to prevent timing attacks

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};
use zeroize::Zeroizing;

use crate::config::PasswordConfig;
use crate::error::{AuthError, AuthResult};

/// Password service for hashing and verification
#[derive(Clone)]
pub struct PasswordService {
    config: PasswordConfig,
}

impl PasswordService {
    /// Create a new password service
    pub fn new(config: PasswordConfig) -> Self {
        Self { config }
    }

    /// Hash a password using Argon2id
    pub fn hash_password(&self, password: &str) -> AuthResult<String> {
        // Validate password strength first
        self.validate_password_strength(password)?;

        // Apply pepper if configured
        let password_with_pepper = if let Some(ref pepper) = self.config.pepper {
            Zeroizing::new(format!("{}{}", password, pepper))
        } else {
            Zeroizing::new(password.to_string())
        };

        // Generate salt
        let salt = SaltString::generate(&mut OsRng);

        // Configure Argon2id parameters
        let params = Params::new(
            self.config.memory_cost,
            self.config.time_cost,
            self.config.parallelism,
            Some(self.config.hash_length as usize),
        )
        .map_err(|e| AuthError::Internal(format!("Invalid Argon2 params: {}", e)))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

        // Hash password
        let hash = argon2
            .hash_password(password_with_pepper.as_bytes(), &salt)
            .map_err(|_| AuthError::PasswordHashingFailed)?;

        Ok(hash.to_string())
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> AuthResult<bool> {
        // Apply pepper if configured
        let password_with_pepper = if let Some(ref pepper) = self.config.pepper {
            Zeroizing::new(format!("{}{}", password, pepper))
        } else {
            Zeroizing::new(password.to_string())
        };

        // Parse the stored hash
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| AuthError::PasswordVerificationFailed)?;

        // Verify using constant-time comparison
        let argon2 = Argon2::default();
        match argon2.verify_password(password_with_pepper.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(_) => Err(AuthError::PasswordVerificationFailed),
        }
    }

    /// Validate password strength
    pub fn validate_password_strength(&self, password: &str) -> AuthResult<()> {
        let mut errors = Vec::new();

        // Length check
        if password.len() < self.config.min_password_length {
            errors.push(format!(
                "Password must be at least {} characters",
                self.config.min_password_length
            ));
        }

        if password.len() > self.config.max_password_length {
            errors.push(format!(
                "Password must be at most {} characters",
                self.config.max_password_length
            ));
        }

        // Character class checks
        if self.config.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if self.config.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }

        if self.config.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one digit".to_string());
        }

        if self.config.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            errors.push("Password must contain at least one special character".to_string());
        }

        // Common password check (simplified - in production use a proper list)
        let common_passwords = [
            "password", "123456", "12345678", "qwerty", "abc123",
            "monkey", "1234567", "letmein", "trustno1", "dragon",
            "baseball", "iloveyou", "master", "sunshine", "ashley",
            "michael", "shadow", "123123", "654321", "password1",
        ];

        let lowercase = password.to_lowercase();
        if common_passwords.iter().any(|&common| lowercase.contains(common)) {
            errors.push("Password is too common".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(AuthError::WeakPassword(errors.join("; ")))
        }
    }

    /// Check if a password hash needs rehashing (parameters changed)
    pub fn needs_rehash(&self, hash: &str) -> bool {
        match PasswordHash::new(hash) {
            Ok(parsed_hash) => {
                // Check if the hash uses our current parameters
                if let Some(output) = parsed_hash.hash {
                    // Simple check: if hash length doesn't match, needs rehash
                    output.len() != self.config.hash_length as usize
                } else {
                    true
                }
            }
            Err(_) => true,
        }
    }

    /// Generate a secure random password
    pub fn generate_password(&self, length: usize) -> String {
        use rand::Rng;

        let length = length.max(self.config.min_password_length);

        // Character sets
        let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let lowercase = "abcdefghijklmnopqrstuvwxyz";
        let digits = "0123456789";
        let special = "!@#$%^&*()_+-=[]{}|;:,.<>?";

        let mut rng = rand::thread_rng();
        let mut password = String::with_capacity(length);

        // Ensure at least one of each required character type
        let mut required = Vec::new();
        if self.config.require_uppercase {
            required.push(uppercase.chars().nth(rng.gen_range(0..uppercase.len())).unwrap());
        }
        if self.config.require_lowercase {
            required.push(lowercase.chars().nth(rng.gen_range(0..lowercase.len())).unwrap());
        }
        if self.config.require_digit {
            required.push(digits.chars().nth(rng.gen_range(0..digits.len())).unwrap());
        }
        if self.config.require_special {
            required.push(special.chars().nth(rng.gen_range(0..special.len())).unwrap());
        }

        // Build full character set
        let mut all_chars = String::new();
        all_chars.push_str(uppercase);
        all_chars.push_str(lowercase);
        all_chars.push_str(digits);
        if self.config.require_special {
            all_chars.push_str(special);
        }

        // Fill remaining length with random characters
        let remaining = length.saturating_sub(required.len());
        for _ in 0..remaining {
            password.push(all_chars.chars().nth(rng.gen_range(0..all_chars.len())).unwrap());
        }

        // Insert required characters at random positions
        for c in required {
            let pos = rng.gen_range(0..=password.len());
            password.insert(pos, c);
        }

        password
    }

    /// Calculate password entropy (bits)
    pub fn calculate_entropy(&self, password: &str) -> f64 {
        let mut charset_size = 0;

        if password.chars().any(|c| c.is_lowercase()) {
            charset_size += 26;
        }
        if password.chars().any(|c| c.is_uppercase()) {
            charset_size += 26;
        }
        if password.chars().any(|c| c.is_ascii_digit()) {
            charset_size += 10;
        }
        if password.chars().any(|c| !c.is_alphanumeric()) {
            charset_size += 32; // Approximate special characters
        }

        if charset_size == 0 {
            return 0.0;
        }

        password.len() as f64 * (charset_size as f64).log2()
    }

    /// Get password strength level (0-4)
    pub fn get_strength_level(&self, password: &str) -> PasswordStrength {
        let entropy = self.calculate_entropy(password);

        if entropy < 28.0 {
            PasswordStrength::VeryWeak
        } else if entropy < 36.0 {
            PasswordStrength::Weak
        } else if entropy < 60.0 {
            PasswordStrength::Reasonable
        } else if entropy < 128.0 {
            PasswordStrength::Strong
        } else {
            PasswordStrength::VeryStrong
        }
    }
}

/// Password strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrength {
    /// Less than 28 bits of entropy
    VeryWeak,
    /// 28-35 bits of entropy
    Weak,
    /// 36-59 bits of entropy
    Reasonable,
    /// 60-127 bits of entropy
    Strong,
    /// 128+ bits of entropy
    VeryStrong,
}

impl PasswordStrength {
    /// Get numeric level (0-4)
    pub fn level(&self) -> u8 {
        match self {
            Self::VeryWeak => 0,
            Self::Weak => 1,
            Self::Reasonable => 2,
            Self::Strong => 3,
            Self::VeryStrong => 4,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::VeryWeak => "Very Weak - easily guessable",
            Self::Weak => "Weak - could be cracked quickly",
            Self::Reasonable => "Reasonable - acceptable for most uses",
            Self::Strong => "Strong - good security",
            Self::VeryStrong => "Very Strong - excellent security",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PasswordConfig {
        PasswordConfig {
            // Use lower values for tests to be fast
            memory_cost: 4096,
            time_cost: 1,
            parallelism: 1,
            hash_length: 32,
            salt_length: 16,
            pepper: None,
            min_password_length: 8,
            max_password_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: false,
        }
    }

    #[test]
    fn test_hash_and_verify() {
        let service = PasswordService::new(test_config());
        let password = "MySecureP@ss123";

        let hash = service.hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2id$"));

        // Correct password should verify
        assert!(service.verify_password(password, &hash).unwrap());

        // Wrong password should not verify
        assert!(!service.verify_password("wrongpassword", &hash).unwrap());
    }

    #[test]
    fn test_hash_with_pepper() {
        let mut config = test_config();
        config.pepper = Some("secret-pepper".to_string());
        let service = PasswordService::new(config);

        let password = "MySecureP@ss123";
        let hash = service.hash_password(password).unwrap();

        // Should verify with same service (same pepper)
        assert!(service.verify_password(password, &hash).unwrap());

        // Service without pepper should fail
        let service_no_pepper = PasswordService::new(test_config());
        assert!(!service_no_pepper.verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_password_validation() {
        let service = PasswordService::new(test_config());

        // Valid password
        assert!(service.validate_password_strength("MySecureP@ss123").is_ok());

        // Too short
        assert!(service.validate_password_strength("Short1").is_err());

        // No uppercase
        assert!(service.validate_password_strength("mysecurepass123").is_err());

        // No lowercase
        assert!(service.validate_password_strength("MYSECUREPASS123").is_err());

        // No digit
        assert!(service.validate_password_strength("MySecurePassword").is_err());

        // Common password
        assert!(service.validate_password_strength("Password123").is_err());
    }

    #[test]
    fn test_generate_password() {
        let service = PasswordService::new(test_config());
        let password = service.generate_password(16);

        assert!(password.len() >= 16);
        assert!(service.validate_password_strength(&password).is_ok());
    }

    #[test]
    fn test_password_strength() {
        let service = PasswordService::new(test_config());

        // Very weak
        let strength = service.get_strength_level("abc");
        assert_eq!(strength, PasswordStrength::VeryWeak);

        // Strong
        let strength = service.get_strength_level("MySecureP@ssword123!");
        assert!(strength.level() >= 2);
    }

    #[test]
    fn test_entropy_calculation() {
        let service = PasswordService::new(test_config());

        // Only lowercase: 26^8 = ~37 bits
        let entropy = service.calculate_entropy("abcdefgh");
        assert!(entropy > 30.0 && entropy < 50.0);

        // Mixed case + digits: (26+26+10)^12 = ~71 bits
        let entropy = service.calculate_entropy("Abc123Xyz789");
        assert!(entropy > 60.0 && entropy < 80.0);
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let service = PasswordService::new(test_config());
        let password = "MySecureP@ss123";

        let hash1 = service.hash_password(password).unwrap();
        let hash2 = service.hash_password(password).unwrap();

        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // Both should still verify
        assert!(service.verify_password(password, &hash1).unwrap());
        assert!(service.verify_password(password, &hash2).unwrap());
    }
}
