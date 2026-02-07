//! ResonanceX Alerts - Price Alerts and Notifications
//!
//! This crate provides a flexible alert system for monitoring market conditions
//! and triggering notifications when specified conditions are met.
//!
//! # Alert Types
//!
//! - **Price Alerts**: Trigger when price crosses a threshold
//! - **Volume Alerts**: Trigger on unusual volume
//! - **Spread Alerts**: Trigger when spread exceeds threshold
//! - **Custom Alerts**: User-defined conditions
//!
//! # Example
//!
//! ```ignore
//! use resonancex_alerts::{AlertService, Alert, AlertCondition};
//!
//! let service = AlertService::new();
//!
//! // Create a price alert
//! let alert = Alert::new(
//!     MarketId::new("ETH_IUSD"),
//!     AlertCondition::PriceAbove(dec!(3500)),
//! );
//!
//! service.create_alert(agent_id, alert).await?;
//!
//! // Subscribe to triggered alerts
//! let rx = service.subscribe(agent_id);
//! while let Ok(triggered) = rx.recv() {
//!     println!("Alert triggered: {:?}", triggered);
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// Re-export core types
pub use resonancex_types::{MarketId, Ticker};
pub use resonancex_marketdata::MarketDataUpdate;
pub use openibank_types::AgentId;

/// Alert errors
#[derive(Debug, Error)]
pub enum AlertError {
    #[error("Alert not found: {0}")]
    AlertNotFound(AlertId),

    #[error("Invalid condition: {0}")]
    InvalidCondition(String),

    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),
}

/// Result type for alert operations
pub type AlertResult<T> = Result<T, AlertError>;

/// Alert identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlertId(pub Uuid);

impl AlertId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AlertId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AlertId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Alert status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Alert is active and monitoring
    Active,
    /// Alert has been triggered
    Triggered,
    /// Alert was triggered and is on cooldown
    Cooldown,
    /// Alert is paused
    Paused,
    /// Alert has been cancelled
    Cancelled,
}

/// Alert condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Price goes above threshold
    PriceAbove(Decimal),
    /// Price goes below threshold
    PriceBelow(Decimal),
    /// Price crosses threshold (either direction)
    PriceCross(Decimal),
    /// Price changes by percentage in 24h
    PriceChange24h(Decimal),
    /// Volume exceeds threshold in 24h
    Volume24h(Decimal),
    /// Spread exceeds percentage
    SpreadAbove(Decimal),
    /// Custom condition (expression string)
    Custom(String),
}

impl AlertCondition {
    /// Check if condition is met
    pub fn check(&self, ticker: &Ticker, prev_price: Option<Decimal>) -> bool {
        match self {
            AlertCondition::PriceAbove(threshold) => ticker.last_price > *threshold,
            AlertCondition::PriceBelow(threshold) => ticker.last_price < *threshold,
            AlertCondition::PriceCross(threshold) => {
                if let Some(prev) = prev_price {
                    (prev <= *threshold && ticker.last_price > *threshold)
                        || (prev >= *threshold && ticker.last_price < *threshold)
                } else {
                    false
                }
            }
            AlertCondition::PriceChange24h(threshold) => {
                ticker.change_24h.abs() >= threshold.abs()
            }
            AlertCondition::Volume24h(threshold) => ticker.volume_24h >= *threshold,
            AlertCondition::SpreadAbove(threshold) => {
                if ticker.bid.is_zero() {
                    false
                } else {
                    let spread_percent = ((ticker.ask - ticker.bid) / ticker.bid) * dec!(100);
                    spread_percent > *threshold
                }
            }
            AlertCondition::Custom(_) => false, // Custom conditions need external evaluation
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            AlertCondition::PriceAbove(p) => format!("Price above {}", p),
            AlertCondition::PriceBelow(p) => format!("Price below {}", p),
            AlertCondition::PriceCross(p) => format!("Price crosses {}", p),
            AlertCondition::PriceChange24h(p) => format!("24h change exceeds {}%", p),
            AlertCondition::Volume24h(v) => format!("24h volume exceeds {}", v),
            AlertCondition::SpreadAbove(s) => format!("Spread exceeds {}%", s),
            AlertCondition::Custom(expr) => format!("Custom: {}", expr),
        }
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID
    pub id: AlertId,
    /// Market to monitor
    pub market: MarketId,
    /// Alert condition
    pub condition: AlertCondition,
    /// Current status
    pub status: AlertStatus,
    /// Name/label for the alert
    pub name: Option<String>,
    /// Whether to repeat after triggering
    pub repeat: bool,
    /// Cooldown period in seconds (for repeating alerts)
    pub cooldown_secs: Option<u64>,
    /// Expiration time (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last triggered timestamp
    pub triggered_at: Option<DateTime<Utc>>,
}

impl Alert {
    /// Create a new alert
    pub fn new(market: MarketId, condition: AlertCondition) -> Self {
        Self {
            id: AlertId::new(),
            market,
            condition,
            status: AlertStatus::Active,
            name: None,
            repeat: false,
            cooldown_secs: None,
            expires_at: None,
            created_at: Utc::now(),
            triggered_at: None,
        }
    }

