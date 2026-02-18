//! Arena Mode — 4-bot trading competition with scoring and champion receipt SVG.
//!
//! ## How It Works
//!
//! 1. Four bots (Aggressive, Conservative, Arbitrageur, Market Maker) each
//!    trade against a simulated order book for `n_rounds` rounds.
//! 2. Each trade produces a score contribution based on PnL, speed, and risk.
//! 3. The highest-scoring bot is crowned champion and receives a
//!    special SVG receipt card.
//!
//! ## Scoring
//!
//! ```text
//! score = pnl_score * 0.5 + speed_score * 0.3 + risk_score * 0.2
//! ```
//!
//! All values are normalized to [0, 100].

use crate::{AgentId, IusdAmount, Receipt};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::Ulid;

// ── Bot strategies ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BotStrategy {
    /// Aggressive: large trades, high risk, high reward.
    Aggressive,
    /// Conservative: small trades, low risk, steady gain.
    Conservative,
    /// Arbitrageur: captures spread; depends on market volatility.
    Arbitrageur,
    /// MarketMaker: earns spread on both sides.
    MarketMaker,
}

impl BotStrategy {
    pub fn all() -> [BotStrategy; 4] {
        [
            BotStrategy::Aggressive,
            BotStrategy::Conservative,
            BotStrategy::Arbitrageur,
            BotStrategy::MarketMaker,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BotStrategy::Aggressive  => "Aggressive",
            BotStrategy::Conservative => "Conservative",
            BotStrategy::Arbitrageur => "Arbitrageur",
            BotStrategy::MarketMaker => "MarketMaker",
        }
    }

    /// Base PnL per round (microdollars) — deterministic, no RNG.
    fn base_pnl_per_round(&self) -> i128 {
        match self {
            BotStrategy::Aggressive   => 120_000,   // $0.12/round, high variance
            BotStrategy::Conservative =>  45_000,   // $0.045/round, low variance
            BotStrategy::Arbitrageur  =>  80_000,   // $0.08/round
            BotStrategy::MarketMaker  =>  60_000,   // $0.06/round, very stable
        }
    }

    /// Variance multiplier for each round (deterministic from round index).
    fn round_pnl(&self, round: u32) -> i128 {
        let base = self.base_pnl_per_round();
        // Deterministic variance based on strategy and round
        let var = match self {
            BotStrategy::Aggressive => {
                // Volatile: +/- 60% based on round parity
                let sign: i128 = if round % 3 == 0 { -1 } else { 1 };
                sign * (base * 60 / 100) * (1 + (round as i128 % 5))
            }
            BotStrategy::Conservative => base / 10,
            BotStrategy::Arbitrageur => {
                let sign: i128 = if round % 7 == 0 { -1 } else { 1 };
                sign * base * 30 / 100
            }
            BotStrategy::MarketMaker => base * 5 / 100,
        };
        base + var
    }

    /// Risk score per round (0 = safest, 100 = riskiest).
    fn risk_score(&self) -> u32 {
        match self {
            BotStrategy::Aggressive   => 85,
            BotStrategy::Conservative => 20,
            BotStrategy::Arbitrageur  => 55,
            BotStrategy::MarketMaker  => 35,
        }
    }

    /// Speed score (0-100). Aggressive trades fastest.
    fn speed_score(&self) -> u32 {
        match self {
            BotStrategy::Aggressive   => 90,
            BotStrategy::Conservative => 40,
            BotStrategy::Arbitrageur  => 75,
            BotStrategy::MarketMaker  => 60,
        }
    }
}

// ── Arena result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotResult {
    pub strategy: BotStrategy,
    pub agent_id: AgentId,
    pub total_pnl: IusdAmount,
    pub gross_pnl_micros: i128,
    pub rounds_won: u32,
    pub pnl_score: f64,    // 0-100
    pub speed_score: f64,  // 0-100
    pub risk_score: f64,   // 0-100 (inverted: lower risk → higher score)
    pub composite_score: f64, // weighted composite
    pub rank: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaResult {
    pub arena_id: String,
    pub n_rounds: u32,
    pub results: Vec<BotResult>,
    pub champion: BotStrategy,
    pub champion_agent_id: AgentId,
    pub champion_receipt: Receipt,
    pub run_at: chrono::DateTime<Utc>,
}

