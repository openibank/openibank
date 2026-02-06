//! "Fork this bank" templates

use serde::{Deserialize, Serialize};

/// Bank template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTemplate {
    /// Template ID
    pub id: String,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: TemplateCategory,
    /// Features included
    pub features: Vec<String>,
    /// Required configuration
    pub required_config: Vec<ConfigField>,
    /// Cargo.toml content
    pub cargo_toml: String,
    /// Docker compose content
    pub docker_compose: String,
    /// Setup script
    pub setup_script: String,
}

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateCategory {
    /// Merchant bank
    MerchantBank,
    /// Agent treasury
    AgentTreasury,
    /// Cross-border
    CrossBorder,
    /// Market maker
    MarketMaker,
    /// Compliance hub
    ComplianceHub,
    /// DAO treasury
    DaoTreasury,
}

/// Configuration field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    /// Field name
    pub name: String,
    /// Description
    pub description: String,
    /// Default value
    pub default: Option<String>,
    /// Required
    pub required: bool,
}

/// Template store trait
#[async_trait::async_trait]
pub trait TemplateStore: Send + Sync {
    /// List all templates
    async fn list_templates(&self) -> openibank_types::Result<Vec<BankTemplate>>;

    /// Get a template
    async fn get_template(&self, id: &str) -> openibank_types::Result<BankTemplate>;

    /// Deploy a template
    async fn deploy_template(
        &self,
        id: &str,
        config: std::collections::HashMap<String, String>,
    ) -> openibank_types::Result<String>;
}
