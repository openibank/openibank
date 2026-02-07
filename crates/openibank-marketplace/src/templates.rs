//! "Fork this bank" templates

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// README content
    pub readme: String,
    /// Example .env file
    pub env_example: String,
    /// Starter code files
    pub starter_files: HashMap<String, String>,
    /// Preview image URL
    pub preview_url: Option<String>,
    /// GitHub stars (for sorting)
    pub stars: u32,
    /// Number of deployments
    pub deployments: u32,
}

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Payment gateway
    PaymentGateway,
    /// Neobank
    Neobank,
}

impl TemplateCategory {
    pub fn display_name(&self) -> &str {
        match self {
            Self::MerchantBank => "Merchant Bank",
            Self::AgentTreasury => "Agent Treasury",
            Self::CrossBorder => "Cross-Border Settlement",
            Self::MarketMaker => "Market Maker",
            Self::ComplianceHub => "Compliance Hub",
            Self::DaoTreasury => "DAO Treasury",
            Self::PaymentGateway => "Payment Gateway",
            Self::Neobank => "Neobank",
        }
    }
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
    /// Field type
    pub field_type: ConfigFieldType,
}

/// Field type for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigFieldType {
    String,
    Number,
    Boolean,
    Secret,
    Select(Vec<String>),
}

/// Deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResult {
    /// Deployment ID
    pub deployment_id: String,
    /// Project directory (local)
    pub project_dir: String,
    /// Generated files
    pub files: Vec<String>,
    /// Instructions
    pub instructions: Vec<String>,
    /// Quick start command
    pub quick_start: String,
}

/// Template store trait
#[async_trait::async_trait]
pub trait TemplateStore: Send + Sync {
    /// List all templates
    async fn list_templates(&self) -> openibank_types::Result<Vec<BankTemplate>>;

    /// Get templates by category
    async fn get_by_category(&self, category: TemplateCategory) -> openibank_types::Result<Vec<BankTemplate>>;

    /// Get a template by ID
    async fn get_template(&self, id: &str) -> openibank_types::Result<BankTemplate>;

    /// Deploy a template (generate project files)
    async fn deploy_template(
        &self,
        id: &str,
        config: HashMap<String, String>,
    ) -> openibank_types::Result<DeploymentResult>;

    /// Add a template
    async fn add_template(&self, template: BankTemplate) -> openibank_types::Result<()>;
}

/// In-memory template store with built-in templates
pub struct InMemoryTemplateStore {
    templates: Arc<RwLock<HashMap<String, BankTemplate>>>,
}

impl InMemoryTemplateStore {
    pub fn new() -> Self {
        let store = Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        };

        // We'll populate templates lazily or via init
        store
    }

    /// Initialize with built-in templates
    pub async fn with_builtin_templates() -> Self {
        let store = Self::new();

        // Add built-in templates
        for template in create_builtin_templates() {
            store.templates.write().await.insert(template.id.clone(), template);
        }

        store
    }
}

impl Default for InMemoryTemplateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TemplateStore for InMemoryTemplateStore {
    async fn list_templates(&self) -> openibank_types::Result<Vec<BankTemplate>> {
        let templates = self.templates.read().await;
        let mut list: Vec<BankTemplate> = templates.values().cloned().collect();
        list.sort_by(|a, b| b.stars.cmp(&a.stars));
        Ok(list)
    }

    async fn get_by_category(&self, category: TemplateCategory) -> openibank_types::Result<Vec<BankTemplate>> {
        let templates = self.templates.read().await;
        let filtered: Vec<BankTemplate> = templates
            .values()
            .filter(|t| t.category == category)
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn get_template(&self, id: &str) -> openibank_types::Result<BankTemplate> {
        self.templates
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| openibank_types::OpeniBankError::Internal {
                message: format!("Template {} not found", id),
            })
    }

