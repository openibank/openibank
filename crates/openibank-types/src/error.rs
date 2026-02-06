//! Error types for OpeniBank
//!
//! All errors are explicit (Invariant #8: Failure must be explicit).

use thiserror::Error;

/// Result type for OpeniBank operations
pub type Result<T> = std::result::Result<T, OpeniBankError>;

/// OpeniBank error types
#[derive(Debug, Clone, Error)]
pub enum OpeniBankError {
    // ========================================================================
    // Amount Errors
    // ========================================================================

    /// Amount overflow during arithmetic
    #[error("Amount overflow during arithmetic operation")]
    AmountOverflow,

    /// Amount underflow during arithmetic
    #[error("Amount underflow during arithmetic operation")]
    AmountUnderflow,

    /// Division by zero
    #[error("Division by zero")]
    DivisionByZero,

    /// Currency mismatch
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch { expected: String, actual: String },

    // ========================================================================
    // Permit Errors
    // ========================================================================

    /// Permit has been revoked
    #[error("Permit {permit_id} has been revoked")]
    PermitRevoked { permit_id: String },

    /// Permit has expired
    #[error("Permit {permit_id} expired at {expired_at}")]
    PermitExpired { permit_id: String, expired_at: String },

    /// Permit currency not allowed
    #[error("Permit {permit_id} does not allow currency {currency}")]
    PermitCurrencyNotAllowed { permit_id: String, currency: String },

    /// Permit recipient not allowed
    #[error("Permit {permit_id} does not allow recipient {recipient}")]
    PermitRecipientNotAllowed { permit_id: String, recipient: String },

    /// Permit limit exceeded
    #[error("Permit {permit_id} limit exceeded: requested {requested}, remaining {remaining}")]
    PermitLimitExceeded {
        permit_id: String,
        requested: f64,
        remaining: f64,
    },

    /// Permit not found
    #[error("Permit {permit_id} not found")]
    PermitNotFound { permit_id: String },

    // ========================================================================
    // Wallet Errors
    // ========================================================================

    /// Wallet not found
    #[error("Wallet {wallet_id} not found")]
    WalletNotFound { wallet_id: String },

    /// Insufficient funds
    #[error("Insufficient funds in wallet {wallet_id}: requested {requested}, available {available}")]
    InsufficientFunds {
        wallet_id: String,
        requested: f64,
        available: f64,
    },

    /// Compartment not found
    #[error("Compartment {compartment_id} not found in wallet {wallet_id}")]
    CompartmentNotFound {
        wallet_id: String,
        compartment_id: String,
    },

    /// Compartment locked
    #[error("Compartment {compartment_id} is locked until {locked_until}")]
    CompartmentLocked {
        compartment_id: String,
        locked_until: String,
    },

    // ========================================================================
    // Escrow Errors
    // ========================================================================

    /// Escrow not found
    #[error("Escrow {escrow_id} not found")]
    EscrowNotFound { escrow_id: String },

    /// Escrow already released
    #[error("Escrow {escrow_id} has already been released")]
    EscrowAlreadyReleased { escrow_id: String },

    /// Escrow conditions not met
    #[error("Escrow {escrow_id} conditions not met: {remaining} of {total} conditions remaining")]
    EscrowConditionsNotMet {
        escrow_id: String,
        remaining: usize,
        total: usize,
    },

    /// Escrow expired
    #[error("Escrow {escrow_id} has expired")]
    EscrowExpired { escrow_id: String },

    /// Escrow in dispute
    #[error("Escrow {escrow_id} is in dispute")]
    EscrowInDispute { escrow_id: String },

    // ========================================================================
    // Commitment Errors
    // ========================================================================

    /// Commitment not found
    #[error("Commitment {commitment_id} not found")]
    CommitmentNotFound { commitment_id: String },

    /// Commitment already fulfilled
    #[error("Commitment {commitment_id} has already been fulfilled")]
    CommitmentAlreadyFulfilled { commitment_id: String },

    /// Commitment failed
    #[error("Commitment {commitment_id} failed: {reason}")]
    CommitmentFailed {
        commitment_id: String,
        reason: String,
    },

