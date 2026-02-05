//! MapleResonatorAgent - Wraps OpeniBank agents as Maple Resonators
//!
//! This is the core bridge type. Each OpeniBank agent (Buyer, Seller, Arbiter)
//! is wrapped in a MapleResonatorAgent that:
//! - Holds a Maple ResonatorHandle for runtime participation
//! - Maintains the OpeniBank Wallet for economic operations
//! - Tracks an AgentBrain for LLM reasoning
//! - Logs all activity for dashboard display

use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use openibank_core::{Amount, ResonatorId, Wallet};
use openibank_agents::{AgentBrain, BuyerAgent, SellerAgent, ArbiterAgent, Service};
use openibank_agent_kernel::KernelTrace;
use openibank_ledger::Ledger;

use crate::bridge::{ResonatorAgentRole, AgentPresenceState, build_resonator_profile};

// ============================================================================
// Activity Tracking
// ============================================================================

/// A single activity entry in the agent's log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    /// When this activity occurred
    pub timestamp: DateTime<Utc>,
    /// Category of activity
    pub category: ActivityCategory,
    /// Human-readable description
    pub description: String,
    /// Optional associated data (JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Categories of agent activities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityCategory {
    /// Agent was created/registered
    Created,
    /// Presence state changed
    PresenceChanged,
    /// Wallet balance changed
    BalanceChanged,
    /// Trade-related activity
    Trade,
    /// LLM reasoning occurred
    LLMReasoning,
    /// Invoice issued/received
    Invoice,
    /// Escrow created/released/refunded
    Escrow,
    /// Dispute opened/resolved
    Dispute,
    /// Service published/updated
    ServicePublished,
    /// Budget/permit related
    Budget,
    /// Receipt generated
    Receipt,
    /// Error occurred
    Error,
}

/// Agent activity log with bounded size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivity {
    /// All activity entries (newest first)
    entries: Vec<ActivityEntry>,
    /// Maximum number of entries to keep
    max_entries: usize,
}

impl AgentActivity {
    /// Create a new activity log
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Log an activity
    pub fn log(&mut self, category: ActivityCategory, description: impl Into<String>) {
        self.log_with_data(category, description, None);
    }

    /// Log an activity with associated data
    pub fn log_with_data(
        &mut self,
        category: ActivityCategory,
        description: impl Into<String>,
        data: Option<serde_json::Value>,
    ) {
        self.entries.insert(0, ActivityEntry {
            timestamp: Utc::now(),
            category,
            description: description.into(),
            data,
        });

        // Trim to max size
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }
    }

    /// Get all entries
    pub fn entries(&self) -> &[ActivityEntry] {
        &self.entries
    }

    /// Get entries filtered by category
    pub fn entries_by_category(&self, category: &ActivityCategory) -> Vec<&ActivityEntry> {
        self.entries.iter().filter(|e| &e.category == category).collect()
    }

    /// Get the most recent N entries
    pub fn recent(&self, n: usize) -> &[ActivityEntry] {
        let end = n.min(self.entries.len());
        &self.entries[..end]
    }

    /// Total number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the log empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for AgentActivity {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ============================================================================
// Agent Inner Type (Buyer, Seller, or Arbiter)
// ============================================================================

/// The inner OpeniBank agent wrapped by a Resonator
pub enum AgentInner {
    /// A buyer agent
    Buyer(BuyerAgent),
    /// A seller agent
    Seller(SellerAgent),
    /// An arbiter agent
    Arbiter(ArbiterAgent),
}

impl AgentInner {
    /// Get the agent's ResonatorId
    pub fn id(&self) -> &ResonatorId {
        match self {
            AgentInner::Buyer(a) => a.id(),
            AgentInner::Seller(a) => a.id(),
            AgentInner::Arbiter(a) => a.id(),
        }
    }

    /// Get the agent's wallet balance (if applicable)
    pub fn balance(&self) -> Option<Amount> {
        match self {
            AgentInner::Buyer(a) => Some(a.balance()),
            AgentInner::Seller(a) => Some(a.balance()),
            AgentInner::Arbiter(_) => None,
        }
    }

    /// Get the agent's role
    pub fn role(&self) -> ResonatorAgentRole {
        match self {
            AgentInner::Buyer(_) => ResonatorAgentRole::Buyer,
            AgentInner::Seller(_) => ResonatorAgentRole::Seller,
            AgentInner::Arbiter(_) => ResonatorAgentRole::Arbiter,
        }
    }

    /// Get the agent's name
    pub fn name(&self) -> String {
        self.id().0.clone()
    }