    async fn deploy_template(
        &self,
        id: &str,
        config: HashMap<String, String>,
    ) -> openibank_types::Result<DeploymentResult> {
        let template = self.get_template(id).await?;

        // Validate required fields
        for field in &template.required_config {
            if field.required && !config.contains_key(&field.name) && field.default.is_none() {
                return Err(openibank_types::OpeniBankError::InvalidInput {
                    field: field.name.clone(),
                    reason: "Required field is missing".to_string(),
                });
            }
        }

        // Generate project name
        let project_name = config
            .get("project_name")
            .cloned()
            .unwrap_or_else(|| format!("openibank-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()));

        // Apply config to template files
        let mut files = Vec::new();

        // Process each template file
        let cargo_toml = apply_config(&template.cargo_toml, &config, &project_name);
        let docker_compose = apply_config(&template.docker_compose, &config, &project_name);
        let setup_script = apply_config(&template.setup_script, &config, &project_name);
        let readme = apply_config(&template.readme, &config, &project_name);
        let env_example = apply_config(&template.env_example, &config, &project_name);

        files.push("Cargo.toml".to_string());
        files.push("docker-compose.yml".to_string());
        files.push("setup.sh".to_string());
        files.push("README.md".to_string());
        files.push(".env.example".to_string());

        for (path, _) in &template.starter_files {
            files.push(path.clone());
        }

        // Update deployment count
        {
            let mut templates = self.templates.write().await;
            if let Some(t) = templates.get_mut(id) {
                t.deployments += 1;
            }
        }

        let deployment_id = uuid::Uuid::new_v4().to_string();

        Ok(DeploymentResult {
            deployment_id,
            project_dir: project_name.clone(),
            files,
            instructions: vec![
                format!("cd {}", project_name),
                "cp .env.example .env".to_string(),
                "# Edit .env with your configuration".to_string(),
                "chmod +x setup.sh && ./setup.sh".to_string(),
                "cargo build --release".to_string(),
                "docker-compose up -d".to_string(),
            ],
            quick_start: format!(
                "git clone https://github.com/openibank/templates/{}.git {} && cd {} && ./setup.sh",
                id, project_name, project_name
            ),
        })
    }

    async fn add_template(&self, template: BankTemplate) -> openibank_types::Result<()> {
        self.templates
            .write()
            .await
            .insert(template.id.clone(), template);
        Ok(())
    }
}

/// Apply configuration to template content
fn apply_config(content: &str, config: &HashMap<String, String>, project_name: &str) -> String {
    let mut result = content.replace("{{PROJECT_NAME}}", project_name);

    for (key, value) in config {
        let placeholder = format!("{{{{{}}}}}", key.to_uppercase());
        result = result.replace(&placeholder, value);
    }

    result
}

/// Create built-in templates
fn create_builtin_templates() -> Vec<BankTemplate> {
    vec![
        BankTemplate {
            id: "merchant-bank".to_string(),
            name: "Merchant Bank".to_string(),
            description: "Full-featured merchant banking platform with payment processing, escrow, and receipts".to_string(),
            category: TemplateCategory::MerchantBank,
            features: vec![
                "Payment Processing".to_string(),
                "Escrow Services".to_string(),
                "Multi-currency Support".to_string(),
                "Real-time Receipts".to_string(),
                "Webhook Notifications".to_string(),
                "Admin Dashboard".to_string(),
            ],
            required_config: vec![
                ConfigField {
                    name: "project_name".to_string(),
                    description: "Your project name".to_string(),
                    default: Some("my-merchant-bank".to_string()),
                    required: true,
                    field_type: ConfigFieldType::String,
                },
                ConfigField {
                    name: "issuer_endpoint".to_string(),
                    description: "IUSD Issuer API endpoint".to_string(),
                    default: Some("https://issuer.openibank.com".to_string()),
                    required: true,
                    field_type: ConfigFieldType::String,
                },
            ],
            cargo_toml: r#"[package]
name = "{{PROJECT_NAME}}"
version = "0.1.0"
edition = "2021"

[dependencies]
openibank-types = "0.1"
openibank-ledger = "0.1"
openibank-escrow = "0.1"
openibank-receipts = "0.1"
openibank-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
"#.to_string(),
            docker_compose: r#"version: '3.8'
services:
  bank:
    build: .
    ports:
      - "8080:8080"
    environment:
      - ISSUER_ENDPOINT={{ISSUER_ENDPOINT}}
      - DATABASE_URL=postgres://postgres:postgres@db:5432/bank
    depends_on:
      - db
  db:
    image: postgres:15
    environment:
      - POSTGRES_DB=bank
      - POSTGRES_PASSWORD=postgres
"#.to_string(),
            setup_script: r#"#!/bin/bash
set -e
echo "Setting up {{PROJECT_NAME}}..."
cargo build
echo "Setup complete! Run 'cargo run' to start."
"#.to_string(),
            readme: r#"# {{PROJECT_NAME}}

A merchant banking platform built on OpeniBank.

## Quick Start

```bash
cp .env.example .env
cargo run
```

## Features

- Payment Processing
- Escrow Services
- Multi-currency Support
- Real-time Receipts

## API

See the [API documentation](./docs/api.md) for details.
"#.to_string(),
            env_example: r#"# OpeniBank Configuration
ISSUER_ENDPOINT={{ISSUER_ENDPOINT}}
DATABASE_URL=postgres://localhost:5432/bank
PORT=8080
"#.to_string(),
            starter_files: HashMap::from([
                ("src/main.rs".to_string(), r#"use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(|| async { "OK" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
"#.to_string()),
            ]),
            preview_url: Some("https://openibank.com/templates/merchant-bank.png".to_string()),
            stars: 150,
            deployments: 42,
        },
        BankTemplate {
            id: "agent-treasury".to_string(),
            name: "AI Agent Treasury".to_string(),
            description: "Treasury management for AI agents with budgets, permits, and spending limits".to_string(),
            category: TemplateCategory::AgentTreasury,
            features: vec![
                "Budget Management".to_string(),
                "Spending Permits".to_string(),
                "Multi-agent Support".to_string(),
                "Audit Trail".to_string(),
                "MAPLE Integration".to_string(),
            ],
            required_config: vec![
                ConfigField {
                    name: "project_name".to_string(),
                    description: "Your project name".to_string(),
                    default: Some("my-agent-treasury".to_string()),
                    required: true,
                    field_type: ConfigFieldType::String,
                },
            ],
            cargo_toml: r#"[package]
name = "{{PROJECT_NAME}}"
version = "0.1.0"
edition = "2021"

[dependencies]
openibank-types = "0.1"
openibank-wallet = "0.1"
openibank-permits = "0.1"
openibank-maple = "0.1"
tokio = { version = "1", features = ["full"] }
"#.to_string(),
            docker_compose: r#"version: '3.8'
services:
  treasury:
    build: .
    ports:
      - "8080:8080"
"#.to_string(),
            setup_script: r#"#!/bin/bash
echo "Setting up {{PROJECT_NAME}}..."
cargo build
"#.to_string(),
            readme: r#"# {{PROJECT_NAME}}

AI Agent Treasury management on OpeniBank.
"#.to_string(),
            env_example: "PORT=8080\n".to_string(),
            starter_files: HashMap::new(),
            preview_url: None,
            stars: 89,
            deployments: 23,
        },
        BankTemplate {
            id: "payment-gateway".to_string(),
            name: "Payment Gateway".to_string(),
            description: "Stripe-like payment gateway with checkout, webhooks, and multi-currency".to_string(),
            category: TemplateCategory::PaymentGateway,
            features: vec![
                "Checkout Sessions".to_string(),
                "Webhook Events".to_string(),
                "Multi-currency".to_string(),
                "Idempotency".to_string(),
                "PCI Compliance Ready".to_string(),
            ],
            required_config: vec![
                ConfigField {
                    name: "project_name".to_string(),
                    description: "Your project name".to_string(),
                    default: Some("my-gateway".to_string()),
                    required: true,
                    field_type: ConfigFieldType::String,
                },
                ConfigField {
                    name: "webhook_secret".to_string(),
                    description: "Secret for webhook signatures".to_string(),
                    default: None,
                    required: true,
                    field_type: ConfigFieldType::Secret,
                },
            ],
            cargo_toml: r#"[package]
name = "{{PROJECT_NAME}}"
version = "0.1.0"
edition = "2021"

[dependencies]
openibank-types = "0.1"
openibank-sdk = "0.1"
openibank-crypto = "0.1"
tokio = { version = "1", features = ["full"] }
axum = "0.7"
"#.to_string(),
            docker_compose: "version: '3.8'\nservices:\n  gateway:\n    build: .\n    ports:\n      - \"8080:8080\"\n".to_string(),
            setup_script: "#!/bin/bash\ncargo build\n".to_string(),
            readme: "# {{PROJECT_NAME}}\n\nPayment Gateway on OpeniBank.\n".to_string(),
            env_example: "WEBHOOK_SECRET={{WEBHOOK_SECRET}}\n".to_string(),
            starter_files: HashMap::new(),
            preview_url: None,
            stars: 120,
            deployments: 35,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_templates() {
        let store = InMemoryTemplateStore::with_builtin_templates().await;
        let templates = store.list_templates().await.unwrap();
        assert!(!templates.is_empty());
    }

    #[tokio::test]
    async fn test_get_template() {
        let store = InMemoryTemplateStore::with_builtin_templates().await;
        let template = store.get_template("merchant-bank").await.unwrap();
        assert_eq!(template.name, "Merchant Bank");
    }

    #[tokio::test]
    async fn test_deploy_template() {
        let store = InMemoryTemplateStore::with_builtin_templates().await;

        let config = HashMap::from([
            ("project_name".to_string(), "my-test-bank".to_string()),
            ("issuer_endpoint".to_string(), "https://test.openibank.com".to_string()),
        ]);

        let result = store.deploy_template("merchant-bank", config).await.unwrap();

        assert_eq!(result.project_dir, "my-test-bank");
        assert!(!result.files.is_empty());
        assert!(!result.instructions.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_category() {
        let store = InMemoryTemplateStore::with_builtin_templates().await;
        let templates = store.get_by_category(TemplateCategory::MerchantBank).await.unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "merchant-bank");
    }
}
