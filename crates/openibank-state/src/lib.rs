//! OpeniBank Shared State - Unified state management for the OpeniBank ecosystem
//!
//! This crate provides the central `SystemState` that is shared across:
//! - The Playground web service (owns the state)
//! - The CLI (connects via HTTP API)
//! - The Dashboard (reads via SSE events)
//!
//! # Architecture
//!
//! ```text
//! Playground (port 8080)  ←── owns SystemState
//!     ↑ HTTP API
//! CLI commands ───────────→ calls /api/* endpoints on :8080
//! Dashboard ←─────────────← SSE /api/events stream
//! ```

pub mod events;
pub mod activity;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use openibank_core::Amount;
use openibank_ledger::Ledger;
use openibank_issuer::{Issuer, IssuerConfig};
use openibank_maple::{
    IBankRuntime, IBankRuntimeConfig, MapleResonatorAgent,
    bridge::ResonatorAgentRole,
    IBankAccountability, TradeCouplingManager, TradeCommitmentManager,
};

pub use events::SystemEvent;
pub use activity::ActivityEntry;

// ============================================================================
// Agent Registry
// ============================================================================

/// Registry of all agents in the system
pub struct AgentRegistry {
    /// All agents indexed by their ID
    pub agents: HashMap<String, MapleResonatorAgent>,
    /// Total trade count across all agents
    pub trade_count: u32,
    /// Total trading volume (in cents)
    pub total_volume: u64,
    /// Transaction history
    pub transactions: Vec<TransactionRecord>,
    /// All receipts
    pub receipts: Vec<ReceiptRecord>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            trade_count: 0,
            total_volume: 0,
            transactions: Vec::new(),
            receipts: Vec::new(),
        }
    }

    /// Get agent count
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Get buyer count
    pub fn buyer_count(&self) -> usize {
        self.agents.values()
            .filter(|a| a.role() == ResonatorAgentRole::Buyer)
            .count()
    }

    /// Get seller count
    pub fn seller_count(&self) -> usize {
        self.agents.values()
            .filter(|a| a.role() == ResonatorAgentRole::Seller)
            .count()
    }

    /// Get arbiter count
    pub fn arbiter_count(&self) -> usize {
        self.agents.values()
            .filter(|a| a.role() == ResonatorAgentRole::Arbiter)
            .count()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Record of a completed transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub service_name: String,
    pub amount: u64,
    pub status: TransactionStatus,
    pub receipt_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    InEscrow,
    Completed,
    Failed,
    Disputed,
    Refunded,
}

/// Record of a receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRecord {
    pub receipt_id: String,
    pub receipt_type: String,
    pub actor: String,
    pub description: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// System State
// ============================================================================

/// Central system state shared by all components
///
/// The Playground web service owns this state. CLI and Dashboard
/// interact with it via HTTP API and SSE events.
pub struct SystemState {
    /// Maple runtime (iBank config)
    pub runtime: IBankRuntime,
    /// Shared ledger
    pub ledger: Arc<Ledger>,
    /// IUSD issuer
    pub issuer: Arc<RwLock<Issuer>>,
    /// All registered agents
    pub agents: Arc<RwLock<AgentRegistry>>,
    /// Event bus for SSE streaming
    pub events: broadcast::Sender<SystemEvent>,
    /// System activity log
    pub activity_log: Arc<RwLock<Vec<ActivityEntry>>>,
    /// When the system was started
    pub started_at: DateTime<Utc>,

    // ====================================================================
    // Maple Deep Integration Managers
    // ====================================================================

    /// AAS accountability service (identity, capabilities, commitments)
    pub accountability: Arc<IBankAccountability>,
    /// Trade coupling manager (buyer↔seller connections)
    pub coupling_manager: Arc<TradeCouplingManager>,
    /// Trade commitment lifecycle manager
    pub commitment_manager: Arc<TradeCommitmentManager>,
}

impl SystemState {
    /// Create a new SystemState with default configuration
    pub async fn new() -> Result<Self, openibank_maple::runtime::IBankRuntimeError> {
        Self::with_config(IBankRuntimeConfig::default()).await
    }

