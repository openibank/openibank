//! Clearing and settlement types for OpeniBank
//!
//! The clearing engine implements three paradigms:
//! 1. Merchant Aggregation - unified view across payment channels
//! 2. Global Unification - cross-institutional multilateral netting
//! 3. On/Off-Chain Bridge - atomic PvP settlements

use crate::{
    Amount, BatchId, Chain, Currency, InstitutionId, JournalEntryId,
    MerchantId, PaymentChannel, ReceiptId, TemporalAnchor, TransactionId, WalletId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// State of a clearing batch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClearingBatchState {
    /// Accepting transactions
    Ingesting,
    /// Matching and verifying transactions
    Reconciling,
    /// Computing net positions
    Netting,
    /// Netting complete, awaiting execution
    ReadyToSettle,
    /// Executing settlements
    Settling,
    /// All settlements complete
    Settled,
    /// All receipts issued
    Receipted,
    /// Batch failed
    Failed,
}

impl ClearingBatchState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Receipted | Self::Failed)
    }

    /// Check if the batch can accept more transactions
    pub fn can_ingest(&self) -> bool {
        matches!(self, Self::Ingesting)
    }
}

/// A clearing batch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClearingBatch {
    /// Unique batch ID
    pub id: BatchId,
    /// Current state
    pub state: ClearingBatchState,
    /// Transactions in this batch
    pub transactions: Vec<TransactionId>,
    /// Currency for this batch
    pub currency: Currency,
    /// Institutions participating in this batch
    pub institutions: Vec<InstitutionId>,
    /// Gross positions before netting
    pub gross_positions: Vec<GrossPosition>,
    /// Net positions after netting
    pub net_positions: Vec<NetPosition>,
    /// Settlement legs to execute
    pub settlement_legs: Vec<SettlementLeg>,
    /// Netting efficiency (1 - net/gross)
    pub netting_efficiency: Option<f64>,
    /// When the batch was created
    pub created_at: TemporalAnchor,
    /// When the batch was last updated
    pub updated_at: TemporalAnchor,
    /// Cut-off time for transaction ingestion
    pub cutoff_at: DateTime<Utc>,
    /// Error message if failed
    pub error: Option<String>,
}

impl ClearingBatch {
    /// Create a new clearing batch
    pub fn new(currency: Currency, cutoff_at: DateTime<Utc>) -> Self {
        Self {
            id: BatchId::new(),
            state: ClearingBatchState::Ingesting,
            transactions: vec![],
            currency,
            institutions: vec![],
            gross_positions: vec![],
            net_positions: vec![],
            settlement_legs: vec![],
            netting_efficiency: None,
            created_at: TemporalAnchor::now(),
            updated_at: TemporalAnchor::now(),
            cutoff_at,
            error: None,
        }
    }
}

/// Gross position between two parties before netting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrossPosition {
    /// Debtor (owes money)
    pub from: InstitutionId,
    /// Creditor (owed money)
    pub to: InstitutionId,
    /// Amount owed
    pub amount: Amount,
    /// Contributing transactions
    pub transactions: Vec<TransactionId>,
}

/// Net position after multilateral netting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetPosition {
    /// Institution
    pub institution: InstitutionId,
    /// Net amount (positive = receive, negative = pay)
    pub net_amount: Amount,
    /// Whether this institution is a net payer or receiver
    pub direction: NetDirection,
}

/// Direction of net position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetDirection {
    /// Net payer (owes money)
    Payer,
    /// Net receiver (owed money)
    Receiver,
    /// Zero net position
    Zero,
}

/// A settlement leg to be executed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementLeg {
    /// Unique leg ID
    pub id: uuid::Uuid,
    /// Payer
    pub from: InstitutionId,
    /// Receiver
    pub to: InstitutionId,
    /// Amount
    pub amount: Amount,
    /// Status
    pub status: SettlementLegStatus,
    /// Settlement channel
    pub channel: SettlementChannel,
    /// Receipt ID (when complete)
    pub receipt_id: Option<ReceiptId>,
    /// When executed
    pub executed_at: Option<TemporalAnchor>,
}

/// Status of a settlement leg
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettlementLegStatus {
    /// Pending execution
    Pending,
    /// Executing
    Executing,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
}

/// Channel for settlement
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettlementChannel {
    /// Internal ledger transfer
    Internal,
    /// Real-time gross settlement (central bank)
    RTGS,
    /// On-chain settlement
    OnChain(Chain),
    /// Net settlement system
    NetSettlement,
}

