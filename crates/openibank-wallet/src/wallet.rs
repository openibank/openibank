//! Wallet implementation

use crate::{Compartment, CompartmentPurpose};
use openibank_types::*;
use std::collections::HashMap;

/// A programmable wallet
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Wallet {
    /// Wallet ID
    pub id: WalletId,
    /// Owner
    pub owner: OwnerId,
    /// Compartments
    pub compartments: Vec<Compartment>,
    /// Active permits
    pub permits: Vec<PermitId>,
    /// When created
    pub created_at: TemporalAnchor,
}

impl Wallet {
    /// Create a new wallet
    pub fn new(owner: OwnerId) -> Self {
        let mut wallet = Self {
            id: WalletId::new(),
            owner,
            compartments: vec![],
            permits: vec![],
            created_at: TemporalAnchor::now(),
        };

        // Create default operating compartment
        wallet
            .compartments
            .push(Compartment::new("Operating", CompartmentPurpose::Operating));

        wallet
    }

    /// Get total balance across all compartments
    pub fn total_balance(&self, currency: &Currency) -> Amount {
        self.compartments
            .iter()
            .fold(Amount::zero(*currency), |acc, c| {
                acc.checked_add(c.balance(currency)).unwrap_or(acc)
            })
    }

    /// Get balances by currency
    pub fn balances(&self) -> HashMap<Currency, Amount> {
        let mut balances = HashMap::new();
        for compartment in &self.compartments {
            for (currency, amount) in &compartment.balances {
                let entry = balances
                    .entry(*currency)
                    .or_insert_with(|| Amount::zero(*currency));
                if let Ok(new_total) = entry.checked_add(*amount) {
                    *entry = new_total;
                }
            }
        }
        balances
    }

    /// Find compartment by ID
    pub fn compartment(&self, id: &CompartmentId) -> Option<&Compartment> {
        self.compartments.iter().find(|c| &c.id == id)
    }

    /// Find compartment by ID (mutable)
    pub fn compartment_mut(&mut self, id: &CompartmentId) -> Option<&mut Compartment> {
        self.compartments.iter_mut().find(|c| &c.id == id)
    }
}
