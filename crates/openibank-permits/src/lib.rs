//! OpeniBank Permits - Spend permits and authorization
//!
//! SpendPermits are the agent-native "currency of authority".

pub use openibank_types::{SpendPermit, SpendPermitBuilder, PermitScope, RecipientPolicy};

/// Re-export types
pub mod prelude {
    pub use openibank_types::{
        SpendPermit, SpendPermitBuilder, PermitScope, RecipientPolicy,
        SpendPurpose, SpendCategory, PermitCondition, PermitId,
    };
}
