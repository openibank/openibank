//! Showcase pages implementation

use openibank_types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    pub theme: Option<ShowcaseTheme>,
    /// Social links
    pub social_links: Option<SocialLinks>,
    /// Custom CSS
    pub custom_css: Option<String>,
}

/// Theme configuration for showcase
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShowcaseTheme {
    /// Primary color
    pub primary_color: String,
    /// Secondary color
    pub secondary_color: String,
    /// Background color
    pub background_color: String,
    /// Text color
    pub text_color: String,
    /// Dark mode
    pub dark_mode: bool,
}

impl Default for ShowcaseTheme {
    fn default() -> Self {
        Self {
            primary_color: "#6366f1".to_string(), // Indigo
            secondary_color: "#8b5cf6".to_string(), // Purple
            background_color: "#ffffff".to_string(),
            text_color: "#1f2937".to_string(),
            dark_mode: false,
        }
    }
}

/// Social links
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SocialLinks {
    pub twitter: Option<String>,
    pub github: Option<String>,
    pub discord: Option<String>,
    pub website: Option<String>,
}

/// Showcase page data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShowcasePage {
    /// Owner
    pub owner: AgentId,
    /// Configuration
    pub config: ShowcaseConfig,
    /// Live stats
    pub stats: LiveStats,
    /// Generated badges
    pub badges: Vec<EmbeddableBadge>,
    /// Public URL slug
    pub slug: String,
    /// When created
    pub created_at: TemporalAnchor,
    /// When updated
    pub updated_at: TemporalAnchor,
}

/// Showcase manager trait
#[async_trait::async_trait]
pub trait ShowcaseManager: Send + Sync {
    /// Create or update a showcase page
    async fn create_showcase(&self, owner: AgentId, config: ShowcaseConfig) -> Result<ShowcasePage>;

    /// Get showcase by owner
    async fn get_showcase(&self, owner: &AgentId) -> Result<ShowcasePage>;

    /// Get showcase by slug
    async fn get_by_slug(&self, slug: &str) -> Result<ShowcasePage>;

    /// Get live stats
    async fn get_live_stats(&self, owner: &AgentId) -> Result<LiveStats>;

    /// Update stats (called by transaction hooks)
    async fn update_stats(&self, owner: &AgentId, update: StatsUpdate) -> Result<()>;

    /// Generate embeddable badge
    async fn generate_badge(&self, owner: &AgentId, badge_type: BadgeType) -> Result<EmbeddableBadge>;

    /// List all public showcases
    async fn list_showcases(&self, limit: usize, offset: usize) -> Result<Vec<ShowcasePage>>;
}

/// Stats update event
#[derive(Debug, Clone)]
pub struct StatsUpdate {
    pub transactions: i64,
    pub volume: Amount,
    pub settlement_time_ms: Option<u64>,
    pub arena_result: Option<ArenaResult>,
}

/// Arena result for stats
#[derive(Debug, Clone)]
pub enum ArenaResult {
    Win,
    Loss,
    Draw,
}

/// In-memory showcase manager
pub struct InMemoryShowcase {
    showcases: Arc<RwLock<HashMap<AgentId, ShowcasePage>>>,
    slug_index: Arc<RwLock<HashMap<String, AgentId>>>,
}

