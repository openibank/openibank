//! OpeniBank Marketplace - The viral engine
//!
//! The marketplace makes OpeniBank a platform, not just a product:
//! - Service Registry: Agents list their financial services
//! - Showcase: Personal pages with live stats and badges
//! - Arena: Competitive benchmarking with real stakes
//! - Templates: "Fork this bank" one-click deployments
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      MARKETPLACE                            │
//! ├─────────────────┬─────────────────┬─────────────────────────┤
//! │    Registry     │     Arena       │       Templates         │
//! │  ┌───────────┐  │  ┌───────────┐  │  ┌───────────────────┐  │
//! │  │ Listings  │  │  │ Challenges│  │  │ "Fork this bank"  │  │
//! │  │ Contracts │  │  │ Stakes    │  │  │ One-click deploy  │  │
//! │  │ Reputation│  │  │ Leaderbd  │  │  │ Starter kits      │  │
//! │  └───────────┘  │  └───────────┘  │  └───────────────────┘  │
//! ├─────────────────┴─────────────────┴─────────────────────────┤
//! │                       SHOWCASE                              │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │ Personal pages • Live stats • Embeddable badges       │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example: Listing a Service
//!
//! ```ignore
//! use openibank_marketplace::*;
//!
//! let registry = InMemoryRegistry::new();
//!
//! let listing_id = registry.list_service(
//!     provider,
//!     ServiceDescriptor {
//!         name: "Fast Payments".to_string(),
//!         category: ServiceCategory::PaymentProcessing,
//!         // ...
//!     },
//!     PricingModel {
//!         per_transaction: Some(Amount::iusd(0.10)),
//!         // ...
//!     },
//! ).await?;
//! ```

pub use openibank_types::{
    // Marketplace types
    MarketplaceListing, ServiceDescriptor, ServiceCategory, PricingModel,
    ReputationScore, ListingStatus, ServiceContract, ServiceTerms, ContractStatus,
    ServiceQuery, SortBy, EmbeddableBadge, BadgeType, LiveStats,
    // Arena types
    ArenaMatch, ArenaChallenge, ArenaStatus, ArenaParticipant,
    ArenaResults, ArenaRanking, Leaderboard, Timeframe,
};

pub mod registry;
pub mod showcase;
pub mod arena;
pub mod templates;

pub use registry::{ServiceRegistry, InMemoryRegistry};
pub use showcase::{ShowcaseManager, InMemoryShowcase, ShowcaseConfig, ShowcasePage, ShowcaseTheme, SocialLinks, StatsUpdate, ArenaResult};
pub use arena::{ArenaEngine, InMemoryArena, ArenaEscrow};
pub use templates::{TemplateStore, InMemoryTemplateStore, BankTemplate, TemplateCategory, ConfigField, ConfigFieldType, DeploymentResult};
