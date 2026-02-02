//! Cryptographic utilities for OpeniBank
//!
//! All signing and verification is done using Ed25519.
//! Keys are stored as hex-encoded strings for serialization.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};

/// A keypair for signing operations
#[derive(Clone)]
pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Create from a seed (32 bytes)
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        Self { signing_key }
    }

    /// Get the public key as a hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().as_bytes())
    }

    /// Get the secret key as a hex string (be careful with this!)
    pub fn secret_key_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> String {
        let signature = self.signing_key.sign(message);
        hex::encode(signature.to_bytes())
    }

    /// Get the verifying key for verification
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }
}

/// Verify a signature against a public key
pub fn verify_signature(public_key_hex: &str, message: &[u8], signature_hex: &str) -> Result<()> {
    let public_key_bytes: [u8; 32] = hex::decode(public_key_hex)
        .map_err(|e| CoreError::CryptoError {
            message: format!("Invalid public key hex: {}", e),
        })?
        .try_into()
        .map_err(|_| CoreError::CryptoError {
            message: "Public key must be 32 bytes".to_string(),
        })?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|e| {
        CoreError::CryptoError {
            message: format!("Invalid public key: {}", e),
        }
    })?;

    let signature_bytes: [u8; 64] = hex::decode(signature_hex)
        .map_err(|e| CoreError::CryptoError {
            message: format!("Invalid signature hex: {}", e),
        })?
        .try_into()
        .map_err(|_| CoreError::CryptoError {
            message: "Signature must be 64 bytes".to_string(),
        })?;

    let signature = Signature::from_bytes(&signature_bytes);

    verifying_key.verify(message, &signature)?;
    Ok(())
}

/// Compute SHA256 hash of data
pub fn hash_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Hash any serializable object
pub fn hash_object<T: Serialize>(obj: &T) -> Result<String> {
    let json = serde_json::to_vec(obj)?;
    Ok(hash_sha256(&json))
}

/// Stored representation of a public key with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKeyInfo {
    pub key_id: String,
    pub public_key_hex: String,
    pub algorithm: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PublicKeyInfo {
    pub fn from_keypair(key_id: impl Into<String>, keypair: &Keypair) -> Self {
        Self {
            key_id: key_id.into(),
            public_key_hex: keypair.public_key_hex(),
            algorithm: "Ed25519".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn verify(&self, message: &[u8], signature_hex: &str) -> Result<()> {
        verify_signature(&self.public_key_hex, message, signature_hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = Keypair::generate();
        assert_eq!(kp.public_key_hex().len(), 64); // 32 bytes = 64 hex chars
        assert_eq!(kp.secret_key_hex().len(), 64);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = Keypair::generate();
        let message = b"Hello, OpeniBank!";
        let signature = kp.sign(message);

        assert!(verify_signature(&kp.public_key_hex(), message, &signature).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let kp = Keypair::generate();
        let message = b"Hello, OpeniBank!";
        let signature = kp.sign(message);

        let wrong_message = b"Wrong message";
        assert!(verify_signature(&kp.public_key_hex(), wrong_message, &signature).is_err());
    }

    #[test]
    fn test_hash_sha256() {
        let data = b"test data";
        let hash = hash_sha256(data);
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars

        // Same input should produce same output
        assert_eq!(hash, hash_sha256(data));
    }

    #[test]
    fn test_hash_object() {
        #[derive(Serialize)]
        struct TestObj {
            name: String,
            value: u32,
        }

        let obj = TestObj {
            name: "test".to_string(),
            value: 42,
        };

        let hash = hash_object(&obj).unwrap();
        assert_eq!(hash.len(), 64);

        // Same object should produce same hash
        assert_eq!(hash, hash_object(&obj).unwrap());
    }
}
