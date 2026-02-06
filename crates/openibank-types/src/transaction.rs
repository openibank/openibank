//! Transaction types for OpeniBank
//!
//! Transactions represent the movement of value in the system, always
//! backed by permits and commitments.

use crate::{
    Amount, Chain, CommitmentId, Currency, PermitId, ReceiptId, ResonatorId,
    TemporalAnchor, TransactionId, WalletId,
};
use serde::{Deserialize, Serialize};

/// Payment channel through which a transaction is processed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentChannel {
    /// Internal wallet-to-wallet transfer within OpeniBank
    Internal,
    /// Card network (Visa/Mastercard simulation)
    CardNetwork,
    /// ACH bank transfer simulation
    ACH,
    /// Wire transfer simulation
    Wire,
    /// On-chain blockchain transaction
    Blockchain(Chain),
    /// Agent-to-agent via capability discovery
    AgentDirect,
    /// Via marketplace transaction
    Marketplace,
    /// Via arena challenge escrow
    Arena,
}

impl PaymentChannel {
    /// Check if this channel is on-chain
    pub fn is_on_chain(&self) -> bool {
        matches!(self, Self::Blockchain(_))
    }

    /// Check if this channel is internal
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal | Self::AgentDirect)
    }

    /// Get estimated settlement time in seconds
    pub fn estimated_settlement_seconds(&self) -> u64 {
        match self {
            Self::Internal | Self::AgentDirect => 0,
            Self::Marketplace | Self::Arena => 1,
            Self::Blockchain(Chain::Solana) => 1,
            Self::Blockchain(Chain::Base | Chain::Arbitrum | Chain::Optimism) => 2,
            Self::Blockchain(Chain::Ethereum) => 12,
            Self::Blockchain(Chain::Bitcoin) => 600,
            Self::Blockchain(_) => 15,
            Self::CardNetwork => 86400,     // 1 day
            Self::ACH => 86400 * 2,          // 2 days
            Self::Wire => 86400,             // 1 day
        }
    }
}

impl Default for PaymentChannel {
    fn default() -> Self {
        Self::Internal
    }
}

/// Status of a transaction in its lifecycle
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction proposed (LLM proposed intent)
    Proposed,
    /// Commitment created by Resonator
    Committed,
    /// Funds held in escrow
    Escrowed,
    /// In clearing batch, awaiting netting
    Clearing,
    /// Settlement in progress
    Settling,
    /// Settlement complete
    Settled,
    /// Receipt issued (final state)
    Receipted,
    /// Transaction failed
    Failed(FailureReason),
    /// Transaction disputed
    Disputed,
    /// Transaction reversed/refunded
    Reversed,
    /// Transaction expired before completion
    Expired,
}

impl TransactionStatus {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Receipted | Self::Failed(_) | Self::Reversed | Self::Expired
        )
    }

    /// Check if this transaction succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Receipted)
    }

    /// Check if this transaction is still in progress
    pub fn is_pending(&self) -> bool {
        matches!(
            self,
            Self::Proposed
                | Self::Committed
                | Self::Escrowed
                | Self::Clearing
                | Self::Settling
                | Self::Settled
        )
    }
}

/// Reason for transaction failure
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureReason {
    /// Insufficient funds in wallet
    InsufficientFunds,
    /// Permit expired
    PermitExpired,
    /// Permit limit exceeded
    PermitExceeded,
    /// Counterparty not allowed by permit
    CounterpartyNotAllowed,
    /// Commitment validation failed
    CommitmentInvalid,
    /// Policy check failed
    PolicyViolation { policy: String },
    /// Settlement failed
    SettlementFailed { reason: String },
    /// Network error
    NetworkError { message: String },
    /// Timeout waiting for confirmation
    Timeout,
    /// Manually cancelled
    Cancelled,
    /// Other error
    Other { message: String },
}

/// Transaction metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionMetadata {
    /// Human-readable description
    pub description: Option<String>,
    /// Category for reporting
    pub category: Option<String>,
    /// Reference ID from external system
    pub external_reference: Option<String>,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Custom key-value pairs
    pub custom: std::collections::HashMap<String, String>,
}

