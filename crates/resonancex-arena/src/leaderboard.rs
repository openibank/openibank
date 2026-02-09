//! # ResonanceX Arena Leaderboard System
//!
//! Real-time leaderboard tracking with multiple ranking metrics, historical snapshots,
//! and a comprehensive badge/achievement integration system.
//!
//! ## Features
//!
//! - **Multi-Metric Rankings**: PnL, Sharpe Ratio, Win Rate, Volume, and custom metrics
//! - **Real-Time Updates**: Live leaderboard updates with efficient delta tracking
//! - **Historical Snapshots**: Periodic snapshots for trend analysis and replay
//! - **Badge Integration**: Display earned badges and achievements on leaderboards
//! - **Tier System**: Visual ranking tiers (Diamond, Platinum, Gold, Silver, Bronze)
//!
//! ## Example
//!
//! ```ignore
//! use resonancex_arena::leaderboard::{Leaderboard, RankingMetric, LeaderboardConfig};
//!
//! let config = LeaderboardConfig::default();
//! let mut leaderboard = Leaderboard::new(competition_id, config);
//!
//! // Update agent performance
//! leaderboard.update_agent_performance(agent_id, &performance);
//!
//! // Get rankings by different metrics
//! let pnl_rankings = leaderboard.get_rankings(RankingMetric::PnL, 10);
//! let sharpe_rankings = leaderboard.get_rankings(RankingMetric::SharpeRatio, 10);
//!
//! // Take a snapshot for historical tracking
//! leaderboard.take_snapshot();
//! ```

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::achievements::BadgeDisplay;
use crate::CompetitionId;
use openibank_types::AgentId;

// ============================================================================
// Ranking Metrics
// ============================================================================

/// Available metrics for ranking agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RankingMetric {
    /// Absolute profit and loss
    PnL,
    /// Percentage return on initial capital
    PnLPercent,
    /// Risk-adjusted returns (Sharpe ratio)
    SharpeRatio,
    /// Sortino ratio (downside risk-adjusted)
    SortinoRatio,
    /// Win rate percentage
    WinRate,
    /// Total trading volume
    Volume,
    /// Number of trades executed
    TradeCount,
    /// Maximum drawdown (lower is better, ranked ascending)
    MaxDrawdown,
    /// Profit factor (gross profit / gross loss)
    ProfitFactor,
    /// Average trade duration
    AvgTradeDuration,
    /// Consistency score (custom metric)
    ConsistencyScore,
    /// Composite score combining multiple metrics
    CompositeScore,
}

impl RankingMetric {
    /// Whether higher values are better (true) or lower values are better (false)
    pub fn higher_is_better(&self) -> bool {
        match self {
            Self::MaxDrawdown => false, // Lower drawdown is better
            Self::AvgTradeDuration => false, // Faster trades (for speed competitions)
            _ => true,
        }
    }

    /// Get display name for the metric
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PnL => "Profit & Loss",
            Self::PnLPercent => "Return %",
            Self::SharpeRatio => "Sharpe Ratio",
            Self::SortinoRatio => "Sortino Ratio",
            Self::WinRate => "Win Rate",
            Self::Volume => "Trading Volume",
            Self::TradeCount => "Trade Count",
            Self::MaxDrawdown => "Max Drawdown",
            Self::ProfitFactor => "Profit Factor",
            Self::AvgTradeDuration => "Avg Trade Duration",
            Self::ConsistencyScore => "Consistency",
            Self::CompositeScore => "Overall Score",
        }
    }

    /// Get emoji indicator for the metric
    pub fn icon(&self) -> &'static str {
        match self {
            Self::PnL | Self::PnLPercent => "$",
            Self::SharpeRatio | Self::SortinoRatio => "~",
            Self::WinRate => "%",
            Self::Volume => "#",
            Self::TradeCount => "#",
            Self::MaxDrawdown => "v",
            Self::ProfitFactor => "x",
            Self::AvgTradeDuration => "@",
            Self::ConsistencyScore => "*",
            Self::CompositeScore => "+",
        }
    }
}

// ============================================================================
// Ranking Tiers
// ============================================================================

/// Visual ranking tier for leaderboard display
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RankingTier {
    /// Top 1% - Elite performers
    Diamond,
    /// Top 5% - Exceptional performers
    Platinum,
    /// Top 15% - Strong performers
    Gold,
    /// Top 35% - Above average performers
    Silver,
    /// Top 60% - Average performers
    Bronze,
    /// Bottom 40% - Developing performers
    Iron,
    /// New participants without enough data
    Unranked,
}

