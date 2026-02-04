//! Playground HTTP client for CLI → Playground communication
//!
//! When the Playground is running on :8080, the CLI routes all commands
//! through its API so that state is unified and visible in the web dashboard.

use anyhow::{Context, Result};
use colored::*;
use serde::Deserialize;

/// HTTP client that connects to the Playground API
pub struct PlaygroundClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub balance: u64,
    pub trade_count: u32,
    pub has_resonator: bool,
    pub services: Option<Vec<ServiceInfo>>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub price: u64,
}

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub name: String,
    pub version: String,
    pub llm_available: bool,
    pub llm_provider: Option<String>,
    pub agents: AgentsStatus,
    pub trading: TradingStatus,
    pub issuer: IssuerStatus,
    pub uptime_seconds: u64,
    pub maple_runtime: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AgentsStatus {
    pub total: usize,
    pub buyers: usize,
    pub sellers: usize,
    pub arbiters: usize,
}

#[derive(Debug, Deserialize)]
pub struct TradingStatus {
    pub trade_count: u32,
    pub total_volume: u64,
    pub total_volume_display: String,
}

#[derive(Debug, Deserialize)]
pub struct IssuerStatus {
    pub total_supply: u64,
    pub remaining_supply: u64,
    pub total_supply_display: String,
}

impl PlaygroundClient {
    /// Create a new client pointing to a Playground URL
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Check if the playground is reachable
    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/status", self.base_url))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Get full system status
    pub async fn get_status(&self) -> Result<StatusResponse> {
        let resp = self.client
            .get(format!("{}/api/status", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse status response")
    }

    /// Create a buyer agent
    pub async fn create_buyer(&self, name: &str, funding: u64) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/agents/buyer", self.base_url))
            .json(&serde_json::json!({ "name": name, "funding": funding }))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await.unwrap_or_default();
            anyhow::bail!("{}", err.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error"));
        }

        resp.json().await.context("Failed to parse response")
    }

    /// Create a seller agent
    pub async fn create_seller(&self, name: &str, service_name: &str, price: u64) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/agents/seller", self.base_url))
            .json(&serde_json::json!({
                "name": name,
                "service_name": service_name,
                "price": price
            }))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await.unwrap_or_default();
            anyhow::bail!("{}", err.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error"));
        }

        resp.json().await.context("Failed to parse response")
    }

    /// List all agents
    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>> {
        let resp = self.client
            .get(format!("{}/api/agents", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        let data: serde_json::Value = resp.json().await.context("Failed to parse response")?;
        let agents = data.get("agents").and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect())
            .unwrap_or_default();

        Ok(agents)
    }

    /// Get a single agent
    pub async fn get_agent(&self, id: &str) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/api/agents/{}", self.base_url, id))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await.unwrap_or_default();
            anyhow::bail!("{}", err.get("message").and_then(|m| m.as_str()).unwrap_or("Agent not found"));
        }

