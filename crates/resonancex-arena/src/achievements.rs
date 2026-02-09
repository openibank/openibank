//! # ResonanceX Arena Achievements System
//!
//! A comprehensive achievement and badge system for gamifying trading competitions.
//!
//! ## Features
//!
//! - **Badge Definitions**: Pre-defined and custom achievement badges
//! - **Rarity Tiers**: Common, Rare, Epic, Legendary, and Mythic tiers
//! - **Progress Tracking**: Track achievement progress with milestones
//! - **Unlock Conditions**: Flexible condition system for unlocking achievements
//! - **Display Integration**: Badges displayable on profiles and leaderboards
//!
//! ## Badge Categories
//!
//! - **Milestones**: First Trade, 100 Trades, 1000 Trades, etc.
//! - **Performance**: Profit milestones, win streaks, Sharpe achievements
//! - **Consistency**: Daily trading, monthly profits, low drawdown
//! - **Competition**: Tournament wins, rankings, participation
//! - **Special**: Seasonal events, limited-time achievements
//!
//! ## Example
//!
//! ```ignore
//! use resonancex_arena::achievements::{AchievementTracker, AchievementEvent};
//!
//! let mut tracker = AchievementTracker::new(agent_id);
//!
//! // Process trading events
//! let unlocked = tracker.process_event(AchievementEvent::TradeCompleted {
//!     pnl: dec!(100),
//!     volume: dec!(1000),
//!     is_win: true,
//! });
//!
//! for badge in unlocked {
//!     println!("Unlocked: {} ({})", badge.name, badge.rarity);
//! }
//! ```

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::CompetitionId;
use openibank_types::AgentId;

// ============================================================================
// Achievement Identifiers
// ============================================================================

/// Unique achievement identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AchievementId(pub String);

impl AchievementId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AchievementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Rarity Tiers
// ============================================================================

/// Achievement rarity tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    /// Most common achievements (50%+ of players can earn)
    Common,
    /// Uncommon achievements (20-50% of players)
    Uncommon,
    /// Rare achievements (5-20% of players)
    Rare,
    /// Epic achievements (1-5% of players)
    Epic,
    /// Legendary achievements (<1% of players)
    Legendary,
    /// Mythic achievements (exceptionally rare, special events)
    Mythic,
}

impl Rarity {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
            Self::Mythic => "Mythic",
        }
    }

    /// Get color for display
    pub fn color(&self) -> &'static str {
        match self {
            Self::Common => "#9CA3AF",    // Gray
            Self::Uncommon => "#10B981",  // Green
            Self::Rare => "#3B82F6",      // Blue
            Self::Epic => "#8B5CF6",      // Purple
            Self::Legendary => "#F59E0B", // Orange
            Self::Mythic => "#EC4899",    // Pink
        }
    }

    /// Get XP reward multiplier
    pub fn xp_multiplier(&self) -> u32 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 5,
            Self::Epic => 10,
            Self::Legendary => 25,
            Self::Mythic => 100,
        }
    }

    /// Get base points for this rarity
    pub fn base_points(&self) -> u32 {
        match self {
            Self::Common => 10,
            Self::Uncommon => 25,
            Self::Rare => 50,
            Self::Epic => 100,
            Self::Legendary => 250,
            Self::Mythic => 1000,
        }
    }
}

// ============================================================================
// Achievement Categories
// ============================================================================

/// Categories for organizing achievements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementCategory {
    /// Trading milestones (first trade, trade counts)
    Milestones,
    /// Profit-related achievements
    Profits,
    /// Win streaks and consistency
    Streaks,
    /// Risk management achievements
    RiskManagement,
    /// Volume and activity
    Volume,
    /// Competition-specific
    Competition,
    /// Time-based (daily login, trading days)
    Dedication,
    /// Social and community
    Social,
    /// Seasonal and limited-time
    Seasonal,
    /// Hidden/secret achievements
    Secret,
}

impl AchievementCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Milestones => "Milestones",
            Self::Profits => "Profits",
            Self::Streaks => "Streaks",
            Self::RiskManagement => "Risk Management",
            Self::Volume => "Volume",
            Self::Competition => "Competition",
            Self::Dedication => "Dedication",
            Self::Social => "Social",
            Self::Seasonal => "Seasonal",
            Self::Secret => "Secret",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Milestones => "*",
            Self::Profits => "$",
            Self::Streaks => "~",
            Self::RiskManagement => "#",
            Self::Volume => "^",
            Self::Competition => "!",
            Self::Dedication => "@",
            Self::Social => "&",
            Self::Seasonal => "%",
            Self::Secret => "?",
        }
    }
}

// ============================================================================
// Achievement Definition
// ============================================================================

/// Definition of an achievement/badge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier
    pub id: AchievementId,
    /// Display name
    pub name: String,
    /// Description of how to earn
    pub description: String,
    /// Flavor text shown after earning
    pub flavor_text: Option<String>,
    /// Category
    pub category: AchievementCategory,
    /// Rarity tier
    pub rarity: Rarity,
    /// Icon identifier or URL
    pub icon: String,
    /// Unlock conditions
    pub conditions: Vec<UnlockCondition>,
    /// Is this achievement hidden until unlocked?
    pub is_hidden: bool,
    /// Is this achievement earnable multiple times?
    pub is_repeatable: bool,
    /// Cooldown between repeats (if repeatable)
    pub repeat_cooldown_hours: Option<u32>,
    /// Points awarded
    pub points: u32,
    /// XP reward
    pub xp_reward: u32,
    /// Prerequisites (other achievements that must be unlocked first)
    pub prerequisites: Vec<AchievementId>,
    /// Expiration (for limited-time achievements)
    pub expires_at: Option<DateTime<Utc>>,
    /// When this achievement was added
    pub added_at: DateTime<Utc>,
}

