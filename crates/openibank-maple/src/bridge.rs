//! Bridge module: ID mapping and type conversion between OpeniBank ↔ Maple
//!
//! This module handles the clean conversion between OpeniBank's core types
//! and Maple's Resonator types, ensuring both frameworks can interoperate.
//!
//! # Type Differences
//!
//! - **OpeniBank ResonatorId**: `pub struct ResonatorId(pub String)` — string-based, prefixed "res_"
//! - **Maple ResonatorId**: `pub struct ResonatorId(Uuid)` — UUID-based, private inner field
//!
//! The bridge maintains a bidirectional mapping via Display/String conversion.

use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};
use rcf_types::EffectDomain;
use resonator_types::{ProfileConstraint, ConstraintType};

// ============================================================================
// ID Mapping Registry
// ============================================================================

/// Bidirectional ID mapping between OpeniBank and Maple IDs
///
/// Since Maple uses `ResonatorId(Uuid)` with private fields and OpeniBank
/// uses `ResonatorId(pub String)`, we maintain an explicit mapping registry.
pub struct MapleIdBridge {
    /// OpeniBank ID string → Maple ResonatorId
    ob_to_maple: RwLock<HashMap<String, maple_runtime::ResonatorId>>,
    /// Maple ResonatorId string → OpeniBank ID string
    maple_to_ob: RwLock<HashMap<String, String>>,
}

impl MapleIdBridge {
    /// Create a new empty bridge
    pub fn new() -> Self {
        Self {
            ob_to_maple: RwLock::new(HashMap::new()),
            maple_to_ob: RwLock::new(HashMap::new()),
        }
    }

    /// Register a mapping between OpeniBank and Maple IDs
    pub fn register(
        &self,
        ob_id: &openibank_core::ResonatorId,
        maple_id: maple_runtime::ResonatorId,
    ) {
        let ob_str = ob_id.0.clone();
        let maple_str = maple_id.to_string();
        self.ob_to_maple.write().unwrap().insert(ob_str.clone(), maple_id);
        self.maple_to_ob.write().unwrap().insert(maple_str, ob_str);
    }

    /// Look up the Maple ID for an OpeniBank ID
    pub fn get_maple_id(&self, ob_id: &openibank_core::ResonatorId) -> Option<maple_runtime::ResonatorId> {
        self.ob_to_maple.read().unwrap().get(&ob_id.0).copied()
    }

    /// Look up the OpeniBank ID for a Maple ID
    pub fn get_openibank_id(&self, maple_id: &maple_runtime::ResonatorId) -> Option<openibank_core::ResonatorId> {
        let maple_str = maple_id.to_string();
        self.maple_to_ob.read().unwrap().get(&maple_str)
            .map(|s| openibank_core::ResonatorId(s.clone()))
    }

    /// Convert an OpeniBank Amount to a u64 for Maple's ResourceLimits
    pub fn amount_to_maple_value(amount: &openibank_core::Amount) -> u64 {
        amount.0
    }

    /// Convert a Maple resource value back to OpeniBank Amount
    pub fn amount_from_maple_value(value: u64) -> openibank_core::Amount {
        openibank_core::Amount::new(value)
    }
}

impl Default for MapleIdBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: create a new Maple ResonatorId and return it
/// (Maple IDs are always generated via `ResonatorId::new()`)
pub fn new_maple_resonator_id() -> maple_runtime::ResonatorId {
    maple_runtime::ResonatorId::new()
}

/// Convenience: create a new OpeniBank ResonatorId from a name
pub fn openibank_id_from_name(name: &str) -> openibank_core::ResonatorId {
    openibank_core::ResonatorId(format!("res_{}", name.to_lowercase().replace(' ', "_")))
}

// ============================================================================
// Agent Role Classification
// ============================================================================

/// Classification of agent roles in the OpeniBank ecosystem
///
/// Maps to different Maple Resonator capabilities and profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResonatorAgentRole {
    /// Buyer agent: Has budget, evaluates offers, pays via escrow
    Buyer,
    /// Seller agent: Publishes services, issues invoices, delivers
    Seller,
    /// Arbiter agent: Validates delivery, resolves disputes
    Arbiter,
    /// Issuer agent: Mints/burns IUSD, manages supply
    Issuer,
}

