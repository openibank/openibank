//! Domain error types.

use thiserror::Error;

/// All errors that can occur in the OpeniBank domain layer.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("invalid signature bytes")]
    InvalidSignatureBytes,

    #[error("invalid public key bytes")]
    InvalidPublicKeyBytes,

    #[error("signature verification failed")]
    SignatureVerificationFailed,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    #[error("permit error: {0}")]
    Permit(#[from] PermitError),
}

/// Errors from spend permit validation.
#[derive(Debug, Error)]
pub enum PermitError {
    #[error("permit expired")]
    Expired,

    #[error("permit amount exceeded: requested {requested}, remaining {remaining}")]
    AmountExceeded {
        requested: String,
        remaining: String,
    },

    #[error("wrong grantee: expected {expected}, got {actual}")]
    WrongGrantee { expected: String, actual: String },

    #[error("invalid signature")]
    InvalidSignature,
}

/// Errors from receipt operations.
#[derive(Debug, Error)]
pub enum ReceiptError {
    #[error("signature verification failed")]
    SignatureVerificationFailed,

    #[error("invalid signature bytes")]
    InvalidSignatureBytes,

    #[error("invalid public key bytes")]
    InvalidPublicKeyBytes,

    #[error("serialization error: {0}")]
    Serialization(String),
}