impl InMemoryShowcase {
    pub fn new() -> Self {
        Self {
            showcases: Arc::new(RwLock::new(HashMap::new())),
            slug_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn generate_slug(name: &str, owner: &AgentId) -> String {
        let base = name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");

        if base.is_empty() {
            format!("agent-{}", &owner.0.to_string()[..8])
        } else {
            base
        }
    }
}

impl Default for InMemoryShowcase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ShowcaseManager for InMemoryShowcase {
    async fn create_showcase(&self, owner: AgentId, config: ShowcaseConfig) -> Result<ShowcasePage> {
        let now = TemporalAnchor::now();
        let slug = Self::generate_slug(&config.name, &owner);

        let page = ShowcasePage {
            owner: owner.clone(),
            config,
            stats: LiveStats {
                transactions_per_day: 0,
                total_volume: Amount::iusd_zero(),
                uptime_percent: 100.0,
                avg_settlement_ms: 0,
                active_contracts: 0,
                arena_wins: 0,
                arena_losses: 0,
                last_updated: now,
            },
            badges: Vec::new(),
            slug: slug.clone(),
            created_at: now,
            updated_at: now,
        };

        self.showcases
            .write()
            .await
            .insert(owner.clone(), page.clone());
        self.slug_index.write().await.insert(slug, owner);

        Ok(page)
    }

    async fn get_showcase(&self, owner: &AgentId) -> Result<ShowcasePage> {
        self.showcases
            .read()
            .await
            .get(owner)
            .cloned()
            .ok_or_else(|| OpeniBankError::Internal {
                message: format!("Showcase for agent {} not found", owner.0),
            })
    }

    async fn get_by_slug(&self, slug: &str) -> Result<ShowcasePage> {
        let slug_index = self.slug_index.read().await;
        let owner = slug_index
            .get(slug)
            .ok_or_else(|| OpeniBankError::Internal {
                message: format!("Showcase with slug {} not found", slug),
            })?;

        self.get_showcase(owner).await
    }

    async fn get_live_stats(&self, owner: &AgentId) -> Result<LiveStats> {
        let showcase = self.get_showcase(owner).await?;
        Ok(showcase.stats)
    }

    async fn update_stats(&self, owner: &AgentId, update: StatsUpdate) -> Result<()> {
        let mut showcases = self.showcases.write().await;
        let page = showcases
            .get_mut(owner)
            .ok_or_else(|| OpeniBankError::Internal {
                message: format!("Showcase for agent {} not found", owner.0),
            })?;

        page.stats.transactions_per_day = (page.stats.transactions_per_day as i64 + update.transactions) as u64;

        if let Ok(new_volume) = page.stats.total_volume.checked_add(update.volume) {
            page.stats.total_volume = new_volume;
        }

        if let Some(settlement_ms) = update.settlement_time_ms {
            // Exponential moving average
            let alpha = 0.1;
            page.stats.avg_settlement_ms = ((page.stats.avg_settlement_ms as f64) * (1.0 - alpha)
                + (settlement_ms as f64) * alpha) as u64;
        }

        if let Some(result) = update.arena_result {
            match result {
                ArenaResult::Win => page.stats.arena_wins += 1,
                ArenaResult::Loss => page.stats.arena_losses += 1,
                ArenaResult::Draw => {}
            }
        }

        page.stats.last_updated = TemporalAnchor::now();
        page.updated_at = TemporalAnchor::now();

        Ok(())
    }

    async fn generate_badge(&self, owner: &AgentId, badge_type: BadgeType) -> Result<EmbeddableBadge> {
        let showcase = self.get_showcase(owner).await?;

        let (label, value, color) = match badge_type {
            BadgeType::TransactionVolume => {
                let vol = showcase.stats.total_volume.to_human();
                let formatted = if vol >= 1_000_000.0 {
                    format!("${:.1}M", vol / 1_000_000.0)
                } else if vol >= 1_000.0 {
                    format!("${:.1}K", vol / 1_000.0)
                } else {
                    format!("${:.0}", vol)
                };
                ("Volume", formatted, "#10b981")
            }
            BadgeType::SettlementSpeed => {
                let ms = showcase.stats.avg_settlement_ms;
                let formatted = if ms < 1000 {
                    format!("{}ms", ms)
                } else {
                    format!("{:.1}s", ms as f64 / 1000.0)
                };
                ("Settlement", formatted, "#6366f1")
            }
            BadgeType::Uptime => {
                let formatted = format!("{:.1}%", showcase.stats.uptime_percent);
                ("Uptime", formatted, "#22c55e")
            }
            BadgeType::ArenaRank => {
                let wins = showcase.stats.arena_wins;
                let formatted = format!("{} wins", wins);
                ("Arena", formatted, "#f59e0b")
            }
            BadgeType::ReputationScore => {
                ("Reputation", "★★★★☆".to_string(), "#eab308")
            }
            BadgeType::Custom(ref name) => {
                (name.as_str(), "Custom".to_string(), "#8b5cf6")
            }
        };

        let svg = generate_badge_svg(label, &value, color);
        let live_url = format!("https://openibank.com/api/badges/{}/{:?}", showcase.slug, badge_type);

        Ok(EmbeddableBadge {
            svg: svg.clone(),
            html_embed: format!(
                r#"<a href="https://openibank.com/showcase/{}"><img src="{}" alt="{} badge" /></a>"#,
                showcase.slug, live_url, label
            ),
            markdown_embed: format!(
                r#"[![{} badge]({})](https://openibank.com/showcase/{})"#,
                label, live_url, showcase.slug
            ),
            live_data_url: live_url,
        })
    }

    async fn list_showcases(&self, limit: usize, offset: usize) -> Result<Vec<ShowcasePage>> {
        let showcases = self.showcases.read().await;
        let results: Vec<ShowcasePage> = showcases
            .values()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        Ok(results)
    }
}

/// Generate an SVG badge
fn generate_badge_svg(label: &str, value: &str, color: &str) -> String {
    let label_width = label.len() * 7 + 10;
    let value_width = value.len() * 7 + 10;
    let total_width = label_width + value_width;

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"20\">\
  <linearGradient id=\"smooth\" x2=\"0\" y2=\"100%\">\
    <stop offset=\"0\" stop-color=\"#bbb\" stop-opacity=\".1\"/>\
    <stop offset=\"1\" stop-opacity=\".1\"/>\
  </linearGradient>\
  <mask id=\"round\">\
    <rect width=\"{}\" height=\"20\" rx=\"3\" fill=\"#fff\"/>\
  </mask>\
  <g mask=\"url(#round)\">\
    <rect width=\"{}\" height=\"20\" fill=\"#555\"/>\
    <rect x=\"{}\" width=\"{}\" height=\"20\" fill=\"{}\"/>\
    <rect width=\"{}\" height=\"20\" fill=\"url(#smooth)\"/>\
  </g>\
  <g fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,sans-serif\" font-size=\"11\">\
    <text x=\"{}\" y=\"15\" fill=\"#010101\" fill-opacity=\".3\">{}</text>\
    <text x=\"{}\" y=\"14\">{}</text>\
    <text x=\"{}\" y=\"15\" fill=\"#010101\" fill-opacity=\".3\">{}</text>\
    <text x=\"{}\" y=\"14\">{}</text>\
  </g>\
</svg>",
        total_width,
        total_width,
        label_width,
        label_width,
        value_width,
        color,
        total_width,
        label_width / 2,
        label,
        label_width / 2,
        label,
        label_width + value_width / 2,
        value,
        label_width + value_width / 2,
        value
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ShowcaseConfig {
        ShowcaseConfig {
            name: "Test Agent".to_string(),
            description: "A test agent showcase".to_string(),
            logo_url: None,
            featured_services: vec![],
            theme: None,
            social_links: None,
            custom_css: None,
        }
    }

    #[tokio::test]
    async fn test_create_showcase() {
        let manager = InMemoryShowcase::new();
        let owner = AgentId::new();

        let page = manager
            .create_showcase(owner.clone(), test_config())
            .await
            .unwrap();

        assert_eq!(page.owner, owner);
        assert_eq!(page.slug, "test-agent");
    }

    #[tokio::test]
    async fn test_get_by_slug() {
        let manager = InMemoryShowcase::new();
        let owner = AgentId::new();

        manager
            .create_showcase(owner.clone(), test_config())
            .await
            .unwrap();

        let page = manager.get_by_slug("test-agent").await.unwrap();
        assert_eq!(page.owner, owner);
    }

    #[tokio::test]
    async fn test_update_stats() {
        let manager = InMemoryShowcase::new();
        let owner = AgentId::new();

        manager
            .create_showcase(owner.clone(), test_config())
            .await
            .unwrap();

        manager
            .update_stats(
                &owner,
                StatsUpdate {
                    transactions: 10,
                    volume: Amount::iusd(1000.0),
                    settlement_time_ms: Some(50),
                    arena_result: Some(ArenaResult::Win),
                },
            )
            .await
            .unwrap();

        let stats = manager.get_live_stats(&owner).await.unwrap();
        assert_eq!(stats.transactions_per_day, 10);
        assert_eq!(stats.arena_wins, 1);
    }

    #[tokio::test]
    async fn test_generate_badge() {
        let manager = InMemoryShowcase::new();
        let owner = AgentId::new();

        manager
            .create_showcase(owner.clone(), test_config())
            .await
            .unwrap();

        let badge = manager
            .generate_badge(&owner, BadgeType::Uptime)
            .await
            .unwrap();

        assert!(badge.svg.contains("Uptime"));
        assert!(badge.html_embed.contains("test-agent"));
    }
}
