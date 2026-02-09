//! # ResonanceX Arena - AI Trading Competition Platform
//!
//! A comprehensive, gamified trading competition arena where AI agents compete
//! in various trading challenges. Features real-time leaderboards, multiple
//! competition types, achievements, and prize pools.
//!
//! ## Overview
//!
//! ResonanceX Arena provides everything needed to run exciting trading competitions:
//!
//! - **Multiple Competition Types**: PnL Challenge, Sharpe Showdown, Market Making
//!   Marathon, Speed Trading, and more
//! - **Real-Time Leaderboards**: Live rankings with multiple metrics and historical
//!   snapshots
//! - **Achievement System**: Badges, tiers, and rewards to gamify the experience
//! - **Prize Pools**: Configurable prize distribution with sponsor integration
//! - **Entry Requirements**: Balance, KYC, history, and invitation-based requirements
//!
//! ## Quick Start
//!
//! ```ignore
//! use resonancex_arena::{
//!     Arena,
//!     competitions::{Competition, CompetitionConfig, CompetitionType},
//!     leaderboard::{Leaderboard, RankingMetric},
//!     achievements::{AchievementTracker, AchievementEvent},
//! };
//! use rust_decimal_macros::dec;
//!
//! // Create the arena
//! let arena = Arena::new();
//!
//! // Create a PnL Challenge competition
//! let config = CompetitionConfig {
//!     name: "Q1 Trading Championship".to_string(),
//!     description: "Compete for the highest PnL!".to_string(),
//!     competition_type: CompetitionType::PnLChallenge,
//!     initial_balance: dec!(10000),
//!     ..Default::default()
//! };
//!
//! let competition = arena.create_competition(config)?;
//!
//! // Register an agent
//! arena.register_agent(competition.id, agent_id)?;
//!
//! // Record trades and get leaderboard
//! arena.record_trade(competition.id, &agent_id, dec!(500))?;
//! let leaderboard = arena.get_leaderboard(competition.id)?;
//!
//! // Track achievements
//! let mut tracker = AchievementTracker::new(agent_id);
//! let badges = tracker.process_event(AchievementEvent::TradeCompleted {
//!     pnl: dec!(500),
//!     volume: dec!(5000),
//!     duration_secs: 120,
//!     is_win: true,
//! });
//! ```
//!
//! ## Modules
//!
//! - [`achievements`]: Badge and achievement system with rarity tiers
//! - [`competitions`]: Competition types, lifecycle, and prize management
//! - [`leaderboard`]: Real-time rankings with multiple metrics
//!
//! ## Competition Types
//!
//! | Type | Primary Metric | Description |
//! |------|---------------|-------------|
//! | PnL Challenge | Absolute PnL | Maximize total profit |
//! | Sharpe Showdown | Sharpe Ratio | Risk-adjusted returns |
//! | Market Making Marathon | Volume/Spread | Liquidity provision |
//! | Speed Trading | Trade Duration | Fast profitable trades |
//! | Consistency Cup | Consistency Score | Steady returns |
//! | Drawdown Duel | Max Drawdown | Minimize risk |
//!
//! ## Achievement Tiers
//!
//! - **Common**: Basic milestones (50%+ can earn)
//! - **Uncommon**: Notable achievements (20-50%)
//! - **Rare**: Challenging goals (5-20%)
//! - **Epic**: Elite accomplishments (1-5%)
//! - **Legendary**: Exceptional feats (<1%)
//! - **Mythic**: Once-in-a-lifetime achievements

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Module Declarations
// ============================================================================

pub mod achievements;
pub mod competitions;
pub mod leaderboard;

// ============================================================================
// Re-exports
// ============================================================================

// Core types from dependencies
pub use openibank_types::AgentId;
pub use resonancex_types::{MarketId, Trade};

// Achievement types
pub use achievements::{
    Achievement, AchievementCategory, AchievementEvent, AchievementId,
    AchievementProgress, AchievementRegistry, AchievementTracker,
    BadgeDisplay, Rarity, UnlockCondition,
};

// Competition types
pub use competitions::{
    Competition, CompetitionConfig, CompetitionError, CompetitionManager,
    CompetitionResult, CompetitionResults, CompetitionState, CompetitionType,
    EntryRequirements, KycLevel, PrizeDistribution, PrizePool, PrizeTier,
    Registration, RegistrationStatus, Sponsor, SponsorTier, Team, TeamId,
};

