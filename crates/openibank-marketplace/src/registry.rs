//! Service registry implementation

use openibank_types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    async fn search(&self, query: ServiceQuery) -> Result<Vec<MarketplaceListing>>;

    /// Get a listing
    async fn get_listing(&self, id: &ListingId) -> Result<MarketplaceListing>;

    /// Update a listing
    async fn update_listing(
        &self,
        id: &ListingId,
        service: Option<ServiceDescriptor>,
        pricing: Option<PricingModel>,
    ) -> Result<()>;

    /// Change listing status
    async fn set_status(&self, id: &ListingId, status: ListingStatus) -> Result<()>;

    /// Hire a service
    async fn hire(
        &self,
        consumer: AgentId,
        listing: ListingId,
        terms: ServiceTerms,
    ) -> Result<ServiceContract>;

    /// Get contracts for an agent
    async fn get_contracts(&self, agent: &AgentId) -> Result<Vec<ServiceContract>>;

    /// Update reputation based on transaction
    async fn record_transaction(
        &self,
        listing_id: &ListingId,
        success: bool,
        response_time_ms: u64,
        volume: Amount,
    ) -> Result<()>;
}

/// In-memory service registry
pub struct InMemoryRegistry {
    listings: Arc<RwLock<HashMap<ListingId, MarketplaceListing>>>,
    contracts: Arc<RwLock<HashMap<ServiceContractId, ServiceContract>>>,
    provider_index: Arc<RwLock<HashMap<AgentId, Vec<ListingId>>>>,
    category_index: Arc<RwLock<HashMap<ServiceCategory, Vec<ListingId>>>>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self {
            listings: Arc::new(RwLock::new(HashMap::new())),
            contracts: Arc::new(RwLock::new(HashMap::new())),
            provider_index: Arc::new(RwLock::new(HashMap::new())),
            category_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ServiceRegistry for InMemoryRegistry {
    async fn list_service(
        &self,
        provider: AgentId,
        service: ServiceDescriptor,
        pricing: PricingModel,
    ) -> Result<ListingId> {
        let listing_id = ListingId::new();
        let now = TemporalAnchor::now();

        let listing = MarketplaceListing {
            id: listing_id.clone(),
            provider: provider.clone(),
            institution: None,
            service: service.clone(),
            pricing,
            capabilities: service.resonator_profile.capabilities.clone(),
            reputation: ReputationScore::new(),
            status: ListingStatus::Active,
            created_at: now,
            updated_at: now,
            tags: vec![service.category.display_name().to_string()],
        };

        // Store listing
        self.listings
            .write()
            .await
            .insert(listing_id.clone(), listing);

        // Update provider index
        self.provider_index
            .write()
            .await
            .entry(provider)
            .or_default()
            .push(listing_id.clone());

        // Update category index
        self.category_index
            .write()
            .await
            .entry(service.category)
            .or_default()
            .push(listing_id.clone());

        Ok(listing_id)
    }

    async fn search(&self, query: ServiceQuery) -> Result<Vec<MarketplaceListing>> {
        let listings = self.listings.read().await;
        let mut results: Vec<MarketplaceListing> = listings
            .values()
            .filter(|l| {
                // Filter by status
                if l.status != ListingStatus::Active {
                    return false;
                }

                // Filter by category
                if let Some(ref cat) = query.category {
                    if &l.service.category != cat {
                        return false;
                    }
                }

                // Filter by minimum reputation
                if let Some(min_rep) = query.min_reputation {
                    if l.reputation.overall < min_rep {
                        return false;
                    }
                }

                // Filter by search text
                if let Some(ref text) = query.query {
                    let text_lower = text.to_lowercase();
                    if !l.service.name.to_lowercase().contains(&text_lower)
                        && !l.service.description.to_lowercase().contains(&text_lower)
                    {
                        return false;
                    }
                }

                // Filter by currency
                if let Some(ref currency) = query.currency {
                    if !l.service.supported_currencies.contains(currency) {
                        return false;
                    }
                }

                // Filter by tags
                if !query.tags.is_empty() {
                    let has_tag = query.tags.iter().any(|t| l.tags.contains(t));
                    if !has_tag {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort
        if let Some(sort_by) = query.sort_by {
            match sort_by {
                SortBy::Reputation => {
                    results.sort_by(|a, b| b.reputation.overall.cmp(&a.reputation.overall));
                }
                SortBy::Volume => {
                    results.sort_by(|a, b| b.reputation.volume.cmp(&a.reputation.volume));
                }
                SortBy::PriceAsc => {
                    // Sort by attention cost as proxy for price
                    results.sort_by(|a, b| a.pricing.attention_cost.cmp(&b.pricing.attention_cost));
                }
                SortBy::PriceDesc => {
                    results.sort_by(|a, b| b.pricing.attention_cost.cmp(&a.pricing.attention_cost));
                }
                SortBy::Newest => {
                    results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                }
                SortBy::Name => {
                    results.sort_by(|a, b| a.service.name.cmp(&b.service.name));
                }
            }
        }

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);
        let results: Vec<_> = results.into_iter().skip(offset).take(limit).collect();

        Ok(results)
    }

    async fn get_listing(&self, id: &ListingId) -> Result<MarketplaceListing> {
        self.listings
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| OpeniBankError::ListingNotFound {
                listing_id: id.0.to_string(),
            })
    }

    async fn update_listing(
        &self,
        id: &ListingId,
        service: Option<ServiceDescriptor>,
        pricing: Option<PricingModel>,
    ) -> Result<()> {
        let mut listings = self.listings.write().await;
        let listing = listings
            .get_mut(id)
            .ok_or_else(|| OpeniBankError::ListingNotFound {
                listing_id: id.0.to_string(),
            })?;

        if let Some(s) = service {
            listing.service = s;
        }
        if let Some(p) = pricing {
            listing.pricing = p;
        }
        listing.updated_at = TemporalAnchor::now();

        Ok(())
    }

    async fn set_status(&self, id: &ListingId, status: ListingStatus) -> Result<()> {
        let mut listings = self.listings.write().await;
        let listing = listings
            .get_mut(id)
            .ok_or_else(|| OpeniBankError::ListingNotFound {
                listing_id: id.0.to_string(),
            })?;

        listing.status = status;
        listing.updated_at = TemporalAnchor::now();

        Ok(())
    }

    async fn hire(
        &self,
        consumer: AgentId,
        listing_id: ListingId,
        terms: ServiceTerms,
    ) -> Result<ServiceContract> {
        let listing = self.get_listing(&listing_id).await?;

        if listing.status != ListingStatus::Active {
            return Err(OpeniBankError::ListingNotActive {
                listing_id: listing_id.0.to_string(),
            });
        }

        let contract_id = ServiceContractId::new();
        let now = TemporalAnchor::now();

        let expires_at = terms.duration_days.map(|days| {
            chrono::Utc::now() + chrono::Duration::days(days as i64)
        });

        let contract = ServiceContract {
            id: contract_id.clone(),
            listing: listing_id,
            consumer,
            provider: listing.provider,
            terms,
            status: ContractStatus::Active,
            created_at: now,
            expires_at,
            transaction_count: 0,
            volume_used: Amount::zero(listing.pricing.base_fee.map(|f| f.currency).unwrap_or(Currency::iusd())),
        };

        self.contracts
            .write()
            .await
            .insert(contract_id, contract.clone());

        Ok(contract)
    }

    async fn get_contracts(&self, agent: &AgentId) -> Result<Vec<ServiceContract>> {
        let contracts = self.contracts.read().await;
        let results: Vec<ServiceContract> = contracts
            .values()
            .filter(|c| &c.consumer == agent || &c.provider == agent)
            .cloned()
            .collect();

        Ok(results)
    }

    async fn record_transaction(
        &self,
        listing_id: &ListingId,
        success: bool,
        response_time_ms: u64,
        volume: Amount,
    ) -> Result<()> {
        let mut listings = self.listings.write().await;
        let listing = listings
            .get_mut(listing_id)
            .ok_or_else(|| OpeniBankError::ListingNotFound {
                listing_id: listing_id.0.to_string(),
            })?;

        let rep = &mut listing.reputation;
        rep.receipt_count += 1;

        // Update success rate with exponential moving average
        let alpha = 0.1;
        let success_val = if success { 100.0 } else { 0.0 };
        rep.success_rate = rep.success_rate * (1.0 - alpha) + success_val * alpha;

        // Update response time with moving average
        rep.avg_response_time_ms = ((rep.avg_response_time_ms as f64) * (1.0 - alpha)
            + (response_time_ms as f64) * alpha) as u64;

        // Update volume
        if let Ok(new_vol) = rep.volume.checked_add(volume) {
            rep.volume = new_vol;
        }

        // Recalculate overall score
        rep.overall = calculate_overall_score(rep);
        rep.last_updated = TemporalAnchor::now();

        Ok(())
    }
}

/// Calculate overall reputation score from components
fn calculate_overall_score(rep: &ReputationScore) -> u8 {
    // Weighted average of components
    let success_weight = 0.4;
    let uptime_weight = 0.3;
    let response_weight = 0.2;
    let volume_weight = 0.1;

    // Normalize response time (lower is better, cap at 1000ms)
    let response_score = 100.0 - (rep.avg_response_time_ms as f64).min(1000.0) / 10.0;

    // Normalize volume (log scale, more is better)
    let volume_score = ((rep.volume.to_human().log10() + 1.0) * 20.0).min(100.0);

    let score = rep.success_rate * success_weight
        + rep.uptime_percent * uptime_weight
        + response_score * response_weight
        + volume_score * volume_weight;

    score.min(100.0).max(0.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> ServiceDescriptor {
        ServiceDescriptor {
            name: "Test Payment Service".to_string(),
            category: ServiceCategory::PaymentProcessing,
            description: "A test payment processing service".to_string(),
            api_schema: None,
            resonator_profile: ResonatorProfile {
                profile_type: ResonatorProfileType::Wallet,
                capabilities: vec!["pay".to_string()],
                attention_cost: 1,
            },
            demo_available: true,
            supported_currencies: vec![Currency::iusd()],
        }
    }

    fn test_pricing() -> PricingModel {
        PricingModel {
            base_fee: Some(Amount::iusd(1.0)),
            per_transaction: Some(Amount::iusd(0.10)),
            percentage: Some(0.5),
            subscription: None,
            attention_cost: 1,
        }
    }

    #[tokio::test]
    async fn test_list_and_get_service() {
        let registry = InMemoryRegistry::new();
        let provider = AgentId::new();

        let listing_id = registry
            .list_service(provider.clone(), test_service(), test_pricing())
            .await
            .unwrap();

        let listing = registry.get_listing(&listing_id).await.unwrap();
        assert_eq!(listing.provider, provider);
        assert_eq!(listing.service.name, "Test Payment Service");
    }

    #[tokio::test]
    async fn test_search() {
        let registry = InMemoryRegistry::new();
        let provider = AgentId::new();

        registry
            .list_service(provider, test_service(), test_pricing())
            .await
            .unwrap();

        let results = registry
            .search(ServiceQuery {
                category: Some(ServiceCategory::PaymentProcessing),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_hire_service() {
        let registry = InMemoryRegistry::new();
        let provider = AgentId::new();
        let consumer = AgentId::new();

        let listing_id = registry
            .list_service(provider.clone(), test_service(), test_pricing())
            .await
            .unwrap();

        let contract = registry
            .hire(
                consumer.clone(),
                listing_id,
                ServiceTerms {
                    duration_days: Some(30),
                    max_transactions: Some(1000),
                    max_volume: None,
                    custom_terms: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(contract.consumer, consumer);
        assert_eq!(contract.provider, provider);
        assert_eq!(contract.status, ContractStatus::Active);
    }

    #[tokio::test]
    async fn test_reputation_update() {
        let registry = InMemoryRegistry::new();
        let provider = AgentId::new();

        let listing_id = registry
            .list_service(provider, test_service(), test_pricing())
            .await
            .unwrap();

        // Record some transactions
        for _ in 0..10 {
            registry
                .record_transaction(&listing_id, true, 50, Amount::iusd(100.0))
                .await
                .unwrap();
        }

        let listing = registry.get_listing(&listing_id).await.unwrap();
        assert_eq!(listing.reputation.receipt_count, 10);
        assert!(listing.reputation.overall > 50);
    }
}