    /// Get wallet reference for buyer/seller
    pub fn wallet(&self) -> Option<&Wallet> {
        match self {
            AgentInner::Buyer(a) => Some(a.wallet()),
            AgentInner::Seller(a) => Some(a.wallet()),
            AgentInner::Arbiter(_) => None,
        }
    }

    /// Get services (seller only)
    pub fn services(&self) -> Vec<&Service> {
        match self {
            AgentInner::Seller(a) => a.services().iter().collect(),
            _ => vec![],
        }
    }

    /// Get kernel trace (if supported)
    pub fn kernel_trace(&self) -> Option<&KernelTrace> {
        match self {
            AgentInner::Buyer(a) => Some(a.kernel_trace()),
            AgentInner::Seller(a) => Some(a.kernel_trace()),
            AgentInner::Arbiter(a) => Some(a.kernel_trace()),
        }
    }

    /// Set active commitment context for gating
    pub fn set_active_commitment(&mut self, commitment_id: impl Into<String>, approved: bool) {
        match self {
            AgentInner::Buyer(a) => a.set_active_commitment(commitment_id, approved),
            AgentInner::Seller(a) => a.set_active_commitment(commitment_id, approved),
            AgentInner::Arbiter(a) => a.set_active_commitment(commitment_id, approved),
        }
    }

    /// Clear active commitment context
    pub fn clear_active_commitment(&mut self) {
        match self {
            AgentInner::Buyer(a) => a.clear_active_commitment(),
            AgentInner::Seller(a) => a.clear_active_commitment(),
            AgentInner::Arbiter(a) => a.clear_active_commitment(),
        }
    }
}

// ============================================================================
// MapleResonatorAgent - The Core Bridge Type
// ============================================================================

/// A MapleResonatorAgent wraps an OpeniBank agent as a Maple Resonator
///
/// This is the core bridge type that connects OpeniBank's economic agents
/// to Maple's Resonance Architecture.
///
/// # Usage
///
/// ```ignore
/// let agent = MapleResonatorAgent::new_buyer(
///     "Alice",
///     ledger.clone(),
///     brain,
///     Some(resonator_handle),
/// );
/// ```
pub struct MapleResonatorAgent {
    /// Display name
    pub name: String,

    /// Maple Resonator handle (for runtime interactions)
    pub resonator_handle: Option<maple_runtime::ResonatorHandle>,

    /// The inner OpeniBank agent
    pub agent: AgentInner,

    /// Current presence state
    pub presence: AgentPresenceState,

    /// Activity log for dashboard
    pub activity: AgentActivity,

    /// When this resonator agent was created
    pub created_at: DateTime<Utc>,

    /// Trade count for this agent
    pub trade_count: u32,

    /// LLM model being used (if any)
    pub llm_model: Option<String>,
}

impl MapleResonatorAgent {
    /// Create a new buyer resonator agent
    pub fn new_buyer(
        name: &str,
        ledger: Arc<Ledger>,
        brain: AgentBrain,
        resonator_handle: Option<maple_runtime::ResonatorHandle>,
    ) -> Self {
        let id = openibank_core::ResonatorId(format!("res_{}", name.to_lowercase().replace(' ', "_")));
        let buyer = BuyerAgent::with_brain(id, ledger, brain);

        let mut activity = AgentActivity::default();
        activity.log(
            ActivityCategory::Created,
            format!("Buyer agent '{}' created", name),
        );

        Self {
            name: name.to_string(),
            resonator_handle,
            agent: AgentInner::Buyer(buyer),
            presence: AgentPresenceState::Idle,
            activity,
            created_at: Utc::now(),
            trade_count: 0,
            llm_model: None,
        }
    }

    /// Create a new seller resonator agent
    pub fn new_seller(
        name: &str,
        ledger: Arc<Ledger>,
        brain: AgentBrain,
        resonator_handle: Option<maple_runtime::ResonatorHandle>,
    ) -> Self {
        let id = openibank_core::ResonatorId(format!("res_{}", name.to_lowercase().replace(' ', "_")));
        let seller = SellerAgent::with_brain(id, ledger, brain);

        let mut activity = AgentActivity::default();
        activity.log(
            ActivityCategory::Created,
            format!("Seller agent '{}' created", name),
        );

        Self {
            name: name.to_string(),
            resonator_handle,
            agent: AgentInner::Seller(seller),
            presence: AgentPresenceState::Idle,
            activity,
            created_at: Utc::now(),
            trade_count: 0,
            llm_model: None,
        }
    }

