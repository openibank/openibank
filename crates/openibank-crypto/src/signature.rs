//! Digital signatures for OpeniBank

use crate::{CryptoError, CryptoResult, KeyPair, PublicKey};
use ed25519_dalek::{Signature as Ed25519Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};

/// A digital signature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Hex-encoded signature bytes
    pub signature: String,
    /// Public key of the signer
    pub public_key: PublicKey,
    /// Timestamp of signing
    pub signed_at: i64,
}

impl Signature {
    /// Sign a message
    pub fn sign(keypair: &KeyPair, message: &[u8]) -> CryptoResult<Self> {
        let signature = keypair
            .signing_key()
            .try_sign(message)
            .map_err(|e| CryptoError::SigningFailed(e.to_string()))?;

        Ok(Self {
            signature: hex::encode(signature.to_bytes()),
            public_key: PublicKey::from_keypair(keypair),
            signed_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    /// Verify the signature
    pub fn verify(&self, message: &[u8]) -> CryptoResult<bool> {
        let signature_bytes = hex::decode(&self.signature)
            .map_err(|e| CryptoError::VerificationFailed(e.to_string()))?;

        if signature_bytes.len() != 64 {
            return Err(CryptoError::VerificationFailed(
                "Signature must be 64 bytes".to_string(),
            ));
        }

        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);

        let signature = Ed25519Signature::from_bytes(&sig_array);
        let verifying_key = self.public_key.to_verifying_key()?;

        match verifying_key.verify(message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get the signature as bytes
    pub fn as_bytes(&self) -> CryptoResult<Vec<u8>> {
        hex::decode(&self.signature)
            .map_err(|e| CryptoError::InvalidKeyFormat(e.to_string()))
    }
}

/// Signable trait for types that can be signed
pub trait Signable {
    /// Get the bytes to sign
    fn signable_bytes(&self) -> Vec<u8>;
}

/// Sign any Signable type
pub fn sign<T: Signable>(keypair: &KeyPair, item: &T) -> CryptoResult<Signature> {
    Signature::sign(keypair, &item.signable_bytes())
}

/// Verify signature on any Signable type
pub fn verify<T: Signable>(signature: &Signature, item: &T) -> CryptoResult<bool> {
    signature.verify(&item.signable_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"Hello, OpeniBank!";

        let signature = Signature::sign(&keypair, message).unwrap();
        assert!(signature.verify(message).unwrap());
    }

    #[test]
    fn test_wrong_message_fails() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"Hello, OpeniBank!";
        let wrong_message = b"Hello, World!";

        let signature = Signature::sign(&keypair, message).unwrap();
        assert!(!signature.verify(wrong_message).unwrap());
    }

    #[test]
    fn test_wrong_key_fails() {
        let keypair1 = KeyPair::generate().unwrap();
        let keypair2 = KeyPair::generate().unwrap();
        let message = b"Hello, OpeniBank!";

        let signature = Signature::sign(&keypair1, message).unwrap();

        // Tamper with the public key
        let tampered = Signature {
            public_key: PublicKey::from_keypair(&keypair2),
            ..signature
        };

        assert!(!tampered.verify(message).unwrap());
    }
}
