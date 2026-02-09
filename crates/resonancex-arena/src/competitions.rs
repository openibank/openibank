//! # ResonanceX Arena Competitions
//!
//! Comprehensive competition management system for AI trading agents.
//!
//! ## Competition Types
//!
//! - **PnL Challenge**: Maximize absolute profit and loss
//! - **Sharpe Showdown**: Compete on risk-adjusted returns
//! - **Market Making Marathon**: Best bid-ask spread maintenance over time
//! - **Speed Trading**: Execute profitable trades in the shortest time
//! - **Consistency Cup**: Most consistent positive returns
//! - **Drawdown Duel**: Minimize maximum drawdown while maintaining returns
//!
//! ## Competition Lifecycle
//!
//! 1. **Draft**: Competition being configured
//! 2. **Announced**: Public, accepting registrations
//! 3. **Registration**: Active registration period
//! 4. **Starting**: Registration closed, preparing to start
//! 5. **Running**: Competition is live
//! 6. **Paused**: Temporarily halted
//! 7. **Calculating**: Computing final results
//! 8. **Ended**: Competition complete, results finalized
//! 9. **Cancelled**: Competition was cancelled
//!
//! ## Features
//!
//! - Prize pool management with multiple distribution tiers
//! - Entry requirements (minimum balance, KYC level, history)
//! - Team competitions and solo events
//! - Qualification rounds and finals
//! - Sponsor integration

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::leaderboard::{Leaderboard, LeaderboardConfig, RankingMetric};
use crate::CompetitionId;
use openibank_types::AgentId;
use resonancex_types::MarketId;

// ============================================================================
// Competition Errors
// ============================================================================

/// Errors that can occur in competition operations
#[derive(Debug, Error)]
pub enum CompetitionError {
    #[error("Competition not found: {0}")]
    NotFound(CompetitionId),

    #[error("Competition is not accepting registrations")]
    RegistrationClosed,

    #[error("Competition is full (max {max} participants)")]
    CompetitionFull { max: usize },

    #[error("Agent already registered")]
    AlreadyRegistered,

    #[error("Agent not registered in this competition")]
    NotRegistered,

    #[error("Entry requirements not met: {reason}")]
    RequirementsNotMet { reason: String },

    #[error("Insufficient entry fee: required {required}, provided {provided}")]
    InsufficientEntryFee { required: Decimal, provided: Decimal },

    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: CompetitionState, to: CompetitionState },

    #[error("Competition has not started")]
    NotStarted,

    #[error("Competition has already ended")]
    AlreadyEnded,

    #[error("Competition is paused")]
    Paused,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Prize pool error: {0}")]
    PrizePoolError(String),

    #[error("Team error: {0}")]
    TeamError(String),
}

pub type CompetitionResult<T> = Result<T, CompetitionError>;

// ============================================================================
// Competition Types
// ============================================================================

/// Types of trading competitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompetitionType {
    /// Maximize absolute profit and loss
    PnLChallenge,
    /// Compete on risk-adjusted returns (Sharpe ratio)
    SharpeShowdown,
    /// Best bid-ask spread maintenance over time
    MarketMakingMarathon,
    /// Execute profitable trades in the shortest time
    SpeedTrading,
    /// Most consistent positive returns
    ConsistencyCup,
    /// Minimize maximum drawdown while maintaining returns
    DrawdownDuel,
    /// Highest trading volume
    VolumeVenture,
    /// Custom competition with specified scoring
    Custom,
}

impl CompetitionType {
    /// Get the primary ranking metric for this competition type
    pub fn primary_metric(&self) -> RankingMetric {
        match self {
            Self::PnLChallenge => RankingMetric::PnL,
            Self::SharpeShowdown => RankingMetric::SharpeRatio,
            Self::MarketMakingMarathon => RankingMetric::Volume,
            Self::SpeedTrading => RankingMetric::AvgTradeDuration,
            Self::ConsistencyCup => RankingMetric::ConsistencyScore,
            Self::DrawdownDuel => RankingMetric::MaxDrawdown,
            Self::VolumeVenture => RankingMetric::Volume,
            Self::Custom => RankingMetric::CompositeScore,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PnLChallenge => "PnL Challenge",
            Self::SharpeShowdown => "Sharpe Showdown",
            Self::MarketMakingMarathon => "Market Making Marathon",
            Self::SpeedTrading => "Speed Trading Sprint",
            Self::ConsistencyCup => "Consistency Cup",
            Self::DrawdownDuel => "Drawdown Duel",
            Self::VolumeVenture => "Volume Venture",
            Self::Custom => "Custom Competition",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::PnLChallenge => "Maximize your profit and loss. The agent with the highest absolute returns wins.",
            Self::SharpeShowdown => "Risk-adjusted returns matter. Compete on Sharpe ratio for the best risk/reward balance.",
            Self::MarketMakingMarathon => "Maintain the tightest spreads and highest liquidity over the competition period.",
            Self::SpeedTrading => "Execute profitable trades as fast as possible. Speed and accuracy are key.",
            Self::ConsistencyCup => "Steady wins the race. Most consistent positive returns take the crown.",
            Self::DrawdownDuel => "Keep your drawdowns minimal while generating positive returns.",
            Self::VolumeVenture => "High-frequency traders rejoice. Highest profitable volume wins.",
            Self::Custom => "Custom competition with specialized scoring rules.",
        }
    }

