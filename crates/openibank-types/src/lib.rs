//! OpeniBank Types - Canonical domain types for AI-native banking
//!
//! This crate contains all foundational types for OpeniBank with zero dependencies
//! on other openibank crates. It defines the complete type system for:
//!
//! - Identity types (WalletId, AgentId, ResonatorId, etc.)
//! - Currency and amount types with 18-decimal precision
//! - Transaction and payment channel types
//! - Permit and commitment types
//! - Escrow and clearing types
//! - Marketplace and arena types
//!
//! # Architectural Invariants
//!
//! These types support the core OpeniBank security invariants:
//!
//! 1. Private keys NEVER leave the encrypted vault
//! 2. Every value movement requires: Permit → Commitment → Receipt
//! 3. Escrow by default — funds never move directly to counterparties
//! 4. All 8 MAPLE invariants enforced at the banking layer
//!
//! # Resonance Flow
//!
//! ```text
//! Presence → Coupling → Meaning → Intent → Commitment → Consequence
//! ```

pub mod identity;
pub mod currency;
pub mod amount;
pub mod transaction;
pub mod permit;
pub mod commitment;
pub mod escrow;
pub mod clearing;
pub mod marketplace;
pub mod arena;
pub mod receipt;
pub mod error;

pub use identity::*;
pub use currency::*;
pub use amount::*;
pub use transaction::*;
pub use permit::*;
pub use commitment::*;
pub use escrow::*;
pub use clearing::*;
pub use marketplace::*;
pub use arena::*;
pub use receipt::*;
pub use error::*;

/// Version of the OpeniBank types schema
pub const TYPES_VERSION: &str = "0.1.0";

/// Temporal anchor for causal ordering (from MAPLE)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TemporalAnchor {
    /// Monotonically increasing timestamp
    pub timestamp: i64,
    /// Sequence number for ordering within same timestamp
    pub sequence: u64,
}

impl TemporalAnchor {
    /// Create a new temporal anchor at the current time
    pub fn now() -> Self {
        Self {
            timestamp: chrono::Utc::now().timestamp_millis(),
            sequence: 0,
        }
    }

    /// Create a new temporal anchor with explicit values
    pub fn new(timestamp: i64, sequence: u64) -> Self {
        Self { timestamp, sequence }
    }

    /// Create the next anchor in sequence
    pub fn next(&self) -> Self {
        Self {
            timestamp: self.timestamp,
            sequence: self.sequence + 1,
        }
    }
}

impl Default for TemporalAnchor {
    fn default() -> Self {
        Self::now()
    }
}