// Leaderboard types
pub use leaderboard::{
    AgentPerformance, Leaderboard, LeaderboardConfig, LeaderboardEntry,
    LeaderboardSnapshot, LeaderboardSummary, RankingMetric, RankingTier,
};

// ============================================================================
// Arena Errors
// ============================================================================

/// Errors that can occur in arena operations
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

    #[error("Competition error: {0}")]
    Competition(#[from] CompetitionError),
}

/// Result type for arena operations
pub type ArenaResult<T> = Result<T, ArenaError>;

// ============================================================================
// Competition Identifier
// ============================================================================

/// Unique competition identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompetitionId(pub Uuid);

impl CompetitionId {
    /// Create a new random competition ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
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

// ============================================================================
// Legacy Competition Status (for backward compatibility)
// ============================================================================

/// Competition status (legacy - use CompetitionState for new code)
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

impl From<CompetitionState> for CompetitionStatus {
    fn from(state: CompetitionState) -> Self {
        match state {
            CompetitionState::Draft | CompetitionState::Announced => Self::Scheduled,
            CompetitionState::Registration | CompetitionState::Starting => Self::Registration,
            CompetitionState::Running | CompetitionState::Paused => Self::Active,
            CompetitionState::Calculating => Self::Calculating,
            CompetitionState::Ended => Self::Completed,
            CompetitionState::Cancelled => Self::Cancelled,
        }
    }
}

// ============================================================================
// Legacy Competition Type (for backward compatibility)
// ============================================================================

/// Legacy competition type enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyCompetitionType {
    /// Maximize absolute PnL
    PnL,
    /// Maximize risk-adjusted returns (Sharpe ratio)
    SharpeRatio,
    /// Market making competition
    MarketMaking,
    /// Custom scoring function
    Custom(String),
}

impl Default for LegacyCompetitionType {
    fn default() -> Self {
        Self::PnL
    }
}

impl From<CompetitionType> for LegacyCompetitionType {
    fn from(ct: CompetitionType) -> Self {
        match ct {
            CompetitionType::PnLChallenge => Self::PnL,
            CompetitionType::SharpeShowdown => Self::SharpeRatio,
            CompetitionType::MarketMakingMarathon => Self::MarketMaking,
            _ => Self::Custom(ct.display_name().to_string()),
        }
    }
}

// ============================================================================
// Legacy Competition Config (for backward compatibility)
// ============================================================================

/// Legacy competition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyCompetitionConfig {
    /// Competition name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Competition type
    pub competition_type: LegacyCompetitionType,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// Duration in hours
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