// ── Arena engine ──────────────────────────────────────────────────────────────

/// Run a deterministic 4-bot arena for `n_rounds` rounds.
///
/// Returns an `ArenaResult` with full per-bot stats and a champion receipt.
pub fn run_arena(n_rounds: u32) -> ArenaResult {
    let arena_id = format!("arena_{}", Ulid::new());
    let strategies = BotStrategy::all();

    // Assign agent IDs
    let agents: HashMap<BotStrategy, AgentId> = strategies.iter()
        .map(|&s| (s, AgentId::new(s.name())))
        .collect();

    // Simulate each bot's PnL across rounds
    let mut gross_pnls: HashMap<BotStrategy, i128> = HashMap::new();
    let mut rounds_won: HashMap<BotStrategy, u32> = HashMap::new();

    for round in 0..n_rounds {
        let round_pnls: Vec<(BotStrategy, i128)> = strategies.iter()
            .map(|&s| (s, s.round_pnl(round)))
            .collect();

        // Record gross PnL
        for &(strat, pnl) in &round_pnls {
            *gross_pnls.entry(strat).or_insert(0) += pnl;
        }

        // Determine winner of this round
        if let Some(&(winner, _)) = round_pnls.iter().max_by_key(|&&(_, p)| p) {
            *rounds_won.entry(winner).or_insert(0) += 1;
        }
    }

    // Compute normalized scores
    let max_pnl = gross_pnls.values().cloned().max().unwrap_or(1).max(1) as f64;

    let mut bot_results: Vec<BotResult> = strategies.iter().map(|&s| {
        let gross = *gross_pnls.get(&s).unwrap_or(&0);
        let pnl_score = ((gross as f64 / max_pnl) * 100.0).max(0.0).min(100.0);
        let speed = s.speed_score() as f64;
        let risk_score_raw = s.risk_score() as f64;
        let risk_score = 100.0 - risk_score_raw; // lower risk = higher score

        // Composite: 50% PnL, 30% speed, 20% risk
        let composite = pnl_score * 0.50 + speed * 0.30 + risk_score * 0.20;

        BotResult {
            strategy: s,
            agent_id: agents[&s].clone(),
            total_pnl: IusdAmount::from_micros(gross.unsigned_abs()),
            gross_pnl_micros: gross,
            rounds_won: *rounds_won.get(&s).unwrap_or(&0),
            pnl_score,
            speed_score: speed,
            risk_score,
            composite_score: composite,
            rank: 0, // filled below
        }
    }).collect();

    // Rank bots by composite score (descending)
    bot_results.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());
    for (i, r) in bot_results.iter_mut().enumerate() {
        r.rank = (i + 1) as u32;
    }

    let champion = bot_results[0].strategy;
    let champion_agent_id = bot_results[0].agent_id.clone();

    // Generate champion receipt
    let champion_pnl = bot_results[0].total_pnl;
    let champion_receipt = Receipt::new_unsigned(
        AgentId::new("arena-issuer"),
        champion_agent_id.clone(),
        champion_pnl.micros() as i64,
        &format!("perm_{}", arena_id),
        &format!("commit_{}", arena_id),
        &format!("wl:{}", arena_id),
        &format!("wll_evt_{}", Ulid::new()),
        &format!("{}", blake3::hash(arena_id.as_bytes())),
        &format!(
            "Arena Champion: {} | Score: {:.1} | {} rounds won",
            champion.name(),
            bot_results[0].composite_score,
            bot_results[0].rounds_won,
        ),
    );

    ArenaResult {
        arena_id,
        n_rounds,
        results: bot_results,
        champion,
        champion_agent_id,
        champion_receipt,
        run_at: Utc::now(),
    }
}

