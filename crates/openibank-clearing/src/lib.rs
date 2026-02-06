//! OpeniBank Clearing - Transaction clearing with multilateral netting
//!
//! The clearing engine implements three paradigms:
//! 1. Merchant Aggregation - unified view across payment channels
//! 2. Global Unification - cross-institutional multilateral netting
//! 3. On/Off-Chain Bridge - atomic PvP settlements

pub use openibank_types::{
    ClearingBatch, ClearingBatchState, GrossPosition, NetPosition, NetDirection,
    SettlementLeg, SettlementLegStatus, SettlementChannel, NettingResult,
    ConservationProof, MerchantAggregation, ChannelPosition, ReconciliationResult,
};

pub mod netting;

/// Clearing engine trait
#[async_trait::async_trait]
pub trait ClearingEngine: Send + Sync {
    /// Create a new clearing batch
    async fn create_batch(
        &self,
        currency: openibank_types::Currency,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> openibank_types::Result<ClearingBatch>;

    /// Ingest a transaction into the batch
    async fn ingest(
        &self,
        batch_id: &openibank_types::BatchId,
        transaction_id: openibank_types::TransactionId,
    ) -> openibank_types::Result<()>;

    /// Run reconciliation
    async fn reconcile(
        &self,
        batch_id: &openibank_types::BatchId,
    ) -> openibank_types::Result<ReconciliationResult>;

    /// Compute multilateral netting
    async fn net(
        &self,
        batch_id: &openibank_types::BatchId,
    ) -> openibank_types::Result<NettingResult>;

    /// Execute settlements
    async fn settle(
        &self,
        batch_id: &openibank_types::BatchId,
    ) -> openibank_types::Result<()>;
}
