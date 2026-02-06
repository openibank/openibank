//! OpeniBank Policy - Spending controls and policy enforcement
//!
//! Implements the policy engine for enforcing spending limits,
//! compliance rules, and risk constraints.

use openibank_types::*;
use serde::{Deserialize, Serialize};

/// A policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule type
    pub rule_type: PolicyRuleType,
    /// Whether enabled
    pub enabled: bool,
    /// Priority (lower = higher priority)
    pub priority: u32,
}

/// Types of policy rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyRuleType {
    /// Spending limit
    SpendingLimit {
        limits: SpendingLimits,
    },
    /// Counterparty whitelist/blacklist
    CounterpartyFilter {
        allow: Vec<WalletId>,
        deny: Vec<WalletId>,
    },
    /// Category restrictions
    CategoryRestriction {
        allowed: Vec<SpendCategory>,
        denied: Vec<SpendCategory>,
    },
    /// Time-based rules
    TimeRestriction {
        allowed_hours: Vec<u8>,
        allowed_days: Vec<u8>,
    },
    /// Amount thresholds requiring approval
    ApprovalRequired {
        threshold: Amount,
        approvers: Vec<AgentId>,
    },
    /// Custom rule (evaluated via expression)
    Custom {
        expression: String,
    },
}

/// Result of a policy check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheckResult {
    /// Whether passed
    pub passed: bool,
    /// Rules that were checked
    pub rules_checked: Vec<String>,
    /// Rules that failed
    pub rules_failed: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Requires approval
    pub requires_approval: bool,
    /// Approvers needed
    pub approvers_needed: Vec<AgentId>,
}

impl PolicyCheckResult {
    /// Create a passing result
    pub fn pass() -> Self {
        Self {
            passed: true,
            rules_checked: vec![],
            rules_failed: vec![],
            warnings: vec![],
            requires_approval: false,
            approvers_needed: vec![],
        }
    }

    /// Create a failing result
    pub fn fail(reason: String) -> Self {
        Self {
            passed: false,
            rules_checked: vec![],
            rules_failed: vec![reason],
            warnings: vec![],
            requires_approval: false,
            approvers_needed: vec![],
        }
    }
}

/// Policy engine trait
#[async_trait::async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Check a transaction against policies
    async fn check_transaction(
        &self,
        wallet: &WalletId,
        amount: &Amount,
        recipient: &WalletId,
        category: &SpendCategory,
    ) -> Result<PolicyCheckResult>;

    /// Add a policy rule
    async fn add_rule(&self, wallet: &WalletId, rule: PolicyRule) -> Result<()>;

    /// Remove a policy rule
    async fn remove_rule(&self, wallet: &WalletId, rule_id: &str) -> Result<()>;

    /// Get all rules for a wallet
    async fn get_rules(&self, wallet: &WalletId) -> Result<Vec<PolicyRule>>;
}
