//! Marketplace types for OpeniBank
//!
//! The marketplace is the viral engine - it makes OpeniBank a platform,
//! not just a product. Agents list services, discover capabilities, and
//! hire each other.

use crate::{
    AgentId, Amount, Currency, InstitutionId, ListingId, ResonatorId,
    ServiceContractId, TemporalAnchor,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Category of service offered
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceCategory {
    /// Payment processing
    PaymentProcessing,
    /// Lending protocol
    LendingProtocol,
    /// Insurance underwriting
    InsuranceUnderwriting,
    /// Compliance checking
    ComplianceCheck,
    /// Risk assessment
    RiskAssessment,
    /// Market making
    MarketMaking,
    /// Portfolio management
    PortfolioManagement,
    /// Treasury operations
    TreasuryOps,
    /// Cross-border settlement
    CrossBorderSettlement,
    /// FX/Currency exchange
    CurrencyExchange,
    /// Invoice factoring
    InvoiceFactoring,
    /// Escrow services
    EscrowServices,
    /// Audit services
    AuditServices,
    /// Custom category
    Custom(String),
}

impl ServiceCategory {
    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::PaymentProcessing => "Payment Processing",
            Self::LendingProtocol => "Lending Protocol",
            Self::InsuranceUnderwriting => "Insurance Underwriting",
            Self::ComplianceCheck => "Compliance Check",
            Self::RiskAssessment => "Risk Assessment",
            Self::MarketMaking => "Market Making",
            Self::PortfolioManagement => "Portfolio Management",
            Self::TreasuryOps => "Treasury Operations",
            Self::CrossBorderSettlement => "Cross-Border Settlement",
            Self::CurrencyExchange => "Currency Exchange",
            Self::InvoiceFactoring => "Invoice Factoring",
            Self::EscrowServices => "Escrow Services",
            Self::AuditServices => "Audit Services",
            Self::Custom(name) => name,
        }
    }
}

/// Description of a service's API
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSchema {
    /// OpenAPI/Swagger schema version
    pub version: String,
    /// Endpoints provided
    pub endpoints: Vec<ApiEndpoint>,
    /// Authentication method
    pub auth_method: AuthMethod,
    /// Rate limits
    pub rate_limits: Option<RateLimits>,
}

/// An API endpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// HTTP method
    pub method: String,
    /// Path
    pub path: String,
    /// Description
    pub description: String,
    /// Request schema (JSON Schema)
    pub request_schema: Option<serde_json::Value>,
    /// Response schema (JSON Schema)
    pub response_schema: Option<serde_json::Value>,
}

/// Authentication method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// API key
    ApiKey,
    /// OAuth2
    OAuth2,
    /// MAPLE capability-based
    MapleCapability,
    /// None (public)
    None,
}

/// Rate limits for an API
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimits {
    /// Requests per second
    pub requests_per_second: u32,
    /// Requests per day
    pub requests_per_day: u64,
}

/// Resonator profile for a service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResonatorProfile {
    /// Profile type
    pub profile_type: ResonatorProfileType,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Attention cost
    pub attention_cost: u32,
}

/// Types of resonator profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResonatorProfileType {
    /// Wallet resonator
    Wallet,
    /// Clearing resonator
    Clearing,
    /// Settlement resonator
    Settlement,
    /// Issuer resonator
    Issuer,
    /// Bridge resonator
    Bridge,
    /// Policy resonator
    Policy,
    /// Audit resonator
    Audit,
    /// Marketplace resonator
    Marketplace,
    /// Arena resonator
    Arena,
    /// Custom
    Custom,
}

/// Description of a service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    /// Service name
    pub name: String,
    /// Category
    pub category: ServiceCategory,
    /// Description
    pub description: String,
    /// API schema
    pub api_schema: Option<ApiSchema>,
    /// Resonator profile
    pub resonator_profile: ResonatorProfile,
    /// Whether demo is available
    pub demo_available: bool,
    /// Supported currencies
    pub supported_currencies: Vec<Currency>,
}

/// Pricing model for a service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingModel {
    /// Base fee (fixed)
    pub base_fee: Option<Amount>,
    /// Fee per transaction
    pub per_transaction: Option<Amount>,
    /// Percentage fee (0-100)
    pub percentage: Option<f64>,
    /// Subscription tiers
    pub subscription: Option<SubscriptionTier>,
    /// Attention units consumed
    pub attention_cost: u32,
}

/// Subscription tier
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionTier {
    /// Tier name
    pub name: String,
    /// Monthly fee
    pub monthly_fee: Amount,
    /// Included transactions
    pub included_transactions: u64,
    /// Features included
    pub features: Vec<String>,
}

/// Status of a marketplace listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListingStatus {
    /// Draft (not published)
    Draft,
    /// Pending review
    PendingReview,
    /// Active (published)
    Active,
    /// Paused by provider
    Paused,
    /// Suspended by platform
    Suspended,
    /// Deprecated (no new customers)
    Deprecated,
    /// Removed
    Removed,
}