impl RankingTier {
    /// Determine tier based on percentile rank
    pub fn from_percentile(percentile: f64) -> Self {
        match percentile {
            p if p >= 99.0 => Self::Diamond,
            p if p >= 95.0 => Self::Platinum,
            p if p >= 85.0 => Self::Gold,
            p if p >= 65.0 => Self::Silver,
            p if p >= 40.0 => Self::Bronze,
            p if p >= 0.0 => Self::Iron,
            _ => Self::Unranked,
        }
    }

    /// Get tier display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Diamond => "Diamond",
            Self::Platinum => "Platinum",
            Self::Gold => "Gold",
            Self::Silver => "Silver",
            Self::Bronze => "Bronze",
            Self::Iron => "Iron",
            Self::Unranked => "Unranked",
        }
    }

    /// Get tier color (hex)
    pub fn color(&self) -> &'static str {
        match self {
            Self::Diamond => "#B9F2FF",
            Self::Platinum => "#E5E4E2",
            Self::Gold => "#FFD700",
            Self::Silver => "#C0C0C0",
            Self::Bronze => "#CD7F32",
            Self::Iron => "#5A5A5A",
            Self::Unranked => "#808080",
        }
    }

    /// Get minimum percentile for this tier
    pub fn min_percentile(&self) -> f64 {
        match self {
            Self::Diamond => 99.0,
            Self::Platinum => 95.0,
            Self::Gold => 85.0,
            Self::Silver => 65.0,
            Self::Bronze => 40.0,
            Self::Iron => 0.0,
            Self::Unranked => 0.0,
        }
    }
}

// ============================================================================
// Agent Performance Data
// ============================================================================

/// Comprehensive performance metrics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformance {
    /// Agent identifier
    pub agent_id: AgentId,
    /// Agent display name
    pub display_name: String,
    /// Current balance
    pub balance: Decimal,
    /// Initial balance at competition start
    pub initial_balance: Decimal,
    /// Absolute PnL
    pub pnl: Decimal,
    /// PnL as percentage of initial
    pub pnl_percent: Decimal,
    /// Total trading volume
    pub volume: Decimal,
    /// Number of trades
    pub trade_count: u64,
    /// Number of winning trades
    pub winning_trades: u64,
    /// Number of losing trades
    pub losing_trades: u64,
    /// Win rate (0-100)
    pub win_rate: Decimal,
    /// Gross profit
    pub gross_profit: Decimal,
    /// Gross loss
    pub gross_loss: Decimal,
    /// Profit factor
    pub profit_factor: Option<Decimal>,
    /// Sharpe ratio (annualized)
    pub sharpe_ratio: Option<Decimal>,
    /// Sortino ratio
    pub sortino_ratio: Option<Decimal>,
    /// Maximum drawdown
    pub max_drawdown: Decimal,
    /// Maximum drawdown percentage
    pub max_drawdown_percent: Decimal,
    /// Current drawdown
    pub current_drawdown: Decimal,
    /// Peak balance
    pub peak_balance: Decimal,
    /// Average trade duration in seconds
    pub avg_trade_duration_secs: u64,
    /// Longest winning streak
    pub longest_win_streak: u32,
    /// Current winning streak
    pub current_win_streak: u32,
    /// Longest losing streak
    pub longest_lose_streak: u32,
    /// Current losing streak
    pub current_lose_streak: u32,
    /// Consistency score (0-100)
    pub consistency_score: Decimal,
    /// Composite score (weighted combination)
    pub composite_score: Decimal,
    /// Returns history for Sharpe calculation
    #[serde(skip)]
    pub returns_history: Vec<Decimal>,
    /// Earned badges to display
    pub badges: Vec<BadgeDisplay>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Time in competition (for duration-based metrics)
    pub competition_start: DateTime<Utc>,
}