    /// Set alert name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Make alert repeating
    pub fn repeating(mut self, cooldown_secs: u64) -> Self {
        self.repeat = true;
        self.cooldown_secs = Some(cooldown_secs);
        self
    }

    /// Set expiration
    pub fn expires_at(mut self, time: DateTime<Utc>) -> Self {
        self.expires_at = Some(time);
        self
    }

    /// Check if alert is active
    pub fn is_active(&self) -> bool {
        if self.status != AlertStatus::Active && self.status != AlertStatus::Cooldown {
            return false;
        }

        // Check expiration
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }

        // Check cooldown
        if self.status == AlertStatus::Cooldown {
            if let (Some(triggered), Some(cooldown)) = (self.triggered_at, self.cooldown_secs) {
                let cooldown_end = triggered + chrono::Duration::seconds(cooldown as i64);
                if Utc::now() < cooldown_end {
                    return false;
                }
            }
        }

        true
    }

    /// Mark as triggered
    pub fn trigger(&mut self) {
        self.triggered_at = Some(Utc::now());
        if self.repeat {
            self.status = AlertStatus::Cooldown;
        } else {
            self.status = AlertStatus::Triggered;
        }
    }
}

/// Triggered alert notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredAlert {
    /// Alert that was triggered
    pub alert: Alert,
    /// Ticker at time of trigger
    pub ticker: Ticker,
    /// Trigger timestamp
    pub timestamp: DateTime<Utc>,
}

/// Notification channel trait
#[async_trait::async_trait]
pub trait NotificationChannel: Send + Sync {
    /// Send a notification
    async fn send(&self, alert: &TriggeredAlert) -> AlertResult<()>;

    /// Get channel name
    fn name(&self) -> &str;
}

/// In-memory notification channel for testing
pub struct InMemoryChannel {
    name: String,
    notifications: RwLock<Vec<TriggeredAlert>>,
}

impl InMemoryChannel {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            notifications: RwLock::new(Vec::new()),
        }
    }

    pub fn get_notifications(&self) -> Vec<TriggeredAlert> {
        self.notifications.read().clone()
    }
}

