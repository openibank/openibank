//! Arena competitive benchmarking implementation

use openibank_types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

pub use openibank_types::{
    ArenaAction, ArenaChallenge, ArenaMatch, ArenaParticipant, ArenaRanking, ArenaResults,
    ArenaStats, ArenaStatus, Leaderboard, LeaderboardEntry, ParticipantStatus, PrizeDistribution,
    Timeframe,
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
        starts_at: DateTime<Utc>,
    ) -> Result<ArenaMatch>;

    /// Join a challenge
    async fn join_challenge(
        &self,
        participant: AgentId,
        match_id: ArenaMatchId,
        stake: Amount,
    ) -> Result<EscrowId>;

    /// Submit an action during a match
    async fn submit_action(
        &self,
        match_id: &ArenaMatchId,
        agent: &AgentId,
        action_type: String,
        parameters: serde_json::Value,
    ) -> Result<ArenaAction>;

    /// Start a match (transition from Pending to Running)
    async fn start_match(&self, match_id: &ArenaMatchId) -> Result<()>;

    /// Execute and complete a challenge
    async fn execute_challenge(&self, match_id: &ArenaMatchId) -> Result<ArenaResults>;

    /// Get match status
    async fn get_match(&self, match_id: &ArenaMatchId) -> Result<ArenaMatch>;

    /// Get leaderboard
    async fn get_leaderboard(
        &self,
        category: Option<ServiceCategory>,
        timeframe: Timeframe,
    ) -> Result<Leaderboard>;

    /// Get agent's arena history
    async fn get_agent_history(&self, agent: &AgentId) -> Result<Vec<ArenaMatch>>;
}

/// Escrow integration for arena stakes
#[async_trait::async_trait]
pub trait ArenaEscrow: Send + Sync {
    /// Create escrow for stake
    async fn create_stake_escrow(
        &self,
        match_id: &ArenaMatchId,
        agent: &AgentId,
        amount: Amount,
    ) -> Result<EscrowId>;

    /// Release escrow to winner
    async fn release_to_winner(
        &self,
        escrow_id: &EscrowId,
        winner: &AgentId,
    ) -> Result<()>;

    /// Refund escrow (match cancelled)
    async fn refund(&self, escrow_id: &EscrowId) -> Result<()>;
}

/// In-memory arena engine
pub struct InMemoryArena {
    matches: Arc<RwLock<HashMap<ArenaMatchId, ArenaMatch>>>,
    leaderboard_cache: Arc<RwLock<HashMap<(Option<ServiceCategory>, Timeframe), Leaderboard>>>,
    agent_stats: Arc<RwLock<HashMap<AgentId, AgentArenaStats>>>,
}

/// Stats for an agent across all arena matches
#[derive(Debug, Clone, Default)]
struct AgentArenaStats {
    wins: u32,
    losses: u32,
    draws: u32,
    total_earnings: Amount,
    total_staked: Amount,
    matches_played: u32,
    avg_score: f64,
}

