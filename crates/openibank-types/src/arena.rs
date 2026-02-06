//! Arena types for OpeniBank
//!
//! The arena is the competitive benchmarking system where agents compete
//! on financial tasks with real stakes. It creates buzz and provides
//! verifiable performance data.

use crate::{
    AgentId, Amount, ArenaMatchId, ChallengeId, Currency, EscrowId,
    ReceiptId, TemporalAnchor,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of arena challenge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArenaChallenge {
    /// Best execution challenge - maximize trading performance
    BestExecution {
        /// Initial capital
        initial_capital: Amount,
        /// Trading pairs allowed
        pairs: Vec<TradingPair>,
        /// Duration in ticks
        ticks: u32,
        /// Metrics to optimize
        metrics: Vec<ExecutionMetric>,
    },
    /// Lowest fee challenge - process payments with minimal fees
    LowestFee {
        /// Number of transactions to process
        transaction_count: u32,
        /// Transaction parameters
        params: PaymentParams,
    },
    /// Fastest settlement challenge - minimize settlement time
    FastestSettlement {
        /// Number of settlements
        settlement_count: u32,
        /// Settlement parameters
        params: SettlementParams,
    },
    /// Best risk score - most accurate risk predictions
    BestRiskScore {
        /// Portfolio to analyze
        portfolio: PortfolioParams,
        /// Risk metrics to predict
        metrics: Vec<RiskMetric>,
    },
    /// Custom challenge
    Custom {
        /// Challenge specification
        spec: ChallengeSpec,
    },
}

/// A trading pair
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradingPair {
    /// Base currency
    pub base: Currency,
    /// Quote currency
    pub quote: Currency,
}

impl TradingPair {
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }
}

/// Execution metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionMetric {
    /// Total return
    TotalReturn,
    /// Sharpe ratio
    SharpeRatio,
    /// Maximum drawdown
    MaxDrawdown,
    /// Win rate
    WinRate,
    /// Profit factor
    ProfitFactor,
}

/// Payment parameters for fee challenges
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaymentParams {
    /// Average transaction size
    pub avg_amount: Amount,
    /// Variance in transaction size
    pub variance: f64,
    /// Target success rate
    pub target_success_rate: f64,
}

/// Settlement parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementParams {
    /// Cross-border ratio
    pub cross_border_ratio: f64,
    /// Multi-currency
    pub multi_currency: bool,
    /// Currencies involved
    pub currencies: Vec<Currency>,
}

/// Portfolio parameters for risk challenges
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioParams {
    /// Number of positions
    pub position_count: u32,
    /// Total value
    pub total_value: Amount,
    /// Asset classes included
    pub asset_classes: Vec<String>,
}

/// Risk metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskMetric {
    /// Value at Risk
    VaR,
    /// Expected Shortfall
    ExpectedShortfall,
    /// Tail risk
    TailRisk,
    /// Correlation risk
    CorrelationRisk,
    /// Liquidity risk
    LiquidityRisk,
}

/// Custom challenge specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChallengeSpec {
    /// Challenge name
    pub name: String,
    /// Description
    pub description: String,
    /// Parameters (JSON)
    pub parameters: serde_json::Value,
    /// Scoring function
    pub scoring: ScoringFunction,
    /// Duration in seconds
    pub duration_seconds: u64,
}

/// Scoring function for challenges
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoringFunction {
    /// Maximize value
    Maximize { metric: String },
    /// Minimize value
    Minimize { metric: String },
    /// Custom formula
    Formula { expression: String },
}

/// Status of an arena match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArenaStatus {
    /// Accepting participants
    Open,
    /// Waiting for start
    Pending,
    /// Running
    Running,
    /// Computing results
    Computing,
    /// Completed
    Completed,
    /// Cancelled
    Cancelled,
    /// Disputed
    Disputed,
}

