//! Showcase pages

use openibank_types::*;

/// Showcase configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShowcaseConfig {
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Services to highlight
    pub featured_services: Vec<ListingId>,
    /// Custom theme
    pub theme: Option<String>,
}

/// Showcase manager trait
#[async_trait::async_trait]
pub trait ShowcaseManager: Send + Sync {
    /// Create or update a showcase page
    async fn create_showcase(
        &self,
        owner: AgentId,
        config: ShowcaseConfig,
    ) -> Result<()>;

    /// Get live stats
    async fn get_live_stats(
        &self,
        owner: &AgentId,
    ) -> Result<LiveStats>;

    /// Generate embeddable badge
    async fn generate_badge(
        &self,
        owner: &AgentId,
        badge_type: BadgeType,
    ) -> Result<EmbeddableBadge>;
}
