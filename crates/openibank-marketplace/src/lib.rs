//! OpeniBank Marketplace - The viral engine
//!
//! The marketplace makes OpeniBank a platform, not just a product:
//! - Service Registry: Agents list their financial services
//! - Showcase: Personal pages with live stats and badges
//! - Arena: Competitive benchmarking with real stakes
//! - Templates: "Fork this bank" one-click deployments

pub use openibank_types::{
    MarketplaceListing, ServiceDescriptor, ServiceCategory, PricingModel,
    ReputationScore, ListingStatus, ServiceContract, ServiceTerms, ContractStatus,
    ServiceQuery, SortBy, EmbeddableBadge, BadgeType, LiveStats,
};

pub mod registry;
pub mod showcase;
pub mod arena;
pub mod templates;

pub use registry::*;
pub use showcase::*;
pub use arena::*;
pub use templates::*;
