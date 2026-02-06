//! Arena competitive benchmarking

use openibank_types::*;
pub use openibank_types::{
    ArenaMatch, ArenaChallenge, ArenaStatus, ArenaParticipant,
    ArenaResults, ArenaRanking, Leaderboard, Timeframe,
};

/// Arena engine trait
#[async_trait::async_trait]
pub trait ArenaEngine: Send + Sync {
    /// Create a challenge
    async fn create_challenge(
        &self,
        creator: AgentId,
        challenge: ArenaChallenge,
        min_stake: Amount,
        max_participants: u32,
        starts_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<ArenaMatch>;

    /// Join a challenge
    async fn join_challenge(
        &self,
        participant: AgentId,
        match_id: ArenaMatchId,
        stake: Amount,
    ) -> Result<()>;

    /// Execute a challenge
    async fn execute_challenge(
        &self,
        match_id: ArenaMatchId,
    ) -> Result<ArenaResults>;

    /// Get leaderboard
    async fn get_leaderboard(
        &self,
        category: Option<ServiceCategory>,
        timeframe: Timeframe,
    ) -> Result<Leaderboard>;
}