impl Achievement {
    /// Create a new achievement
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: AchievementCategory,
        rarity: Rarity,
    ) -> Self {
        let rarity_copy = rarity;
        Self {
            id: AchievementId::new(id),
            name: name.into(),
            description: description.into(),
            flavor_text: None,
            category,
            rarity,
            icon: "default_badge".to_string(),
            conditions: Vec::new(),
            is_hidden: false,
            is_repeatable: false,
            repeat_cooldown_hours: None,
            points: rarity_copy.base_points(),
            xp_reward: rarity_copy.base_points() * rarity_copy.xp_multiplier(),
            prerequisites: Vec::new(),
            expires_at: None,
            added_at: Utc::now(),
        }
    }

    /// Builder: set icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Builder: set conditions
    pub fn with_conditions(mut self, conditions: Vec<UnlockCondition>) -> Self {
        self.conditions = conditions;
        self
    }

    /// Builder: add a single condition
    pub fn with_condition(mut self, condition: UnlockCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Builder: set hidden
    pub fn hidden(mut self) -> Self {
        self.is_hidden = true;
        self
    }

    /// Builder: set repeatable
    pub fn repeatable(mut self, cooldown_hours: u32) -> Self {
        self.is_repeatable = true;
        self.repeat_cooldown_hours = Some(cooldown_hours);
        self
    }

    /// Builder: set flavor text
    pub fn with_flavor(mut self, text: impl Into<String>) -> Self {
        self.flavor_text = Some(text.into());
        self
    }

    /// Builder: set prerequisites
    pub fn requires(mut self, prerequisites: Vec<AchievementId>) -> Self {
        self.prerequisites = prerequisites;
        self
    }

    /// Builder: set expiration
    pub fn expires(mut self, at: DateTime<Utc>) -> Self {
        self.expires_at = Some(at);
        self
    }

    /// Check if achievement is currently earnable
    pub fn is_active(&self) -> bool {
        match self.expires_at {
            Some(exp) => Utc::now() < exp,
            None => true,
        }
    }
}

// ============================================================================
// Unlock Conditions
// ============================================================================

/// Conditions that must be met to unlock an achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnlockCondition {
    /// Complete N trades total
    TradeCount { min: u64 },
    /// Complete N trades in a single day
    DailyTradeCount { min: u64 },
    /// Achieve total PnL
    TotalPnL { min: Decimal },
    /// Achieve PnL in a single trade
    SingleTradePnL { min: Decimal },
    /// Achieve PnL in a single day
    DailyPnL { min: Decimal },
    /// Win streak of N trades
    WinStreak { min: u32 },
    /// Achieve win rate
    WinRate { min: Decimal },
    /// Trade for N consecutive days
    ConsecutiveTradingDays { min: u32 },
    /// Total trading volume
    TotalVolume { min: Decimal },
    /// Volume in a single day
    DailyVolume { min: Decimal },
    /// Sharpe ratio
    SharpeRatio { min: Decimal },
    /// Maximum drawdown below threshold
    MaxDrawdownBelow { max: Decimal },
    /// Profit factor above threshold
    ProfitFactorAbove { min: Decimal },
    /// Participate in N competitions
    CompetitionParticipation { min: u32 },
    /// Win N competitions
    CompetitionWins { min: u32 },
    /// Finish in top N of a competition
    CompetitionPlacement { max_rank: usize },
    /// Hold position for N seconds
    PositionDuration { min_secs: u64 },
    /// Execute trade within N milliseconds
    TradeSpeed { max_ms: u64 },
    /// Account age in days
    AccountAge { min_days: u32 },
    /// Specific date requirement (for seasonal)
    DateRange { start: DateTime<Utc>, end: DateTime<Utc> },
    /// Custom condition with expression
    Custom { expression: String, description: String },
    /// All conditions must be met
    All { conditions: Vec<UnlockCondition> },
    /// Any condition must be met
    Any { conditions: Vec<UnlockCondition> },
}

// ============================================================================
// Achievement Progress
// ============================================================================

/// Progress towards unlocking an achievement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementProgress {
    /// Achievement ID
    pub achievement_id: AchievementId,
    /// Current progress values for each condition
    pub progress: HashMap<usize, ProgressValue>,
    /// Percentage complete (0-100)
    pub percent_complete: f64,
    /// Whether fully unlocked
    pub is_complete: bool,
    /// Times unlocked (for repeatable achievements)
    pub unlock_count: u32,
    /// Last unlock time
    pub last_unlocked: Option<DateTime<Utc>>,
    /// First started progressing
    pub started_at: DateTime<Utc>,
}

/// Progress value for a single condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressValue {
    /// Current value
    pub current: Decimal,
    /// Required value
    pub required: Decimal,
    /// Is this condition met?
    pub is_met: bool,
}

impl AchievementProgress {
    /// Create new progress tracker
    pub fn new(achievement: &Achievement) -> Self {
        let mut progress = HashMap::new();

        for (i, condition) in achievement.conditions.iter().enumerate() {
            let required = condition.target_value();
            progress.insert(i, ProgressValue {
                current: Decimal::ZERO,
                required,
                is_met: false,
            });
        }

        Self {
            achievement_id: achievement.id.clone(),
            progress,
            percent_complete: 0.0,
            is_complete: false,
            unlock_count: 0,
            last_unlocked: None,
            started_at: Utc::now(),
        }
    }

