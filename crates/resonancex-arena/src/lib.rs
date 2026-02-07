//! ResonanceX Arena - Trading Competition Platform
//!
//! This crate provides a competitive trading arena where AI agents can
//! compete in trading competitions with real or simulated markets.
//!
//! # Features
//!
//! - **Competitions**: Time-bounded trading competitions with rules
//! - **Leaderboards**: Real-time ranking based on PnL, Sharpe ratio, etc.
//! - **Sandboxed Trading**: Isolated environments for fair competition
//! - **Agent Registration**: Register and manage competing agents
//!
//! # Competition Types
//!
//! - **PnL Competition**: Maximize profit and loss
//! - **Sharpe Ratio**: Risk-adjusted returns
//! - **Market Making**: Best bid-ask spread maintenance
//! - **Arbitrage**: Cross-market price discovery
//!
//! # Example
//!
//! ```ignore
//! use resonancex_arena::{Arena, Competition, CompetitionConfig};
//!
//! let arena = Arena::new();
//!
//! // Create a new competition
//! let competition = arena.create_competition(CompetitionConfig {
//!     name: "Q1 Trading Challenge".to_string(),
//!     start_time: Utc::now() + Duration::hours(1),
//!     duration: Duration::days(7),
//!     markets: vec![MarketId::new("ETH_IUSD")],
//!     initial_balance: dec!(10000),
//!     ..Default::default()
//! }).await?;
//!
//! // Register an agent
//! arena.register_agent(competition.id, agent_id).await?;
//!
//! // Get leaderboard
//! let leaderboard = arena.get_leaderboard(competition.id).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// Re-export core types
pub use resonancex_types::{MarketId, Trade};
pub use openibank_types::AgentId;

/// Arena errors
#[derive(Debug, Error)]
pub enum ArenaError {
    #[error("Competition not found: {0}")]
    CompetitionNotFound(CompetitionId),

    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Agent already registered")]
    AlreadyRegistered,

    #[error("Registration closed")]
    RegistrationClosed,

    #[error("Competition not started")]
    NotStarted,

    #[error("Competition ended")]
    CompetitionEnded,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Result type for arena operations
pub type ArenaResult<T> = Result<T, ArenaError>;

/// Competition identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompetitionId(pub Uuid);

impl CompetitionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CompetitionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CompetitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Competition status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompetitionStatus {
    /// Competition is scheduled but not started
    Scheduled,
    /// Registration is open
    Registration,
    /// Competition is active
    Active,
    /// Competition has ended, results being calculated
    Calculating,
    /// Competition is complete
    Completed,
    /// Competition was cancelled
    Cancelled,
}

/// Competition type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompetitionType {
    /// Maximize absolute PnL
    PnL,
    /// Maximize risk-adjusted returns (Sharpe ratio)
    SharpeRatio,
    /// Market making competition
    MarketMaking,
    /// Custom scoring function
    Custom(String),
}

impl Default for CompetitionType {
    fn default() -> Self {
        Self::PnL
    }
}

/// Competition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionConfig {
    /// Competition name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Competition type
    pub competition_type: CompetitionType,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// Duration
    pub duration_hours: u64,
    /// Markets available for trading
    pub markets: Vec<MarketId>,
    /// Initial balance per agent
    pub initial_balance: Decimal,
    /// Maximum agents
    pub max_agents: Option<usize>,
    /// Entry fee (optional)
    pub entry_fee: Option<Decimal>,
    /// Prize pool
    pub prize_pool: Option<Decimal>,
}

impl Default for CompetitionConfig {
    fn default() -> Self {
        Self {
            name: "Trading Competition".to_string(),
            description: None,
            competition_type: CompetitionType::PnL,
            start_time: Utc::now(),
            duration_hours: 24,
            markets: vec![MarketId::new("ETH_IUSD")],
            initial_balance: dec!(10000),
            max_agents: Some(100),
            entry_fee: None,
            prize_pool: None,
        }
    }
}

/// Competition data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Competition {
    /// Competition ID
    pub id: CompetitionId,
    /// Configuration
    pub config: CompetitionConfig,
    /// Current status
    pub status: CompetitionStatus,
    /// Registered agent count
    pub agent_count: usize,
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl Competition {
    /// Create a new competition
    pub fn new(config: CompetitionConfig) -> Self {
        Self {
            id: CompetitionId::new(),
            config,
            status: CompetitionStatus::Scheduled,
            agent_count: 0,
            created_at: Utc::now(),
        }
    }

    /// Check if registration is open
    pub fn is_registration_open(&self) -> bool {
        matches!(self.status, CompetitionStatus::Scheduled | CompetitionStatus::Registration)
    }

    /// Check if competition is active
    pub fn is_active(&self) -> bool {
        self.status == CompetitionStatus::Active
    }

    /// Get end time
    pub fn end_time(&self) -> DateTime<Utc> {
        self.config.start_time + chrono::Duration::hours(self.config.duration_hours as i64)
    }
}