impl InMemoryArena {
    pub fn new() -> Self {
        Self {
            matches: Arc::new(RwLock::new(HashMap::new())),
            leaderboard_cache: Arc::new(RwLock::new(HashMap::new())),
            agent_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Calculate scores for all participants based on challenge type
    fn calculate_scores(&self, arena_match: &ArenaMatch) -> Vec<(AgentId, f64)> {
        let mut scores: Vec<(AgentId, f64)> = arena_match
            .participants
            .iter()
            .map(|p| {
                // Score based on actions taken - this would be challenge-specific
                let action_score: f64 = p.actions.iter().map(|a| {
                    a.result
                        .as_ref()
                        .and_then(|r| r.get("score"))
                        .and_then(|s| s.as_f64())
                        .unwrap_or(0.0)
                }).sum();

                (p.agent.clone(), action_score)
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    /// Distribute prizes based on rankings
    fn distribute_prizes(
        &self,
        prize_pool: Amount,
        rankings: &[ArenaRanking],
    ) -> Vec<PrizeDistribution> {
        if rankings.is_empty() {
            return vec![];
        }

        let mut distributions = Vec::new();

        // Simple distribution: 60% to 1st, 30% to 2nd, 10% to 3rd
        let distribution_percentages: [u8; 3] = [60, 30, 10];

        for (i, ranking) in rankings.iter().take(3).enumerate() {
            if let Ok(amount) = prize_pool.percentage(distribution_percentages[i]) {
                distributions.push(PrizeDistribution {
                    agent: ranking.agent.clone(),
                    amount,
                    reason: format!("Rank #{}", ranking.rank),
                    transaction_id: None,
                });
            }
        }

        distributions
    }

    /// Update agent stats after match completion
    async fn update_agent_stats(&self, results: &ArenaResults, participants: &[ArenaParticipant]) {
        let mut stats = self.agent_stats.write().await;

        for dist in &results.distribution {
            let agent_stats = stats.entry(dist.agent.clone()).or_default();
            if let Ok(new_earnings) = agent_stats.total_earnings.checked_add(dist.amount) {
                agent_stats.total_earnings = new_earnings;
            }
        }

        // Update win/loss counts
        if let Some(ref winner) = results.winner {
            let winner_stats = stats.entry(winner.clone()).or_default();
            winner_stats.wins += 1;
            winner_stats.matches_played += 1;

            for p in participants {
                if &p.agent != winner {
                    let loser_stats = stats.entry(p.agent.clone()).or_default();
                    loser_stats.losses += 1;
                    loser_stats.matches_played += 1;
                }
            }
        }
    }
}

impl Default for InMemoryArena {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ArenaEngine for InMemoryArena {
    async fn create_challenge(
        &self,
        creator: AgentId,
        challenge: ArenaChallenge,
        min_stake: Amount,
        max_participants: u32,
        starts_at: DateTime<Utc>,
    ) -> Result<ArenaMatch> {
        let match_id = ArenaMatchId::new();
        let now = TemporalAnchor::now();

        let arena_match = ArenaMatch {
            id: match_id.clone(),
            challenge,
            creator,
            participants: Vec::new(),
            status: ArenaStatus::Open,
            min_stake,
            max_participants,
            starts_at,
            ends_at: None,
            results: None,
            created_at: now,
        };

        self.matches
            .write()
            .await
            .insert(match_id, arena_match.clone());

        Ok(arena_match)
    }

    async fn join_challenge(
        &self,
        participant: AgentId,
        match_id: ArenaMatchId,
        stake: Amount,
    ) -> Result<EscrowId> {
        let mut matches = self.matches.write().await;
        let arena_match = matches
            .get_mut(&match_id)
            .ok_or_else(|| OpeniBankError::MatchNotFound {
                match_id: match_id.0.to_string(),
            })?;

        if !arena_match.can_join() {
            return Err(OpeniBankError::MatchNotAccepting {
                match_id: match_id.0.to_string(),
            });
        }

        if stake < arena_match.min_stake {
            return Err(OpeniBankError::InsufficientStake {
                required: arena_match.min_stake.to_human(),
                provided: stake.to_human(),
            });
        }

        // Check if already joined
        if arena_match.participants.iter().any(|p| p.agent == participant) {
            return Err(OpeniBankError::AlreadyParticipating {
                agent_id: participant.0.to_string(),
                match_id: match_id.0.to_string(),
            });
        }

        // Create escrow for stake (mock escrow ID)
        let escrow_id = EscrowId::new();

        let arena_participant = ArenaParticipant {
            agent: participant,
            stake,
            escrow: escrow_id.clone(),
            score: 0.0,
            rank: None,
            status: ParticipantStatus::Registered,
            actions: Vec::new(),
        };

        arena_match.participants.push(arena_participant);

        // Close registration if max reached
        if arena_match.participants.len() as u32 >= arena_match.max_participants {
            arena_match.status = ArenaStatus::Pending;
        }

        Ok(escrow_id)
    }

    async fn submit_action(
        &self,
        match_id: &ArenaMatchId,
        agent: &AgentId,
        action_type: String,
        parameters: serde_json::Value,
    ) -> Result<ArenaAction> {
        let mut matches = self.matches.write().await;
        let arena_match = matches
            .get_mut(match_id)
            .ok_or_else(|| OpeniBankError::MatchNotFound {
                match_id: match_id.0.to_string(),
            })?;

        if arena_match.status != ArenaStatus::Running {
            return Err(OpeniBankError::MatchNotAccepting {
                match_id: match_id.0.to_string(),
            });
        }

        let participant = arena_match
            .participants
            .iter_mut()
            .find(|p| &p.agent == agent)
            .ok_or_else(|| OpeniBankError::MatchNotFound {
                match_id: match_id.0.to_string(),
            })?;

        // Simulate action result (in production, this would be challenge-specific)
        let result = Some(serde_json::json!({
            "success": true,
            "score": 10.0,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));

        let action = ArenaAction {
            action_type,
            parameters,
            timestamp: TemporalAnchor::now(),
            result,
            receipt: None,
        };

        participant.actions.push(action.clone());
        participant.status = ParticipantStatus::Active;

        Ok(action)
    }

    async fn start_match(&self, match_id: &ArenaMatchId) -> Result<()> {
        let mut matches = self.matches.write().await;
        let arena_match = matches
            .get_mut(match_id)
            .ok_or_else(|| OpeniBankError::MatchNotFound {
                match_id: match_id.0.to_string(),
            })?;

        if arena_match.status != ArenaStatus::Pending && arena_match.status != ArenaStatus::Open {
            return Err(OpeniBankError::MatchNotAccepting {
                match_id: match_id.0.to_string(),
            });
        }

        if arena_match.participants.len() < 2 {
            return Err(OpeniBankError::InvalidInput {
                field: "participants".to_string(),
                reason: "At least 2 participants required".to_string(),
            });
        }

        arena_match.status = ArenaStatus::Running;

        // Set all participants to Ready
        for p in &mut arena_match.participants {
            p.status = ParticipantStatus::Ready;
        }

        Ok(())
    }

    async fn execute_challenge(&self, match_id: &ArenaMatchId) -> Result<ArenaResults> {
        let mut matches = self.matches.write().await;
        let arena_match = matches
            .get_mut(match_id)
            .ok_or_else(|| OpeniBankError::MatchNotFound {
                match_id: match_id.0.to_string(),
            })?;

        arena_match.status = ArenaStatus::Computing;

        // Calculate scores
        let scores = self.calculate_scores(arena_match);

        // Build rankings
        let rankings: Vec<ArenaRanking> = scores
            .iter()
            .enumerate()
            .map(|(i, (agent, score))| {
                let metrics = arena_match
                    .participants
                    .iter()
                    .find(|p| &p.agent == agent)
                    .map(|p| {
                        let mut m = std::collections::HashMap::new();
                        m.insert("actions".to_string(), p.actions.len() as f64);
                        m.insert("score".to_string(), *score);
                        m
                    })
                    .unwrap_or_default();

                ArenaRanking {
                    rank: (i + 1) as u32,
                    agent: agent.clone(),
                    score: *score,
                    metrics,
                }
            })
            .collect();

        // Calculate prize pool
        let prize_pool = arena_match.prize_pool();

        // Distribute prizes
        let distribution = self.distribute_prizes(prize_pool, &rankings);

        // Update participant scores and ranks
        for (i, (agent, score)) in scores.iter().enumerate() {
            if let Some(p) = arena_match.participants.iter_mut().find(|p| &p.agent == agent) {
                p.score = *score;
                p.rank = Some((i + 1) as u32);
                p.status = ParticipantStatus::Finished;
            }
        }

        // Calculate stats
        let total_actions: u64 = arena_match
            .participants
            .iter()
            .map(|p| p.actions.len() as u64)
            .sum();

        let avg_score = if !scores.is_empty() {
            scores.iter().map(|(_, s)| s).sum::<f64>() / scores.len() as f64
        } else {
            0.0
        };

        let max_score = scores.first().map(|(_, s)| *s).unwrap_or(0.0);

        let stats = ArenaStats {
            duration_seconds: chrono::Utc::now()
                .signed_duration_since(arena_match.starts_at)
                .num_seconds()
                .max(0) as u64,
            total_actions,
            avg_score,
            max_score,
            total_volume: prize_pool,
        };

        let winner = rankings.first().map(|r| r.agent.clone());

        let results = ArenaResults {
            rankings,
            winner,
            prize_pool,
            distribution,
            stats,
            verification_hash: format!("arena_{}_{}", match_id.0, chrono::Utc::now().timestamp()),
        };

        // Update match
        arena_match.status = ArenaStatus::Completed;
        arena_match.ends_at = Some(chrono::Utc::now());
        arena_match.results = Some(results.clone());

        // Update agent stats
        let participants = arena_match.participants.clone();
        drop(matches);
        self.update_agent_stats(&results, &participants).await;

        Ok(results)
    }

    async fn get_match(&self, match_id: &ArenaMatchId) -> Result<ArenaMatch> {
        self.matches
            .read()
            .await
            .get(match_id)
            .cloned()
            .ok_or_else(|| OpeniBankError::MatchNotFound {
                match_id: match_id.0.to_string(),
            })
    }

    async fn get_leaderboard(
        &self,
        category: Option<ServiceCategory>,
        timeframe: Timeframe,
    ) -> Result<Leaderboard> {
        let stats = self.agent_stats.read().await;

        let mut entries: Vec<LeaderboardEntry> = stats
            .iter()
            .filter(|(_, s)| s.matches_played > 0)
            .map(|(agent, s)| {
                let win_rate = if s.matches_played > 0 {
                    (s.wins as f64) / (s.matches_played as f64) * 100.0
                } else {
                    0.0
                };

                LeaderboardEntry {
                    rank: 0,
                    agent: agent.clone(),
                    wins: s.wins,
                    losses: s.losses,
                    draws: s.draws,
                    earnings: s.total_earnings,
                    win_rate,
                    avg_score: s.avg_score,
                }
            })
            .collect();

        // Sort by wins then earnings
        entries.sort_by(|a, b| {
            b.wins
                .cmp(&a.wins)
                .then(b.earnings.cmp(&a.earnings))
        });

        // Assign ranks
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.rank = (i + 1) as u32;
        }

        Ok(Leaderboard {
            category,
            timeframe,
            entries,
            updated_at: TemporalAnchor::now(),
        })
    }

    async fn get_agent_history(&self, agent: &AgentId) -> Result<Vec<ArenaMatch>> {
        let matches = self.matches.read().await;
        let history: Vec<ArenaMatch> = matches
            .values()
            .filter(|m| m.participants.iter().any(|p| &p.agent == agent))
            .cloned()
            .collect();

        Ok(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_challenge() -> ArenaChallenge {
        ArenaChallenge::BestExecution {
            initial_capital: Amount::iusd(10000.0),
            pairs: vec![TradingPair::new(Currency::eth(), Currency::iusd())],
            ticks: 100,
            metrics: vec![ExecutionMetric::TotalReturn],
        }
    }

    #[tokio::test]
    async fn test_create_and_join_challenge() {
        let arena = InMemoryArena::new();
        let creator = AgentId::new();
        let participant = AgentId::new();

        let arena_match = arena
            .create_challenge(
                creator.clone(),
                test_challenge(),
                Amount::iusd(100.0),
                10,
                chrono::Utc::now() + chrono::Duration::hours(1),
            )
            .await
            .unwrap();

        assert_eq!(arena_match.status, ArenaStatus::Open);

        let escrow_id = arena
            .join_challenge(participant, arena_match.id.clone(), Amount::iusd(100.0))
            .await
            .unwrap();

        let updated_match = arena.get_match(&arena_match.id).await.unwrap();
        assert_eq!(updated_match.participants.len(), 1);
        assert!(!escrow_id.0.is_nil());
    }

    #[tokio::test]
    async fn test_full_match_flow() {
        let arena = InMemoryArena::new();
        let creator = AgentId::new();
        let p1 = AgentId::new();
        let p2 = AgentId::new();

        // Create match
        let arena_match = arena
            .create_challenge(
                creator,
                test_challenge(),
                Amount::iusd(100.0),
                10,
                chrono::Utc::now(),
            )
            .await
            .unwrap();

        // Join
        arena
            .join_challenge(p1.clone(), arena_match.id.clone(), Amount::iusd(100.0))
            .await
            .unwrap();
        arena
            .join_challenge(p2.clone(), arena_match.id.clone(), Amount::iusd(150.0))
            .await
            .unwrap();

        // Start
        arena.start_match(&arena_match.id).await.unwrap();

        // Submit actions
        arena
            .submit_action(
                &arena_match.id,
                &p1,
                "trade".to_string(),
                serde_json::json!({"pair": "ETH/IUSD", "amount": 100}),
            )
            .await
            .unwrap();

        arena
            .submit_action(
                &arena_match.id,
                &p2,
                "trade".to_string(),
                serde_json::json!({"pair": "ETH/IUSD", "amount": 200}),
            )
            .await
            .unwrap();

        // Execute
        let results = arena.execute_challenge(&arena_match.id).await.unwrap();

        assert!(results.winner.is_some());
        assert!(!results.rankings.is_empty());
        assert_eq!(results.stats.total_actions, 2);
    }

    #[tokio::test]
    async fn test_leaderboard() {
        let arena = InMemoryArena::new();

        let leaderboard = arena
            .get_leaderboard(None, Timeframe::AllTime)
            .await
            .unwrap();

        assert_eq!(leaderboard.timeframe, Timeframe::AllTime);
    }
}
