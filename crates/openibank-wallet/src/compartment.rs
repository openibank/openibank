//! Wallet compartments

use crate::CompartmentPurpose;
use openibank_types::*;
use std::collections::HashMap;

/// A compartment within a wallet
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Compartment {
    /// Compartment ID
    pub id: CompartmentId,
    /// Name
    pub name: String,
    /// Balances by currency
    pub balances: HashMap<Currency, Amount>,
    /// Purpose
    pub purpose: CompartmentPurpose,
    /// Budget constraints (optional)
    pub budget: Option<BudgetId>,
    /// Locked until (optional)
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,
    /// When created
    pub created_at: TemporalAnchor,
}

impl Compartment {
    /// Create a new compartment
    pub fn new(name: impl Into<String>, purpose: CompartmentPurpose) -> Self {
        Self {
            id: CompartmentId::new(),
            name: name.into(),
            balances: HashMap::new(),
            purpose,
            budget: None,
            locked_until: None,
            created_at: TemporalAnchor::now(),
        }
    }

    /// Get balance for a currency
    pub fn balance(&self, currency: &Currency) -> Amount {
        self.balances
            .get(currency)
            .cloned()
            .unwrap_or_else(|| Amount::zero(*currency))
    }

    /// Check if compartment is locked
    pub fn is_locked(&self) -> bool {
        if let Some(until) = self.locked_until {
            chrono::Utc::now() < until
        } else {
            false
        }
    }
}
