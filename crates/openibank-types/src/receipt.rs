//! Receipt types for OpeniBank
//!
//! Receipts are cryptographically verifiable proofs of actions.
//! Every consequential action produces a receipt.

use crate::{
    Amount, CommitmentId, JournalEntryId, ReceiptId, ResonatorId, TemporalAnchor,
    TransactionId,
};
use serde::{Deserialize, Serialize};

/// Type of receipt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReceiptType {
    /// Transaction receipt
    Transaction,
    /// Escrow lock receipt
    EscrowLock,
    /// Escrow release receipt
    EscrowRelease,
    /// Settlement receipt
    Settlement,
    /// Issuance receipt
    Issuance,
    /// Permit grant receipt
    PermitGrant,
    /// Clearing batch receipt
    Clearing,
    /// Arena match receipt
    Arena,
    /// Custom receipt
    Custom,
}

/// Outcome that the receipt attests to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiptOutcome {
    /// Successful completion
    Success {
        /// Summary of what was accomplished
        summary: String,
    },
    /// Partial completion
    Partial {
        /// What was completed
        completed: String,
        /// What was not completed
        remaining: String,
    },
    /// Failed
    Failed {
        /// Error message
        error: String,
        /// Error code
        code: String,
    },
}

impl ReceiptOutcome {
    /// Check if outcome was successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// Verification data for a receipt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationData {
    /// Hash of the receipt content
    pub content_hash: String,
    /// Merkle proof (if applicable)
    pub merkle_proof: Option<Vec<String>>,
    /// Root hash (if part of a batch)
    pub root_hash: Option<String>,
    /// Previous receipt in chain (if applicable)
    pub previous_receipt: Option<ReceiptId>,
    /// Block height (if on-chain)
    pub block_height: Option<u64>,
    /// Chain transaction hash (if on-chain)
    pub chain_tx_hash: Option<String>,
}

/// Cryptographic signature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoSignature {
    /// Signature algorithm
    pub algorithm: String,
    /// Public key (or key ID)
    pub public_key: String,
    /// Signature bytes (hex encoded)
    pub signature: String,
    /// Timestamp of signing
    pub signed_at: TemporalAnchor,
}

/// A cryptographic Receipt
///
/// Receipts provide verifiable proof that an action was taken.
/// They are immutable and can be independently verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Unique receipt ID
    pub id: ReceiptId,
    /// Receipt type
    pub receipt_type: ReceiptType,
    /// Commitment this receipt is for
    pub commitment_id: CommitmentId,
    /// Outcome
    pub outcome: ReceiptOutcome,
    /// Transactions produced
    pub transactions: Vec<TransactionId>,
    /// Ledger entries produced
    pub ledger_entries: Vec<JournalEntryId>,
    /// Resonator that produced this receipt
    pub produced_by: ResonatorId,
    /// Amount involved (if applicable)
    pub amount: Option<Amount>,
    /// Verification data
    pub verification: VerificationData,
    /// Cryptographic signature
    pub signature: CryptoSignature,
    /// When the receipt was produced
    pub produced_at: TemporalAnchor,
}

impl Receipt {
    /// Check if the receipt is for a successful outcome
    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Get a short summary suitable for display
    pub fn summary(&self) -> String {
        match &self.outcome {
            ReceiptOutcome::Success { summary } => summary.clone(),
            ReceiptOutcome::Partial { completed, .. } => format!("Partial: {}", completed),
            ReceiptOutcome::Failed { error, .. } => format!("Failed: {}", error),
        }
    }
}

/// Request to verify a receipt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptVerificationRequest {
    /// Receipt ID to verify
    pub receipt_id: ReceiptId,
    /// Expected content hash (optional)
    pub expected_hash: Option<String>,
    /// Whether to verify on-chain (if applicable)
    pub verify_on_chain: bool,
}

/// Result of receipt verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptVerificationResult {
    /// Whether verification passed
    pub valid: bool,
    /// Content hash matches
    pub content_hash_valid: bool,
    /// Signature valid
    pub signature_valid: bool,
    /// Merkle proof valid (if applicable)
    pub merkle_valid: Option<bool>,
    /// On-chain verification (if applicable)
    pub on_chain_valid: Option<bool>,
    /// Error message (if invalid)
    pub error: Option<String>,
}

impl ReceiptVerificationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            valid: true,
            content_hash_valid: true,
            signature_valid: true,
            merkle_valid: None,
            on_chain_valid: None,
            error: None,
        }
    }

    /// Create an invalid result
    pub fn invalid(error: String) -> Self {
        Self {
            valid: false,
            content_hash_valid: false,
            signature_valid: false,
            merkle_valid: None,
            on_chain_valid: None,
            error: Some(error),
        }
    }
}

/// A chain of receipts for audit trail
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptChain {
    /// Starting receipt
    pub root: ReceiptId,
    /// Chain of receipt IDs
    pub chain: Vec<ReceiptId>,
    /// Total value through the chain
    pub total_value: Amount,
    /// Chain hash
    pub chain_hash: String,
}

/// Batch of receipts (for clearing batches, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptBatch {
    /// Batch ID
    pub batch_id: crate::BatchId,
    /// Receipts in this batch
    pub receipts: Vec<ReceiptId>,
    /// Merkle root
    pub merkle_root: String,
    /// Batch signature
    pub signature: CryptoSignature,
    /// When produced
    pub produced_at: TemporalAnchor,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_outcome() {
        let success = ReceiptOutcome::Success {
            summary: "Transfer complete".to_string(),
        };
        assert!(success.is_success());

        let failed = ReceiptOutcome::Failed {
            error: "Insufficient funds".to_string(),
            code: "INSUFFICIENT_FUNDS".to_string(),
        };
        assert!(!failed.is_success());
    }

    #[test]
    fn test_verification_result() {
        let valid = ReceiptVerificationResult::valid();
        assert!(valid.valid);

        let invalid = ReceiptVerificationResult::invalid("Hash mismatch".to_string());
        assert!(!invalid.valid);
        assert!(invalid.error.is_some());
    }
}