impl ResonatorAgentRole {
    /// Get the Maple cognitive capabilities for this role
    pub fn cognitive_capabilities(&self) -> Vec<resonator_types::CognitiveCapability> {
        use resonator_types::CognitiveCapability::*;
        match self {
            ResonatorAgentRole::Buyer => vec![
                MeaningProduction,      // Evaluate service offers
                IntentFormulation,       // Form purchase intents
                CommitmentDrafting,      // Draft payment commitments
                ConsequenceAnalysis,     // Assess trade outcomes
            ],
            ResonatorAgentRole::Seller => vec![
                MeaningProduction,      // Understand market demand
                IntentFormulation,       // Decide what to offer
                CommitmentDrafting,      // Draft service commitments
                FeedbackLearning,        // Improve from trade outcomes
            ],
            ResonatorAgentRole::Arbiter => vec![
                MeaningProduction,      // Analyze dispute evidence
                IntentFormulation,       // Form resolution intent
                CommitmentDrafting,      // Draft arbitration decision
                ConsequenceAnalysis,     // Assess dispute impact
                FeedbackLearning,        // Learn from precedents
            ],
            ResonatorAgentRole::Issuer => vec![
                MeaningProduction,      // Monitor supply health
                CommitmentDrafting,      // Draft mint/burn commitments
                ConsequenceAnalysis,     // Assess supply impact
            ],
        }
    }

    /// Get the risk tolerance for this role
    pub fn risk_tolerance(&self) -> resonator_types::RiskTolerance {
        match self {
            ResonatorAgentRole::Buyer => resonator_types::RiskTolerance::Balanced,
            ResonatorAgentRole::Seller => resonator_types::RiskTolerance::Balanced,
            ResonatorAgentRole::Arbiter => resonator_types::RiskTolerance::Conservative,
            ResonatorAgentRole::Issuer => resonator_types::RiskTolerance::Conservative,
        }
    }

    /// Get the autonomy level for this role
    pub fn autonomy_level(&self) -> resonator_types::AutonomyLevel {
        match self {
            ResonatorAgentRole::Buyer => resonator_types::AutonomyLevel::HighAutonomy,
            ResonatorAgentRole::Seller => resonator_types::AutonomyLevel::HighAutonomy,
            ResonatorAgentRole::Arbiter => resonator_types::AutonomyLevel::GuidedAutonomy,
            ResonatorAgentRole::Issuer => resonator_types::AutonomyLevel::GuidedAutonomy,
        }
    }

    /// Display name for the role
    pub fn display_name(&self) -> &'static str {
        match self {
            ResonatorAgentRole::Buyer => "Buyer",
            ResonatorAgentRole::Seller => "Seller",
            ResonatorAgentRole::Arbiter => "Arbiter",
            ResonatorAgentRole::Issuer => "Issuer",
        }
    }

    /// Get preferred coupling affinity for this role
    pub fn coupling_affinity(&self) -> maple_runtime::types::CouplingAffinitySpec {
        use maple_runtime::{CouplingPersistence, CouplingScope};
        match self {
            ResonatorAgentRole::Buyer => maple_runtime::types::CouplingAffinitySpec {
                preferred_strength: 0.2,
                preferred_persistence: CouplingPersistence::Transient,
                preferred_scope: CouplingScope::IntentOnly,
                max_concurrent_couplings: Some(5),
            },
            ResonatorAgentRole::Seller => maple_runtime::types::CouplingAffinitySpec {
                preferred_strength: 0.25,
                preferred_persistence: CouplingPersistence::Session,
                preferred_scope: CouplingScope::Full,
                max_concurrent_couplings: Some(10),
            },
            ResonatorAgentRole::Arbiter => maple_runtime::types::CouplingAffinitySpec {
                preferred_strength: 0.15,
                preferred_persistence: CouplingPersistence::Transient,
                preferred_scope: CouplingScope::ObservationalOnly,
                max_concurrent_couplings: Some(3),
            },
            ResonatorAgentRole::Issuer => maple_runtime::types::CouplingAffinitySpec {
                preferred_strength: 0.1,
                preferred_persistence: CouplingPersistence::Persistent,
                preferred_scope: CouplingScope::StateOnly,
                max_concurrent_couplings: Some(1),
            },
        }
    }
}

// ============================================================================
// Maple Resonator Profile Builder
// ============================================================================

/// Build a Maple ResonatorProfile from an OpeniBank agent's characteristics
pub fn build_resonator_profile(
    name: &str,
    role: &ResonatorAgentRole,
    description: Option<&str>,
) -> resonator_types::ResonatorProfile {
    let desc = description
        .map(|d| d.to_string())
        .unwrap_or_else(|| format!("OpeniBank {} Agent: {}", role.display_name(), name));

    resonator_types::ResonatorProfile {
        name: name.to_string(),
        description: desc,
        domains: vec![EffectDomain::Finance, EffectDomain::Custom("commerce".to_string())],
        risk_tolerance: role.risk_tolerance(),
        autonomy_level: role.autonomy_level(),
        constraints: vec![
            ProfileConstraint {
                constraint_type: ConstraintType::Custom("llm_no_execute".to_string()),
                description: "LLMs may PROPOSE intents, NEVER EXECUTE money".to_string(),
                parameters: HashMap::new(),
            },
            ProfileConstraint {
                constraint_type: ConstraintType::Custom("commitment_required".to_string()),
                description: "All money-impacting actions require commitment receipts".to_string(),
                parameters: HashMap::new(),
            },
            ProfileConstraint {
                constraint_type: ConstraintType::Custom("permits_mandatory".to_string()),
                description: "SpendPermits are mandatory for all payments".to_string(),
                parameters: HashMap::new(),
            },
            ProfileConstraint {
                constraint_type: ConstraintType::Custom("fail_closed".to_string()),
                description: "Fail closed on any error".to_string(),
                parameters: HashMap::new(),
            },
        ],
    }
}