    /// Get recommended duration in hours
    pub fn recommended_duration_hours(&self) -> u64 {
        match self {
            Self::SpeedTrading => 4,
            Self::PnLChallenge => 168, // 1 week
            Self::SharpeShowdown => 168,
            Self::MarketMakingMarathon => 336, // 2 weeks
            Self::ConsistencyCup => 720, // 1 month
            Self::DrawdownDuel => 168,
            Self::VolumeVenture => 72, // 3 days
            Self::Custom => 168,
        }
    }
}

// ============================================================================
// Competition State
// ============================================================================

/// Competition lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompetitionState {
    /// Competition is being configured
    Draft,
    /// Competition is announced, visible to public
    Announced,
    /// Registration period is active
    Registration,
    /// Registration closed, preparing to start
    Starting,
    /// Competition is live
    Running,
    /// Temporarily paused
    Paused,
    /// Computing final results
    Calculating,
    /// Competition is complete
    Ended,
    /// Competition was cancelled
    Cancelled,
}

impl CompetitionState {
    /// Check if state allows new registrations
    pub fn accepts_registrations(&self) -> bool {
        matches!(self, Self::Announced | Self::Registration)
    }

    /// Check if state allows trading
    pub fn allows_trading(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if competition is considered active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }

    /// Check if competition is finished
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Ended | Self::Cancelled)
    }

    /// Get valid transitions from this state
    pub fn valid_transitions(&self) -> Vec<CompetitionState> {
        match self {
            Self::Draft => vec![Self::Announced, Self::Cancelled],
            Self::Announced => vec![Self::Registration, Self::Cancelled],
            Self::Registration => vec![Self::Starting, Self::Cancelled],
            Self::Starting => vec![Self::Running, Self::Cancelled],
            Self::Running => vec![Self::Paused, Self::Calculating, Self::Cancelled],
            Self::Paused => vec![Self::Running, Self::Calculating, Self::Cancelled],
            Self::Calculating => vec![Self::Ended],
            Self::Ended => vec![],
            Self::Cancelled => vec![],
        }
    }

    /// Check if transition to target state is valid
    pub fn can_transition_to(&self, target: CompetitionState) -> bool {
        self.valid_transitions().contains(&target)
    }
}

// ============================================================================
// Entry Requirements
// ============================================================================

/// KYC verification levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum KycLevel {
    /// No verification required
    None,
    /// Basic email verification
    Basic,
    /// Identity verification
    Standard,
    /// Full verification with proof of funds
    Enhanced,
    /// Institutional level verification
    Institutional,
}

impl Default for KycLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Requirements to enter a competition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRequirements {
    /// Minimum balance required
    pub min_balance: Option<Decimal>,
    /// Minimum KYC level required
    pub min_kyc_level: KycLevel,
    /// Minimum number of previous trades
    pub min_trade_count: Option<u64>,
    /// Minimum days since account creation
    pub min_account_age_days: Option<u32>,
    /// Required previous competition placements
    pub required_placements: Vec<RequiredPlacement>,
    /// Must have specific badges/achievements
    pub required_badges: Vec<String>,
    /// Whitelist of allowed agents (empty = all allowed)
    pub whitelist: HashSet<AgentId>,
    /// Blacklist of blocked agents
    pub blacklist: HashSet<AgentId>,
    /// Maximum entries per user/organization
    pub max_entries_per_user: Option<usize>,
    /// Invitation code required
    pub invitation_only: bool,
    /// Geographic restrictions (country codes)
    pub allowed_regions: Option<HashSet<String>>,
    /// Blocked regions
    pub blocked_regions: HashSet<String>,
}

impl Default for EntryRequirements {
    fn default() -> Self {
        Self {
            min_balance: None,
            min_kyc_level: KycLevel::None,
            min_trade_count: None,
            min_account_age_days: None,
            required_placements: Vec::new(),
            required_badges: Vec::new(),
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            max_entries_per_user: None,
            invitation_only: false,
            allowed_regions: None,
            blocked_regions: HashSet::new(),
        }
    }
}

