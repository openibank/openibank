//! Activity logging for system-wide tracking
//!
//! ActivityEntry represents any notable event in the system,
//! from agent creation to LLM reasoning to trade completion.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single activity entry in the system log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    /// Unique ID for this entry
    pub id: String,
    /// When this activity occurred
    pub timestamp: DateTime<Utc>,
    /// Category of activity
    pub category: ActivityCategory,
    /// Severity/importance level
    pub level: ActivityLevel,
    /// Source agent or component
    pub source: String,
    /// Human-readable description
    pub description: String,
    /// Optional structured data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Categories of system activities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityCategory {
    /// System startup/shutdown
    System,
    /// Agent lifecycle (create, destroy)
    AgentLifecycle,
    /// Trading activity
    Trade,
    /// LLM reasoning
    LLM,
    /// Issuer operations (mint/burn)
    Issuer,
    /// Ledger entries
    Ledger,
    /// Escrow operations
    Escrow,
    /// Receipt generation
    Receipt,
    /// Dispute/arbitration
    Dispute,
    /// Maple runtime events
    MapleRuntime,
    /// Wallet/budget operations
    Wallet,
    /// Error
    Error,
}

/// Activity importance level
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActivityLevel {
    /// Debug-level detail
    Debug,
    /// Informational
    Info,
    /// Warning (non-critical issues)
    Warning,
    /// Error (something failed)
    Error,
    /// Critical (system-level failure)
    Critical,
}

impl ActivityEntry {
    /// Create a new info-level activity
    pub fn info(
        source: impl Into<String>,
        category: ActivityCategory,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("act_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            category,
            level: ActivityLevel::Info,
            source: source.into(),
            description: description.into(),
            data: None,
        }
    }

    /// Create an error activity
    pub fn error(
        source: impl Into<String>,
        category: ActivityCategory,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("act_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            category,
            level: ActivityLevel::Error,
            source: source.into(),
            description: description.into(),
            data: None,
        }
    }

    /// Create a warning activity
    pub fn warning(
        source: impl Into<String>,
        category: ActivityCategory,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("act_{}", uuid::Uuid::new_v4()),
            timestamp: Utc::now(),
            category,
            level: ActivityLevel::Warning,
            source: source.into(),
            description: description.into(),
            data: None,
        }
    }

    /// Attach structured data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Create a system startup entry
    pub fn system_started() -> Self {
        Self::info("system", ActivityCategory::System, "OpeniBank system started with Maple iBank runtime")
    }

    /// Create an agent created entry
    pub fn agent_created(name: &str, role: &str) -> Self {
        Self::info(
            "agent_registry",
            ActivityCategory::AgentLifecycle,
            format!("Agent '{}' created as {} with Maple Resonator", name, role),
        )
    }

    /// Create a trade completed entry
    pub fn trade_completed(buyer: &str, seller: &str, service: &str, amount: u64) -> Self {
        Self::info(
            "marketplace",
            ActivityCategory::Trade,
            format!(
                "Trade: {} bought '{}' from {} for ${:.2}",
                buyer, service, seller, amount as f64 / 100.0
            ),
        )
    }

    /// Create an LLM reasoning entry
    pub fn llm_reasoning(agent_name: &str, action: &str, model: Option<&str>) -> Self {
        let model_str = model.unwrap_or("deterministic");
        Self::info(
            agent_name,
            ActivityCategory::LLM,
            format!("LLM ({}) reasoning: {}", model_str, action),
        )
    }

    /// Create an issuer mint entry
    pub fn iusd_minted(account: &str, amount: u64) -> Self {
        Self::info(
            "issuer",
            ActivityCategory::Issuer,
            format!("Minted ${:.2} IUSD to {}", amount as f64 / 100.0, account),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_entry_creation() {
        let entry = ActivityEntry::info("test", ActivityCategory::System, "Test entry");
        assert!(entry.id.starts_with("act_"));
        assert_eq!(entry.level, ActivityLevel::Info);
        assert_eq!(entry.category, ActivityCategory::System);
    }

    #[test]
    fn test_activity_serialization() {
        let entry = ActivityEntry::trade_completed("Alice", "DataCorp", "Data Analysis", 10000);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("DataCorp"));

        let back: ActivityEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.category, ActivityCategory::Trade);
    }

    #[test]
    fn test_with_data() {
        let entry = ActivityEntry::info("test", ActivityCategory::System, "Test")
            .with_data(serde_json::json!({"key": "value"}));
        assert!(entry.data.is_some());
    }
}
