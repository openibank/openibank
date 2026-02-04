//! TradeCouplingManager - Buyer↔Seller coupling management
//!
//! Creates transient couplings between buyers and sellers during trades.
//! Couplings represent the active relationship between trading parties,
//! consuming attention budget and enabling coordinated trade execution.
//!
//! # Maple Coupling Rules
//! - Initial strength must be <= 0.3
//! - Coupling requires available attention budget
//! - Strengthening is gradual (0.1 increments on success)
//! - Weakening halves strength on failure/dispute

use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};

use maple_runtime::{
    ResonatorHandle, CouplingHandle,
    ResonatorId, CouplingParams,
    CouplingScope, CouplingPersistence, SymmetryType,
    CouplingError,
};
use maple_runtime::runtime_core::handle::DecouplingResult;

// ============================================================================
// TradeCouplingManager
// ============================================================================

/// Manages buyer↔seller couplings during trades
///
/// Each trade creates a transient coupling between buyer and seller.
/// The coupling consumes attention budget and represents the active
/// trading relationship.
pub struct TradeCouplingManager {
    /// Track active trade couplings: coupling_id_str → TradeCoupling info
    active_couplings: RwLock<HashMap<String, TradeCouplingInfo>>,
}

/// Information about an active trade coupling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeCouplingInfo {
    pub coupling_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub strength: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub trade_id: Option<String>,
    pub status: CouplingStatus,
}

/// Status of a trade coupling
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CouplingStatus {
    /// Coupling is active (trade in progress)
    Active,
    /// Coupling was strengthened (trade succeeded)
    Strengthened,
    /// Coupling was weakened (trade failed/disputed)
    Weakened,
    /// Coupling was decoupled (trade completed)
    Decoupled,
}

impl TradeCouplingManager {
    /// Create a new coupling manager
    pub fn new() -> Self {
        Self {
            active_couplings: RwLock::new(HashMap::new()),
        }
    }

    /// Establish a buyer↔seller coupling for a trade
    ///
    /// Creates a transient coupling with:
    /// - Initial strength: 0.2 (must be <= 0.3 per Maple rules)
    /// - Attention cost: 50 units
    /// - Scope: IntentOnly (buyer's purchase intent)
    /// - Symmetry: Asymmetric (buyer initiates)
    pub async fn establish_trade_coupling(
        &self,
        buyer_handle: &ResonatorHandle,
        seller_id: ResonatorId,
        trade_id: Option<String>,
    ) -> Result<CouplingHandle, CouplingError> {
        let params = CouplingParams {
            source: buyer_handle.id,
            target: seller_id,
            initial_strength: 0.2,
            initial_attention_cost: 50,
            persistence: CouplingPersistence::Transient,
            scope: CouplingScope::IntentOnly,
            symmetry: SymmetryType::Asymmetric { primary: buyer_handle.id },
        };

        tracing::info!(
            "Establishing trade coupling: buyer {:?} → seller {:?} (strength: 0.2)",
            buyer_handle.id,
            seller_id,
        );

        let coupling_handle = buyer_handle.couple_with(seller_id, params).await?;

        // Track the coupling
        let info = TradeCouplingInfo {
            coupling_id: coupling_handle.id.to_string(),
            buyer_id: buyer_handle.id.to_string(),
            seller_id: seller_id.to_string(),
            strength: 0.2,
            created_at: chrono::Utc::now(),
            trade_id,
            status: CouplingStatus::Active,
        };

        if let Ok(mut couplings) = self.active_couplings.write() {
            couplings.insert(coupling_handle.id.to_string(), info);
        }

        Ok(coupling_handle)
    }

    /// Strengthen coupling on successful trade (gradual trust building)
    pub async fn strengthen_on_success(
        &self,
        coupling: &CouplingHandle,
    ) -> Result<(), CouplingError> {
        tracing::info!(
            "Strengthening coupling {:?} by 0.1 (trade success)",
            coupling.id
        );

        coupling.strengthen(0.1).await?;

        // Update tracking
        if let Ok(mut couplings) = self.active_couplings.write() {
            if let Some(info) = couplings.get_mut(&coupling.id.to_string()) {
                info.strength += 0.1;
                info.status = CouplingStatus::Strengthened;
            }
        }

        Ok(())
    }

    /// Weaken coupling on trade failure/dispute (halve strength)
    pub async fn weaken_on_failure(
        &self,
        coupling: &CouplingHandle,
    ) -> Result<(), CouplingError> {
        tracing::info!(
            "Weakening coupling {:?} by 0.5 (trade failure)",
            coupling.id
        );

        coupling.weaken(0.5).await?;

        // Update tracking
        if let Ok(mut couplings) = self.active_couplings.write() {
            if let Some(info) = couplings.get_mut(&coupling.id.to_string()) {
                info.strength *= 0.5;
                info.status = CouplingStatus::Weakened;
            }
        }

        Ok(())
    }

    /// Decouple after trade completes
    pub async fn decouple_after_trade(
        &self,
        coupling: CouplingHandle,
    ) -> Result<DecouplingResult, CouplingError> {
        let coupling_id_str = coupling.id.to_string();

        tracing::info!(
            "Decoupling {:?} (trade complete)",
            coupling.id
        );

        let result = coupling.decouple().await?;

        // Update tracking
        if let Ok(mut couplings) = self.active_couplings.write() {
            if let Some(info) = couplings.get_mut(&coupling_id_str) {
                info.status = CouplingStatus::Decoupled;
            }
        }

        Ok(result)
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get all active couplings
    pub fn active_couplings(&self) -> Vec<TradeCouplingInfo> {
        self.active_couplings
            .read()
            .map(|c| c.values().filter(|info| info.status == CouplingStatus::Active).cloned().collect())
            .unwrap_or_default()
    }

    /// Get all couplings (including completed)
    pub fn all_couplings(&self) -> Vec<TradeCouplingInfo> {
        self.active_couplings
            .read()
            .map(|c| c.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get coupling count
    pub fn count(&self) -> usize {
        self.active_couplings
            .read()
            .map(|c| c.values().filter(|info| info.status == CouplingStatus::Active).count())
            .unwrap_or(0)
    }

    /// Get coupling info by ID
    pub fn get_coupling_info(&self, coupling_id: &str) -> Option<TradeCouplingInfo> {
        self.active_couplings
            .read()
            .ok()
            .and_then(|c| c.get(coupling_id).cloned())
    }
}

impl Default for TradeCouplingManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Dashboard Types
// ============================================================================

/// Coupling summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingsSummary {
    pub active_count: usize,
    pub total_count: usize,
    pub couplings: Vec<TradeCouplingInfo>,
}

impl TradeCouplingManager {
    /// Get coupling summary for dashboard
    pub fn dashboard_summary(&self) -> CouplingsSummary {
        let all = self.all_couplings();
        let active = all.iter().filter(|c| c.status == CouplingStatus::Active).count();
        CouplingsSummary {
            active_count: active,
            total_count: all.len(),
            couplings: all,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coupling_manager_new() {
        let manager = TradeCouplingManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.active_couplings().is_empty());
    }

    #[test]
    fn test_dashboard_summary() {
        let manager = TradeCouplingManager::new();
        let summary = manager.dashboard_summary();
        assert_eq!(summary.active_count, 0);
        assert_eq!(summary.total_count, 0);
    }
}
