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

pub mod accountability;
pub mod attention;
pub mod bridge;
pub mod commitments;
pub mod couplings;
pub mod resonator_agent;
pub mod runtime;
pub mod worldline;

// Re-exports for convenience
pub use accountability::{AccountabilityInfo, IBankAccountability};
pub use attention::{AttentionBudgetInfo, AttentionManager, AttentionSummary};
pub use bridge::{
    build_resonator_profile, new_maple_resonator_id, openibank_id_from_name, AgentPresenceState,
    MapleIdBridge, ResonatorAgentRole,
};
pub use commitments::{CommitmentsSummary, TradeCommitmentManager, TradeCommitmentRecord};
pub use couplings::{CouplingsSummary, TradeCouplingInfo, TradeCouplingManager};
pub use resonator_agent::{ActivityEntry, AgentActivity, MapleResonatorAgent};
pub use runtime::{IBankRuntime, IBankRuntimeConfig};
pub use worldline::{
    ActionKind, AgentSnapshot, CommitmentGatePort, MapleAdapterError, MapleWorldlineRuntime,
    RunMetadata, WorldLineReader, WorldLineWriter, WorldlineEventRecord,
};

// Re-export Maple types that consumers will need
pub use maple_runtime::{
    config::ibank_runtime_config, AttentionBudget, AttentionBudgetSpec, Coupling, CouplingHandle,
    CouplingParams, CouplingPersistence, CouplingScope, MapleRuntime, PresenceState,
    ResonatorHandle, ResonatorProfile, ResonatorSpec,
};

// Re-export AAS types for consumers
pub use aas_service::AasError;
pub use aas_types::{AgentId, CommitmentOutcome, PolicyDecisionCard};
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
        assert!(
            runtime.is_ok(),
            "IBankRuntime bootstrap failed: {:?}",
            runtime.err()
        );
        runtime.unwrap().shutdown().await.expect("shutdown failed");
    }
}