impl ArenaStatus {
    /// Check if match is accepting participants
    pub fn can_join(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Check if match is active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if match is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

/// A participant in an arena match
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaParticipant {
    /// Agent ID
    pub agent: AgentId,
    /// Stake amount
    pub stake: Amount,
    /// Escrow holding stake
    pub escrow: EscrowId,
    /// Current score
    pub score: f64,
    /// Rank (1 = first place)
    pub rank: Option<u32>,
    /// Status
    pub status: ParticipantStatus,
    /// Actions taken
    pub actions: Vec<ArenaAction>,
}

/// Status of a participant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticipantStatus {
    /// Registered
    Registered,
    /// Ready
    Ready,
    /// Active
    Active,
    /// Finished
    Finished,
    /// Disqualified
    Disqualified,
    /// Withdrew
    Withdrew,
}

/// An action taken in the arena
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaAction {
    /// Action type
    pub action_type: String,
    /// Parameters
    pub parameters: serde_json::Value,
    /// Timestamp
    pub timestamp: TemporalAnchor,
    /// Result
    pub result: Option<serde_json::Value>,
    /// Receipt
    pub receipt: Option<ReceiptId>,
}

/// Results of an arena match
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaResults {
    /// Final rankings
    pub rankings: Vec<ArenaRanking>,
    /// Winner
    pub winner: Option<AgentId>,
    /// Total prize pool
    pub prize_pool: Amount,
    /// Distribution
    pub distribution: Vec<PrizeDistribution>,
    /// Statistics
    pub stats: ArenaStats,
    /// Verification hash
    pub verification_hash: String,
}

/// A ranking entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaRanking {
    /// Rank (1 = first)
    pub rank: u32,
    /// Agent
    pub agent: AgentId,
    /// Final score
    pub score: f64,
    /// Metrics
    pub metrics: std::collections::HashMap<String, f64>,
}

/// Prize distribution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrizeDistribution {
    /// Recipient
    pub agent: AgentId,
    /// Amount won
    pub amount: Amount,
    /// Reason
    pub reason: String,
    /// Transaction
    pub transaction_id: Option<crate::TransactionId>,
}

/// Statistics from an arena match
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaStats {
    /// Duration in seconds
    pub duration_seconds: u64,
    /// Total actions taken
    pub total_actions: u64,
    /// Average score
    pub avg_score: f64,
    /// Highest score
    pub max_score: f64,
    /// Total volume transacted
    pub total_volume: Amount,
}

/// An arena match
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaMatch {
    /// Match ID
    pub id: ArenaMatchId,
    /// Challenge definition
    pub challenge: ArenaChallenge,
    /// Creator
    pub creator: AgentId,
    /// Participants
    pub participants: Vec<ArenaParticipant>,
    /// Status
    pub status: ArenaStatus,
    /// Minimum stake
    pub min_stake: Amount,
    /// Maximum participants
    pub max_participants: u32,
    /// Start time
    pub starts_at: DateTime<Utc>,
    /// End time
    pub ends_at: Option<DateTime<Utc>>,
    /// Results (when completed)
    pub results: Option<ArenaResults>,
    /// Created at
    pub created_at: TemporalAnchor,
}

impl ArenaMatch {
    /// Check if match can accept more participants
    pub fn can_join(&self) -> bool {
        self.status.can_join()
            && (self.participants.len() as u32) < self.max_participants
    }

    /// Get current participant count
    pub fn participant_count(&self) -> u32 {
        self.participants.len() as u32
    }

    /// Calculate total prize pool
    pub fn prize_pool(&self) -> Amount {
        self.participants
            .iter()
            .fold(Amount::zero(self.min_stake.currency), |acc, p| {
                acc.checked_add(p.stake).unwrap_or(acc)
            })
    }
}

/// Leaderboard entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// Rank
    pub rank: u32,
    /// Agent
    pub agent: AgentId,
    /// Wins
    pub wins: u32,
    /// Losses
    pub losses: u32,
    /// Draws
    pub draws: u32,
    /// Total earnings
    pub earnings: Amount,
    /// Win rate
    pub win_rate: f64,
    /// Average score
    pub avg_score: f64,
}

/// Leaderboard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Leaderboard {
    /// Category
    pub category: Option<crate::ServiceCategory>,
    /// Timeframe
    pub timeframe: Timeframe,
    /// Entries
    pub entries: Vec<LeaderboardEntry>,
    /// Last updated
    pub updated_at: TemporalAnchor,
}

/// Timeframe for leaderboards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    /// Daily
    Daily,
    /// Weekly
    Weekly,
    /// Monthly
    Monthly,
    /// All time
    AllTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_status() {
        assert!(ArenaStatus::Open.can_join());
        assert!(!ArenaStatus::Running.can_join());
        assert!(ArenaStatus::Running.is_active());
        assert!(ArenaStatus::Completed.is_terminal());
    }

    #[test]
    fn test_trading_pair() {
        let pair = TradingPair::new(Currency::eth(), Currency::iusd());
        assert_eq!(pair.base, Currency::eth());
        assert_eq!(pair.quote, Currency::iusd());
    }

    #[test]
    fn test_challenge_types() {
        let challenge = ArenaChallenge::BestExecution {
            initial_capital: Amount::iusd(10000.0),
            pairs: vec![TradingPair::new(Currency::eth(), Currency::iusd())],
            ticks: 1000,
            metrics: vec![ExecutionMetric::SharpeRatio],
        };

        match challenge {
            ArenaChallenge::BestExecution { ticks, .. } => {
                assert_eq!(ticks, 1000);
            }
            _ => panic!("Wrong variant"),
        }
    }
}
