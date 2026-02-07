//! OpeniBank Crypto - Cryptographic primitives for AI-native banking
//!
//! This crate provides:
//! - Key generation and management
//! - Digital signatures (Ed25519)
//! - Hashing (SHA-256)
//! - Receipt signing and verification
//!
//! # Security Invariant
//!
//! **Private keys NEVER leave the encrypted vault.**

pub mod keys;
pub mod signature;
pub mod hash;
pub mod vault;

pub use keys::*;
pub use signature::*;
pub use hash::*;
pub use vault::*;

use thiserror::Error;

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("Vault error: {0}")]
    VaultError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

pub type CryptoResult<T> = Result<T, CryptoError>;