impl AgentPerformance {
    /// Create new performance tracker for an agent
    pub fn new(agent_id: AgentId, display_name: String, initial_balance: Decimal) -> Self {
        let now = Utc::now();
        Self {
            agent_id,
            display_name,
            balance: initial_balance,
            initial_balance,
            pnl: Decimal::ZERO,
            pnl_percent: Decimal::ZERO,
            volume: Decimal::ZERO,
            trade_count: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: Decimal::ZERO,
            gross_profit: Decimal::ZERO,
            gross_loss: Decimal::ZERO,
            profit_factor: None,
            sharpe_ratio: None,
            sortino_ratio: None,
            max_drawdown: Decimal::ZERO,
            max_drawdown_percent: Decimal::ZERO,
            current_drawdown: Decimal::ZERO,
            peak_balance: initial_balance,
            avg_trade_duration_secs: 0,
            longest_win_streak: 0,
            current_win_streak: 0,
            longest_lose_streak: 0,
            current_lose_streak: 0,
            consistency_score: dec!(50), // Start at neutral
            composite_score: Decimal::ZERO,
            returns_history: Vec::new(),
            badges: Vec::new(),
            updated_at: now,
            competition_start: now,
        }
    }

    /// Record a completed trade
    pub fn record_trade(&mut self, trade_pnl: Decimal, trade_volume: Decimal, duration_secs: u64) {
        // Update PnL
        self.pnl += trade_pnl;
        self.balance += trade_pnl;
        self.volume += trade_volume;
        self.trade_count += 1;

        // Calculate return for this trade
        let trade_return = if self.initial_balance > Decimal::ZERO {
            trade_pnl / self.initial_balance
        } else {
            Decimal::ZERO
        };
        self.returns_history.push(trade_return);

        // Update win/loss tracking
        if trade_pnl > Decimal::ZERO {
            self.winning_trades += 1;
            self.gross_profit += trade_pnl;
            self.current_win_streak += 1;
            self.current_lose_streak = 0;
            if self.current_win_streak > self.longest_win_streak {
                self.longest_win_streak = self.current_win_streak;
            }
        } else if trade_pnl < Decimal::ZERO {
            self.losing_trades += 1;
            self.gross_loss += trade_pnl.abs();
            self.current_lose_streak += 1;
            self.current_win_streak = 0;
            if self.current_lose_streak > self.longest_lose_streak {
                self.longest_lose_streak = self.current_lose_streak;
            }
        }

        // Update derived metrics
        self.recalculate_metrics(duration_secs);
        self.updated_at = Utc::now();
    }

    /// Recalculate all derived metrics
    fn recalculate_metrics(&mut self, new_trade_duration: u64) {
        // PnL percentage
        if self.initial_balance > Decimal::ZERO {
            self.pnl_percent = (self.pnl / self.initial_balance) * dec!(100);
        }

        // Win rate
        if self.trade_count > 0 {
            self.win_rate = Decimal::from(self.winning_trades * 100) / Decimal::from(self.trade_count);
        }

        // Profit factor
        if self.gross_loss > Decimal::ZERO {
            self.profit_factor = Some(self.gross_profit / self.gross_loss);
        }

        // Update peak and drawdown
        if self.balance > self.peak_balance {
            self.peak_balance = self.balance;
            self.current_drawdown = Decimal::ZERO;
        } else {
            self.current_drawdown = self.peak_balance - self.balance;
            if self.current_drawdown > self.max_drawdown {
                self.max_drawdown = self.current_drawdown;
            }
        }

        // Max drawdown percentage
        if self.peak_balance > Decimal::ZERO {
            self.max_drawdown_percent = (self.max_drawdown / self.peak_balance) * dec!(100);
        }

        // Average trade duration (moving average)
        if self.trade_count > 0 {
            let total_duration = self.avg_trade_duration_secs * (self.trade_count - 1) as u64 + new_trade_duration;
            self.avg_trade_duration_secs = total_duration / self.trade_count;
        }

        // Sharpe ratio (simplified - assumes risk-free rate of 0)
        self.sharpe_ratio = self.calculate_sharpe_ratio();

        // Sortino ratio
        self.sortino_ratio = self.calculate_sortino_ratio();

        // Consistency score
        self.consistency_score = self.calculate_consistency_score();

        // Composite score
        self.composite_score = self.calculate_composite_score();
    }

    /// Calculate Sharpe ratio from returns history
    fn calculate_sharpe_ratio(&self) -> Option<Decimal> {
        if self.returns_history.len() < 2 {
            return None;
        }

        let n = Decimal::from(self.returns_history.len() as u64);
        let sum: Decimal = self.returns_history.iter().sum();
        let mean = sum / n;

        let variance: Decimal = self.returns_history
            .iter()
            .map(|r| (*r - mean) * (*r - mean))
            .sum::<Decimal>() / n;

        // Standard deviation
        let std_dev = decimal_sqrt(variance)?;

        if std_dev.is_zero() {
            return None;
        }

        // Annualized Sharpe (assuming daily returns, 252 trading days)
        let annualized_factor = decimal_sqrt(dec!(252))?;
        Some((mean / std_dev) * annualized_factor)
    }