/// Agent statistics in a competition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    /// Agent ID
    pub agent_id: AgentId,
    /// Current balance
    pub balance: Decimal,
    /// Total PnL
    pub pnl: Decimal,
    /// PnL percentage
    pub pnl_percent: Decimal,
    /// Number of trades
    pub trade_count: u64,
    /// Win rate (0-1)
    pub win_rate: Decimal,
    /// Sharpe ratio
    pub sharpe_ratio: Option<Decimal>,
    /// Maximum drawdown
    pub max_drawdown: Decimal,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl AgentStats {
    /// Create initial stats for an agent
    pub fn new(agent_id: AgentId, initial_balance: Decimal) -> Self {
        Self {
            agent_id,
            balance: initial_balance,
            pnl: Decimal::ZERO,
            pnl_percent: Decimal::ZERO,
            trade_count: 0,
            win_rate: Decimal::ZERO,
            sharpe_ratio: None,
            max_drawdown: Decimal::ZERO,
            updated_at: Utc::now(),
        }
    }

    /// Update stats with a trade
    pub fn record_trade(&mut self, pnl: Decimal) {
        self.pnl += pnl;
        self.trade_count += 1;
        self.balance += pnl;

        // Recalculate PnL percentage
        let initial = self.balance - self.pnl;
        if !initial.is_zero() {
            self.pnl_percent = (self.pnl / initial) * dec!(100);
        }

        self.updated_at = Utc::now();
    }
}

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Rank (1-indexed)
    pub rank: usize,
    /// Agent stats
    pub stats: AgentStats,
    /// Score (depends on competition type)
    pub score: Decimal,
}

/// Arena state
pub struct Arena {
    /// Active competitions
    competitions: RwLock<HashMap<CompetitionId, Competition>>,
    /// Agent registrations: competition_id -> set of agent_ids
    registrations: RwLock<HashMap<CompetitionId, HashMap<AgentId, AgentStats>>>,
}