/// Required placement from previous competition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredPlacement {
    /// Competition type
    pub competition_type: Option<CompetitionType>,
    /// Maximum rank required (e.g., top 10)
    pub max_rank: usize,
    /// Within how many days
    pub within_days: Option<u32>,
}

/// Agent profile for requirement verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_id: AgentId,
    pub balance: Decimal,
    pub kyc_level: KycLevel,
    pub total_trade_count: u64,
    pub account_created: DateTime<Utc>,
    pub badges: HashSet<String>,
    pub placements: Vec<CompetitionPlacement>,
    pub owner_id: Option<String>,
    pub region: Option<String>,
    pub invitation_codes: HashSet<String>,
}

/// Historical competition placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionPlacement {
    pub competition_id: CompetitionId,
    pub competition_type: CompetitionType,
    pub rank: usize,
    pub ended_at: DateTime<Utc>,
}

impl EntryRequirements {
    /// Verify if an agent meets all requirements
    pub fn verify(&self, profile: &AgentProfile, invitation_code: Option<&str>) -> CompetitionResult<()> {
        // Check blacklist first
        if self.blacklist.contains(&profile.agent_id) {
            return Err(CompetitionError::RequirementsNotMet {
                reason: "Agent is not eligible for this competition".to_string(),
            });
        }

        // Check whitelist if not empty
        if !self.whitelist.is_empty() && !self.whitelist.contains(&profile.agent_id) {
            return Err(CompetitionError::RequirementsNotMet {
                reason: "Agent is not on the whitelist".to_string(),
            });
        }

        // Check invitation
        if self.invitation_only {
            match invitation_code {
                Some(code) if profile.invitation_codes.contains(code) => {}
                _ => {
                    return Err(CompetitionError::RequirementsNotMet {
                        reason: "Valid invitation code required".to_string(),
                    });
                }
            }
        }

        // Check balance
        if let Some(min) = self.min_balance {
            if profile.balance < min {
                return Err(CompetitionError::RequirementsNotMet {
                    reason: format!("Minimum balance of {} required", min),
                });
            }
        }

        // Check KYC
        if profile.kyc_level < self.min_kyc_level {
            return Err(CompetitionError::RequirementsNotMet {
                reason: format!("KYC level {:?} or higher required", self.min_kyc_level),
            });
        }

        // Check trade count
        if let Some(min) = self.min_trade_count {
            if profile.total_trade_count < min {
                return Err(CompetitionError::RequirementsNotMet {
                    reason: format!("Minimum {} trades required", min),
                });
            }
        }

        // Check account age
        if let Some(min_days) = self.min_account_age_days {
            let age = Utc::now() - profile.account_created;
            if age.num_days() < min_days as i64 {
                return Err(CompetitionError::RequirementsNotMet {
                    reason: format!("Account must be at least {} days old", min_days),
                });
            }
        }

        // Check required badges
        for badge in &self.required_badges {
            if !profile.badges.contains(badge) {
                return Err(CompetitionError::RequirementsNotMet {
                    reason: format!("Badge '{}' required", badge),
                });
            }
        }

        // Check required placements
        for req in &self.required_placements {
            let qualifying = profile.placements.iter().any(|p| {
                // Check competition type if specified
                if let Some(ct) = req.competition_type {
                    if p.competition_type != ct {
                        return false;
                    }
                }

                // Check rank
                if p.rank > req.max_rank {
                    return false;
                }

                // Check recency
                if let Some(days) = req.within_days {
                    let age = Utc::now() - p.ended_at;
                    if age.num_days() > days as i64 {
                        return false;
                    }
                }

                true
            });

            if !qualifying {
                return Err(CompetitionError::RequirementsNotMet {
                    reason: format!("Required placement: top {} finish", req.max_rank),
                });
            }
        }

        // Check region
        if let Some(ref region) = profile.region {
            if self.blocked_regions.contains(region) {
                return Err(CompetitionError::RequirementsNotMet {
                    reason: "Region not eligible".to_string(),
                });
            }

            if let Some(ref allowed) = self.allowed_regions {
                if !allowed.contains(region) {
                    return Err(CompetitionError::RequirementsNotMet {
                        reason: "Region not eligible".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Prize Pool
// ============================================================================

/// Prize distribution tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrizeTier {
    /// Tier name (e.g., "1st Place", "Top 10")
    pub name: String,
    /// Rank range (inclusive)
    pub rank_start: usize,
    pub rank_end: usize,
    /// Prize amount per winner
    pub amount: Decimal,
    /// Is this a percentage of pool or fixed amount
    pub is_percentage: bool,
    /// Special rewards (badges, titles, etc.)
    pub special_rewards: Vec<String>,
}

/// Prize pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrizePool {
    /// Total pool amount
    pub total_amount: Decimal,
    /// Currency
    pub currency: String,
    /// Distribution tiers
    pub tiers: Vec<PrizeTier>,
    /// Sponsor contributions
    pub sponsors: Vec<Sponsor>,
    /// Minimum participants for prizes to be awarded
    pub min_participants: usize,
    /// Whether to scale prizes based on participation
    pub scale_with_participation: bool,
    /// Entry fees contribution to pool (percentage)
    pub entry_fee_contribution: Decimal,
    /// Is prize pool guaranteed
    pub guaranteed: bool,
}

impl Default for PrizePool {
    fn default() -> Self {
        Self {
            total_amount: Decimal::ZERO,
            currency: "IUSD".to_string(),
            tiers: Vec::new(),
            sponsors: Vec::new(),
            min_participants: 10,
            scale_with_participation: false,
            entry_fee_contribution: dec!(50), // 50% of entry fees go to pool
            guaranteed: false,
        }
    }
}

impl PrizePool {
    /// Create a standard prize pool with common distribution
    pub fn standard(total: Decimal) -> Self {
        Self {
            total_amount: total,
            tiers: vec![
                PrizeTier {
                    name: "Grand Champion".to_string(),
                    rank_start: 1,
                    rank_end: 1,
                    amount: dec!(40), // 40%
                    is_percentage: true,
                    special_rewards: vec!["Champion Badge".to_string(), "Exclusive Title".to_string()],
                },
                PrizeTier {
                    name: "Runner Up".to_string(),
                    rank_start: 2,
                    rank_end: 2,
                    amount: dec!(20), // 20%
                    is_percentage: true,
                    special_rewards: vec!["Silver Badge".to_string()],
                },
                PrizeTier {
                    name: "Third Place".to_string(),
                    rank_start: 3,
                    rank_end: 3,
                    amount: dec!(10), // 10%
                    is_percentage: true,
                    special_rewards: vec!["Bronze Badge".to_string()],
                },
                PrizeTier {
                    name: "Top 10".to_string(),
                    rank_start: 4,
                    rank_end: 10,
                    amount: dec!(3), // 3% each = 21% total
                    is_percentage: true,
                    special_rewards: vec!["Top 10 Badge".to_string()],
                },
                PrizeTier {
                    name: "Top 25".to_string(),
                    rank_start: 11,
                    rank_end: 25,
                    amount: dec!(0.6), // 0.6% each = 9% total
                    is_percentage: true,
                    special_rewards: vec!["Finalist Badge".to_string()],
                },
            ],
            ..Default::default()
        }
    }

    /// Calculate prize for a specific rank
    pub fn calculate_prize(&self, rank: usize) -> Option<(Decimal, Vec<String>)> {
        for tier in &self.tiers {
            if rank >= tier.rank_start && rank <= tier.rank_end {
                let amount = if tier.is_percentage {
                    (tier.amount / dec!(100)) * self.total_amount
                } else {
                    tier.amount
                };
                return Some((amount, tier.special_rewards.clone()));
            }
        }
        None
    }

    /// Get total prizes allocated
    pub fn total_allocated(&self) -> Decimal {
        self.tiers.iter().map(|t| {
            let per_winner = if t.is_percentage {
                (t.amount / dec!(100)) * self.total_amount
            } else {
                t.amount
            };
            let winners = t.rank_end - t.rank_start + 1;
            per_winner * Decimal::from(winners as u64)
        }).sum()
    }

    /// Add entry fee contribution
    pub fn add_entry_fee(&mut self, fee: Decimal) {
        let contribution = fee * (self.entry_fee_contribution / dec!(100));
        self.total_amount += contribution;
    }
}

/// Sponsor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sponsor {
    pub name: String,
    pub logo_url: Option<String>,
    pub contribution: Decimal,
    pub tier: SponsorTier,
    pub special_prizes: Vec<SpecialPrize>,
}

/// Sponsor tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SponsorTier {
    Title,
    Platinum,
    Gold,
    Silver,
    Bronze,
}

/// Special prizes from sponsors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialPrize {
    pub name: String,
    pub description: String,
    pub criteria: String,
    pub value: Decimal,
}

// ============================================================================
// Team Support
// ============================================================================

/// Team identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub Uuid);

impl TeamId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TeamId {
    fn default() -> Self {
        Self::new()
    }
}

/// Team in a team competition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub captain: AgentId,
    pub members: Vec<AgentId>,
    pub max_members: usize,
    pub created_at: DateTime<Utc>,
}

impl Team {
    pub fn new(name: String, captain: AgentId, max_members: usize) -> Self {
        Self {
            id: TeamId::new(),
            name,
            captain: captain.clone(),
            members: vec![captain],
            max_members,
            created_at: Utc::now(),
        }
    }