    /// Create a new arbiter resonator agent
    pub fn new_arbiter(
        name: &str,
        ledger: Arc<Ledger>,
        brain: AgentBrain,
        resonator_handle: Option<maple_runtime::ResonatorHandle>,
    ) -> Self {
        let id = openibank_core::ResonatorId(format!("res_{}", name.to_lowercase().replace(' ', "_")));
        let arbiter = ArbiterAgent::with_brain(id, ledger, brain);

        let mut activity = AgentActivity::default();
        activity.log(
            ActivityCategory::Created,
            format!("Arbiter agent '{}' created", name),
        );

        Self {
            name: name.to_string(),
            resonator_handle,
            agent: AgentInner::Arbiter(arbiter),
            presence: AgentPresenceState::Idle,
            activity,
            created_at: Utc::now(),
            trade_count: 0,
            llm_model: None,
        }
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get the agent's OpeniBank ResonatorId
    pub fn id(&self) -> &ResonatorId {
        self.agent.id()
    }

    /// Get the agent's role
    pub fn role(&self) -> ResonatorAgentRole {
        self.agent.role()
    }

    /// Get the agent's wallet balance
    pub fn balance(&self) -> Option<Amount> {
        self.agent.balance()
    }

    /// Get the wallet reference
    pub fn wallet(&self) -> Option<&Wallet> {
        self.agent.wallet()
    }

    /// Get services (seller only)
    pub fn services(&self) -> Vec<&Service> {
        self.agent.services()
    }

    /// Get the Maple ResonatorProfile for this agent
    pub fn maple_profile(&self) -> resonator_types::ResonatorProfile {
        build_resonator_profile(&self.name, &self.agent.role(), None)
    }

    // ========================================================================
    // Agent Type Access (for operations)
    // ========================================================================

    /// Get mutable reference to buyer agent (if this is a buyer)
    pub fn as_buyer_mut(&mut self) -> Option<&mut BuyerAgent> {
        match &mut self.agent {
            AgentInner::Buyer(a) => Some(a),
            _ => None,
        }
    }

    /// Get mutable reference to seller agent (if this is a seller)
    pub fn as_seller_mut(&mut self) -> Option<&mut SellerAgent> {
        match &mut self.agent {
            AgentInner::Seller(a) => Some(a),
            _ => None,
        }
    }

    /// Get mutable reference to arbiter agent (if this is an arbiter)
    pub fn as_arbiter_mut(&mut self) -> Option<&mut ArbiterAgent> {
        match &mut self.agent {
            AgentInner::Arbiter(a) => Some(a),
            _ => None,
        }
    }

    /// Get reference to buyer agent (if this is a buyer)
    pub fn as_buyer(&self) -> Option<&BuyerAgent> {
        match &self.agent {
            AgentInner::Buyer(a) => Some(a),
            _ => None,
        }
    }

    /// Get reference to seller agent (if this is a seller)
    pub fn as_seller(&self) -> Option<&SellerAgent> {
        match &self.agent {
            AgentInner::Seller(a) => Some(a),
            _ => None,
        }
    }

    /// Get reference to arbiter agent (if this is an arbiter)
    pub fn as_arbiter(&self) -> Option<&ArbiterAgent> {
        match &self.agent {
            AgentInner::Arbiter(a) => Some(a),
            _ => None,
        }
    }

    // ========================================================================
    // Presence Management
    // ========================================================================

    /// Update the agent's presence state
    pub async fn set_presence(&mut self, state: AgentPresenceState) {
        let old = self.presence.clone();
        self.presence = state.clone();

        self.activity.log(
            ActivityCategory::PresenceChanged,
            format!("Presence: {:?} → {:?}", old, state),
        );

        // Signal to Maple runtime if we have a handle
        if let Some(ref handle) = self.resonator_handle {
            let maple_presence = state.to_maple_presence();
            if let Err(e) = handle.signal_presence(maple_presence).await {
                tracing::warn!("Failed to signal presence to Maple runtime: {:?}", e);
            }
        }
    }

    // ========================================================================
    // Activity Logging
    // ========================================================================

    /// Log a trade activity
    pub fn log_trade(&mut self, description: impl Into<String>, data: Option<serde_json::Value>) {
        self.trade_count += 1;
        self.activity.log_with_data(ActivityCategory::Trade, description, data);
    }

    /// Log an LLM reasoning activity
    pub fn log_llm_reasoning(&mut self, description: impl Into<String>, data: Option<serde_json::Value>) {
        self.activity.log_with_data(ActivityCategory::LLMReasoning, description, data);
    }

    /// Log a balance change
    pub fn log_balance_change(&mut self, description: impl Into<String>) {
        self.activity.log(ActivityCategory::BalanceChanged, description);
    }

    /// Log an error
    pub fn log_error(&mut self, description: impl Into<String>) {
        self.activity.log(ActivityCategory::Error, description);
    }

    /// Get the full activity log
    pub fn activity_log(&self) -> &AgentActivity {
        &self.activity
    }

    /// Get kernel trace (if available)
    pub fn kernel_trace(&self) -> Option<&KernelTrace> {
        self.agent.kernel_trace()
    }

    /// Set active commitment for kernel gate
    pub fn set_active_commitment(&mut self, commitment_id: impl Into<String>, approved: bool) {
        self.agent.set_active_commitment(commitment_id, approved);
    }

    /// Clear active commitment for kernel gate
    pub fn clear_active_commitment(&mut self) {
        self.agent.clear_active_commitment();
    }

    // ========================================================================
    // Serialization (for API/Dashboard)
    // ========================================================================

    /// Serialize agent state for API responses
    pub fn to_api_info(&self) -> AgentApiInfo {
        AgentApiInfo {
            id: self.id().0.clone(),
            name: self.name.clone(),
            role: self.role(),
            balance: self.balance().map(|a| a.0),
            presence: self.presence.clone(),
            trade_count: self.trade_count,
            services: self.services().iter().map(|s| ServiceInfo {
                name: s.name.clone(),
                description: s.description.clone(),
                price: s.price.0,
            }).collect(),
            llm_model: self.llm_model.clone(),
            created_at: self.created_at,
            has_resonator: self.resonator_handle.is_some(),
            recent_activity: self.activity.recent(10).to_vec(),
            kernel_trace_events: self.kernel_trace().map(|t| t.events.len()).unwrap_or(0),
        }
    }
}

// ============================================================================
// API Response Types
// ============================================================================

/// Agent info for API responses / dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApiInfo {
    pub id: String,
    pub name: String,
    pub role: ResonatorAgentRole,
    pub balance: Option<u64>,
    pub presence: AgentPresenceState,
    pub trade_count: u32,
    pub services: Vec<ServiceInfo>,
    pub llm_model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub has_resonator: bool,
    pub recent_activity: Vec<ActivityEntry>,
    pub kernel_trace_events: usize,
}

