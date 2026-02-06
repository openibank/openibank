//! Wallet delegation

use openibank_types::*;

/// A delegation of spending authority
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletDelegation {
    /// Source wallet
    pub from_wallet: WalletId,
    /// Delegated to agent
    pub to_agent: AgentId,
    /// Compartment delegated
    pub compartment: CompartmentId,
    /// Spending limits
    pub limits: SpendingLimits,
    /// Whether sub-delegation is allowed
    pub allow_sub_delegation: bool,
    /// When created
    pub created_at: TemporalAnchor,
    /// When expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether revoked
    pub revoked: bool,
}

impl WalletDelegation {
    /// Check if delegation is active
    pub fn is_active(&self) -> bool {
        if self.revoked {
            return false;
        }
        if let Some(expires) = self.expires_at {
            chrono::Utc::now() < expires
        } else {
            true
        }
    }
}