#[async_trait::async_trait]
impl NotificationChannel for InMemoryChannel {
    async fn send(&self, alert: &TriggeredAlert) -> AlertResult<()> {
        self.notifications.write().push(alert.clone());
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Alert service for managing alerts
pub struct AlertService {
    /// Alerts by agent
    alerts: RwLock<HashMap<AgentId, HashMap<AlertId, Alert>>>,
    /// Previous ticker prices for cross detection
    prev_prices: RwLock<HashMap<MarketId, Decimal>>,
    /// Maximum alerts per agent
    max_alerts_per_agent: usize,
}

impl AlertService {
    /// Create a new alert service
    pub fn new() -> Self {
        Self {
            alerts: RwLock::new(HashMap::new()),
            prev_prices: RwLock::new(HashMap::new()),
            max_alerts_per_agent: 100,
        }
    }

    /// Create a new alert
    pub fn create_alert(&self, agent_id: AgentId, alert: Alert) -> AlertResult<AlertId> {
        let mut alerts = self.alerts.write();
        let agent_alerts = alerts.entry(agent_id).or_insert_with(HashMap::new);

        if agent_alerts.len() >= self.max_alerts_per_agent {
            return Err(AlertError::LimitExceeded(format!(
                "Maximum {} alerts per agent",
                self.max_alerts_per_agent
            )));
        }

        let id = alert.id;
        agent_alerts.insert(id, alert);
        Ok(id)
    }

    /// Get an alert
    pub fn get_alert(&self, agent_id: &AgentId, alert_id: AlertId) -> AlertResult<Alert> {
        self.alerts
            .read()
            .get(agent_id)
            .and_then(|alerts| alerts.get(&alert_id))
            .cloned()
            .ok_or(AlertError::AlertNotFound(alert_id))
    }

    /// List alerts for an agent
    pub fn list_alerts(&self, agent_id: &AgentId) -> Vec<Alert> {
        self.alerts
            .read()
            .get(agent_id)
            .map(|alerts| alerts.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Cancel an alert
    pub fn cancel_alert(&self, agent_id: &AgentId, alert_id: AlertId) -> AlertResult<()> {
        let mut alerts = self.alerts.write();
        if let Some(agent_alerts) = alerts.get_mut(agent_id) {
            if let Some(alert) = agent_alerts.get_mut(&alert_id) {
                alert.status = AlertStatus::Cancelled;
                return Ok(());
            }
        }
        Err(AlertError::AlertNotFound(alert_id))
    }

    /// Delete an alert
    pub fn delete_alert(&self, agent_id: &AgentId, alert_id: AlertId) -> AlertResult<()> {
        let mut alerts = self.alerts.write();
        if let Some(agent_alerts) = alerts.get_mut(agent_id) {
            if agent_alerts.remove(&alert_id).is_some() {
                return Ok(());
            }
        }
        Err(AlertError::AlertNotFound(alert_id))
    }

    /// Process a ticker update and check all alerts
    pub fn process_ticker(&self, ticker: &Ticker) -> Vec<(AgentId, TriggeredAlert)> {
        let mut triggered = Vec::new();
        let prev_price = self.prev_prices.read().get(&ticker.market).copied();

        let mut alerts = self.alerts.write();

        for (agent_id, agent_alerts) in alerts.iter_mut() {
            for alert in agent_alerts.values_mut() {
                if alert.market != ticker.market {
                    continue;
                }

                if !alert.is_active() {
                    continue;
                }

                if alert.condition.check(ticker, prev_price) {
                    alert.trigger();
                    triggered.push((
                        agent_id.clone(),
                        TriggeredAlert {
                            alert: alert.clone(),
                            ticker: ticker.clone(),
                            timestamp: Utc::now(),
                        },
                    ));
                }
            }
        }

        // Update previous price
        self.prev_prices.write().insert(ticker.market.clone(), ticker.last_price);

        triggered
    }
}

impl Default for AlertService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ticker(price: Decimal) -> Ticker {
        let mut ticker = Ticker::new(MarketId::new("ETH_IUSD"));
        ticker.last_price = price;
        ticker.bid = price - dec!(1);
        ticker.ask = price + dec!(1);
        ticker.volume_24h = dec!(1000);
        ticker.change_24h = dec!(5);
        ticker
    }

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            MarketId::new("ETH_IUSD"),
            AlertCondition::PriceAbove(dec!(3000)),
        )
        .with_name("ETH above 3000");

        assert_eq!(alert.name, Some("ETH above 3000".to_string()));
        assert!(alert.is_active());
    }

    #[test]
    fn test_price_above_condition() {
        let condition = AlertCondition::PriceAbove(dec!(3000));

        let ticker_below = test_ticker(dec!(2900));
        let ticker_above = test_ticker(dec!(3100));

        assert!(!condition.check(&ticker_below, None));
        assert!(condition.check(&ticker_above, None));
    }

    #[test]
    fn test_price_cross_condition() {
        let condition = AlertCondition::PriceCross(dec!(3000));

        let ticker = test_ticker(dec!(3100));

        // No previous price - should not trigger
        assert!(!condition.check(&ticker, None));

        // Crossing from below
        assert!(condition.check(&ticker, Some(dec!(2900))));

        // Not crossing (both above)
        assert!(!condition.check(&ticker, Some(dec!(3050))));
    }

    #[test]
    fn test_alert_service() {
        let service = AlertService::new();
        let agent_id = AgentId::new();

        let alert = Alert::new(
            MarketId::new("ETH_IUSD"),
            AlertCondition::PriceAbove(dec!(3000)),
        );

        let alert_id = service.create_alert(agent_id.clone(), alert).unwrap();

        let retrieved = service.get_alert(&agent_id, alert_id).unwrap();
        assert_eq!(retrieved.id, alert_id);

        let alerts = service.list_alerts(&agent_id);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_alert_trigger() {
        let service = AlertService::new();
        let agent_id = AgentId::new();

        let alert = Alert::new(
            MarketId::new("ETH_IUSD"),
            AlertCondition::PriceAbove(dec!(3000)),
        );

        service.create_alert(agent_id.clone(), alert).unwrap();

        // Price below threshold - no trigger
        let ticker1 = test_ticker(dec!(2900));
        let triggered1 = service.process_ticker(&ticker1);
        assert!(triggered1.is_empty());

        // Price above threshold - should trigger
        let ticker2 = test_ticker(dec!(3100));
        let triggered2 = service.process_ticker(&ticker2);
        assert_eq!(triggered2.len(), 1);
        assert_eq!(triggered2[0].0, agent_id);
    }

    #[test]
    fn test_repeating_alert() {
        let mut alert = Alert::new(
            MarketId::new("ETH_IUSD"),
            AlertCondition::PriceAbove(dec!(3000)),
        )
        .repeating(60);

        assert!(alert.repeat);
        assert_eq!(alert.cooldown_secs, Some(60));

        // Trigger the alert
        alert.trigger();
        assert_eq!(alert.status, AlertStatus::Cooldown);
    }

    #[test]
    fn test_condition_descriptions() {
        let cond1 = AlertCondition::PriceAbove(dec!(3000));
        assert_eq!(cond1.description(), "Price above 3000");

        let cond2 = AlertCondition::Volume24h(dec!(1000000));
        assert_eq!(cond2.description(), "24h volume exceeds 1000000");
    }

    #[test]
    fn test_cancel_and_delete() {
        let service = AlertService::new();
        let agent_id = AgentId::new();

        let alert = Alert::new(
            MarketId::new("ETH_IUSD"),
            AlertCondition::PriceAbove(dec!(3000)),
        );
        let alert_id = service.create_alert(agent_id.clone(), alert).unwrap();

        // Cancel
        service.cancel_alert(&agent_id, alert_id).unwrap();
        let cancelled = service.get_alert(&agent_id, alert_id).unwrap();
        assert_eq!(cancelled.status, AlertStatus::Cancelled);

        // Delete
        service.delete_alert(&agent_id, alert_id).unwrap();
        assert!(service.get_alert(&agent_id, alert_id).is_err());
    }
}