    /// Policy check failed
    #[error("Policy check failed for commitment: {reason}")]
    PolicyCheckFailed { reason: String },

    // ========================================================================
    // Transaction Errors
    // ========================================================================

    /// Transaction not found
    #[error("Transaction {transaction_id} not found")]
    TransactionNotFound { transaction_id: String },

    /// Transaction already complete
    #[error("Transaction {transaction_id} is already complete")]
    TransactionAlreadyComplete { transaction_id: String },

    /// Transaction failed
    #[error("Transaction {transaction_id} failed: {reason}")]
    TransactionFailed {
        transaction_id: String,
        reason: String,
    },

    // ========================================================================
    // Clearing Errors
    // ========================================================================

    /// Batch not found
    #[error("Clearing batch {batch_id} not found")]
    BatchNotFound { batch_id: String },

    /// Batch not accepting transactions
    #[error("Clearing batch {batch_id} is not accepting transactions (state: {state})")]
    BatchNotIngesting { batch_id: String, state: String },

    /// Netting failed
    #[error("Netting failed for batch {batch_id}: {reason}")]
    NettingFailed { batch_id: String, reason: String },

    /// Conservation violation
    #[error("Conservation violation in batch {batch_id}: net sum is {net_sum}")]
    ConservationViolation { batch_id: String, net_sum: String },

    // ========================================================================
    // Marketplace Errors
    // ========================================================================

    /// Listing not found
    #[error("Marketplace listing {listing_id} not found")]
    ListingNotFound { listing_id: String },

    /// Listing not active
    #[error("Marketplace listing {listing_id} is not active")]
    ListingNotActive { listing_id: String },

    /// Contract not found
    #[error("Service contract {contract_id} not found")]
    ContractNotFound { contract_id: String },

    // ========================================================================
    // Arena Errors
    // ========================================================================

    /// Match not found
    #[error("Arena match {match_id} not found")]
    MatchNotFound { match_id: String },

    /// Match not accepting participants
    #[error("Arena match {match_id} is not accepting participants")]
    MatchNotAccepting { match_id: String },

    /// Match full
    #[error("Arena match {match_id} is full")]
    MatchFull { match_id: String },

    /// Already participating
    #[error("Agent {agent_id} is already participating in match {match_id}")]
    AlreadyParticipating { agent_id: String, match_id: String },

    /// Insufficient stake
    #[error("Insufficient stake: required {required}, provided {provided}")]
    InsufficientStake { required: f64, provided: f64 },

    // ========================================================================
    // Receipt Errors
    // ========================================================================

    /// Receipt not found
    #[error("Receipt {receipt_id} not found")]
    ReceiptNotFound { receipt_id: String },

    /// Receipt verification failed
    #[error("Receipt {receipt_id} verification failed: {reason}")]
    ReceiptVerificationFailed { receipt_id: String, reason: String },

    /// Invalid signature
    #[error("Invalid signature: {reason}")]
    InvalidSignature { reason: String },

    // ========================================================================
    // Security Errors
    // ========================================================================

    /// Unauthorized action
    #[error("Unauthorized: {reason}")]
    Unauthorized { reason: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {limit} requests per {window}")]
    RateLimitExceeded { limit: u32, window: String },

    // ========================================================================
    // General Errors
    // ========================================================================

    /// Internal error
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// Invalid input
    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },

    /// Not implemented
    #[error("Not implemented: {feature}")]
    NotImplemented { feature: String },
}

