//! AttentionManager - Attention budget monitoring and gating
//!
//! Monitors and queries Maple's attention allocation system for each agent.
//! Attention budgets gate operations — an agent with exhausted attention
//! cannot create new couplings or execute trades.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use maple_runtime::{
    ResonatorHandle, AttentionBudget,
};

// ============================================================================
// Attention Budget Info (for dashboard)
// ============================================================================

/// Attention budget information for a single agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudgetInfo {
    /// Agent identifier
    pub agent_id: String,
    /// Agent name
    pub agent_name: String,
    /// Total attention capacity
    pub total_capacity: u64,
    /// Currently used attention
    pub used: u64,
    /// Available attention
    pub available: u64,
    /// Utilization percentage (0.0 - 1.0)
    pub utilization: f64,
    /// Whether exhaustion is imminent
    pub is_exhaustion_imminent: bool,
    /// Per-coupling attention allocations
    pub coupling_allocations: HashMap<String, u64>,
}

impl AttentionBudgetInfo {
    /// Create from a Maple AttentionBudget
    pub fn from_budget(agent_id: &str, agent_name: &str, budget: &AttentionBudget) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            total_capacity: budget.total_capacity,
            used: budget.used(),
            available: budget.available(),
            utilization: budget.utilization(),
            is_exhaustion_imminent: budget.is_exhaustion_imminent(),
            coupling_allocations: budget.allocated.iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
        }
    }
}

// ============================================================================
// AttentionManager
// ============================================================================

/// Manages attention budget queries and trade gating
///
/// Uses Maple's attention allocation system to:
/// - Check if an agent has sufficient attention for a trade
/// - Monitor attention utilization across all agents
/// - Detect imminent exhaustion for early warnings
pub struct AttentionManager;

impl AttentionManager {
    /// Create a new attention manager
    pub fn new() -> Self {
        Self
    }

    /// Check if an agent has enough attention for a trade
    ///
    /// A trade requires ~50 attention units for the coupling.
    /// Returns false if:
    /// - Agent has no attention budget
    /// - Agent cannot allocate 50 units
    /// - Agent's attention is exhaustion-imminent
    pub async fn can_trade(handle: &ResonatorHandle) -> bool {
        const TRADE_ATTENTION_COST: u64 = 50;

        if let Some(budget) = handle.attention_status().await {
            budget.can_allocate(TRADE_ATTENTION_COST) && !budget.is_exhaustion_imminent()
        } else {
            // No budget tracked = allow (conservative default)
            true
        }
    }

    /// Get attention status for an agent
    pub async fn get_status(
        handle: &ResonatorHandle,
        agent_name: &str,
    ) -> Option<AttentionBudgetInfo> {
        handle.attention_status().await.map(|budget| {
            AttentionBudgetInfo::from_budget(
                &handle.id.to_string(),
                agent_name,
                &budget,
            )
        })
    }

    /// Check if an agent's attention is exhaustion-imminent
    pub async fn is_exhaustion_imminent(handle: &ResonatorHandle) -> bool {
        handle
            .attention_status()
            .await
            .map(|b| b.is_exhaustion_imminent())
            .unwrap_or(false)
    }
}

impl Default for AttentionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Dashboard Types
// ============================================================================

/// Attention summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionSummary {
    pub agent_budgets: Vec<AttentionBudgetInfo>,
    pub total_capacity: u64,
    pub total_used: u64,
    pub agents_exhaustion_imminent: Vec<String>,
}

impl AttentionSummary {
    /// Create from a list of agent budgets
    pub fn from_budgets(budgets: Vec<AttentionBudgetInfo>) -> Self {
        let total_capacity: u64 = budgets.iter().map(|b| b.total_capacity).sum();
        let total_used: u64 = budgets.iter().map(|b| b.used).sum();
        let exhaustion_imminent: Vec<String> = budgets
            .iter()
            .filter(|b| b.is_exhaustion_imminent)
            .map(|b| b.agent_name.clone())
            .collect();

        Self {
            agent_budgets: budgets,
            total_capacity,
            total_used,
            agents_exhaustion_imminent: exhaustion_imminent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_budget_info() {
        let budget = AttentionBudget::new(1000);
        let info = AttentionBudgetInfo::from_budget("res_alice", "Alice", &budget);

        assert_eq!(info.total_capacity, 1000);
        assert_eq!(info.used, 0);
        // Available = total_capacity - safety_reserve (default 100)
        assert!(info.available > 0);
        assert!(info.available <= 1000);
        assert!(!info.is_exhaustion_imminent);
    }

    #[test]
    fn test_attention_summary() {
        let budget1 = AttentionBudget::new(1000);
        let info1 = AttentionBudgetInfo::from_budget("res_alice", "Alice", &budget1);

        let budget2 = AttentionBudget::new(500);
        let info2 = AttentionBudgetInfo::from_budget("res_bob", "Bob", &budget2);

        let summary = AttentionSummary::from_budgets(vec![info1, info2]);
        assert_eq!(summary.total_capacity, 1500);
        assert_eq!(summary.total_used, 0);
        assert!(summary.agents_exhaustion_imminent.is_empty());
    }
}