    /// Calculate Sortino ratio (only considers downside deviation)
    fn calculate_sortino_ratio(&self) -> Option<Decimal> {
        if self.returns_history.len() < 2 {
            return None;
        }

        let n = Decimal::from(self.returns_history.len() as u64);
        let sum: Decimal = self.returns_history.iter().sum();
        let mean = sum / n;

        // Only consider negative returns for downside deviation
        let downside_returns: Vec<Decimal> = self.returns_history
            .iter()
            .filter(|r| **r < Decimal::ZERO)
            .copied()
            .collect();

        if downside_returns.is_empty() {
            return Some(dec!(999)); // No downside = excellent
        }

        let downside_variance: Decimal = downside_returns
            .iter()
            .map(|r| r * r)
            .sum::<Decimal>() / Decimal::from(downside_returns.len() as u64);

        let downside_dev = decimal_sqrt(downside_variance)?;

        if downside_dev.is_zero() {
            return None;
        }

        let annualized_factor = decimal_sqrt(dec!(252))?;
        Some((mean / downside_dev) * annualized_factor)
    }

    /// Calculate consistency score based on multiple factors
    fn calculate_consistency_score(&self) -> Decimal {
        let mut score = dec!(50); // Start at neutral

        // Win rate component (up to +20)
        if self.win_rate > dec!(50) {
            score += (self.win_rate - dec!(50)) * dec!(0.4);
        }

        // Low drawdown bonus (up to +15)
        if self.max_drawdown_percent < dec!(10) {
            score += dec!(15);
        } else if self.max_drawdown_percent < dec!(20) {
            score += dec!(10);
        } else if self.max_drawdown_percent < dec!(30) {
            score += dec!(5);
        }

        // Profit factor bonus (up to +15)
        if let Some(pf) = self.profit_factor {
            if pf > dec!(2) {
                score += dec!(15);
            } else if pf > dec!(1.5) {
                score += dec!(10);
            } else if pf > dec!(1) {
                score += dec!(5);
            }
        }

        // Clamp to 0-100
        score.max(Decimal::ZERO).min(dec!(100))
    }

    /// Calculate composite score (weighted combination of metrics)
    fn calculate_composite_score(&self) -> Decimal {
        let mut score = Decimal::ZERO;

        // PnL component (30% weight)
        score += self.pnl_percent * dec!(0.3);

        // Sharpe component (25% weight, scaled)
        if let Some(sharpe) = self.sharpe_ratio {
            score += sharpe * dec!(10) * dec!(0.25);
        }

        // Win rate component (20% weight)
        score += self.win_rate * dec!(0.2);

        // Low drawdown bonus (15% weight)
        let drawdown_score = dec!(100) - self.max_drawdown_percent.min(dec!(100));
        score += drawdown_score * dec!(0.15);

        // Consistency component (10% weight)
        score += self.consistency_score * dec!(0.1);

        score
    }

    /// Get metric value by type
    pub fn get_metric(&self, metric: RankingMetric) -> Decimal {
        match metric {
            RankingMetric::PnL => self.pnl,
            RankingMetric::PnLPercent => self.pnl_percent,
            RankingMetric::SharpeRatio => self.sharpe_ratio.unwrap_or_default(),
            RankingMetric::SortinoRatio => self.sortino_ratio.unwrap_or_default(),
            RankingMetric::WinRate => self.win_rate,
            RankingMetric::Volume => self.volume,
            RankingMetric::TradeCount => Decimal::from(self.trade_count),
            RankingMetric::MaxDrawdown => self.max_drawdown_percent,
            RankingMetric::ProfitFactor => self.profit_factor.unwrap_or_default(),
            RankingMetric::AvgTradeDuration => Decimal::from(self.avg_trade_duration_secs),
            RankingMetric::ConsistencyScore => self.consistency_score,
            RankingMetric::CompositeScore => self.composite_score,
        }
    }
}