impl OpeniBankError {
    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create an invalid input error
    pub fn invalid_input(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// Create an unauthorized error
    pub fn unauthorized(reason: impl Into<String>) -> Self {
        Self::Unauthorized {
            reason: reason.into(),
        }
    }

    /// Check if this is a retriable error
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            Self::Internal { .. } | Self::RateLimitExceeded { .. }
        )
    }

    /// Get an error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::AmountOverflow => "AMOUNT_OVERFLOW",
            Self::AmountUnderflow => "AMOUNT_UNDERFLOW",
            Self::DivisionByZero => "DIVISION_BY_ZERO",
            Self::CurrencyMismatch { .. } => "CURRENCY_MISMATCH",
            Self::PermitRevoked { .. } => "PERMIT_REVOKED",
            Self::PermitExpired { .. } => "PERMIT_EXPIRED",
            Self::PermitCurrencyNotAllowed { .. } => "PERMIT_CURRENCY_NOT_ALLOWED",
            Self::PermitRecipientNotAllowed { .. } => "PERMIT_RECIPIENT_NOT_ALLOWED",
            Self::PermitLimitExceeded { .. } => "PERMIT_LIMIT_EXCEEDED",
            Self::PermitNotFound { .. } => "PERMIT_NOT_FOUND",
            Self::WalletNotFound { .. } => "WALLET_NOT_FOUND",
            Self::InsufficientFunds { .. } => "INSUFFICIENT_FUNDS",
            Self::CompartmentNotFound { .. } => "COMPARTMENT_NOT_FOUND",
            Self::CompartmentLocked { .. } => "COMPARTMENT_LOCKED",
            Self::EscrowNotFound { .. } => "ESCROW_NOT_FOUND",
            Self::EscrowAlreadyReleased { .. } => "ESCROW_ALREADY_RELEASED",
            Self::EscrowConditionsNotMet { .. } => "ESCROW_CONDITIONS_NOT_MET",
            Self::EscrowExpired { .. } => "ESCROW_EXPIRED",
            Self::EscrowInDispute { .. } => "ESCROW_IN_DISPUTE",
            Self::CommitmentNotFound { .. } => "COMMITMENT_NOT_FOUND",
            Self::CommitmentAlreadyFulfilled { .. } => "COMMITMENT_ALREADY_FULFILLED",
            Self::CommitmentFailed { .. } => "COMMITMENT_FAILED",
            Self::PolicyCheckFailed { .. } => "POLICY_CHECK_FAILED",
            Self::TransactionNotFound { .. } => "TRANSACTION_NOT_FOUND",
            Self::TransactionAlreadyComplete { .. } => "TRANSACTION_ALREADY_COMPLETE",
            Self::TransactionFailed { .. } => "TRANSACTION_FAILED",
            Self::BatchNotFound { .. } => "BATCH_NOT_FOUND",
            Self::BatchNotIngesting { .. } => "BATCH_NOT_INGESTING",
            Self::NettingFailed { .. } => "NETTING_FAILED",
            Self::ConservationViolation { .. } => "CONSERVATION_VIOLATION",
            Self::ListingNotFound { .. } => "LISTING_NOT_FOUND",
            Self::ListingNotActive { .. } => "LISTING_NOT_ACTIVE",
            Self::ContractNotFound { .. } => "CONTRACT_NOT_FOUND",
            Self::MatchNotFound { .. } => "MATCH_NOT_FOUND",
            Self::MatchNotAccepting { .. } => "MATCH_NOT_ACCEPTING",
            Self::MatchFull { .. } => "MATCH_FULL",
            Self::AlreadyParticipating { .. } => "ALREADY_PARTICIPATING",
            Self::InsufficientStake { .. } => "INSUFFICIENT_STAKE",
            Self::ReceiptNotFound { .. } => "RECEIPT_NOT_FOUND",
            Self::ReceiptVerificationFailed { .. } => "RECEIPT_VERIFICATION_FAILED",
            Self::InvalidSignature { .. } => "INVALID_SIGNATURE",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            Self::Internal { .. } => "INTERNAL_ERROR",
            Self::InvalidInput { .. } => "INVALID_INPUT",
            Self::NotImplemented { .. } => "NOT_IMPLEMENTED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = OpeniBankError::InsufficientFunds {
            wallet_id: "test".to_string(),
            requested: 100.0,
            available: 50.0,
        };
        assert_eq!(err.error_code(), "INSUFFICIENT_FUNDS");
    }

    #[test]
    fn test_retriable_errors() {
        let internal = OpeniBankError::internal("test");
        assert!(internal.is_retriable());

        let not_found = OpeniBankError::WalletNotFound {
            wallet_id: "test".to_string(),
        };
        assert!(!not_found.is_retriable());
    }
}
