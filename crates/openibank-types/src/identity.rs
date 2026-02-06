//! Identity types for OpeniBank
//!
//! All identity types are strongly typed wrappers around UUIDs to prevent
//! accidental mixing of different ID types.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Macro to generate ID types with common implementations
macro_rules! define_id_type {
    ($name:ident, $prefix:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Create a new random ID
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create from an existing UUID
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Parse from a string (with or without prefix)
            pub fn parse(s: &str) -> Result<Self, uuid::Error> {
                let s = s.strip_prefix(concat!($prefix, "_")).unwrap_or(s);
                Ok(Self(Uuid::parse_str(s)?))
            }

            /// Get the inner UUID
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Convert to prefixed string
            pub fn to_prefixed_string(&self) -> String {
                format!("{}_{}", $prefix, self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}_{}", $prefix, self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl AsRef<Uuid> for $name {
            fn as_ref(&self) -> &Uuid {
                &self.0
            }
        }
    };
}

// Core identity types
define_id_type!(WalletId, "wallet", "Unique identifier for a programmable wallet");
define_id_type!(AgentId, "agent", "Unique identifier for an AI agent");
define_id_type!(ResonatorId, "res", "Unique identifier for a MAPLE Resonator");
define_id_type!(MerchantId, "merchant", "Unique identifier for a merchant");
define_id_type!(InstitutionId, "inst", "Unique identifier for a financial institution");

// Operational identity types
define_id_type!(PermitId, "permit", "Unique identifier for a spend permit");
define_id_type!(CommitmentId, "commit", "Unique identifier for a commitment");
define_id_type!(ReceiptId, "receipt", "Unique identifier for a cryptographic receipt");
define_id_type!(TransactionId, "tx", "Unique identifier for a transaction");
define_id_type!(BatchId, "batch", "Unique identifier for a clearing batch");
define_id_type!(EscrowId, "escrow", "Unique identifier for an escrow");

// Wallet component identity types
define_id_type!(CompartmentId, "compartment", "Unique identifier for a wallet compartment");
define_id_type!(BudgetId, "budget", "Unique identifier for a budget policy");

// Marketplace identity types
define_id_type!(ListingId, "listing", "Unique identifier for a marketplace listing");
define_id_type!(ServiceContractId, "contract", "Unique identifier for a service contract");

// Arena identity types
define_id_type!(ArenaMatchId, "match", "Unique identifier for an arena competition match");
define_id_type!(ChallengeId, "challenge", "Unique identifier for an arena challenge");

// Audit identity types
define_id_type!(AuditEntryId, "audit", "Unique identifier for an audit log entry");
define_id_type!(JournalEntryId, "journal", "Unique identifier for a ledger journal entry");

/// Represents the owner of a resource (can be Agent or Resonator)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnerId {
    /// Owned by an agent
    Agent(AgentId),
    /// Owned by a resonator
    Resonator(ResonatorId),
    /// Owned by an institution
    Institution(InstitutionId),
}

impl OwnerId {
    /// Create an agent owner
    pub fn agent(id: AgentId) -> Self {
        Self::Agent(id)
    }

    /// Create a resonator owner
    pub fn resonator(id: ResonatorId) -> Self {
        Self::Resonator(id)
    }

    /// Create an institution owner
    pub fn institution(id: InstitutionId) -> Self {
        Self::Institution(id)
    }
}

impl fmt::Display for OwnerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agent(id) => write!(f, "{}", id),
            Self::Resonator(id) => write!(f, "{}", id),
            Self::Institution(id) => write!(f, "{}", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_id_creation() {
        let id = WalletId::new();
        let s = id.to_string();
        assert!(s.starts_with("wallet_"));
    }

    #[test]
    fn test_id_parsing() {
        let id = WalletId::new();
        let s = id.to_string();
        let parsed = WalletId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_id_equality() {
        let uuid = Uuid::new_v4();
        let id1 = AgentId::from_uuid(uuid);
        let id2 = AgentId::from_uuid(uuid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_owner_id_variants() {
        let agent = OwnerId::agent(AgentId::new());
        let resonator = OwnerId::resonator(ResonatorId::new());

        match agent {
            OwnerId::Agent(_) => {}
            _ => panic!("Expected Agent variant"),
        }

        match resonator {
            OwnerId::Resonator(_) => {}
            _ => panic!("Expected Resonator variant"),
        }
    }
}
