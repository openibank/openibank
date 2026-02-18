//! OpeniBank-Maple Bridge — Isolation layer between OpeniBank and the Maple AI Framework.
//!
//! This crate has two roles:
//!
//! 1. **Stable adapter traits** ([`traits`]) — the ONLY surface through which all other
//!    OpeniBank crates access Maple. Shielded from Maple API changes.
//!
//! 2. **Maple integration** — wraps Maple's WorldLine, CommitmentGate, and Resonator
//!    identity behind those stable traits.
//!
//! # Backend Selection
//!
//! ```text
//! OPENIBANK_MODE=local-sim    → LocalSimBackend (embedded WAL, zero external deps)
//! OPENIBANK_MODE=maple-native → MapleNativeBackend (full WorldLine + CommitmentGate)
//! (unset / auto)             → LocalSim (MapleNative via IBankRuntime separately)
//! ```
//!
//! # The Three Invariant Pillars
//!
//! ```text
//! WorldLineBackend    — append-only hash-chained event ledger
//! CommitmentBackend   — Intent → Commitment → Consequence (no bypass)
//! ResonatorIdentity   — EVM + ed25519 identity (vault never exports keys)
//! ```
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
//! │    ├─ Resonator: ArbiterAgent                              │
//! │    └─ Couplings (buyer ↔ seller, escrow ↔ arbiter)        │
//! │                                                             │
//! │  8 Invariants enforced:                                     │
//! │    Presence → Coupling → Meaning → Intent                  │
//! │    ════════════ COMMITMENT BOUNDARY ════════════            │
//! │    Commitment → Consequence                                │
//! └─────────────────────────────────────────────────────────────┘
//! ```

// ── Stable abstraction layer (P1 spec) ───────────────────────────────────────
pub mod traits;
pub mod local_sim;
pub mod factory;

// ── Maple integration modules ─────────────────────────────────────────────────
pub mod accountability;
pub mod attention;
pub mod bridge;
pub mod commitments;
pub mod couplings;
pub mod resonator_agent;
pub mod runtime;
pub mod worldline;

// ── Stable trait re-exports (primary API for other OpeniBank crates) ─────────
pub use traits::{
    CommitmentBackend, CommitmentError, CommitmentHandle, CommitmentId, ConsequenceProof,
    IdentityError, ResonatorId, ResonatorIdentity, WllError, WllEvent, WllEventId, WllEventType,
    WorldLineBackend,
};
pub use local_sim::{LocalSimCommitment, LocalSimIdentity, LocalSimWorldLine};
pub use factory::{AdapterMode, BackendStack, MapleAdapterConfig, create_backends, create_identity, create_demo_identities};

// ── Maple integration re-exports ──────────────────────────────────────────────
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
// Note: rcf_commitment::CommitmentId is available as rcf_commitment::CommitmentId
// (not re-exported here to avoid conflict with traits::CommitmentId, our stable abstraction).
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
