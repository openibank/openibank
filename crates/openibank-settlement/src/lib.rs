//! OpeniBank Settlement - Settlement execution and finality

pub use openibank_types::{SettlementLeg, SettlementLegStatus, SettlementChannel};

/// Settlement executor trait
#[async_trait::async_trait]
pub trait SettlementExecutor: Send + Sync {
    /// Execute a settlement leg
    async fn execute_leg(
        &self,
        leg: &SettlementLeg,
    ) -> openibank_types::Result<openibank_types::ReceiptId>;

    /// Confirm finality of a settlement
    async fn confirm_finality(
        &self,
        receipt_id: &openibank_types::ReceiptId,
    ) -> openibank_types::Result<bool>;
}