    /// Update progress for a condition
    pub fn update(&mut self, condition_index: usize, value: Decimal) {
        if let Some(prog) = self.progress.get_mut(&condition_index) {
            prog.current = value;
            prog.is_met = value >= prog.required;
        }
        self.recalculate_completion();
    }

    /// Increment progress for a condition
    pub fn increment(&mut self, condition_index: usize, delta: Decimal) {
        if let Some(prog) = self.progress.get_mut(&condition_index) {
            prog.current += delta;
            prog.is_met = prog.current >= prog.required;
        }
        self.recalculate_completion();
    }

    fn recalculate_completion(&mut self) {
        if self.progress.is_empty() {
            self.percent_complete = 100.0;
            self.is_complete = true;
            return;
        }

        let total_percent: f64 = self.progress.values()
            .map(|p| {
                if p.required.is_zero() {
                    100.0
                } else {
                    let ratio = p.current / p.required;
                    (ratio.to_string().parse::<f64>().unwrap_or(0.0) * 100.0).min(100.0)
                }
            })
            .sum();

        self.percent_complete = total_percent / self.progress.len() as f64;
        self.is_complete = self.progress.values().all(|p| p.is_met);
    }

    /// Mark as unlocked
    pub fn unlock(&mut self) {
        self.is_complete = true;
        self.percent_complete = 100.0;
        self.unlock_count += 1;
        self.last_unlocked = Some(Utc::now());
    }
}

impl UnlockCondition {
    /// Get the target value for this condition
    pub fn target_value(&self) -> Decimal {
        match self {
            Self::TradeCount { min } => Decimal::from(*min),
            Self::DailyTradeCount { min } => Decimal::from(*min),
            Self::TotalPnL { min } => *min,
            Self::SingleTradePnL { min } => *min,
            Self::DailyPnL { min } => *min,
            Self::WinStreak { min } => Decimal::from(*min),
            Self::WinRate { min } => *min,
            Self::ConsecutiveTradingDays { min } => Decimal::from(*min),
            Self::TotalVolume { min } => *min,
            Self::DailyVolume { min } => *min,
            Self::SharpeRatio { min } => *min,
            Self::MaxDrawdownBelow { max } => *max,
            Self::ProfitFactorAbove { min } => *min,
            Self::CompetitionParticipation { min } => Decimal::from(*min),
            Self::CompetitionWins { min } => Decimal::from(*min),
            Self::CompetitionPlacement { max_rank } => Decimal::from(*max_rank as u64),
            Self::PositionDuration { min_secs } => Decimal::from(*min_secs),
            Self::TradeSpeed { max_ms } => Decimal::from(*max_ms),
            Self::AccountAge { min_days } => Decimal::from(*min_days),
            Self::DateRange { .. } => Decimal::ONE,
            Self::Custom { .. } => Decimal::ONE,
            Self::All { conditions } => Decimal::from(conditions.len() as u64),
            Self::Any { .. } => Decimal::ONE,
        }
    }
}

// ============================================================================
// Badge Display
// ============================================================================

/// Badge information for display purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeDisplay {
    /// Achievement ID
    pub id: AchievementId,
    /// Badge name
    pub name: String,
    /// Icon
    pub icon: String,
    /// Rarity
    pub rarity: Rarity,
    /// When earned
    pub earned_at: DateTime<Utc>,
    /// Is this a featured badge
    pub is_featured: bool,
}

impl BadgeDisplay {
    pub fn from_achievement(achievement: &Achievement, earned_at: DateTime<Utc>) -> Self {
        Self {
            id: achievement.id.clone(),
            name: achievement.name.clone(),
            icon: achievement.icon.clone(),
            rarity: achievement.rarity,
            earned_at,
            is_featured: false,
        }
    }
}

// ============================================================================
// Achievement Events
// ============================================================================