/// Service info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub price: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ledger() -> Arc<Ledger> {
        Arc::new(Ledger::new())
    }

    #[test]
    fn test_create_buyer_agent() {
        let brain = AgentBrain::deterministic();
        let agent = MapleResonatorAgent::new_buyer("Alice", test_ledger(), brain, None);
        assert_eq!(agent.name, "Alice");
        assert_eq!(agent.role(), ResonatorAgentRole::Buyer);
        assert!(agent.balance().is_some());
        assert!(!agent.activity.is_empty());
    }

    #[test]
    fn test_create_seller_agent() {
        let brain = AgentBrain::deterministic();
        let agent = MapleResonatorAgent::new_seller("DataCorp", test_ledger(), brain, None);
        assert_eq!(agent.name, "DataCorp");
        assert_eq!(agent.role(), ResonatorAgentRole::Seller);
    }

    #[test]
    fn test_create_arbiter_agent() {
        let brain = AgentBrain::deterministic();
        let agent = MapleResonatorAgent::new_arbiter("Judge", test_ledger(), brain, None);
        assert_eq!(agent.name, "Judge");
        assert_eq!(agent.role(), ResonatorAgentRole::Arbiter);
        assert!(agent.balance().is_none()); // Arbiters don't have wallets
    }

    #[test]
    fn test_activity_logging() {
        let brain = AgentBrain::deterministic();
        let mut agent = MapleResonatorAgent::new_buyer("Alice", test_ledger(), brain, None);

        agent.log_trade("Bought service from DataCorp for $100", None);
        agent.log_trade("Bought service from CloudAI for $200", None);
        agent.log_balance_change("Balance: $500 → $400");

        assert_eq!(agent.trade_count, 2);
        // 1 created + 2 trades + 1 balance change = 4
        assert_eq!(agent.activity.len(), 4);
    }

    #[test]
    fn test_api_info() {
        let brain = AgentBrain::deterministic();
        let agent = MapleResonatorAgent::new_buyer("Alice", test_ledger(), brain, None);
        let info = agent.to_api_info();

        assert_eq!(info.name, "Alice");
        assert_eq!(info.role, ResonatorAgentRole::Buyer);
        assert!(!info.has_resonator);
    }
}