    pub fn add_member(&mut self, agent_id: AgentId) -> CompetitionResult<()> {
        if self.members.len() >= self.max_members {
            return Err(CompetitionError::TeamError("Team is full".to_string()));
        }
        if self.members.contains(&agent_id) {
            return Err(CompetitionError::TeamError("Already a member".to_string()));
        }
        self.members.push(agent_id);
        Ok(())
    }

    pub fn remove_member(&mut self, agent_id: &AgentId) -> CompetitionResult<()> {
        if *agent_id == self.captain {
            return Err(CompetitionError::TeamError("Cannot remove captain".to_string()));
        }
        self.members.retain(|m| m != agent_id);
        Ok(())
    }
}

// ============================================================================
// Competition Configuration
// ============================================================================

/// Full competition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionConfig {
    /// Competition name
    pub name: String,
    /// Description
    pub description: String,
    /// Competition type
    pub competition_type: CompetitionType,
    /// Custom scoring configuration (for Custom type)
    pub custom_scoring: Option<CustomScoring>,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// Registration opens
    pub registration_opens: DateTime<Utc>,
    /// Registration closes
    pub registration_closes: DateTime<Utc>,
    /// Duration in hours
    pub duration_hours: u64,
    /// Available markets
    pub markets: Vec<MarketId>,
    /// Initial balance per participant
    pub initial_balance: Decimal,
    /// Maximum participants
    pub max_participants: Option<usize>,
    /// Minimum participants to start
    pub min_participants: usize,
    /// Entry fee
    pub entry_fee: Option<Decimal>,
    /// Entry requirements
    pub requirements: EntryRequirements,
    /// Prize pool
    pub prize_pool: PrizePool,
    /// Is this a team competition
    pub team_competition: bool,
    /// Team size limits
    pub team_size: Option<(usize, usize)>, // (min, max)
    /// Leaderboard configuration
    pub leaderboard_config: LeaderboardConfig,
    /// Rules and regulations URL
    pub rules_url: Option<String>,
    /// Public visibility
    pub is_public: bool,
    /// Featured competition
    pub is_featured: bool,
    /// Tags for discovery
    pub tags: Vec<String>,
}