/// Intent proof from the meaning/intent phase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentProof {
    /// The original intent message
    pub intent_message: String,
    /// Hash of the intent
    pub intent_hash: String,
    /// Resonator that formed the intent
    pub resonator_id: ResonatorId,
    /// When the intent was formed
    pub formed_at: TemporalAnchor,
}

/// Commitment proof from the commitment phase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentProof {
    /// The commitment ID
    pub commitment_id: CommitmentId,
    /// Hash of the commitment
    pub commitment_hash: String,
    /// Signature from the committing resonator
    pub signature: String,
    /// When the commitment was made
    pub committed_at: TemporalAnchor,
}

/// Reference to a permit used for authorization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermitRef {
    /// The permit ID
    pub permit_id: PermitId,
    /// Amount authorized by this permit for this tx
    pub authorized_amount: Amount,
    /// Hash of the permit at time of use
    pub permit_hash: String,
}

/// A transaction in OpeniBank
///
/// Every transaction follows the resonance flow:
/// Presence → Coupling → Meaning → Intent → Commitment → Consequence
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction ID
    pub id: TransactionId,
    /// Source wallet
    pub from: WalletId,
    /// Destination wallet
    pub to: WalletId,
    /// Amount being transferred
    pub amount: Amount,
    /// Intent proof (from meaning phase)
    pub intent: IntentProof,
    /// Commitment proof (accountability)
    pub commitment: CommitmentProof,
    /// Permit used for authorization
    pub permit: PermitRef,
    /// Payment channel
    pub channel: PaymentChannel,
    /// Current status
    pub status: TransactionStatus,
    /// Receipt ID (set when receipted)
    pub receipt_id: Option<ReceiptId>,
    /// When the transaction was created
    pub created_at: TemporalAnchor,
    /// When the transaction was last updated
    pub updated_at: TemporalAnchor,
    /// Additional metadata
    pub metadata: TransactionMetadata,
}

impl Transaction {
    /// Check if the transaction is complete
    pub fn is_complete(&self) -> bool {
        self.status.is_terminal()
    }

    /// Check if the transaction succeeded
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Get the currency of this transaction
    pub fn currency(&self) -> Currency {
        self.amount.currency
    }
}

/// Direction of a transaction relative to a wallet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionDirection {
    /// Outgoing (debit)
    Outgoing,
    /// Incoming (credit)
    Incoming,
}

/// Summary of a transaction for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    pub id: TransactionId,
    pub amount: Amount,
    pub direction: TransactionDirection,
    pub counterparty: WalletId,
    pub status: TransactionStatus,
    pub channel: PaymentChannel,
    pub created_at: TemporalAnchor,
    pub description: Option<String>,
}

/// Filter for querying transactions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionFilter {
    /// Filter by wallet (as sender or receiver)
    pub wallet: Option<WalletId>,
    /// Filter by direction
    pub direction: Option<TransactionDirection>,
    /// Filter by status
    pub status: Option<Vec<TransactionStatus>>,
    /// Filter by channel
    pub channel: Option<Vec<PaymentChannel>>,
    /// Filter by currency
    pub currency: Option<Currency>,
    /// Minimum amount
    pub min_amount: Option<Amount>,
    /// Maximum amount
    pub max_amount: Option<Amount>,
    /// Created after
    pub created_after: Option<TemporalAnchor>,
    /// Created before
    pub created_before: Option<TemporalAnchor>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_channel_settlement_times() {
        assert_eq!(PaymentChannel::Internal.estimated_settlement_seconds(), 0);
        assert!(PaymentChannel::ACH.estimated_settlement_seconds() > 86400);
    }

    #[test]
    fn test_transaction_status_states() {
        assert!(TransactionStatus::Receipted.is_terminal());
        assert!(TransactionStatus::Receipted.is_success());
        assert!(!TransactionStatus::Committed.is_terminal());
        assert!(TransactionStatus::Committed.is_pending());
    }
}