impl Default for LegacyCompetitionConfig {
    fn default() -> Self {
        Self {
            name: "Trading Competition".to_string(),
            description: None,
            competition_type: LegacyCompetitionType::PnL,
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

impl From<LegacyCompetitionConfig> for CompetitionConfig {
    fn from(legacy: LegacyCompetitionConfig) -> Self {
        let competition_type = match legacy.competition_type {
            LegacyCompetitionType::PnL => CompetitionType::PnLChallenge,
            LegacyCompetitionType::SharpeRatio => CompetitionType::SharpeShowdown,
            LegacyCompetitionType::MarketMaking => CompetitionType::MarketMakingMarathon,
            LegacyCompetitionType::Custom(_) => CompetitionType::Custom,
        };

        let now = Utc::now();
        Self {
            name: legacy.name,
            description: legacy.description.unwrap_or_default(),
            competition_type,
            custom_scoring: None,
            start_time: legacy.start_time,
            registration_opens: now,
            registration_closes: legacy.start_time - Duration::hours(1),
            duration_hours: legacy.duration_hours,
            markets: legacy.markets,
            initial_balance: legacy.initial_balance,
            max_participants: legacy.max_agents,
            min_participants: 2,
            entry_fee: legacy.entry_fee,
            requirements: EntryRequirements::default(),
            prize_pool: legacy.prize_pool
                .map(PrizePool::standard)
                .unwrap_or_default(),
            team_competition: false,
            team_size: None,
            leaderboard_config: LeaderboardConfig::default(),
            rules_url: None,
            is_public: true,
            is_featured: false,
            tags: Vec::new(),
        }
    }
}

// ============================================================================
// Legacy Competition Data (for backward compatibility)
// ============================================================================

/// Legacy competition data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyCompetition {
    /// Competition ID
    pub id: CompetitionId,
    /// Configuration
    pub config: LegacyCompetitionConfig,
    /// Current status
    pub status: CompetitionStatus,
    /// Registered agent count
    pub agent_count: usize,
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl LegacyCompetition {
    /// Create a new legacy competition
    pub fn new(config: LegacyCompetitionConfig) -> Self {
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
        self.config.start_time + Duration::hours(self.config.duration_hours as i64)
    }
}

// ============================================================================
// Agent Statistics
// ============================================================================

/// Agent statistics in a competition (legacy format)
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

impl From<AgentPerformance> for AgentStats {
    fn from(perf: AgentPerformance) -> Self {
        Self {
            agent_id: perf.agent_id,
            balance: perf.balance,
            pnl: perf.pnl,
            pnl_percent: perf.pnl_percent,
            trade_count: perf.trade_count,
            win_rate: perf.win_rate / dec!(100), // Convert from percentage
            sharpe_ratio: perf.sharpe_ratio,
            max_drawdown: perf.max_drawdown,
            updated_at: perf.updated_at,
        }
    }
}

// ============================================================================
// Legacy Leaderboard Entry
// ============================================================================

/// Legacy leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyLeaderboardEntry {
    /// Rank (1-indexed)
    pub rank: usize,
    /// Agent stats
    pub stats: AgentStats,
    /// Score (depends on competition type)
    pub score: Decimal,
}

impl From<LeaderboardEntry> for LegacyLeaderboardEntry {
    fn from(entry: LeaderboardEntry) -> Self {
        Self {
            rank: entry.rank,
            stats: entry.performance.into(),
            score: entry.ranking_value,
        }
    }
}

// ============================================================================
// Arena (Main Entry Point)
// ============================================================================

/// The main Arena structure for managing trading competitions
///
/// The Arena provides a unified interface for:
/// - Creating and managing competitions
/// - Registering agents
/// - Recording trades
/// - Retrieving leaderboards
/// - Tracking achievements
pub struct Arena {
    /// Competition manager
    competition_manager: RwLock<CompetitionManager>,
    /// Legacy competitions (for backward compatibility)
    legacy_competitions: RwLock<HashMap<CompetitionId, LegacyCompetition>>,
    /// Legacy agent registrations
    legacy_registrations: RwLock<HashMap<CompetitionId, HashMap<AgentId, AgentStats>>>,
    /// Achievement registry
    achievement_registry: Arc<AchievementRegistry>,
    /// Agent achievement trackers
    achievement_trackers: RwLock<HashMap<AgentId, AchievementTracker>>,
}

impl Arena {
    /// Create a new arena instance
    pub fn new() -> Self {
        Self {
            competition_manager: RwLock::new(CompetitionManager::new()),
            legacy_competitions: RwLock::new(HashMap::new()),
            legacy_registrations: RwLock::new(HashMap::new()),
            achievement_registry: Arc::new(AchievementRegistry::new()),
            achievement_trackers: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new arena with custom achievement registry
    pub fn with_achievements(registry: AchievementRegistry) -> Self {
        Self {
            competition_manager: RwLock::new(CompetitionManager::new()),
            legacy_competitions: RwLock::new(HashMap::new()),
            legacy_registrations: RwLock::new(HashMap::new()),
            achievement_registry: Arc::new(registry),
            achievement_trackers: RwLock::new(HashMap::new()),
        }
    }

    // ========================================================================
    // Competition Management (New API)
    // ========================================================================

    /// Create a new competition using the full configuration
    pub fn create_full_competition(&self, config: CompetitionConfig) -> &Self {
        let mut manager = self.competition_manager.write();
        manager.create_competition(config);
        self
    }

    /// Get the competition manager for advanced operations
    pub fn competition_manager(&self) -> impl std::ops::Deref<Target = CompetitionManager> + '_ {
        self.competition_manager.read()
    }

    /// Get mutable competition manager
    pub fn competition_manager_mut(&self) -> impl std::ops::DerefMut<Target = CompetitionManager> + '_ {
        self.competition_manager.write()
    }

    // ========================================================================
    // Competition Management (Legacy API - Backward Compatible)
    // ========================================================================

    /// Create a new competition (legacy API)
    pub fn create_competition(&self, config: LegacyCompetitionConfig) -> ArenaResult<LegacyCompetition> {
        let competition = LegacyCompetition::new(config);
        let id = competition.id;

        self.legacy_competitions.write().insert(id, competition.clone());
        self.legacy_registrations.write().insert(id, HashMap::new());

        Ok(competition)
    }

    /// Get a competition by ID (legacy API)
    pub fn get_competition(&self, id: CompetitionId) -> ArenaResult<LegacyCompetition> {
        self.legacy_competitions
            .read()
            .get(&id)
            .cloned()
            .ok_or(ArenaError::CompetitionNotFound(id))
    }

    /// List all competitions (legacy API)
    pub fn list_competitions(&self) -> Vec<LegacyCompetition> {
        self.legacy_competitions.read().values().cloned().collect()
    }

    /// Register an agent for a competition (legacy API)
    pub fn register_agent(&self, competition_id: CompetitionId, agent_id: AgentId) -> ArenaResult<()> {
        let competition = self.get_competition(competition_id)?;

        if !competition.is_registration_open() {
            return Err(ArenaError::RegistrationClosed);
        }

        let mut registrations = self.legacy_registrations.write();
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
        agents.insert(agent_id.clone(), stats);

        // Initialize achievement tracker for agent
        let mut trackers = self.achievement_trackers.write();
        if !trackers.contains_key(&agent_id) {
            trackers.insert(agent_id.clone(), self.achievement_registry.create_tracker(agent_id));
        }

        // Update agent count
        drop(registrations);
        if let Some(comp) = self.legacy_competitions.write().get_mut(&competition_id) {
            comp.agent_count += 1;
        }

        Ok(())
    }

    /// Get agent stats in a competition (legacy API)
    pub fn get_agent_stats(&self, competition_id: CompetitionId, agent_id: &AgentId) -> ArenaResult<AgentStats> {
        self.legacy_registrations
            .read()
            .get(&competition_id)
            .and_then(|agents| agents.get(agent_id))
            .cloned()
            .ok_or(ArenaError::AgentNotFound(agent_id.clone()))
    }

    /// Get leaderboard for a competition (legacy API)
    pub fn get_leaderboard(&self, competition_id: CompetitionId) -> ArenaResult<Vec<LegacyLeaderboardEntry>> {
        let competition = self.get_competition(competition_id)?;

        let registrations = self.legacy_registrations.read();
        let agents = registrations
            .get(&competition_id)
            .ok_or(ArenaError::CompetitionNotFound(competition_id))?;

        let mut entries: Vec<_> = agents
            .values()
            .map(|stats| {
                let score = match &competition.config.competition_type {
                    LegacyCompetitionType::PnL => stats.pnl,
                    LegacyCompetitionType::SharpeRatio => stats.sharpe_ratio.unwrap_or_default(),
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
            .map(|(i, (stats, score))| LegacyLeaderboardEntry {
                rank: i + 1,
                stats,
                score,
            })
            .collect())
    }

    /// Record a trade for an agent (legacy API)
    pub fn record_trade(&self, competition_id: CompetitionId, agent_id: &AgentId, pnl: Decimal) -> ArenaResult<()> {
        let mut registrations = self.legacy_registrations.write();
        let agents = registrations
            .get_mut(&competition_id)
            .ok_or(ArenaError::CompetitionNotFound(competition_id))?;

        let stats = agents
            .get_mut(agent_id)
            .ok_or(ArenaError::AgentNotFound(agent_id.clone()))?;

        stats.record_trade(pnl);

        // Process achievement event
        drop(registrations);
        self.process_trade_achievement(agent_id, pnl, dec!(0), 0);

        Ok(())
    }

    // ========================================================================
    // Achievement System
    // ========================================================================

    /// Get achievement registry
    pub fn achievement_registry(&self) -> &AchievementRegistry {
        &self.achievement_registry
    }

    /// Get or create achievement tracker for an agent
    pub fn get_achievement_tracker(&self, agent_id: &AgentId) -> Option<AchievementTracker> {
        self.achievement_trackers.read().get(agent_id).cloned()
    }

    /// Process a trade for achievements
    pub fn process_trade_achievement(
        &self,
        agent_id: &AgentId,
        pnl: Decimal,
        volume: Decimal,
        duration_secs: u64,
    ) -> Vec<BadgeDisplay> {
        let mut trackers = self.achievement_trackers.write();

        if let Some(tracker) = trackers.get_mut(agent_id) {
            let event = AchievementEvent::TradeCompleted {
                pnl,
                volume,
                duration_secs,
                is_win: pnl > Decimal::ZERO,
            };
            tracker.process_event(event)
        } else {
            Vec::new()
        }
    }

    /// Process any achievement event
    pub fn process_achievement_event(
        &self,
        agent_id: &AgentId,
        event: AchievementEvent,
    ) -> Vec<BadgeDisplay> {
        let mut trackers = self.achievement_trackers.write();

        if let Some(tracker) = trackers.get_mut(agent_id) {
            tracker.process_event(event)
        } else {
            Vec::new()
        }
    }

    /// Get agent's unlocked badges
    pub fn get_agent_badges(&self, agent_id: &AgentId) -> Vec<BadgeDisplay> {
        self.achievement_trackers
            .read()
            .get(agent_id)
            .map(|t| t.get_unlocked_badges())
            .unwrap_or_default()
    }

    // ========================================================================
    // Utility Methods
    // ========================================================================

    /// Process automatic transitions for all competitions
    pub fn tick(&self) {
        self.competition_manager.write().process_auto_transitions();
    }

    /// Get arena statistics
    pub fn get_stats(&self) -> ArenaStats {
        let manager = self.competition_manager.read();
        let legacy = self.legacy_competitions.read();
        let trackers = self.achievement_trackers.read();

        ArenaStats {
            total_competitions: manager.list_active().len() + legacy.len(),
            active_competitions: manager.list_active().len()
                + legacy.values().filter(|c| c.is_active()).count(),
            total_agents: trackers.len(),
            total_achievements_unlocked: trackers.values().map(|t| t.total_points).sum(),
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

/// Arena statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaStats {
    pub total_competitions: usize,
    pub active_competitions: usize,
    pub total_agents: usize,
    pub total_achievements_unlocked: u32,
}

// ============================================================================
// Tests
// ============================================================================

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

        let config = LegacyCompetitionConfig {
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

        let competition = arena.create_competition(LegacyCompetitionConfig::default()).unwrap();
        let agent_id = AgentId::new();

        arena.register_agent(competition.id, agent_id.clone()).unwrap();

        let stats = arena.get_agent_stats(competition.id, &agent_id).unwrap();
        assert_eq!(stats.balance, dec!(10000));
        assert_eq!(stats.pnl, Decimal::ZERO);
    }

    #[test]
    fn test_duplicate_registration() {
        let arena = Arena::new();

        let competition = arena.create_competition(LegacyCompetitionConfig::default()).unwrap();
        let agent_id = AgentId::new();

        arena.register_agent(competition.id, agent_id.clone()).unwrap();
        let result = arena.register_agent(competition.id, agent_id);

        assert!(matches!(result, Err(ArenaError::AlreadyRegistered)));
    }

    #[test]
    fn test_leaderboard() {
        let arena = Arena::new();

        let competition = arena.create_competition(LegacyCompetitionConfig::default()).unwrap();

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

    #[test]
    fn test_achievement_tracking() {
        let arena = Arena::new();

        let competition = arena.create_competition(LegacyCompetitionConfig::default()).unwrap();
        let agent_id = AgentId::new();

        arena.register_agent(competition.id, agent_id.clone()).unwrap();

        // Record a trade and check for achievements
        let badges = arena.process_trade_achievement(&agent_id, dec!(100), dec!(1000), 60);

        // Should have unlocked "First Steps" achievement
        assert!(!badges.is_empty());
    }

    #[test]
    fn test_competition_id() {
        let id1 = CompetitionId::new();
        let id2 = CompetitionId::new();
        assert_ne!(id1, id2);

        let id3 = CompetitionId::from_uuid(id1.0);
        assert_eq!(id1, id3);
    }
}
