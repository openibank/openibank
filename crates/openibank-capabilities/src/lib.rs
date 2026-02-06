//! OpeniBank Capabilities - Viral adoption protocol
//!
//! Agents automatically discover each other's capabilities, creating
//! network effects where every new agent increases value for all.

use serde::{Deserialize, Serialize};
use openibank_types::*;

/// Standard capabilities that agents can implement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardCapability {
    /// Accept payments
    AcceptPayment,
    /// Send payments
    SendPayment,
    /// Issue invoices
    IssueInvoice,
    /// Hold escrow
    EscrowHold,
    /// Release escrow
    EscrowRelease,
    /// Clear transactions
    ClearTransactions,
    /// Settle positions
    SettlePositions,
    /// Provide price quotes
    ProvideQuote,
    /// Assess risk
    AssessRisk,
    /// Check compliance
    CheckCompliance,
    /// Market making
    MarketMake,
    /// Treasury management
    ManageTreasury,
}

/// Capability manifest for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    /// Version
    pub version: String,
    /// Standard capabilities
    pub capabilities: Vec<StandardCapability>,
    /// Custom capabilities
    pub custom_capabilities: Vec<String>,
    /// API endpoints
    pub endpoints: Vec<Endpoint>,
    /// Supported currencies
    pub supported_currencies: Vec<Currency>,
    /// Pricing model
    pub pricing: PricingModel,
    /// Reputation score
    pub reputation: Option<ReputationScore>,
    /// Arena rank
    pub arena_rank: Option<u32>,
}

/// API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    /// Path
    pub path: String,
    /// Method
    pub method: String,
    /// Description
    pub description: String,
}

impl CapabilityManifest {
    /// Create a new manifest
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            capabilities: vec![],
            custom_capabilities: vec![],
            endpoints: vec![],
            supported_currencies: vec![Currency::iusd()],
            pricing: PricingModel {
                base_fee: None,
                per_transaction: None,
                percentage: None,
                subscription: None,
                attention_cost: 10,
            },
            reputation: None,
            arena_rank: None,
        }
    }

    /// Add a capability
    pub fn with_capability(mut self, cap: StandardCapability) -> Self {
        self.capabilities.push(cap);
        self
    }
}

impl Default for CapabilityManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability discovery trait
#[async_trait::async_trait]
pub trait CapabilityDiscovery: Send + Sync {
    /// Discover capabilities of an agent
    async fn discover(&self, agent: &AgentId) -> Result<CapabilityManifest>;

    /// Search for agents with specific capabilities
    async fn search(&self, capabilities: &[StandardCapability]) -> Result<Vec<AgentId>>;

    /// Register own capabilities
    async fn register(&self, manifest: CapabilityManifest) -> Result<()>;
}
