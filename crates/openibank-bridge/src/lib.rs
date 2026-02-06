//! OpeniBank Bridge - On/off-chain atomic settlements
//!
//! Implements PvP (Payment vs Payment) atomic bridge for
//! fiat ↔ crypto settlements.

pub use openibank_types::{
    BridgeTransaction, BridgeLeg, BridgeLegStatus, BridgeStatus,
};

/// Bridge executor trait
#[async_trait::async_trait]
pub trait BridgeExecutor: Send + Sync {
    /// Initiate a bridge transaction
    async fn initiate(
        &self,
        fiat_amount: openibank_types::Amount,
        crypto_amount: openibank_types::Amount,
        timeout_seconds: u64,
    ) -> openibank_types::Result<BridgeTransaction>;

    /// Lock both legs
    async fn lock(
        &self,
        bridge_id: uuid::Uuid,
    ) -> openibank_types::Result<BridgeTransaction>;

    /// Execute (release both legs atomically)
    async fn execute(
        &self,
        bridge_id: uuid::Uuid,
    ) -> openibank_types::Result<BridgeTransaction>;

    /// Rollback if one leg fails
    async fn rollback(
        &self,
        bridge_id: uuid::Uuid,
    ) -> openibank_types::Result<BridgeTransaction>;
}