/// Custom scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomScoring {
    pub formula: String,
    pub weights: HashMap<RankingMetric, Decimal>,
    pub bonuses: Vec<ScoringBonus>,
    pub penalties: Vec<ScoringPenalty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringBonus {
    pub name: String,
    pub condition: String,
    pub points: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringPenalty {
    pub name: String,
    pub condition: String,
    pub points: Decimal,
}

impl Default for CompetitionConfig {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            name: "Trading Competition".to_string(),
            description: "Compete against other AI agents in trading excellence".to_string(),
            competition_type: CompetitionType::PnLChallenge,
            custom_scoring: None,
            start_time: now + Duration::hours(24),
            registration_opens: now,
            registration_closes: now + Duration::hours(23),
            duration_hours: 168, // 1 week
            markets: vec![MarketId::new("ETH_IUSD")],
            initial_balance: dec!(10000),
            max_participants: Some(1000),
            min_participants: 10,
            entry_fee: None,
            requirements: EntryRequirements::default(),
            prize_pool: PrizePool::default(),
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
// Competition
// ============================================================================

/// Participant registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub agent_id: AgentId,
    pub team_id: Option<TeamId>,
    pub registered_at: DateTime<Utc>,
    pub entry_fee_paid: Decimal,
    pub status: RegistrationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationStatus {
    Pending,
    Confirmed,
    Withdrawn,
    Disqualified,
}

/// A trading competition
#[derive(Debug)]
pub struct Competition {
    /// Competition ID
    pub id: CompetitionId,
    /// Configuration
    pub config: CompetitionConfig,
    /// Current state
    pub state: CompetitionState,
    /// Registrations
    registrations: HashMap<AgentId, Registration>,
    /// Teams (for team competitions)
    teams: HashMap<TeamId, Team>,
    /// Leaderboard
    leaderboard: Leaderboard,
    /// State history
    state_history: Vec<(CompetitionState, DateTime<Utc>)>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Ended timestamp
    pub ended_at: Option<DateTime<Utc>>,
    /// Final results
    final_results: Option<CompetitionResults>,
}

impl Competition {
    /// Create a new competition
    pub fn new(config: CompetitionConfig) -> Self {
        let id = CompetitionId::new();
        let leaderboard = Leaderboard::new(id, config.leaderboard_config.clone());

        Self {
            id,
            config,
            state: CompetitionState::Draft,
            registrations: HashMap::new(),
            teams: HashMap::new(),
            leaderboard,
            state_history: vec![(CompetitionState::Draft, Utc::now())],
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            final_results: None,
        }
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: CompetitionState) -> CompetitionResult<()> {
        if !self.state.can_transition_to(new_state) {
            return Err(CompetitionError::InvalidStateTransition {
                from: self.state,
                to: new_state,
            });
        }

        let now = Utc::now();

        match new_state {
            CompetitionState::Running => {
                self.started_at = Some(now);
            }
            CompetitionState::Ended => {
                self.ended_at = Some(now);
            }
            _ => {}
        }

        self.state = new_state;
        self.state_history.push((new_state, now));
        Ok(())
    }

    /// Register an agent
    pub fn register(
        &mut self,
        profile: &AgentProfile,
        invitation_code: Option<&str>,
        entry_fee_paid: Decimal,
    ) -> CompetitionResult<Registration> {
        // Check state
        if !self.state.accepts_registrations() {
            return Err(CompetitionError::RegistrationClosed);
        }

        // Check time window
        let now = Utc::now();
        if now < self.config.registration_opens || now > self.config.registration_closes {
            return Err(CompetitionError::RegistrationClosed);
        }

        // Check capacity
        if let Some(max) = self.config.max_participants {
            if self.registrations.len() >= max {
                return Err(CompetitionError::CompetitionFull { max });
            }
        }

        // Check already registered
        if self.registrations.contains_key(&profile.agent_id) {
            return Err(CompetitionError::AlreadyRegistered);
        }

        // Check requirements
        self.config.requirements.verify(profile, invitation_code)?;

        // Check entry fee
        if let Some(required) = self.config.entry_fee {
            if entry_fee_paid < required {
                return Err(CompetitionError::InsufficientEntryFee {
                    required,
                    provided: entry_fee_paid,
                });
            }
        }

        // Add to prize pool
        self.config.prize_pool.add_entry_fee(entry_fee_paid);

        // Create registration
        let registration = Registration {
            agent_id: profile.agent_id.clone(),
            team_id: None,
            registered_at: Utc::now(),
            entry_fee_paid,
            status: RegistrationStatus::Confirmed,
        };

        self.registrations.insert(profile.agent_id.clone(), registration.clone());

        // Add to leaderboard
        self.leaderboard.register_agent(
            profile.agent_id.clone(),
            format!("Agent_{}", &profile.agent_id.to_string()[..8]),
            self.config.initial_balance,
        );

        Ok(registration)
    }

    /// Withdraw from competition
    pub fn withdraw(&mut self, agent_id: &AgentId) -> CompetitionResult<()> {
        let registration = self.registrations
            .get_mut(agent_id)
            .ok_or(CompetitionError::NotRegistered)?;

        if self.state.is_active() {
            return Err(CompetitionError::InvalidStateTransition {
                from: self.state,
                to: self.state,
            });
        }

        registration.status = RegistrationStatus::Withdrawn;
        Ok(())
    }

    /// Get participant count
    pub fn participant_count(&self) -> usize {
        self.registrations
            .values()
            .filter(|r| r.status == RegistrationStatus::Confirmed)
            .count()
    }

    /// Check if agent is registered
    pub fn is_registered(&self, agent_id: &AgentId) -> bool {
        self.registrations.get(agent_id)
            .map(|r| r.status == RegistrationStatus::Confirmed)
            .unwrap_or(false)
    }

    /// Record a trade
    pub fn record_trade(
        &mut self,
        agent_id: &AgentId,
        pnl: Decimal,
        volume: Decimal,
        duration_secs: u64,
    ) -> CompetitionResult<()> {
        if !self.state.allows_trading() {
            if self.state == CompetitionState::Paused {
                return Err(CompetitionError::Paused);
            }
            return Err(CompetitionError::NotStarted);
        }

        if !self.is_registered(agent_id) {
            return Err(CompetitionError::NotRegistered);
        }

        self.leaderboard.record_trade(agent_id, pnl, volume, duration_secs);
        Ok(())
    }

    /// Get leaderboard
    pub fn get_leaderboard(&mut self) -> &mut Leaderboard {
        &mut self.leaderboard
    }

    /// Get registrations
    pub fn get_registrations(&self) -> &HashMap<AgentId, Registration> {
        &self.registrations
    }

    /// Calculate end time
    pub fn end_time(&self) -> DateTime<Utc> {
        self.config.start_time + Duration::hours(self.config.duration_hours as i64)
    }

    /// Check if competition should auto-start
    pub fn should_auto_start(&self) -> bool {
        if self.state != CompetitionState::Starting {
            return false;
        }

        let now = Utc::now();
        let has_enough = self.participant_count() >= self.config.min_participants;

        now >= self.config.start_time && has_enough
    }

    /// Check if competition should auto-end
    pub fn should_auto_end(&self) -> bool {
        if self.state != CompetitionState::Running {
            return false;
        }

        Utc::now() >= self.end_time()
    }

    /// Calculate final results
    pub fn calculate_results(&mut self) -> CompetitionResult<CompetitionResults> {
        if self.state != CompetitionState::Calculating {
            self.transition_to(CompetitionState::Calculating)?;
        }

        let rankings = self.leaderboard.get_rankings(
            self.config.competition_type.primary_metric(),
            self.config.max_participants.unwrap_or(usize::MAX),
        );

        let mut prize_distributions = Vec::new();
        for entry in &rankings {
            if let Some((amount, rewards)) = self.config.prize_pool.calculate_prize(entry.rank) {
                prize_distributions.push(PrizeDistribution {
                    agent_id: entry.performance.agent_id.clone(),
                    rank: entry.rank,
                    prize_amount: amount,
                    special_rewards: rewards,
                });
            }
        }

        let results = CompetitionResults {
            competition_id: self.id,
            final_rankings: rankings,
            prize_distributions,
            total_participants: self.participant_count(),
            total_trades: self.registrations.values()
                .filter_map(|r| self.leaderboard.get_performance(&r.agent_id))
                .map(|p| p.trade_count)
                .sum(),
            total_volume: self.registrations.values()
                .filter_map(|r| self.leaderboard.get_performance(&r.agent_id))
                .map(|p| p.volume)
                .sum(),
            ended_at: Utc::now(),
        };

        self.final_results = Some(results.clone());
        self.transition_to(CompetitionState::Ended)?;

        Ok(results)
    }

    /// Get final results
    pub fn get_results(&self) -> Option<&CompetitionResults> {
        self.final_results.as_ref()
    }

    // Team operations

    /// Create a team (for team competitions)
    pub fn create_team(&mut self, name: String, captain: AgentId) -> CompetitionResult<Team> {
        if !self.config.team_competition {
            return Err(CompetitionError::TeamError("Not a team competition".to_string()));
        }

        if !self.is_registered(&captain) {
            return Err(CompetitionError::NotRegistered);
        }

        let max_size = self.config.team_size.map(|(_, max)| max).unwrap_or(5);
        let team = Team::new(name, captain.clone(), max_size);

        // Update registration
        if let Some(reg) = self.registrations.get_mut(&captain) {
            reg.team_id = Some(team.id.clone());
        }

        self.teams.insert(team.id.clone(), team.clone());
        Ok(team)
    }

    /// Join a team
    pub fn join_team(&mut self, team_id: &TeamId, agent_id: AgentId) -> CompetitionResult<()> {
        if !self.is_registered(&agent_id) {
            return Err(CompetitionError::NotRegistered);
        }

        let team = self.teams.get_mut(team_id)
            .ok_or(CompetitionError::TeamError("Team not found".to_string()))?;

        team.add_member(agent_id.clone())?;

        if let Some(reg) = self.registrations.get_mut(&agent_id) {
            reg.team_id = Some(team_id.clone());
        }

        Ok(())
    }

    /// Get teams
    pub fn get_teams(&self) -> &HashMap<TeamId, Team> {
        &self.teams
    }
}

/// Final competition results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionResults {
    pub competition_id: CompetitionId,
    pub final_rankings: Vec<crate::leaderboard::LeaderboardEntry>,
    pub prize_distributions: Vec<PrizeDistribution>,
    pub total_participants: usize,
    pub total_trades: u64,
    pub total_volume: Decimal,
    pub ended_at: DateTime<Utc>,
}

/// Prize distribution for a single winner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrizeDistribution {
    pub agent_id: AgentId,
    pub rank: usize,
    pub prize_amount: Decimal,
    pub special_rewards: Vec<String>,
}

