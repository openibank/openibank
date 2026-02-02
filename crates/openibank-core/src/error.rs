//! Error types for OpeniBank core operations
//!
//! All errors are designed to fail closed - when in doubt, deny the action.

use thiserror::Error;

/// Core errors that can occur during OpeniBank operations
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Permit expired at {expired_at}")]
    PermitExpired { expired_at: String },

    #[error("Permit amount {requested} exceeds remaining {remaining}")]
    PermitExceeded { requested: u64, remaining: u64 },

    #[error("Permit not valid for counterparty {counterparty}")]
    PermitCounterpartyMismatch { counterparty: String },

    #[error("Permit not valid for asset class {asset_class}")]
    PermitAssetMismatch { asset_class: String },

    #[error("Budget limit exceeded: {message}")]
    BudgetExceeded { message: String },

    #[error("Budget rate limit exceeded: {current_rate}/s exceeds {max_rate}/s")]
    RateLimitExceeded { current_rate: u64, max_rate: u64 },

    #[error("Counterparty {counterparty} not in allowlist")]
    CounterpartyNotAllowed { counterparty: String },

    #[error("Counterparty {counterparty} is in denylist")]
    CounterpartyDenied { counterparty: String },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Missing required evidence: {field}")]
    MissingEvidence { field: String },

    #[error("Evidence hash mismatch")]
    EvidenceHashMismatch,

    #[error("Commitment already executed: {commitment_id}")]
    CommitmentAlreadyExecuted { commitment_id: String },

    #[error("Insufficient balance: have {available}, need {required}")]
    InsufficientBalance { available: u64, required: u64 },

    #[error("Invalid amount: {message}")]
    InvalidAmount { message: String },

    #[error("Wallet compartment {compartment} not found")]
    CompartmentNotFound { compartment: String },

    #[error("Escrow {escrow_id} not found")]
    EscrowNotFound { escrow_id: String },

    #[error("Escrow conditions not met: {reason}")]
    EscrowConditionsNotMet { reason: String },

    #[error("Cryptographic error: {message}")]
    CryptoError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Policy violation: {message}")]
    PolicyViolation { message: String },
}

impl From<ed25519_dalek::SignatureError> for CoreError {
    fn from(_: ed25519_dalek::SignatureError) -> Self {
        CoreError::InvalidSignature
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        CoreError::SerializationError {
            message: e.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