/// Simple decimal square root approximation using Newton's method
fn decimal_sqrt(value: Decimal) -> Option<Decimal> {
    if value < Decimal::ZERO {
        return None;
    }
    if value.is_zero() {
        return Some(Decimal::ZERO);
    }

    let mut guess = value / dec!(2);
    for _ in 0..20 {
        let new_guess = (guess + value / guess) / dec!(2);
        if (new_guess - guess).abs() < dec!(0.0000001) {
            return Some(new_guess);
        }
        guess = new_guess;
    }
    Some(guess)
}

// ============================================================================
// Leaderboard Entry
// ============================================================================

/// A single entry in the leaderboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Current rank (1-indexed)
    pub rank: usize,
    /// Previous rank (for showing movement)
    pub previous_rank: Option<usize>,
    /// Rank change since last update (+ve = improved, -ve = dropped)
    pub rank_change: i32,
    /// Agent performance data
    pub performance: AgentPerformance,
    /// The metric value used for this ranking
    pub ranking_value: Decimal,
    /// Agent's tier based on percentile
    pub tier: RankingTier,
    /// Percentile rank (0-100)
    pub percentile: f64,
    /// Is this entry highlighted (e.g., current user)
    pub highlighted: bool,
}

impl LeaderboardEntry {
    /// Check if rank improved
    pub fn rank_improved(&self) -> bool {
        self.rank_change > 0
    }

    /// Check if rank dropped
    pub fn rank_dropped(&self) -> bool {
        self.rank_change < 0
    }

    /// Get rank movement indicator
    pub fn rank_indicator(&self) -> &'static str {
        match self.rank_change {
            c if c > 0 => "^",
            c if c < 0 => "v",
            _ => "-",
        }
    }
}

// ============================================================================
// Leaderboard Snapshot
// ============================================================================

/// Historical snapshot of leaderboard state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    /// Unique snapshot ID
    pub id: Uuid,
    /// Competition ID
    pub competition_id: CompetitionId,
    /// Metric this snapshot is for
    pub metric: RankingMetric,
    /// Timestamp of snapshot
    pub timestamp: DateTime<Utc>,
    /// Top N entries at this time
    pub entries: Vec<LeaderboardEntry>,
    /// Total participants at snapshot time
    pub total_participants: usize,
}

impl LeaderboardSnapshot {
    /// Create a new snapshot
    pub fn new(
        competition_id: CompetitionId,
        metric: RankingMetric,
        entries: Vec<LeaderboardEntry>,
        total_participants: usize,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            competition_id,
            metric,
            timestamp: Utc::now(),
            entries,
            total_participants,
        }
    }
}

// ============================================================================
// Leaderboard Configuration
// ============================================================================

/// Configuration for leaderboard behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardConfig {
    /// Primary ranking metric
    pub primary_metric: RankingMetric,
    /// Secondary metrics to track
    pub secondary_metrics: Vec<RankingMetric>,
    /// How often to take snapshots (in seconds)
    pub snapshot_interval_secs: u64,
    /// Maximum number of snapshots to retain
    pub max_snapshots: usize,
    /// Number of entries to show in leaderboard
    pub display_limit: usize,
    /// Whether to show badges on leaderboard
    pub show_badges: bool,
    /// Whether to show rank changes
    pub show_rank_changes: bool,
    /// Weights for composite score calculation
    pub composite_weights: CompositeWeights,
}

/// Weights for composite score calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeWeights {
    pub pnl: Decimal,
    pub sharpe: Decimal,
    pub win_rate: Decimal,
    pub drawdown: Decimal,
    pub consistency: Decimal,
}

impl Default for CompositeWeights {
    fn default() -> Self {
        Self {
            pnl: dec!(0.30),
            sharpe: dec!(0.25),
            win_rate: dec!(0.20),
            drawdown: dec!(0.15),
            consistency: dec!(0.10),
        }
    }
}

impl Default for LeaderboardConfig {
    fn default() -> Self {
        Self {
            primary_metric: RankingMetric::PnL,
            secondary_metrics: vec![
                RankingMetric::SharpeRatio,
                RankingMetric::WinRate,
                RankingMetric::Volume,
            ],
            snapshot_interval_secs: 3600, // Every hour
            max_snapshots: 168, // 7 days worth
            display_limit: 100,
            show_badges: true,
            show_rank_changes: true,
            composite_weights: CompositeWeights::default(),
        }
    }
}