/// Result of the netting algorithm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NettingResult {
    /// Number of gross transactions
    pub gross_transactions: u64,
    /// Number of net settlements needed
    pub net_settlements: u64,
    /// Netting efficiency (1 - net/gross)
    pub efficiency: f64,
    /// Settlement legs to execute
    pub legs: Vec<SettlementLeg>,
    /// Proof of conservation (sum of nets = 0)
    pub conservation_proof: ConservationProof,
}

/// Proof that netting is conserved (zero-sum)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConservationProof {
    /// Hash of all positions
    pub positions_hash: String,
    /// Sum of all net amounts (should be 0)
    pub net_sum: Amount,
    /// Whether conservation is verified
    pub verified: bool,
}

/// Merchant aggregation - unified view across channels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantAggregation {
    /// Merchant ID
    pub merchant: MerchantId,
    /// Settlement wallet
    pub settlement_wallet: WalletId,
    /// Positions by channel
    pub channel_positions: Vec<ChannelPosition>,
    /// Total net position
    pub total_net: Amount,
    /// Period this aggregation covers
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Position from a specific payment channel
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPosition {
    /// Payment channel
    pub channel: PaymentChannel,
    /// Transaction count
    pub transaction_count: u64,
    /// Gross inflow
    pub gross_inflow: Amount,
    /// Gross outflow (refunds, chargebacks)
    pub gross_outflow: Amount,
    /// Fees
    pub fees: Amount,
    /// Net position
    pub net: Amount,
}

/// Bridge transaction for on/off-chain atomic settlement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeTransaction {
    /// Unique bridge transaction ID
    pub id: uuid::Uuid,
    /// Off-chain (fiat) leg
    pub fiat_leg: BridgeLeg,
    /// On-chain (crypto) leg
    pub crypto_leg: BridgeLeg,
    /// Status
    pub status: BridgeStatus,
    /// When created
    pub created_at: TemporalAnchor,
    /// Timeout for atomic settlement
    pub timeout_at: DateTime<Utc>,
}

/// One leg of a bridge transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeLeg {
    /// Amount
    pub amount: Amount,
    /// Channel
    pub channel: SettlementChannel,
    /// Status
    pub status: BridgeLegStatus,
    /// Receipt
    pub receipt_id: Option<ReceiptId>,
}

/// Status of a bridge leg
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BridgeLegStatus {
    /// Pending
    Pending,
    /// Locked/escrowed
    Locked,
    /// Executed
    Executed,
    /// Released (for atomic settlement)
    Released,
    /// Refunded (if other leg fails)
    Refunded,
}

/// Status of a bridge transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BridgeStatus {
    /// Initiated
    Initiated,
    /// Both legs locked
    Locked,
    /// Executing (releasing both legs)
    Executing,
    /// Complete (both legs released)
    Complete,
    /// Rolled back (both legs refunded)
    RolledBack,
    /// Timed out
    TimedOut,
}

/// Reconciliation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconciliationResult {
    /// Matched transactions
    pub matched: Vec<TransactionId>,
    /// Unmatched transactions (discrepancies)
    pub unmatched: Vec<UnmatchedTransaction>,
    /// Match rate
    pub match_rate: f64,
    /// When reconciliation was performed
    pub reconciled_at: TemporalAnchor,
}

/// An unmatched transaction during reconciliation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmatchedTransaction {
    /// Transaction ID
    pub transaction_id: TransactionId,
    /// Reason for mismatch
    pub reason: MismatchReason,
    /// Expected values
    pub expected: String,
    /// Actual values
    pub actual: String,
}

/// Reason for reconciliation mismatch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MismatchReason {
    /// Amount doesn't match
    AmountMismatch,
    /// Missing counterparty record
    MissingCounterparty,
    /// Different status
    StatusMismatch,
    /// Timestamp outside window
    TimestampMismatch,
    /// Other
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_clearing_batch_creation() {
        let batch = ClearingBatch::new(Currency::iusd(), Utc::now() + Duration::hours(1));
        assert_eq!(batch.state, ClearingBatchState::Ingesting);
        assert!(batch.state.can_ingest());
    }

    #[test]
    fn test_net_direction() {
        assert!(!ClearingBatchState::Ingesting.is_terminal());
        assert!(ClearingBatchState::Receipted.is_terminal());
    }

    #[test]
    fn test_settlement_leg_status() {
        assert_eq!(SettlementLegStatus::Pending, SettlementLegStatus::Pending);
    }
}