/// Events that can trigger achievement progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AchievementEvent {
    /// A trade was completed
    TradeCompleted {
        pnl: Decimal,
        volume: Decimal,
        duration_secs: u64,
        is_win: bool,
    },
    /// Daily stats updated
    DailyStats {
        trade_count: u64,
        pnl: Decimal,
        volume: Decimal,
        trading_day_streak: u32,
    },
    /// Overall stats updated
    OverallStats {
        total_trades: u64,
        total_pnl: Decimal,
        total_volume: Decimal,
        win_rate: Decimal,
        sharpe_ratio: Option<Decimal>,
        max_drawdown: Decimal,
        profit_factor: Option<Decimal>,
        win_streak: u32,
        account_age_days: u32,
    },
    /// Competition event
    CompetitionEvent {
        competition_id: CompetitionId,
        event_type: CompetitionEventType,
    },
    /// Social event
    SocialEvent {
        event_type: SocialEventType,
    },
    /// Custom event
    Custom {
        event_name: String,
        data: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompetitionEventType {
    Registered,
    Completed { rank: usize, total_participants: usize },
    Won,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SocialEventType {
    ProfileViewed { count: u64 },
    FollowerGained { total: u64 },
    Referred { count: u64 },
}

// ============================================================================
// Achievement Tracker
// ============================================================================

/// Tracks achievements for a single agent
#[derive(Debug, Clone)]
pub struct AchievementTracker {
    /// Agent ID
    pub agent_id: AgentId,
    /// All available achievements
    achievements: HashMap<AchievementId, Achievement>,
    /// Progress for each achievement
    progress: HashMap<AchievementId, AchievementProgress>,
    /// Unlocked achievement IDs
    unlocked: HashSet<AchievementId>,
    /// Featured badges (for profile display)
    featured_badges: Vec<AchievementId>,
    /// Total points earned
    pub total_points: u32,
    /// Total XP earned
    pub total_xp: u32,
    /// Lifetime stats for tracking
    stats: AgentAchievementStats,
}

/// Stats used for achievement tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentAchievementStats {
    pub total_trades: u64,
    pub total_pnl: Decimal,
    pub total_volume: Decimal,
    pub win_rate: Decimal,
    pub current_win_streak: u32,
    pub longest_win_streak: u32,
    pub sharpe_ratio: Option<Decimal>,
    pub max_drawdown: Decimal,
    pub profit_factor: Option<Decimal>,
    pub trading_day_streak: u32,
    pub longest_trading_day_streak: u32,
    pub competitions_entered: u32,
    pub competitions_won: u32,
    pub best_placement: Option<usize>,
    pub account_age_days: u32,
    pub daily_pnl: Decimal,
    pub daily_trades: u64,
    pub daily_volume: Decimal,
    pub last_trade_date: Option<DateTime<Utc>>,
}

impl AchievementTracker {
    /// Create a new tracker with default achievements
    pub fn new(agent_id: AgentId) -> Self {
        let mut tracker = Self {
            agent_id,
            achievements: HashMap::new(),
            progress: HashMap::new(),
            unlocked: HashSet::new(),
            featured_badges: Vec::new(),
            total_points: 0,
            total_xp: 0,
            stats: AgentAchievementStats::default(),
        };

        // Register all default achievements
        for achievement in Self::default_achievements() {
            tracker.register_achievement(achievement);
        }

        tracker
    }

    /// Register an achievement
    pub fn register_achievement(&mut self, achievement: Achievement) {
        let progress = AchievementProgress::new(&achievement);
        self.progress.insert(achievement.id.clone(), progress);
        self.achievements.insert(achievement.id.clone(), achievement);
    }

    /// Process an event and check for unlocks
    pub fn process_event(&mut self, event: AchievementEvent) -> Vec<BadgeDisplay> {
        // Update stats based on event
        self.update_stats(&event);

        // Check all achievements
        let mut newly_unlocked = Vec::new();

        let achievement_ids: Vec<AchievementId> = self.achievements.keys().cloned().collect();

        for id in achievement_ids {
            if self.unlocked.contains(&id) {
                let achievement = self.achievements.get(&id).unwrap();
                if !achievement.is_repeatable {
                    continue;
                }

                // Check cooldown for repeatable
                if let Some(hours) = achievement.repeat_cooldown_hours {
                    if let Some(progress) = self.progress.get(&id) {
                        if let Some(last) = progress.last_unlocked {
                            let elapsed = Utc::now() - last;
                            if elapsed.num_hours() < hours as i64 {
                                continue;
                            }
                        }
                    }
                }
            }

            if self.check_achievement(&id) {
                if let Some(badge) = self.unlock_achievement(&id) {
                    newly_unlocked.push(badge);
                }
            }
        }

        newly_unlocked
    }

    /// Update internal stats based on event
    fn update_stats(&mut self, event: &AchievementEvent) {
        match event {
            AchievementEvent::TradeCompleted { pnl, volume, is_win, .. } => {
                self.stats.total_trades += 1;
                self.stats.total_pnl += *pnl;
                self.stats.total_volume += *volume;
                self.stats.daily_trades += 1;
                self.stats.daily_pnl += *pnl;
                self.stats.daily_volume += *volume;

                if *is_win {
                    self.stats.current_win_streak += 1;
                    if self.stats.current_win_streak > self.stats.longest_win_streak {
                        self.stats.longest_win_streak = self.stats.current_win_streak;
                    }
                } else {
                    self.stats.current_win_streak = 0;
                }

                self.stats.last_trade_date = Some(Utc::now());
            }
            AchievementEvent::DailyStats { trade_count, pnl, volume, trading_day_streak } => {
                self.stats.daily_trades = *trade_count;
                self.stats.daily_pnl = *pnl;
                self.stats.daily_volume = *volume;
                self.stats.trading_day_streak = *trading_day_streak;
                if *trading_day_streak > self.stats.longest_trading_day_streak {
                    self.stats.longest_trading_day_streak = *trading_day_streak;
                }
            }
            AchievementEvent::OverallStats {
                total_trades, total_pnl, total_volume, win_rate,
                sharpe_ratio, max_drawdown, profit_factor, win_streak, account_age_days,
            } => {
                self.stats.total_trades = *total_trades;
                self.stats.total_pnl = *total_pnl;
                self.stats.total_volume = *total_volume;
                self.stats.win_rate = *win_rate;
                self.stats.sharpe_ratio = *sharpe_ratio;
                self.stats.max_drawdown = *max_drawdown;
                self.stats.profit_factor = *profit_factor;
                self.stats.longest_win_streak = *win_streak;
                self.stats.account_age_days = *account_age_days;
            }
            AchievementEvent::CompetitionEvent { event_type, .. } => {
                match event_type {
                    CompetitionEventType::Registered => {
                        self.stats.competitions_entered += 1;
                    }
                    CompetitionEventType::Won => {
                        self.stats.competitions_won += 1;
                    }
                    CompetitionEventType::Completed { rank, .. } => {
                        match self.stats.best_placement {
                            Some(best) if *rank < best => {
                                self.stats.best_placement = Some(*rank);
                            }
                            None => {
                                self.stats.best_placement = Some(*rank);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Check if an achievement should be unlocked
    fn check_achievement(&self, id: &AchievementId) -> bool {
        let achievement = match self.achievements.get(id) {
            Some(a) => a,
            None => return false,
        };

        // Check if active
        if !achievement.is_active() {
            return false;
        }

        // Check prerequisites
        for prereq in &achievement.prerequisites {
            if !self.unlocked.contains(prereq) {
                return false;
            }
        }

        // Check all conditions
        achievement.conditions.iter().all(|c| self.check_condition(c))
    }

    /// Check a single condition
    fn check_condition(&self, condition: &UnlockCondition) -> bool {
        match condition {
            UnlockCondition::TradeCount { min } => self.stats.total_trades >= *min,
            UnlockCondition::DailyTradeCount { min } => self.stats.daily_trades >= *min,
            UnlockCondition::TotalPnL { min } => self.stats.total_pnl >= *min,
            UnlockCondition::SingleTradePnL { min } => self.stats.daily_pnl >= *min, // Simplified
            UnlockCondition::DailyPnL { min } => self.stats.daily_pnl >= *min,
            UnlockCondition::WinStreak { min } => self.stats.longest_win_streak >= *min,
            UnlockCondition::WinRate { min } => self.stats.win_rate >= *min,
            UnlockCondition::ConsecutiveTradingDays { min } => {
                self.stats.longest_trading_day_streak >= *min
            }
            UnlockCondition::TotalVolume { min } => self.stats.total_volume >= *min,
            UnlockCondition::DailyVolume { min } => self.stats.daily_volume >= *min,
            UnlockCondition::SharpeRatio { min } => {
                self.stats.sharpe_ratio.map(|s| s >= *min).unwrap_or(false)
            }
            UnlockCondition::MaxDrawdownBelow { max } => self.stats.max_drawdown <= *max,
            UnlockCondition::ProfitFactorAbove { min } => {
                self.stats.profit_factor.map(|p| p >= *min).unwrap_or(false)
            }
            UnlockCondition::CompetitionParticipation { min } => {
                self.stats.competitions_entered >= *min
            }
            UnlockCondition::CompetitionWins { min } => self.stats.competitions_won >= *min,
            UnlockCondition::CompetitionPlacement { max_rank } => {
                self.stats.best_placement.map(|r| r <= *max_rank).unwrap_or(false)
            }
            UnlockCondition::AccountAge { min_days } => self.stats.account_age_days >= *min_days,
            UnlockCondition::DateRange { start, end } => {
                let now = Utc::now();
                now >= *start && now <= *end
            }
            UnlockCondition::All { conditions } => {
                conditions.iter().all(|c| self.check_condition(c))
            }
            UnlockCondition::Any { conditions } => {
                conditions.iter().any(|c| self.check_condition(c))
            }
            _ => false,
        }
    }

    /// Unlock an achievement
    fn unlock_achievement(&mut self, id: &AchievementId) -> Option<BadgeDisplay> {
        let achievement = self.achievements.get(id)?;
        let now = Utc::now();

        // Update progress
        if let Some(progress) = self.progress.get_mut(id) {
            progress.unlock();
        }

        // Add to unlocked
        self.unlocked.insert(id.clone());

        // Award points and XP
        self.total_points += achievement.points;
        self.total_xp += achievement.xp_reward;

        Some(BadgeDisplay::from_achievement(achievement, now))
    }

    /// Get all unlocked badges
    pub fn get_unlocked_badges(&self) -> Vec<BadgeDisplay> {
        self.unlocked
            .iter()
            .filter_map(|id| {
                let achievement = self.achievements.get(id)?;
                let progress = self.progress.get(id)?;
                Some(BadgeDisplay::from_achievement(
                    achievement,
                    progress.last_unlocked.unwrap_or(Utc::now()),
                ))
            })
            .collect()
    }

    /// Get progress for all achievements
    pub fn get_all_progress(&self) -> Vec<(&Achievement, &AchievementProgress)> {
        self.achievements
            .iter()
            .filter_map(|(id, achievement)| {
                let progress = self.progress.get(id)?;
                Some((achievement, progress))
            })
            .collect()
    }

    /// Get progress for achievements in a category
    pub fn get_category_progress(
        &self,
        category: AchievementCategory,
    ) -> Vec<(&Achievement, &AchievementProgress)> {
        self.get_all_progress()
            .into_iter()
            .filter(|(a, _)| a.category == category)
            .collect()
    }

    /// Set featured badges
    pub fn set_featured_badges(&mut self, badges: Vec<AchievementId>) {
        self.featured_badges = badges
            .into_iter()
            .filter(|id| self.unlocked.contains(id))
            .take(5)
            .collect();
    }

    /// Get featured badges for display
    pub fn get_featured_badges(&self) -> Vec<BadgeDisplay> {
        self.featured_badges
            .iter()
            .filter_map(|id| {
                let achievement = self.achievements.get(id)?;
                let progress = self.progress.get(id)?;
                let mut badge = BadgeDisplay::from_achievement(
                    achievement,
                    progress.last_unlocked.unwrap_or(Utc::now()),
                );
                badge.is_featured = true;
                Some(badge)
            })
            .collect()
    }

    /// Get achievement by ID
    pub fn get_achievement(&self, id: &AchievementId) -> Option<&Achievement> {
        self.achievements.get(id)
    }

    /// Check if achievement is unlocked
    pub fn is_unlocked(&self, id: &AchievementId) -> bool {
        self.unlocked.contains(id)
    }

    /// Get stats snapshot
    pub fn get_stats(&self) -> &AgentAchievementStats {
        &self.stats
    }

    /// Default achievements
    fn default_achievements() -> Vec<Achievement> {
        vec![
            // ========== MILESTONES ==========
            Achievement::new(
                "first_trade",
                "First Steps",
                "Complete your first trade",
                AchievementCategory::Milestones,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::TradeCount { min: 1 })
            .with_flavor("Every journey begins with a single trade.".to_string())
            .with_icon("rocket"),

            Achievement::new(
                "hundred_trades",
                "Centurion",
                "Complete 100 trades",
                AchievementCategory::Milestones,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::TradeCount { min: 100 })
            .with_icon("badge_100"),

            Achievement::new(
                "thousand_trades",
                "Trading Machine",
                "Complete 1,000 trades",
                AchievementCategory::Milestones,
                Rarity::Uncommon,
            )
            .with_condition(UnlockCondition::TradeCount { min: 1000 })
            .with_icon("machine"),

            Achievement::new(
                "ten_thousand_trades",
                "Market Veteran",
                "Complete 10,000 trades",
                AchievementCategory::Milestones,
                Rarity::Rare,
            )
            .with_condition(UnlockCondition::TradeCount { min: 10000 })
            .with_icon("veteran_medal"),

            Achievement::new(
                "hundred_thousand_trades",
                "Trading Legend",
                "Complete 100,000 trades",
                AchievementCategory::Milestones,
                Rarity::Legendary,
            )
            .with_condition(UnlockCondition::TradeCount { min: 100000 })
            .with_flavor("They say you can see the matrix now.".to_string())
            .with_icon("legend_crown"),

            // ========== PROFITS ==========
            Achievement::new(
                "first_profit",
                "In The Green",
                "Make your first profitable trade",
                AchievementCategory::Profits,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::TotalPnL { min: dec!(0.01) })
            .with_icon("green_arrow"),

            Achievement::new(
                "thousand_dollar_club",
                "Four Figures",
                "Accumulate $1,000 in total profits",
                AchievementCategory::Profits,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::TotalPnL { min: dec!(1000) })
            .with_icon("money_1k"),

            Achievement::new(
                "ten_thousand_club",
                "Five Figures",
                "Accumulate $10,000 in total profits",
                AchievementCategory::Profits,
                Rarity::Uncommon,
            )
            .with_condition(UnlockCondition::TotalPnL { min: dec!(10000) })
            .with_icon("money_10k"),

            Achievement::new(
                "hundred_thousand_club",
                "Six Figures",
                "Accumulate $100,000 in total profits",
                AchievementCategory::Profits,
                Rarity::Rare,
            )
            .with_condition(UnlockCondition::TotalPnL { min: dec!(100000) })
            .with_icon("money_100k"),

            Achievement::new(
                "million_dollar_club",
                "Million Dollar Club",
                "Accumulate $1,000,000 in total profits",
                AchievementCategory::Profits,
                Rarity::Epic,
            )
            .with_condition(UnlockCondition::TotalPnL { min: dec!(1000000) })
            .with_flavor("Welcome to the club. Champagne is in the back.".to_string())
            .with_icon("diamond"),

            Achievement::new(
                "ten_million_club",
                "Eight Figure Elite",
                "Accumulate $10,000,000 in total profits",
                AchievementCategory::Profits,
                Rarity::Legendary,
            )
            .with_condition(UnlockCondition::TotalPnL { min: dec!(10000000) })
            .with_icon("trophy_gold"),

            // ========== STREAKS ==========
            Achievement::new(
                "win_streak_5",
                "On Fire",
                "Win 5 trades in a row",
                AchievementCategory::Streaks,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::WinStreak { min: 5 })
            .with_icon("fire"),

            Achievement::new(
                "win_streak_10",
                "Unstoppable",
                "Win 10 trades in a row",
                AchievementCategory::Streaks,
                Rarity::Uncommon,
            )
            .with_condition(UnlockCondition::WinStreak { min: 10 })
            .with_icon("lightning"),

            Achievement::new(
                "win_streak_25",
                "Domination",
                "Win 25 trades in a row",
                AchievementCategory::Streaks,
                Rarity::Rare,
            )
            .with_condition(UnlockCondition::WinStreak { min: 25 })
            .with_icon("crown"),

            Achievement::new(
                "win_streak_50",
                "Perfection Pursuit",
                "Win 50 trades in a row",
                AchievementCategory::Streaks,
                Rarity::Epic,
            )
            .with_condition(UnlockCondition::WinStreak { min: 50 })
            .with_icon("star_gold"),

            Achievement::new(
                "win_streak_100",
                "100 Win Streak",
                "Win 100 trades in a row",
                AchievementCategory::Streaks,
                Rarity::Legendary,
            )
            .with_condition(UnlockCondition::WinStreak { min: 100 })
            .with_flavor("Is this even statistically possible?".to_string())
            .with_icon("infinity"),

            // ========== RISK MANAGEMENT ==========
            Achievement::new(
                "low_drawdown",
                "Steady Hands",
                "Maintain maximum drawdown below 5%",
                AchievementCategory::RiskManagement,
                Rarity::Uncommon,
            )
            .with_conditions(vec![
                UnlockCondition::MaxDrawdownBelow { max: dec!(5) },
                UnlockCondition::TradeCount { min: 100 },
            ])
            .with_icon("shield"),

            Achievement::new(
                "profit_factor_master",
                "Profit Factor Master",
                "Achieve a profit factor above 3.0",
                AchievementCategory::RiskManagement,
                Rarity::Rare,
            )
            .with_conditions(vec![
                UnlockCondition::ProfitFactorAbove { min: dec!(3) },
                UnlockCondition::TradeCount { min: 100 },
            ])
            .with_icon("chart_up"),

            Achievement::new(
                "sharpe_elite",
                "Sharpe Elite",
                "Achieve a Sharpe ratio above 2.0",
                AchievementCategory::RiskManagement,
                Rarity::Epic,
            )
            .with_conditions(vec![
                UnlockCondition::SharpeRatio { min: dec!(2) },
                UnlockCondition::TradeCount { min: 100 },
            ])
            .with_icon("brain"),

            Achievement::new(
                "sharpe_legendary",
                "Sharpe Legendary",
                "Achieve a Sharpe ratio above 3.0",
                AchievementCategory::RiskManagement,
                Rarity::Legendary,
            )
            .with_conditions(vec![
                UnlockCondition::SharpeRatio { min: dec!(3) },
                UnlockCondition::TradeCount { min: 200 },
            ])
            .with_flavor("Risk-adjusted returns that make quants weep.".to_string())
            .with_icon("unicorn"),

            // ========== VOLUME ==========
            Achievement::new(
                "volume_million",
                "Volume Warrior",
                "Trade $1,000,000 in total volume",
                AchievementCategory::Volume,
                Rarity::Uncommon,
            )
            .with_condition(UnlockCondition::TotalVolume { min: dec!(1000000) })
            .with_icon("volume_up"),

            Achievement::new(
                "volume_billion",
                "Whale",
                "Trade $1,000,000,000 in total volume",
                AchievementCategory::Volume,
                Rarity::Epic,
            )
            .with_condition(UnlockCondition::TotalVolume { min: dec!(1000000000) })
            .with_flavor("The market moves when you move.".to_string())
            .with_icon("whale"),

            // ========== COMPETITION ==========
            Achievement::new(
                "first_competition",
                "Competitor",
                "Enter your first competition",
                AchievementCategory::Competition,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::CompetitionParticipation { min: 1 })
            .with_icon("flag"),

            Achievement::new(
                "podium_finish",
                "Podium Finish",
                "Finish in the top 3 of a competition",
                AchievementCategory::Competition,
                Rarity::Rare,
            )
            .with_condition(UnlockCondition::CompetitionPlacement { max_rank: 3 })
            .with_icon("podium"),

            Achievement::new(
                "first_win",
                "Champion",
                "Win a competition",
                AchievementCategory::Competition,
                Rarity::Rare,
            )
            .with_condition(UnlockCondition::CompetitionWins { min: 1 })
            .with_icon("trophy"),

            Achievement::new(
                "five_wins",
                "Serial Winner",
                "Win 5 competitions",
                AchievementCategory::Competition,
                Rarity::Epic,
            )
            .with_condition(UnlockCondition::CompetitionWins { min: 5 })
            .with_icon("trophy_collection"),

            Achievement::new(
                "arena_legend",
                "Arena Legend",
                "Win 20 competitions",
                AchievementCategory::Competition,
                Rarity::Legendary,
            )
            .with_condition(UnlockCondition::CompetitionWins { min: 20 })
            .with_flavor("Your name echoes through the halls of trading fame.".to_string())
            .with_icon("legend_statue"),

            // ========== DEDICATION ==========
            Achievement::new(
                "week_streak",
                "Weekly Warrior",
                "Trade for 7 consecutive days",
                AchievementCategory::Dedication,
                Rarity::Common,
            )
            .with_condition(UnlockCondition::ConsecutiveTradingDays { min: 7 })
            .with_icon("calendar_week"),

            Achievement::new(
                "month_streak",
                "Monthly Master",
                "Trade for 30 consecutive days",
                AchievementCategory::Dedication,
                Rarity::Uncommon,
            )
            .with_condition(UnlockCondition::ConsecutiveTradingDays { min: 30 })
            .with_icon("calendar_month"),

            Achievement::new(
                "year_streak",
                "Year-Round Trader",
                "Trade for 365 consecutive days",
                AchievementCategory::Dedication,
                Rarity::Legendary,
            )
            .with_condition(UnlockCondition::ConsecutiveTradingDays { min: 365 })
            .with_flavor("Sleep is overrated anyway.".to_string())
            .with_icon("calendar_year"),

            // ========== SECRET ==========
            Achievement::new(
                "comeback_king",
                "Comeback King",
                "Recover from a 50% drawdown to achieve new highs",
                AchievementCategory::Secret,
                Rarity::Epic,
            )
            .hidden()
            .with_conditions(vec![
                UnlockCondition::MaxDrawdownBelow { max: dec!(50) },
                UnlockCondition::TotalPnL { min: dec!(0) },
            ])
            .with_flavor("What doesn't kill your portfolio makes it stronger.".to_string())
            .with_icon("phoenix"),

            Achievement::new(
                "night_owl",
                "Night Owl",
                "Execute 1000 trades between midnight and 4 AM",
                AchievementCategory::Secret,
                Rarity::Rare,
            )
            .hidden()
            .with_condition(UnlockCondition::TradeCount { min: 1000 })
            .with_flavor("The markets never sleep, and neither do you.".to_string())
            .with_icon("owl"),
        ]
    }
}

// ============================================================================
// Achievement Registry
// ============================================================================

/// Global registry of all available achievements
#[derive(Debug, Default)]
pub struct AchievementRegistry {
    achievements: HashMap<AchievementId, Achievement>,
}

impl AchievementRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();

        for achievement in AchievementTracker::default_achievements() {
            registry.register(achievement);
        }

        registry
    }

    /// Register a new achievement
    pub fn register(&mut self, achievement: Achievement) {
        self.achievements.insert(achievement.id.clone(), achievement);
    }

    /// Get an achievement by ID
    pub fn get(&self, id: &AchievementId) -> Option<&Achievement> {
        self.achievements.get(id)
    }

    /// List all achievements
    pub fn list_all(&self) -> Vec<&Achievement> {
        self.achievements.values().collect()
    }

    /// List achievements by category
    pub fn list_by_category(&self, category: AchievementCategory) -> Vec<&Achievement> {
        self.achievements
            .values()
            .filter(|a| a.category == category)
            .collect()
    }

    /// List achievements by rarity
    pub fn list_by_rarity(&self, rarity: Rarity) -> Vec<&Achievement> {
        self.achievements
            .values()
            .filter(|a| a.rarity == rarity)
            .collect()
    }

    /// Create tracker for an agent with all registered achievements
    pub fn create_tracker(&self, agent_id: AgentId) -> AchievementTracker {
        let mut tracker = AchievementTracker {
            agent_id,
            achievements: self.achievements.clone(),
            progress: HashMap::new(),
            unlocked: HashSet::new(),
            featured_badges: Vec::new(),
            total_points: 0,
            total_xp: 0,
            stats: AgentAchievementStats::default(),
        };

        for (id, achievement) in &self.achievements {
            let progress = AchievementProgress::new(achievement);
            tracker.progress.insert(id.clone(), progress);
        }

        tracker
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rarity_ordering() {
        assert!(Rarity::Legendary > Rarity::Epic);
        assert!(Rarity::Epic > Rarity::Rare);
        assert!(Rarity::Rare > Rarity::Uncommon);
        assert!(Rarity::Uncommon > Rarity::Common);
    }

    #[test]
    fn test_achievement_creation() {
        let achievement = Achievement::new(
            "test_achievement",
            "Test Achievement",
            "This is a test",
            AchievementCategory::Milestones,
            Rarity::Rare,
        )
        .with_condition(UnlockCondition::TradeCount { min: 10 });

        assert_eq!(achievement.id.0, "test_achievement");
        assert_eq!(achievement.rarity, Rarity::Rare);
        assert_eq!(achievement.conditions.len(), 1);
    }

    #[test]
    fn test_achievement_tracker() {
        let agent_id = AgentId::new();
        let mut tracker = AchievementTracker::new(agent_id);

        // Process a trade event
        let unlocked = tracker.process_event(AchievementEvent::TradeCompleted {
            pnl: dec!(100),
            volume: dec!(1000),
            duration_secs: 60,
            is_win: true,
        });

        // Should unlock "First Steps" achievement
        assert!(!unlocked.is_empty());
        assert!(tracker.is_unlocked(&AchievementId::new("first_trade")));
    }

    #[test]
    fn test_win_streak_tracking() {
        let agent_id = AgentId::new();
        let mut tracker = AchievementTracker::new(agent_id);

        // Process 5 winning trades
        for _ in 0..5 {
            tracker.process_event(AchievementEvent::TradeCompleted {
                pnl: dec!(10),
                volume: dec!(100),
                duration_secs: 30,
                is_win: true,
            });
        }

        assert_eq!(tracker.stats.current_win_streak, 5);
        assert!(tracker.is_unlocked(&AchievementId::new("win_streak_5")));
    }

    #[test]
    fn test_progress_tracking() {
        let achievement = Achievement::new(
            "test",
            "Test",
            "Test",
            AchievementCategory::Milestones,
            Rarity::Common,
        )
        .with_condition(UnlockCondition::TradeCount { min: 10 });

        let mut progress = AchievementProgress::new(&achievement);
        assert_eq!(progress.percent_complete, 0.0);

        progress.update(0, dec!(5));
        assert_eq!(progress.percent_complete, 50.0);
        assert!(!progress.is_complete);

        progress.update(0, dec!(10));
        assert_eq!(progress.percent_complete, 100.0);
        assert!(progress.is_complete);
    }

    #[test]
    fn test_registry() {
        let registry = AchievementRegistry::new();

        let milestone_achievements = registry.list_by_category(AchievementCategory::Milestones);
        assert!(!milestone_achievements.is_empty());

        let legendary_achievements = registry.list_by_rarity(Rarity::Legendary);
        assert!(!legendary_achievements.is_empty());
    }

    #[test]
    fn test_badge_display() {
        let achievement = Achievement::new(
            "test",
            "Test Badge",
            "A test badge",
            AchievementCategory::Milestones,
            Rarity::Epic,
        );

        let badge = BadgeDisplay::from_achievement(&achievement, Utc::now());
        assert_eq!(badge.name, "Test Badge");
        assert_eq!(badge.rarity, Rarity::Epic);
    }
}