/// Reputation score for a provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReputationScore {
    /// Overall score (0-100)
    pub overall: u8,
    /// Number of verified receipts
    pub receipt_count: u64,
    /// Transaction volume
    pub volume: Amount,
    /// Success rate (0-100)
    pub success_rate: f64,
    /// Average response time in ms
    pub avg_response_time_ms: u64,
    /// Uptime percentage
    pub uptime_percent: f64,
    /// Number of disputes
    pub disputes: u32,
    /// Last updated
    pub last_updated: TemporalAnchor,
}

impl ReputationScore {
    /// Create a new reputation score
    pub fn new() -> Self {
        Self {
            overall: 50,
            receipt_count: 0,
            volume: Amount::iusd_zero(),
            success_rate: 100.0,
            avg_response_time_ms: 0,
            uptime_percent: 100.0,
            disputes: 0,
            last_updated: TemporalAnchor::now(),
        }
    }
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self::new()
    }
}

/// A marketplace listing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketplaceListing {
    /// Unique listing ID
    pub id: ListingId,
    /// Provider agent
    pub provider: AgentId,
    /// Institution (optional)
    pub institution: Option<InstitutionId>,
    /// Service description
    pub service: ServiceDescriptor,
    /// Pricing
    pub pricing: PricingModel,
    /// Capabilities offered
    pub capabilities: Vec<String>,
    /// Reputation score
    pub reputation: ReputationScore,
    /// Status
    pub status: ListingStatus,
    /// When created
    pub created_at: TemporalAnchor,
    /// When last updated
    pub updated_at: TemporalAnchor,
    /// Tags for search
    pub tags: Vec<String>,
}

/// Terms for hiring a service
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceTerms {
    /// Duration of contract
    pub duration_days: Option<u32>,
    /// Maximum transactions
    pub max_transactions: Option<u64>,
    /// Maximum volume
    pub max_volume: Option<Amount>,
    /// Custom terms
    pub custom_terms: Option<String>,
}

/// A service contract between consumer and provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceContract {
    /// Contract ID
    pub id: ServiceContractId,
    /// Listing this contract is for
    pub listing: ListingId,
    /// Consumer
    pub consumer: AgentId,
    /// Provider
    pub provider: AgentId,
    /// Terms
    pub terms: ServiceTerms,
    /// Status
    pub status: ContractStatus,
    /// When created
    pub created_at: TemporalAnchor,
    /// When expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Transaction count
    pub transaction_count: u64,
    /// Volume used
    pub volume_used: Amount,
}

/// Status of a service contract
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractStatus {
    /// Pending acceptance
    Pending,
    /// Active
    Active,
    /// Completed (natural expiration)
    Completed,
    /// Cancelled
    Cancelled,
    /// Disputed
    Disputed,
}

/// Query for searching marketplace
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceQuery {
    /// Search text
    pub query: Option<String>,
    /// Filter by category
    pub category: Option<ServiceCategory>,
    /// Filter by minimum reputation
    pub min_reputation: Option<u8>,
    /// Filter by maximum price
    pub max_price: Option<Amount>,
    /// Filter by currency support
    pub currency: Option<Currency>,
    /// Tags to include
    pub tags: Vec<String>,
    /// Sort by
    pub sort_by: Option<SortBy>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Sort options for marketplace search
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortBy {
    /// By reputation (highest first)
    Reputation,
    /// By volume (highest first)
    Volume,
    /// By price (lowest first)
    PriceAsc,
    /// By price (highest first)
    PriceDesc,
    /// By newest
    Newest,
    /// By name
    Name,
}

/// Embeddable badge for showcases
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddableBadge {
    /// SVG content
    pub svg: String,
    /// HTML embed code
    pub html_embed: String,
    /// Markdown embed
    pub markdown_embed: String,
    /// Live data URL
    pub live_data_url: String,
}

/// Badge type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BadgeType {
    /// Transaction volume
    TransactionVolume,
    /// Settlement speed
    SettlementSpeed,
    /// Uptime
    Uptime,
    /// Arena rank
    ArenaRank,
    /// Reputation score
    ReputationScore,
    /// Custom badge
    Custom(String),
}

/// Live stats for a showcase
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveStats {
    /// Transactions per day
    pub transactions_per_day: u64,
    /// Total volume
    pub total_volume: Amount,
    /// Uptime percentage
    pub uptime_percent: f64,
    /// Average settlement time in ms
    pub avg_settlement_ms: u64,
    /// Active contracts
    pub active_contracts: u32,
    /// Arena wins
    pub arena_wins: u32,
    /// Arena losses
    pub arena_losses: u32,
    /// Last updated
    pub last_updated: TemporalAnchor,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_category() {
        assert_eq!(
            ServiceCategory::PaymentProcessing.display_name(),
            "Payment Processing"
        );
    }

    #[test]
    fn test_reputation_score() {
        let score = ReputationScore::new();
        assert_eq!(score.overall, 50);
        assert_eq!(score.success_rate, 100.0);
    }

    #[test]
    fn test_listing_status() {
        assert_ne!(ListingStatus::Active, ListingStatus::Draft);
    }
}
