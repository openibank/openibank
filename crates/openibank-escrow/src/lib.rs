//! OpeniBank Escrow - Commitment-gated escrow management
//!
//! Escrow is the default for all value movements. Funds never move
//! directly to counterparties.

pub use openibank_types::{
    Escrow, EscrowState, ReleaseCondition, ReleaseConditionType,
    CreateEscrowRequest, EscrowActionResult, EscrowDispute,
};

/// Escrow manager trait
#[async_trait::async_trait]
pub trait EscrowManager: Send + Sync {
    /// Create a new escrow
    async fn create_escrow(
        &self,
        request: CreateEscrowRequest,
    ) -> openibank_types::Result<Escrow>;

    /// Check conditions and update state
    async fn check_conditions(
        &self,
        escrow_id: &openibank_types::EscrowId,
    ) -> openibank_types::Result<Escrow>;

    /// Release escrow to payee
    async fn release(
        &self,
        escrow_id: &openibank_types::EscrowId,
    ) -> openibank_types::Result<EscrowActionResult>;

    /// Refund escrow to payer
    async fn refund(
        &self,
        escrow_id: &openibank_types::EscrowId,
    ) -> openibank_types::Result<EscrowActionResult>;
}
