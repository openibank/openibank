//! Key management for OpeniBank

use crate::{CryptoError, CryptoResult};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// A key pair for signing operations
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> CryptoResult<Self> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Create from existing signing key bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> CryptoResult<Self> {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Get the signing key (private - never expose!)
    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Get the verifying key (public)
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key.as_bytes())
    }

    /// Get the signing key bytes (for secure storage only!)
    pub fn signing_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
}

/// Public key reference (safe to share)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey {
    /// Hex-encoded public key
    pub key: String,
    /// Key algorithm
    pub algorithm: KeyAlgorithm,
}

impl PublicKey {
    /// Create from a key pair
    pub fn from_keypair(keypair: &KeyPair) -> Self {
        Self {
            key: keypair.public_key_hex(),
            algorithm: KeyAlgorithm::Ed25519,
        }
    }

    /// Parse the verifying key
    pub fn to_verifying_key(&self) -> CryptoResult<VerifyingKey> {
        let bytes = hex::decode(&self.key)
            .map_err(|e| CryptoError::InvalidKeyFormat(e.to_string()))?;

        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyFormat(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);

        VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| CryptoError::InvalidKeyFormat(e.to_string()))
    }
}

/// Supported key algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyAlgorithm {
    /// Ed25519 (default)
    Ed25519,
}

impl Default for KeyAlgorithm {
    fn default() -> Self {
        Self::Ed25519
    }
}

/// Key identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyId(pub String);

impl KeyId {
    /// Generate a new key ID
    pub fn new() -> Self {
        Self(format!("key_{}", uuid::Uuid::new_v4()))
    }

    /// Create from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for KeyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate().unwrap();
        let public_key = keypair.public_key_hex();
        assert_eq!(public_key.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_keypair_from_bytes() {
        let keypair1 = KeyPair::generate().unwrap();
        let bytes = keypair1.signing_key_bytes();
        let keypair2 = KeyPair::from_bytes(&bytes).unwrap();

        assert_eq!(keypair1.public_key_hex(), keypair2.public_key_hex());
    }

    #[test]
    fn test_public_key_roundtrip() {
        let keypair = KeyPair::generate().unwrap();
        let public = PublicKey::from_keypair(&keypair);
        let verifying = public.to_verifying_key().unwrap();

        assert_eq!(keypair.verifying_key(), &verifying);
    }
}
