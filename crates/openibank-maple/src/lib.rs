//! OpeniBank-Maple Bridge - Integrating Maple AI Framework with OpeniBank
//!
//! This crate bridges OpeniBank's banking primitives with the Maple AI Framework's
//! Resonance Architecture. Every OpeniBank agent becomes a Maple Resonator with
//! the IBank runtime profile.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Maple AI Framework                         │
//! │  MapleRuntime (iBank config)                               │
//! │    ├─ Resonator: BuyerAgent                                │
//! │    │    ├─ Wallet + BudgetPolicy + SpendPermits            │
//! │    │    ├─ AgentBrain (LLM reasoning)                      │
//! │    │    └─ CognitiveContext (meaning → intent)             │
//! │    ├─ Resonator: SellerAgent                               │
//! │    │    ├─ Wallet + Services + Invoices                    │
//! │    │    └─ CognitiveContext                                │
//! │    ├─ Resonator: ArbiterAgent                              │
//! │    │    ├─ DisputeCases + Decisions                        │
//! │    │    └─ CognitiveContext                                │
//! │    └─ Couplings (buyer ↔ seller, escrow ↔ arbiter)        │
//! │                                                             │
//! │  8 Invariants enforced:                                     │
//! │    Presence → Coupling → Meaning → Intent                  │
//! │    ════════════ COMMITMENT BOUNDARY ════════════            │
//! │    Commitment → Consequence                                │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Concepts
//!
//! - **MapleResonatorAgent**: Wraps an OpeniBank agent as a Maple Resonator
//! - **IBankRuntime**: Bootstraps MapleRuntime with `ibank_runtime_config()`
//! - **MapleIdBridge**: ID mapping between OpeniBank ↔ Maple type systems
//! - **IBankAccountability**: AAS integration for identity, capabilities, commitments
//! - **TradeCouplingManager**: Buyer↔seller coupling lifecycle
//! - **AttentionManager**: Attention budget monitoring and trade gating
//! - **TradeCommitmentManager**: RcfCommitment lifecycle for every trade
//! - **ActivityLog**: Tracks all agent activity for dashboard display

pub mod bridge;
pub mod resonator_agent;
pub mod runtime;
pub mod accountability;
pub mod couplings;
pub mod attention;
pub mod commitments;

// Re-exports for convenience
pub use bridge::{
    MapleIdBridge, ResonatorAgentRole, AgentPresenceState,
    build_resonator_profile, new_maple_resonator_id, openibank_id_from_name,
};
pub use resonator_agent::{MapleResonatorAgent, AgentActivity, ActivityEntry};
pub use runtime::{IBankRuntime, IBankRuntimeConfig};
pub use accountability::{IBankAccountability, AccountabilityInfo};
pub use couplings::{TradeCouplingManager, TradeCouplingInfo, CouplingsSummary};
pub use attention::{AttentionManager, AttentionBudgetInfo, AttentionSummary};
pub use commitments::{TradeCommitmentManager, TradeCommitmentRecord, CommitmentsSummary};

// Re-export Maple types that consumers will need
pub use maple_runtime::{
    MapleRuntime, ResonatorHandle, CouplingHandle,
    ResonatorSpec, ResonatorProfile, PresenceState,
    Coupling, CouplingParams, CouplingScope, CouplingPersistence,
    AttentionBudget, AttentionBudgetSpec,
    config::ibank_runtime_config,
};

// Re-export AAS types for consumers
pub use aas_types::{AgentId, PolicyDecisionCard, CommitmentOutcome};
pub use aas_service::AasError;
pub use rcf_commitment::CommitmentId;
pub use rcf_types::IdentityRef;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_id_mapping() {
        let bridge = MapleIdBridge::new();
        let ob_id = openibank_core::ResonatorId::new();
        let maple_id = maple_runtime::ResonatorId::new();

        bridge.register(&ob_id, maple_id);

        let found = bridge.get_maple_id(&ob_id);
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_ibank_runtime_bootstrap() {
        let runtime = IBankRuntime::new(IBankRuntimeConfig::default()).await;
        assert!(runtime.is_ok(), "IBankRuntime bootstrap failed: {:?}", runtime.err());
        runtime.unwrap().shutdown().await.expect("shutdown failed");
    }
}