// ============================================================================
// Competition Manager
// ============================================================================

/// Manages multiple competitions
#[derive(Debug, Default)]
pub struct CompetitionManager {
    competitions: HashMap<CompetitionId, Competition>,
    /// Featured competitions
    featured: Vec<CompetitionId>,
}

impl CompetitionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new competition
    pub fn create_competition(&mut self, config: CompetitionConfig) -> &mut Competition {
        let competition = Competition::new(config);
        let id = competition.id;
        self.competitions.insert(id, competition);
        self.competitions.get_mut(&id).unwrap()
    }

    /// Get a competition by ID
    pub fn get_competition(&self, id: &CompetitionId) -> Option<&Competition> {
        self.competitions.get(id)
    }

    /// Get a mutable competition by ID
    pub fn get_competition_mut(&mut self, id: &CompetitionId) -> Option<&mut Competition> {
        self.competitions.get_mut(id)
    }

    /// List all active competitions
    pub fn list_active(&self) -> Vec<&Competition> {
        self.competitions
            .values()
            .filter(|c| c.state.is_active())
            .collect()
    }

    /// List competitions by state
    pub fn list_by_state(&self, state: CompetitionState) -> Vec<&Competition> {
        self.competitions
            .values()
            .filter(|c| c.state == state)
            .collect()
    }

    /// List featured competitions
    pub fn list_featured(&self) -> Vec<&Competition> {
        self.featured
            .iter()
            .filter_map(|id| self.competitions.get(id))
            .collect()
    }

    /// Set featured competitions
    pub fn set_featured(&mut self, ids: Vec<CompetitionId>) {
        self.featured = ids;
    }

    /// Search competitions by tag
    pub fn search_by_tag(&self, tag: &str) -> Vec<&Competition> {
        self.competitions
            .values()
            .filter(|c| c.config.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .collect()
    }

    /// Process automatic state transitions
    pub fn process_auto_transitions(&mut self) {
        let ids: Vec<CompetitionId> = self.competitions.keys().copied().collect();

        for id in ids {
            if let Some(comp) = self.competitions.get_mut(&id) {
                // Auto-start
                if comp.should_auto_start() {
                    let _ = comp.transition_to(CompetitionState::Running);
                }

                // Auto-end
                if comp.should_auto_end() {
                    let _ = comp.calculate_results();
                }

                // Take snapshots
                if comp.state == CompetitionState::Running && comp.leaderboard.should_snapshot() {
                    comp.leaderboard.take_snapshot();
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_profile() -> AgentProfile {
        AgentProfile {
            agent_id: AgentId::new(),
            balance: dec!(10000),
            kyc_level: KycLevel::Standard,
            total_trade_count: 100,
            account_created: Utc::now() - Duration::days(30),
            badges: HashSet::new(),
            placements: Vec::new(),
            owner_id: None,
            region: Some("US".to_string()),
            invitation_codes: HashSet::new(),
        }
    }

    #[test]
    fn test_competition_creation() {
        let config = CompetitionConfig::default();
        let competition = Competition::new(config);

        assert_eq!(competition.state, CompetitionState::Draft);
        assert_eq!(competition.participant_count(), 0);
    }

    #[test]
    fn test_state_transitions() {
        let mut competition = Competition::new(CompetitionConfig::default());

        assert!(competition.transition_to(CompetitionState::Announced).is_ok());
        assert!(competition.transition_to(CompetitionState::Registration).is_ok());
        assert!(competition.transition_to(CompetitionState::Starting).is_ok());
        assert!(competition.transition_to(CompetitionState::Running).is_ok());
        assert!(competition.started_at.is_some());
    }

    #[test]
    fn test_invalid_transition() {
        let mut competition = Competition::new(CompetitionConfig::default());

        let result = competition.transition_to(CompetitionState::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_registration() {
        let now = Utc::now();
        let config = CompetitionConfig {
            registration_opens: now - Duration::hours(1),
            registration_closes: now + Duration::hours(1),
            ..Default::default()
        };
        let mut competition = Competition::new(config);
        competition.transition_to(CompetitionState::Announced).unwrap();

        let profile = create_test_profile();
        let result = competition.register(&profile, None, Decimal::ZERO);

        assert!(result.is_ok());
        assert_eq!(competition.participant_count(), 1);
        assert!(competition.is_registered(&profile.agent_id));
    }

    #[test]
    fn test_entry_requirements() {
        let requirements = EntryRequirements {
            min_balance: Some(dec!(5000)),
            min_kyc_level: KycLevel::Standard,
            ..Default::default()
        };

        let mut profile = create_test_profile();
        assert!(requirements.verify(&profile, None).is_ok());

        profile.balance = dec!(1000);
        assert!(requirements.verify(&profile, None).is_err());
    }

    #[test]
    fn test_prize_pool() {
        let pool = PrizePool::standard(dec!(100000));

        let (first_prize, _) = pool.calculate_prize(1).unwrap();
        assert_eq!(first_prize, dec!(40000)); // 40%

        let (second_prize, _) = pool.calculate_prize(2).unwrap();
        assert_eq!(second_prize, dec!(20000)); // 20%
    }

    #[test]
    fn test_competition_types() {
        assert_eq!(CompetitionType::PnLChallenge.primary_metric(), RankingMetric::PnL);
        assert_eq!(CompetitionType::SharpeShowdown.primary_metric(), RankingMetric::SharpeRatio);
        assert_eq!(CompetitionType::SpeedTrading.primary_metric(), RankingMetric::AvgTradeDuration);
    }
}