// ============================================================================
// Main Leaderboard
// ============================================================================

/// Real-time leaderboard tracking system
#[derive(Debug)]
pub struct Leaderboard {
    /// Competition this leaderboard is for
    pub competition_id: CompetitionId,
    /// Configuration
    pub config: LeaderboardConfig,
    /// Agent performances indexed by agent ID
    performances: HashMap<AgentId, AgentPerformance>,
    /// Cached rankings by metric (metric -> sorted list of agent IDs)
    rankings_cache: HashMap<RankingMetric, Vec<AgentId>>,
    /// Previous rankings for calculating changes
    previous_rankings: HashMap<RankingMetric, HashMap<AgentId, usize>>,
    /// Historical snapshots
    snapshots: VecDeque<LeaderboardSnapshot>,
    /// Last snapshot time
    last_snapshot: DateTime<Utc>,
    /// Whether cache needs refresh
    cache_dirty: bool,
}

impl Leaderboard {
    /// Create a new leaderboard for a competition
    pub fn new(competition_id: CompetitionId, config: LeaderboardConfig) -> Self {
        Self {
            competition_id,
            config,
            performances: HashMap::new(),
            rankings_cache: HashMap::new(),
            previous_rankings: HashMap::new(),
            snapshots: VecDeque::new(),
            last_snapshot: Utc::now(),
            cache_dirty: false,
        }
    }

    /// Register a new agent
    pub fn register_agent(&mut self, agent_id: AgentId, display_name: String, initial_balance: Decimal) {
        let performance = AgentPerformance::new(agent_id.clone(), display_name, initial_balance);
        self.performances.insert(agent_id, performance);
        self.cache_dirty = true;
    }

    /// Update agent performance with a new trade
    pub fn record_trade(
        &mut self,
        agent_id: &AgentId,
        trade_pnl: Decimal,
        trade_volume: Decimal,
        duration_secs: u64,
    ) -> Option<&AgentPerformance> {
        if let Some(perf) = self.performances.get_mut(agent_id) {
            perf.record_trade(trade_pnl, trade_volume, duration_secs);
            self.cache_dirty = true;
            return Some(perf);
        }
        None
    }

    /// Add badges to an agent's display
    pub fn add_badge(&mut self, agent_id: &AgentId, badge: BadgeDisplay) {
        if let Some(perf) = self.performances.get_mut(agent_id) {
            perf.badges.push(badge);
        }
    }

    /// Get agent performance
    pub fn get_performance(&self, agent_id: &AgentId) -> Option<&AgentPerformance> {
        self.performances.get(agent_id)
    }

    /// Get total number of participants
    pub fn participant_count(&self) -> usize {
        self.performances.len()
    }

    /// Refresh rankings cache
    fn refresh_cache(&mut self) {
        if !self.cache_dirty {
            return;
        }

        // Store previous rankings
        self.previous_rankings = self.rankings_cache
            .iter()
            .map(|(metric, agents)| {
                let ranks: HashMap<AgentId, usize> = agents
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (id.clone(), i + 1))
                    .collect();
                (*metric, ranks)
            })
            .collect();

        // Rebuild all rankings
        let all_metrics = std::iter::once(self.config.primary_metric)
            .chain(self.config.secondary_metrics.iter().copied())
            .collect::<Vec<_>>();

        for metric in all_metrics {
            let mut agents: Vec<_> = self.performances.keys().cloned().collect();

            agents.sort_by(|a, b| {
                let perf_a = self.performances.get(a).unwrap();
                let perf_b = self.performances.get(b).unwrap();
                let val_a = perf_a.get_metric(metric);
                let val_b = perf_b.get_metric(metric);

                if metric.higher_is_better() {
                    val_b.partial_cmp(&val_a).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    val_a.partial_cmp(&val_b).unwrap_or(std::cmp::Ordering::Equal)
                }
            });

            self.rankings_cache.insert(metric, agents);
        }