    /// Create with custom configuration
    pub async fn with_config(
        config: IBankRuntimeConfig,
    ) -> Result<Self, openibank_maple::runtime::IBankRuntimeError> {
        let runtime = IBankRuntime::new(config).await?;
        let ledger = Arc::new(Ledger::new());
        let reserve_cap = Amount::new(10_000_000_00); // $10M reserve cap
        let issuer = Arc::new(RwLock::new(Issuer::new(
            IssuerConfig::default(),
            reserve_cap,
            ledger.clone(),
        )));
        let (events_tx, _) = broadcast::channel(1000);

        // Initialize Maple deep integration managers
        let accountability = Arc::new(IBankAccountability::new());
        let coupling_manager = Arc::new(TradeCouplingManager::new());
        let commitment_manager = Arc::new(TradeCommitmentManager::new());

        Ok(Self {
            runtime,
            ledger,
            issuer,
            agents: Arc::new(RwLock::new(AgentRegistry::new())),
            events: events_tx,
            activity_log: Arc::new(RwLock::new(Vec::new())),
            started_at: Utc::now(),
            accountability,
            coupling_manager,
            commitment_manager,
        })
    }

    /// Broadcast a system event
    pub fn emit_event(&self, event: SystemEvent) {
        // Ignore send errors (no receivers)
        let _ = self.events.send(event);
    }

    /// Subscribe to system events
    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.events.subscribe()
    }

    /// Log a system activity
    pub async fn log_activity(&self, entry: ActivityEntry) {
        let mut log = self.activity_log.write().await;
        log.insert(0, entry);
        // Keep bounded
        if log.len() > 10000 {
            log.truncate(10000);
        }
    }

    /// Get system status summary
    pub async fn status_summary(&self) -> SystemStatusSummary {
        let agents = self.agents.read().await;
        let issuer = self.issuer.read().await;
        let runtime_status = self.runtime.status().await;

        let total_supply = issuer.total_supply().await;
        let remaining_supply = issuer.remaining_supply().await;

        // Maple deep integration stats
        let accountability_info = self.accountability.dashboard_info().await;
        let coupling_summary = self.coupling_manager.dashboard_summary();
        let commitment_summary = self.commitment_manager.dashboard_summary();

        SystemStatusSummary {
            runtime: runtime_status,
            agent_count: agents.agent_count(),
            buyer_count: agents.buyer_count(),
            seller_count: agents.seller_count(),
            arbiter_count: agents.arbiter_count(),
            trade_count: agents.trade_count,
            total_volume: agents.total_volume,
            total_supply: total_supply.0,
            remaining_supply: remaining_supply.0,
            uptime_seconds: (Utc::now() - self.started_at).num_seconds() as u64,
            started_at: self.started_at,
            maple_accountability: accountability_info,
            maple_couplings: coupling_summary,
            maple_commitments: commitment_summary,
        }
    }
}

/// System status summary for API/dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusSummary {
    pub runtime: openibank_maple::runtime::RuntimeStatus,
    pub agent_count: usize,
    pub buyer_count: usize,
    pub seller_count: usize,
    pub arbiter_count: usize,
    pub trade_count: u32,
    pub total_volume: u64,
    pub total_supply: u64,
    pub remaining_supply: u64,
    pub uptime_seconds: u64,
    pub started_at: DateTime<Utc>,
    /// Maple AAS accountability stats
    pub maple_accountability: openibank_maple::AccountabilityInfo,
    /// Maple coupling summary
    pub maple_couplings: openibank_maple::CouplingsSummary,
    /// Maple commitment summary
    pub maple_commitments: openibank_maple::CommitmentsSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_state_creation() {
        let state = SystemState::new().await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_agent_registry() {
        let registry = AgentRegistry::new();
        assert_eq!(registry.agent_count(), 0);
        assert_eq!(registry.buyer_count(), 0);
        assert_eq!(registry.seller_count(), 0);
    }
}
