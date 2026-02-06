//! Service registry

use openibank_types::*;

/// Service registry trait
#[async_trait::async_trait]
pub trait ServiceRegistry: Send + Sync {
    /// List a service
    async fn list_service(
        &self,
        provider: AgentId,
        service: ServiceDescriptor,
        pricing: PricingModel,
    ) -> Result<ListingId>;

    /// Search services
    async fn search(
        &self,
        query: ServiceQuery,
    ) -> Result<Vec<MarketplaceListing>>;

    /// Get a listing
    async fn get_listing(
        &self,
        id: &ListingId,
    ) -> Result<MarketplaceListing>;

    /// Hire a service
    async fn hire(
        &self,
        consumer: AgentId,
        listing: ListingId,
        terms: ServiceTerms,
    ) -> Result<ServiceContract>;
}