        self.cache_dirty = false;
    }

    /// Get rankings for a specific metric
    pub fn get_rankings(&mut self, metric: RankingMetric, limit: usize) -> Vec<LeaderboardEntry> {
        self.refresh_cache();

        let total = self.performances.len();
        if total == 0 {
            return Vec::new();
        }

        let agents = match self.rankings_cache.get(&metric) {
            Some(a) => a.clone(),
            None => return Vec::new(),
        };

        let previous = self.previous_rankings.get(&metric);

        agents
            .into_iter()
            .take(limit)
            .enumerate()
            .filter_map(|(i, agent_id)| {
                let performance = self.performances.get(&agent_id)?.clone();
                let rank = i + 1;
                let previous_rank = previous.and_then(|p| p.get(&agent_id).copied());
                let rank_change = previous_rank.map(|p| p as i32 - rank as i32).unwrap_or(0);
                let percentile = ((total - rank) as f64 / total as f64) * 100.0;
                let tier = RankingTier::from_percentile(percentile);

                Some(LeaderboardEntry {
                    rank,
                    previous_rank,
                    rank_change,
                    ranking_value: performance.get_metric(metric),
                    performance,
                    tier,
                    percentile,
                    highlighted: false,
                })
            })
            .collect()
    }

    /// Get an agent's rank for a specific metric
    pub fn get_agent_rank(&mut self, agent_id: &AgentId, metric: RankingMetric) -> Option<usize> {
        self.refresh_cache();

        self.rankings_cache
            .get(&metric)?
            .iter()
            .position(|id| id == agent_id)
            .map(|p| p + 1)
    }

    /// Get an agent's leaderboard entry
    pub fn get_agent_entry(&mut self, agent_id: &AgentId, metric: RankingMetric) -> Option<LeaderboardEntry> {
        self.refresh_cache();

        let total = self.performances.len();
        let agents = self.rankings_cache.get(&metric)?;
        let rank = agents.iter().position(|id| id == agent_id)? + 1;
        let performance = self.performances.get(agent_id)?.clone();
        let previous = self.previous_rankings.get(&metric);
        let previous_rank = previous.and_then(|p| p.get(agent_id).copied());
        let rank_change = previous_rank.map(|p| p as i32 - rank as i32).unwrap_or(0);
        let percentile = ((total - rank) as f64 / total as f64) * 100.0;
        let tier = RankingTier::from_percentile(percentile);

        Some(LeaderboardEntry {
            rank,
            previous_rank,
            rank_change,
            ranking_value: performance.get_metric(metric),
            performance,
            tier,
            percentile,
            highlighted: true,
        })
    }

    /// Take a snapshot of current rankings
    pub fn take_snapshot(&mut self) -> LeaderboardSnapshot {
        let entries = self.get_rankings(self.config.primary_metric, self.config.display_limit);
        let snapshot = LeaderboardSnapshot::new(
            self.competition_id,
            self.config.primary_metric,
            entries,
            self.performances.len(),
        );

        self.snapshots.push_back(snapshot.clone());

        // Trim old snapshots
        while self.snapshots.len() > self.config.max_snapshots {
            self.snapshots.pop_front();
        }

        self.last_snapshot = Utc::now();
        snapshot
    }

    /// Check if a snapshot is due
    pub fn should_snapshot(&self) -> bool {
        let elapsed = Utc::now() - self.last_snapshot;
        elapsed.num_seconds() >= self.config.snapshot_interval_secs as i64
    }

    /// Get historical snapshots
    pub fn get_snapshots(&self) -> &VecDeque<LeaderboardSnapshot> {
        &self.snapshots
    }

    /// Get snapshots within a time range
    pub fn get_snapshots_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&LeaderboardSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.timestamp >= start && s.timestamp <= end)
            .collect()
    }

    /// Get agent's historical rank progression
    pub fn get_rank_history(&self, agent_id: &AgentId) -> Vec<(DateTime<Utc>, usize)> {
        self.snapshots
            .iter()
            .filter_map(|snapshot| {
                snapshot
                    .entries
                    .iter()
                    .find(|e| e.performance.agent_id == *agent_id)
                    .map(|e| (snapshot.timestamp, e.rank))
            })
            .collect()
    }

    /// Get tier distribution statistics
    pub fn get_tier_distribution(&mut self) -> HashMap<RankingTier, usize> {
        let rankings = self.get_rankings(self.config.primary_metric, usize::MAX);
        let mut distribution = HashMap::new();

        for entry in rankings {
            *distribution.entry(entry.tier).or_insert(0) += 1;
        }

        distribution
    }

    /// Get top performers by tier
    pub fn get_tier_leaders(&mut self, tier: RankingTier, limit: usize) -> Vec<LeaderboardEntry> {
        self.get_rankings(self.config.primary_metric, usize::MAX)
            .into_iter()
            .filter(|e| e.tier == tier)
            .take(limit)
            .collect()
    }

    /// Export leaderboard as JSON-friendly summary
    pub fn export_summary(&mut self) -> LeaderboardSummary {
        let top_entries = self.get_rankings(self.config.primary_metric, 10);
        let tier_dist = self.get_tier_distribution();

        LeaderboardSummary {
            competition_id: self.competition_id,
            primary_metric: self.config.primary_metric,
            total_participants: self.performances.len(),
            top_10: top_entries,
            tier_distribution: tier_dist,
            last_updated: Utc::now(),
            snapshot_count: self.snapshots.len(),
        }
    }
}