// ── Champion SVG receipt ───────────────────────────────────────────────────────

/// Generate a special champion SVG receipt card for the arena winner.
///
/// Adds a golden trophy badge and the leaderboard table.
pub fn render_champion_svg(arena: &ArenaResult) -> String {
    use crate::card::render_svg;

    // First render the standard receipt SVG
    let base_svg = render_svg(&arena.champion_receipt);

    // Build leaderboard rows
    let mut rows = String::new();
    for r in &arena.results {
        let medal = match r.rank { 1 => "🥇", 2 => "🥈", 3 => "🥉", _ => "  " };
        let pnl_str = r.total_pnl.to_display_string();
        let fill_color = if r.rank == 1 { "#ffd700" } else { "#c8d0e0" };
        rows.push_str(&format!(
            "  <text x=\"20\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{} {:13} score:{:5.1}  pnl:{}</text>\n",
            300 + (r.rank as i32 - 1) * 18,
            fill_color,
            medal,
            r.strategy.name(),
            r.composite_score,
            pnl_str,
        ));
    }

    // Inject leaderboard into SVG before closing tag
    let leaderboard = format!(
        r##"  <rect x="0" y="280" width="640" height="100" fill="#0f1729" opacity="0.9"/>
  <text x="20" y="296" font-family="monospace" font-size="11" fill="#4a5568">ARENA LEADERBOARD — {} rounds</text>
{}  <text x="20" y="380" font-family="monospace" font-size="9" fill="#4a5568">openibank.com/arena/{}</text>
</svg>"##,
        arena.n_rounds,
        rows,
        &arena.arena_id[..16.min(arena.arena_id.len())],
    );

    base_svg.replace("</svg>", &leaderboard)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_produces_four_results() {
        let arena = run_arena(10);
        assert_eq!(arena.results.len(), 4);
    }

    #[test]
    fn champion_is_rank_one() {
        let arena = run_arena(20);
        assert_eq!(arena.results[0].rank, 1);
        assert_eq!(arena.results[0].strategy, arena.champion);
    }

    #[test]
    fn ranks_are_1_through_4() {
        let arena = run_arena(5);
        let mut ranks: Vec<u32> = arena.results.iter().map(|r| r.rank).collect();
        ranks.sort();
        assert_eq!(ranks, vec![1, 2, 3, 4]);
    }

    #[test]
    fn composite_scores_monotonically_decrease() {
        let arena = run_arena(50);
        for w in arena.results.windows(2) {
            assert!(w[0].composite_score >= w[1].composite_score,
                "Rank {} score {} should be >= rank {} score {}",
                w[0].rank, w[0].composite_score, w[1].rank, w[1].composite_score);
        }
    }

    #[test]
    fn champion_receipt_has_tagline() {
        let arena = run_arena(10);
        assert!(arena.champion_receipt.tagline.contains("Arena Champion"));
        assert!(arena.champion_receipt.tagline.contains(arena.champion.name()));
    }

    #[test]
    fn champion_svg_contains_leaderboard() {
        let arena = run_arena(10);
        let svg = render_champion_svg(&arena);
        assert!(svg.contains("ARENA LEADERBOARD"), "SVG must contain leaderboard");
        assert!(svg.contains("<svg"), "Must be valid SVG");
    }

    #[test]
    fn deterministic_results_for_same_rounds() {
        let a1 = run_arena(15);
        let a2 = run_arena(15);
        assert_eq!(a1.champion, a2.champion);
        assert_eq!(a1.results[0].strategy, a2.results[0].strategy);
    }

    #[test]
    fn all_pnl_scores_in_0_100() {
        let arena = run_arena(20);
        for r in &arena.results {
            assert!(r.pnl_score >= 0.0 && r.pnl_score <= 100.0,
                "pnl_score {} out of range", r.pnl_score);
            assert!(r.composite_score >= 0.0 && r.composite_score <= 100.0,
                "composite_score {} out of range", r.composite_score);
        }
    }
}