impl Arena {
    /// Create a new arena
    pub fn new() -> Self {
        Self {
            competitions: RwLock::new(HashMap::new()),
            registrations: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new competition
    pub fn create_competition(&self, config: CompetitionConfig) -> ArenaResult<Competition> {
        let competition = Competition::new(config);
        let id = competition.id;

        self.competitions.write().insert(id, competition.clone());
        self.registrations.write().insert(id, HashMap::new());

        Ok(competition)
    }

    /// Get a competition by ID
    pub fn get_competition(&self, id: CompetitionId) -> ArenaResult<Competition> {
        self.competitions
            .read()
            .get(&id)
            .cloned()
            .ok_or(ArenaError::CompetitionNotFound(id))
    }

    /// List all competitions
    pub fn list_competitions(&self) -> Vec<Competition> {
        self.competitions.read().values().cloned().collect()
    }

    /// Register an agent for a competition
    pub fn register_agent(&self, competition_id: CompetitionId, agent_id: AgentId) -> ArenaResult<()> {
        let competition = self.get_competition(competition_id)?;

        if !competition.is_registration_open() {
            return Err(ArenaError::RegistrationClosed);
        }

        let mut registrations = self.registrations.write();
        let agents = registrations
            .get_mut(&competition_id)
            .ok_or(ArenaError::CompetitionNotFound(competition_id))?;

        if agents.contains_key(&agent_id) {
            return Err(ArenaError::AlreadyRegistered);
        }

        if let Some(max) = competition.config.max_agents {
            if agents.len() >= max {
                return Err(ArenaError::RegistrationClosed);
            }
        }

        let stats = AgentStats::new(agent_id.clone(), competition.config.initial_balance);
        agents.insert(agent_id, stats);

        // Update agent count
        drop(registrations);
        if let Some(comp) = self.competitions.write().get_mut(&competition_id) {
            comp.agent_count += 1;
        }

        Ok(())
    }

    /// Get agent stats in a competition
    pub fn get_agent_stats(&self, competition_id: CompetitionId, agent_id: &AgentId) -> ArenaResult<AgentStats> {
        self.registrations
            .read()
            .get(&competition_id)
            .and_then(|agents| agents.get(agent_id))
            .cloned()
            .ok_or(ArenaError::AgentNotFound(agent_id.clone()))
    }

    /// Get leaderboard for a competition
    pub fn get_leaderboard(&self, competition_id: CompetitionId) -> ArenaResult<Vec<LeaderboardEntry>> {
        let competition = self.get_competition(competition_id)?;

        let registrations = self.registrations.read();
        let agents = registrations
            .get(&competition_id)
            .ok_or(ArenaError::CompetitionNotFound(competition_id))?;

        let mut entries: Vec<_> = agents
            .values()
            .map(|stats| {
                let score = match &competition.config.competition_type {
                    CompetitionType::PnL => stats.pnl,
                    CompetitionType::SharpeRatio => stats.sharpe_ratio.unwrap_or_default(),
                    _ => stats.pnl,
                };
                (stats.clone(), score)
            })
            .collect();

        // Sort by score descending
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(entries
            .into_iter()
            .enumerate()
            .map(|(i, (stats, score))| LeaderboardEntry {
                rank: i + 1,
                stats,
                score,
            })
            .collect())
    }

    /// Record a trade for an agent
    pub fn record_trade(&self, competition_id: CompetitionId, agent_id: &AgentId, pnl: Decimal) -> ArenaResult<()> {
        let mut registrations = self.registrations.write();
        let agents = registrations
            .get_mut(&competition_id)
            .ok_or(ArenaError::CompetitionNotFound(competition_id))?;

        let stats = agents
            .get_mut(agent_id)
            .ok_or(ArenaError::AgentNotFound(agent_id.clone()))?;

        stats.record_trade(pnl);
        Ok(())
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_creation() {
        let arena = Arena::new();
        assert!(arena.list_competitions().is_empty());
    }

    #[test]
    fn test_create_competition() {
        let arena = Arena::new();

        let config = CompetitionConfig {
            name: "Test Competition".to_string(),
            ..Default::default()
        };

        let competition = arena.create_competition(config).unwrap();
        assert_eq!(competition.config.name, "Test Competition");
        assert_eq!(arena.list_competitions().len(), 1);
    }

    #[test]
    fn test_register_agent() {
        let arena = Arena::new();

        let competition = arena.create_competition(CompetitionConfig::default()).unwrap();
        let agent_id = AgentId::new();

        arena.register_agent(competition.id, agent_id.clone()).unwrap();

        let stats = arena.get_agent_stats(competition.id, &agent_id).unwrap();
        assert_eq!(stats.balance, dec!(10000));
        assert_eq!(stats.pnl, Decimal::ZERO);
    }

    #[test]
    fn test_duplicate_registration() {
        let arena = Arena::new();

        let competition = arena.create_competition(CompetitionConfig::default()).unwrap();
        let agent_id = AgentId::new();

        arena.register_agent(competition.id, agent_id.clone()).unwrap();
        let result = arena.register_agent(competition.id, agent_id);

        assert!(matches!(result, Err(ArenaError::AlreadyRegistered)));
    }

    #[test]
    fn test_leaderboard() {
        let arena = Arena::new();

        let competition = arena.create_competition(CompetitionConfig::default()).unwrap();

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        arena.register_agent(competition.id, agent1.clone()).unwrap();
        arena.register_agent(competition.id, agent2.clone()).unwrap();

        // Record trades
        arena.record_trade(competition.id, &agent1, dec!(500)).unwrap();
        arena.record_trade(competition.id, &agent2, dec!(1000)).unwrap();

        let leaderboard = arena.get_leaderboard(competition.id).unwrap();

        assert_eq!(leaderboard.len(), 2);
        assert_eq!(leaderboard[0].rank, 1);
        assert_eq!(leaderboard[0].stats.pnl, dec!(1000)); // Agent 2 is first
        assert_eq!(leaderboard[1].rank, 2);
        assert_eq!(leaderboard[1].stats.pnl, dec!(500)); // Agent 1 is second
    }

    #[test]
    fn test_agent_stats_update() {
        let mut stats = AgentStats::new(AgentId::new(), dec!(10000));

        stats.record_trade(dec!(100));
        assert_eq!(stats.pnl, dec!(100));
        assert_eq!(stats.balance, dec!(10100));
        assert_eq!(stats.trade_count, 1);

        stats.record_trade(dec!(-50));
        assert_eq!(stats.pnl, dec!(50));
        assert_eq!(stats.balance, dec!(10050));
        assert_eq!(stats.trade_count, 2);
    }
}