/// Summary of leaderboard state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardSummary {
    pub competition_id: CompetitionId,
    pub primary_metric: RankingMetric,
    pub total_participants: usize,
    pub top_10: Vec<LeaderboardEntry>,
    pub tier_distribution: HashMap<RankingTier, usize>,
    pub last_updated: DateTime<Utc>,
    pub snapshot_count: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranking_metrics() {
        assert!(RankingMetric::PnL.higher_is_better());
        assert!(!RankingMetric::MaxDrawdown.higher_is_better());
    }

    #[test]
    fn test_ranking_tiers() {
        assert_eq!(RankingTier::from_percentile(99.5), RankingTier::Diamond);
        assert_eq!(RankingTier::from_percentile(96.0), RankingTier::Platinum);
        assert_eq!(RankingTier::from_percentile(90.0), RankingTier::Gold);
        assert_eq!(RankingTier::from_percentile(70.0), RankingTier::Silver);
        assert_eq!(RankingTier::from_percentile(50.0), RankingTier::Bronze);
        assert_eq!(RankingTier::from_percentile(20.0), RankingTier::Iron);
    }

    #[test]
    fn test_agent_performance() {
        let agent_id = AgentId::new();
        let mut perf = AgentPerformance::new(agent_id, "TestBot".to_string(), dec!(10000));

        perf.record_trade(dec!(100), dec!(1000), 60);
        assert_eq!(perf.pnl, dec!(100));
        assert_eq!(perf.trade_count, 1);
        assert_eq!(perf.winning_trades, 1);

        perf.record_trade(dec!(-50), dec!(500), 30);
        assert_eq!(perf.pnl, dec!(50));
        assert_eq!(perf.trade_count, 2);
        assert_eq!(perf.losing_trades, 1);
    }

    #[test]
    fn test_leaderboard_rankings() {
        let competition_id = CompetitionId::new();
        let mut leaderboard = Leaderboard::new(competition_id, LeaderboardConfig::default());

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let agent3 = AgentId::new();

        leaderboard.register_agent(agent1.clone(), "Bot1".to_string(), dec!(10000));
        leaderboard.register_agent(agent2.clone(), "Bot2".to_string(), dec!(10000));
        leaderboard.register_agent(agent3.clone(), "Bot3".to_string(), dec!(10000));

        leaderboard.record_trade(&agent1, dec!(500), dec!(5000), 60);
        leaderboard.record_trade(&agent2, dec!(1000), dec!(10000), 120);
        leaderboard.record_trade(&agent3, dec!(300), dec!(3000), 45);

        let rankings = leaderboard.get_rankings(RankingMetric::PnL, 10);

        assert_eq!(rankings.len(), 3);
        assert_eq!(rankings[0].performance.agent_id, agent2); // Highest PnL
        assert_eq!(rankings[1].performance.agent_id, agent1);
        assert_eq!(rankings[2].performance.agent_id, agent3); // Lowest PnL
    }

    #[test]
    fn test_snapshot() {
        let competition_id = CompetitionId::new();
        let mut leaderboard = Leaderboard::new(competition_id, LeaderboardConfig::default());

        let agent = AgentId::new();
        leaderboard.register_agent(agent.clone(), "TestBot".to_string(), dec!(10000));
        leaderboard.record_trade(&agent, dec!(100), dec!(1000), 60);

        let snapshot = leaderboard.take_snapshot();
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.total_participants, 1);
    }

    #[test]
    fn test_decimal_sqrt() {
        let result = decimal_sqrt(dec!(4)).unwrap();
        assert!((result - dec!(2)).abs() < dec!(0.0001));

        let result = decimal_sqrt(dec!(9)).unwrap();
        assert!((result - dec!(3)).abs() < dec!(0.0001));
    }
}
