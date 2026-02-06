//! Budget policies for wallets

use openibank_types::*;

/// A budget policy for spending control
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BudgetPolicy {
    /// Budget ID
    pub id: BudgetId,
    /// Owner
    pub owner: OwnerId,
    /// Spending limits
    pub limits: SpendingLimits,
    /// Allowed categories
    pub allowed_categories: Vec<SpendCategory>,
    /// When created
    pub created_at: TemporalAnchor,
    /// When expires (optional)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl BudgetPolicy {
    /// Create a new budget policy
    pub fn new(owner: OwnerId, limits: SpendingLimits) -> Self {
        Self {
            id: BudgetId::new(),
            owner,
            limits,
            allowed_categories: vec![],
            created_at: TemporalAnchor::now(),
            expires_at: None,
        }
    }

    /// Check if budget can cover an amount
    pub fn can_spend(&self, amount: &Amount) -> bool {
        self.limits.can_spend(amount)
    }
}