        resp.json().await.context("Failed to parse response")
    }

    /// Execute a trade
    pub async fn execute_trade(&self, buyer_id: &str, seller_id: &str) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/trade", self.base_url))
            .json(&serde_json::json!({
                "buyer_id": buyer_id,
                "seller_id": seller_id
            }))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        if !resp.status().is_success() {
            let err: serde_json::Value = resp.json().await.unwrap_or_default();
            anyhow::bail!("{}", err.get("message").and_then(|m| m.as_str()).unwrap_or("Trade failed"));
        }

        resp.json().await.context("Failed to parse response")
    }

    /// Start auto trading
    pub async fn auto_trade(&self, rounds: u32, delay_ms: u64) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/trade/auto", self.base_url))
            .json(&serde_json::json!({
                "rounds": rounds,
                "delay_ms": delay_ms
            }))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Run marketplace simulation
    pub async fn simulate(&self, buyers: u32, sellers: u32, rounds: u32, delay_ms: u64) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/simulate", self.base_url))
            .json(&serde_json::json!({
                "buyers": buyers,
                "sellers": sellers,
                "rounds": rounds,
                "delay_ms": delay_ms
            }))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Get IUSD supply info
    pub async fn get_supply(&self) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/api/issuer/supply", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Get issuer receipts
    pub async fn get_issuer_receipts(&self) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/api/issuer/receipts", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Get ledger accounts
    pub async fn get_ledger_accounts(&self) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/api/ledger/accounts", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Get transactions
    pub async fn get_transactions(&self) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/api/transactions", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Get resonator states
    pub async fn get_resonators(&self) -> Result<serde_json::Value> {
        let resp = self.client
            .get(format!("{}/api/resonators", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }

    /// Reset playground
    pub async fn reset(&self) -> Result<serde_json::Value> {
        let resp = self.client
            .post(format!("{}/api/reset", self.base_url))
            .send()
            .await
            .context("Failed to connect to Playground")?;

        resp.json().await.context("Failed to parse response")
    }
}

/// Display full status from the Playground
pub async fn display_playground_status(client: &PlaygroundClient) -> Result<()> {
    let status = client.get_status().await?;

    println!("{}", "System Status (Connected to Playground)".bright_white().bold());
    println!("{}", "─".repeat(60));

    println!("  {} {}", "Playground:".bright_white(), format!("v{}", status.version).bright_green());
    println!("  {} {}", "Uptime:".bright_white(), format!("{}s", status.uptime_seconds));

    // Maple runtime
    if let Some(maple) = &status.maple_runtime {
        let name = maple.get("instance_name").and_then(|v| v.as_str()).unwrap_or("iBank");
        let resonators = maple.get("resonator_count").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("  {} {} ({} resonators)", "Maple Runtime:".bright_white(), name.bright_green(), resonators);
    }

    // LLM
    if status.llm_available {
        let provider = status.llm_provider.as_deref().unwrap_or("unknown");
        println!("  {} {} {}", "LLM:".bright_white(), "●".bright_green(), provider);
    } else {
        println!("  {} {} {}", "LLM:".bright_white(), "○".yellow(), "Deterministic mode");
    }

    // Agents
    println!();
    println!("{}", "Agents:".bright_white().bold());
    println!("  Total: {} ({} buyers, {} sellers, {} arbiters)",
        format!("{}", status.agents.total).bright_cyan(),
        status.agents.buyers,
        status.agents.sellers,
        status.agents.arbiters,
    );

    // Trading
    println!();
    println!("{}", "Trading:".bright_white().bold());
    println!("  Trades: {}  Volume: {}",
        format!("{}", status.trading.trade_count).bright_cyan(),
        status.trading.total_volume_display.bright_green(),
    );

    // Issuer
    println!();
    println!("{}", "IUSD Supply:".bright_white().bold());
    println!("  Total: {}  Remaining: ${}",
        status.issuer.total_supply_display.bright_cyan(),
        format!("{:.2}", status.issuer.remaining_supply as f64 / 100.0).bright_green(),
    );

    // List agents
    let agents = client.list_agents().await?;
    if !agents.is_empty() {
        println!();
        println!("{}", "Registered Agents:".bright_white().bold());
        for a in &agents {
            let resonator = if a.has_resonator { "⟡".bright_purple().to_string() } else { " ".to_string() };
            let balance_str = format!("${:.2}", a.balance as f64 / 100.0);
            println!("  {} {:12} {:8} {:>10}  {} trades",
                resonator,
                a.name.bright_white(),
                a.role.bright_cyan(),
                balance_str.bright_green(),
                a.trade_count,
            );
        }
    }

    Ok(())
}

/// Display agents list from playground
pub async fn display_agents(client: &PlaygroundClient) -> Result<()> {
    let agents = client.list_agents().await?;

    if agents.is_empty() {
        println!("{}", "No agents registered".yellow());
        println!("  Create agents: {} or {}",
            "openibank agent buyer -n Alice".bright_cyan(),
            "openibank agent seller -n DataCorp --service \"Data Analysis\" --price 10000".bright_cyan(),
        );
        return Ok(());
    }

    let buyers: Vec<&AgentInfo> = agents.iter().filter(|a| a.role == "Buyer").collect();
    let sellers: Vec<&AgentInfo> = agents.iter().filter(|a| a.role == "Seller").collect();

    if !buyers.is_empty() {
        println!("{}", "Buyers:".bright_white().bold());
        for a in &buyers {
            let balance_str = format!("${:.2}", a.balance as f64 / 100.0);
            let res = if a.has_resonator { "⟡ Resonator" } else { "" };
            println!("  {:12} {:>10}  {} trades  {}",
                a.name.bright_white(),
                balance_str.bright_green(),
                a.trade_count,
                res.bright_purple(),
            );
        }
    }

    if !sellers.is_empty() {
        println!("{}", "Sellers:".bright_white().bold());
        for a in &sellers {
            let balance_str = format!("${:.2}", a.balance as f64 / 100.0);
            let svc = a.services.as_ref()
                .and_then(|s| s.first())
                .map(|s| format!("{} @ ${:.2}", s.name, s.price as f64 / 100.0))
                .unwrap_or_default();
            let res = if a.has_resonator { "⟡ Resonator" } else { "" };
            println!("  {:12} {:>10}  {}  {}",
                a.name.bright_white(),
                balance_str.bright_green(),
                svc.bright_cyan(),
                res.bright_purple(),
            );
        }
    }

    Ok(())
}
