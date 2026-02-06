//! OpeniBank Wallet - Programmable wallets with compartments
//!
//! This crate implements programmable wallets that support:
//! - Compartments for segregated funds
//! - Budget policies for spending limits
//! - Delegation to sub-agents
//! - Multi-currency support

use openibank_types::*;

pub mod compartment;
pub mod wallet;
pub mod budget;
pub mod delegation;

pub use compartment::*;
pub use wallet::*;
pub use budget::*;
pub use delegation::*;

/// Purpose of a wallet compartment
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompartmentPurpose {
    /// Operating funds (day-to-day spending)
    Operating,
    /// Reserve/collateral
    Reserve,
    /// Held for escrow
    Escrow,
    /// Revenue compartment
    Revenue,
    /// Delegated to sub-agents
    Delegation,
    /// Marketplace earnings
    Marketplace,
    /// Custom purpose
    Custom(String),
}

impl Default for CompartmentPurpose {
    fn default() -> Self {
        Self::Operating
    }
}