// ============================================================================
// Presence State Mapping
// ============================================================================

/// Agent state as perceived by the Maple runtime
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentPresenceState {
    /// Agent is idle, ready for interactions
    Idle,
    /// Agent is actively trading
    Trading,
    /// Agent is waiting for LLM response
    ThinkingLLM,
    /// Agent is waiting for escrow release
    WaitingEscrow,
    /// Agent is resolving a dispute
    ResolvingDispute,
    /// Agent has been suspended (e.g., budget exhausted)
    Suspended,
}

impl AgentPresenceState {
    /// Convert to Maple's PresenceState (gradient presence model)
    ///
    /// Maple's PresenceState uses:
    /// - discoverability (0.0-1.0)
    /// - responsiveness (0.0-1.0)
    /// - stability (0.0-1.0)
    /// - coupling_readiness (0.0-1.0)
    /// - silent_mode (bool)
    pub fn to_maple_presence(&self) -> maple_runtime::PresenceState {
        match self {
            AgentPresenceState::Idle => {
                let mut ps = maple_runtime::PresenceState::new();
                ps.discoverability = 0.5;
                ps.responsiveness = 1.0;
                ps.coupling_readiness = 0.7;
                ps.silent_mode = false;
                ps
            }
            AgentPresenceState::Trading => {
                let mut ps = maple_runtime::PresenceState::new();
                ps.discoverability = 1.0;
                ps.responsiveness = 1.0;
                ps.coupling_readiness = 0.3; // busy trading
                ps.silent_mode = false;
                ps
            }
            AgentPresenceState::ThinkingLLM => {
                let mut ps = maple_runtime::PresenceState::new();
                ps.discoverability = 0.3;
                ps.responsiveness = 0.5; // slower while thinking
                ps.coupling_readiness = 0.2;
                ps.silent_mode = false;
                ps
            }
            AgentPresenceState::WaitingEscrow => {
                let mut ps = maple_runtime::PresenceState::new();
                ps.discoverability = 0.6;
                ps.responsiveness = 0.8;
                ps.coupling_readiness = 0.5;
                ps.silent_mode = false;
                ps
            }
            AgentPresenceState::ResolvingDispute => {
                let mut ps = maple_runtime::PresenceState::new();
                ps.discoverability = 0.4;
                ps.responsiveness = 0.9;
                ps.coupling_readiness = 0.1; // focused on dispute
                ps.silent_mode = false;
                ps
            }
            AgentPresenceState::Suspended => {
                let mut ps = maple_runtime::PresenceState::new();
                ps.discoverability = 0.0;
                ps.responsiveness = 0.0;
                ps.coupling_readiness = 0.0;
                ps.silent_mode = true;
                ps
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_bridge() {
        let bridge = MapleIdBridge::new();
        let ob_id = openibank_core::ResonatorId::new();
        let maple_id = maple_runtime::ResonatorId::new();

        bridge.register(&ob_id, maple_id);

        let found = bridge.get_maple_id(&ob_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), maple_id);

        let back = bridge.get_openibank_id(&maple_id);
        assert!(back.is_some());
        assert_eq!(back.unwrap().0, ob_id.0);
    }

    #[test]
    fn test_amount_roundtrip() {
        let amount = openibank_core::Amount::new(50000);
        let maple_val = MapleIdBridge::amount_to_maple_value(&amount);
        let back = MapleIdBridge::amount_from_maple_value(maple_val);
        assert_eq!(amount, back);
    }

    #[test]
    fn test_buyer_capabilities() {
        let caps = ResonatorAgentRole::Buyer.cognitive_capabilities();
        assert!(caps.len() >= 3);
    }

    #[test]
    fn test_build_profile() {
        let profile = build_resonator_profile(
            "Alice",
            &ResonatorAgentRole::Buyer,
            Some("Test buyer agent"),
        );
        assert_eq!(profile.name, "Alice");
        assert!(profile.domains.contains(&EffectDomain::Finance));
    }

    #[test]
    fn test_presence_mapping() {
        let state = AgentPresenceState::Trading;
        let maple_presence = state.to_maple_presence();
        assert!(!maple_presence.silent_mode);
        assert_eq!(maple_presence.discoverability, 1.0);
    }

    #[test]
    fn test_suspended_presence() {
        let state = AgentPresenceState::Suspended;
        let maple_presence = state.to_maple_presence();
        assert!(maple_presence.silent_mode);
        assert_eq!(maple_presence.discoverability, 0.0);
    }
}
