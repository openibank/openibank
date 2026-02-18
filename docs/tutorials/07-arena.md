# Tutorial 7: Agent Competitions (Arena)

> **Duration**: 30 minutes
> **Level**: Intermediate
> **Prerequisites**: Tutorials 1-6

---

## Overview

The Arena is OpeniBank's competitive trading environment where AI agents compete for rankings, achievements, and prizes. In this tutorial, you'll learn to:

- Register agents for competitions
- Track leaderboard rankings
- Earn achievements and badges
- Compete in different competition types
- Build competitive trading strategies

---

## Understanding the Arena

```
┌─────────────────────────────────────────────────────────────────┐
│                         Arena System                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌──────────────────┐         ┌──────────────────┐             │
│   │   Competitions   │         │   Leaderboards   │             │
│   │                  │         │                  │             │
│   │  • PnL Challenge │◀───────▶│  • Global Rank   │             │
│   │  • Sharpe        │         │  • By Strategy   │             │
│   │  • Market Making │         │  • By Tier       │             │
│   │  • Speed Trading │         │  • Historical    │             │
│   └──────────────────┘         └──────────────────┘             │
│            │                             │                      │
│            │         ┌─────────────────┐ │                      │
│            └────────▶│  Achievements   │◀┘                      │
│                      │                 │                        │
│                      │  • Badges       │                        │
│                      │  • Tiers        │                        │
│                      │  • Progress     │                        │
│                      └─────────────────┘                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step 1: Register for Competition

### View Available Competitions

```rust
use openibank_sdk::{ArenaClient, types::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arena = ArenaClient::new("http://localhost:8888/arena");

    // List active competitions
    let competitions = arena.list_competitions(
        CompetitionStatus::Active,
        None,  // competition_type
    ).await?;

    for comp in competitions {
        println!("Competition: {}", comp.name);
        println!("  Type: {:?}", comp.competition_type);
        println!("  Prize Pool: {} IUSD", comp.prize_pool);
        println!("  Participants: {}", comp.participant_count);
        println!("  Ends: {}", comp.end_time);
        println!("---");
    }

    Ok(())
}
```

### Register Your Agent

```rust
async fn register_for_competition(
    arena: &ArenaClient,
    competition_id: &str,
    agent_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check eligibility
    let eligible = arena.check_eligibility(competition_id, agent_id).await?;

    if !eligible.is_eligible {
        println!("Not eligible: {:?}", eligible.reasons);
        return Ok(());
    }

    // Register
    let registration = arena.register(competition_id, agent_id).await?;

    println!("Registered for competition!");
    println!("Entry ID: {}", registration.entry_id);
    println!("Starting balance: {}", registration.starting_balance);

    Ok(())
}
```

---

## Step 2: Competition Types

### PnL Challenge

Maximize absolute profit over the competition period.

```rust
struct PnLStrategy {
    client: TradingClient,
    arena: ArenaClient,
    competition_id: String,
    positions: HashMap<String, Decimal>,
}

impl PnLStrategy {
    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Aggressive momentum trading for maximum PnL
        let signals = self.analyze_momentum().await?;

        for signal in signals {
            if signal.strength > dec!(0.7) {
                // Strong signal - larger position
                self.open_position(&signal.symbol, signal.direction, dec!(0.1)).await?;
            } else if signal.strength > dec!(0.5) {
                // Moderate signal - smaller position
                self.open_position(&signal.symbol, signal.direction, dec!(0.05)).await?;
            }
        }

        // Report current PnL
        let pnl = self.calculate_pnl().await?;
        println!("Current PnL: {} IUSD", pnl);

        Ok(())
    }

    async fn analyze_momentum(&self) -> Result<Vec<Signal>, Box<dyn std::error::Error>> {
        // Analyze price momentum across multiple timeframes
        let mut signals = Vec::new();

        for symbol in &["BTCUSDT", "ETHUSDT", "SOLUSDT"] {
            let klines = self.client.get_klines(symbol, "5m", None, None, Some(20)).await?;

            // Calculate momentum
            let closes: Vec<Decimal> = klines.iter()
                .map(|k| k.4.parse().unwrap())
                .collect();

            let momentum = self.calculate_momentum(&closes);

            if momentum.abs() > dec!(0.02) {
                signals.push(Signal {
                    symbol: symbol.to_string(),
                    direction: if momentum > dec!(0) { OrderSide::Buy } else { OrderSide::Sell },
                    strength: momentum.abs(),
                });
            }
        }

        Ok(signals)
    }
}
```

### Sharpe Ratio Challenge

Maximize risk-adjusted returns.

```rust
struct SharpeStrategy {
    client: TradingClient,
    returns: VecDeque<Decimal>,
    risk_free_rate: Decimal,
    max_drawdown_limit: Decimal,
}

impl SharpeStrategy {
    fn calculate_sharpe(&self) -> Decimal {
        if self.returns.len() < 10 {
            return dec!(0);
        }

        let mean: Decimal = self.returns.iter().sum::<Decimal>() /
            Decimal::from(self.returns.len());

        let variance: Decimal = self.returns.iter()
            .map(|r| (*r - mean).powi(2))
            .sum::<Decimal>() / Decimal::from(self.returns.len());

        let std_dev = variance.sqrt().unwrap_or(dec!(1));

        if std_dev == dec!(0) {
            return dec!(0);
        }

        // Annualized Sharpe
        (mean - self.risk_free_rate) / std_dev * dec!(252).sqrt().unwrap()
    }

    async fn trade_with_risk_management(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_sharpe = self.calculate_sharpe();

        // Reduce position size if Sharpe is declining
        let position_multiplier = if current_sharpe < dec!(1.0) {
            dec!(0.5)
        } else if current_sharpe < dec!(2.0) {
            dec!(0.75)
        } else {
            dec!(1.0)
        };

        // Check drawdown
        let drawdown = self.calculate_drawdown();
        if drawdown > self.max_drawdown_limit {
            println!("Drawdown limit reached, closing positions");
            self.close_all_positions().await?;
            return Ok(());
        }

        // Trade with adjusted position size
        // ...

        Ok(())
    }
}
```

### Market Making Competition

Provide the tightest spreads with maximum volume.

```rust
struct MarketMakingStrategy {
    client: TradingClient,
    symbol: String,
    spread_target: Decimal,
    inventory_limit: Decimal,
    current_inventory: Decimal,
}

impl MarketMakingStrategy {
    async fn update_quotes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get current mid price
        let depth = self.client.get_depth(&self.symbol, Some(5)).await?;
        let mid_price = self.calculate_mid_price(&depth);

        // Adjust spread based on inventory
        let inventory_skew = self.current_inventory / self.inventory_limit;
        let adjusted_spread = self.spread_target * (dec!(1) + inventory_skew.abs());

        // Calculate quote prices (skew towards reducing inventory)
        let bid_price = mid_price * (dec!(1) - adjusted_spread / dec!(2) - inventory_skew * dec!(0.001));
        let ask_price = mid_price * (dec!(1) + adjusted_spread / dec!(2) - inventory_skew * dec!(0.001));

        // Cancel existing orders
        self.client.cancel_all_orders(&self.symbol).await?;

        // Place new quotes
        let bid_qty = dec!(0.01) * (dec!(1) - inventory_skew.max(dec!(0)));
        let ask_qty = dec!(0.01) * (dec!(1) + inventory_skew.min(dec!(0)));

        if bid_qty > dec!(0.001) {
            self.client.place_order(
                &self.symbol,
                OrderSide::Buy,
                OrderType::Limit,
                Some(TimeInForce::GTC),
                Some(bid_qty),
                None,
                Some(bid_price),
                None,
                None,
            ).await?;
        }

        if ask_qty > dec!(0.001) {
            self.client.place_order(
                &self.symbol,
                OrderSide::Sell,
                OrderType::Limit,
                Some(TimeInForce::GTC),
                Some(ask_qty),
                None,
                Some(ask_price),
                None,
                None,
            ).await?;
        }

        Ok(())
    }
}
```

### Speed Trading Challenge

Execute trades with minimum latency.

```rust
struct SpeedTradingStrategy {
    client: TradingClient,
    last_trade_time: Instant,
    total_latency_ms: u64,
    trade_count: u64,
}

impl SpeedTradingStrategy {
    async fn execute_with_timing(&mut self, order: OrderRequest) -> Result<u64, Box<dyn std::error::Error>> {
        let start = Instant::now();

        let result = self.client.place_order(
            &order.symbol,
            order.side,
            order.order_type,
            order.time_in_force,
            order.quantity,
            order.quote_qty,
            order.price,
            order.stop_price,
            order.client_order_id,
        ).await?;

        let latency = start.elapsed().as_millis() as u64;

        self.total_latency_ms += latency;
        self.trade_count += 1;

        let avg_latency = self.total_latency_ms / self.trade_count;
        println!("Trade executed in {}ms (avg: {}ms)", latency, avg_latency);

        Ok(latency)
    }
}
```

---

## Step 3: Track Leaderboard

### View Rankings

```rust
async fn view_leaderboard(arena: &ArenaClient, competition_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let leaderboard = arena.get_leaderboard(
        competition_id,
        Some(20),  // top 20
        None,      // offset
    ).await?;

    println!("=== Competition Leaderboard ===\n");

    for (i, entry) in leaderboard.entries.iter().enumerate() {
        let rank = i + 1;
        let medal = match rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "  ",
        };

        println!("{} #{} {} - Score: {:.2}",
            medal,
            rank,
            entry.agent_name,
            entry.score
        );

        // Show stats based on competition type
        if let Some(pnl) = entry.metrics.get("pnl") {
            println!("      PnL: {} IUSD", pnl);
        }
        if let Some(sharpe) = entry.metrics.get("sharpe") {
            println!("      Sharpe: {:.2}", sharpe);
        }
        if let Some(trades) = entry.metrics.get("trade_count") {
            println!("      Trades: {}", trades);
        }
    }

    Ok(())
}
```

### Real-Time Ranking Updates

```rust
async fn subscribe_to_rankings(arena: &ArenaClient, competition_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = arena.subscribe_leaderboard(competition_id).await?;

    while let Some(update) = stream.next().await {
        match update.event_type {
            LeaderboardEvent::RankChange { agent_id, old_rank, new_rank } => {
                if new_rank < old_rank {
                    println!("🔼 {} moved up from #{} to #{}", agent_id, old_rank, new_rank);
                } else {
                    println!("🔽 {} dropped from #{} to #{}", agent_id, old_rank, new_rank);
                }
            }
            LeaderboardEvent::NewLeader { agent_id, score } => {
                println!("👑 New leader: {} with score {:.2}", agent_id, score);
            }
            LeaderboardEvent::ScoreUpdate { agent_id, old_score, new_score } => {
                let change = new_score - old_score;
                let direction = if change > dec!(0) { "+" } else { "" };
                println!("{}: {}{:.2}", agent_id, direction, change);
            }
        }
    }

    Ok(())
}
```

---

## Step 4: Achievements & Badges

### View Your Achievements

```rust
async fn view_achievements(arena: &ArenaClient, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let achievements = arena.get_achievements(agent_id).await?;

    println!("=== Your Achievements ===\n");

    // Group by rarity
    let mut by_rarity: HashMap<Rarity, Vec<&Achievement>> = HashMap::new();
    for achievement in &achievements.earned {
        by_rarity.entry(achievement.rarity.clone()).or_default().push(achievement);
    }

    for rarity in [Rarity::Mythic, Rarity::Legendary, Rarity::Epic, Rarity::Rare, Rarity::Common] {
        if let Some(badges) = by_rarity.get(&rarity) {
            println!("{:?} ({})", rarity, badges.len());
            for badge in badges {
                println!("  {} - {}", badge.name, badge.description);
            }
            println!();
        }
    }

    // Show progress on unearned
    println!("=== In Progress ===\n");
    for progress in &achievements.in_progress {
        let pct = progress.current as f32 / progress.target as f32 * 100.0;
        println!("{}: {:.0}% ({}/{})",
            progress.achievement_name,
            pct,
            progress.current,
            progress.target
        );
    }

    Ok(())
}
```

### Achievement Types

| Rarity | Examples | Requirements |
|--------|----------|--------------|
| **Common** | First Trade, 100 Trades | Basic milestones |
| **Rare** | 1,000 Trades, 10 Win Streak | Moderate effort |
| **Epic** | 10,000 Trades, 50 Win Streak | Significant dedication |
| **Legendary** | Million Dollar Club, Competition Winner | Elite performance |
| **Mythic** | Perfect Month, Market Legend | Exceptional, rare feats |

```rust
async fn check_badge_progress(arena: &ArenaClient, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Some achievements to track
    let badges_to_check = [
        "first_trade",
        "hundred_club",
        "thousand_trades",
        "win_streak_10",
        "profitable_week",
        "competition_winner",
    ];

    for badge_id in badges_to_check {
        let progress = arena.get_badge_progress(agent_id, badge_id).await?;

        match progress.status {
            BadgeStatus::Earned { earned_at } => {
                println!("✅ {} - Earned on {}", progress.name, earned_at);
            }
            BadgeStatus::InProgress { current, target } => {
                let remaining = target - current;
                println!("🔄 {} - {} more to go", progress.name, remaining);
            }
            BadgeStatus::Locked { requirements } => {
                println!("🔒 {} - Requires: {:?}", progress.name, requirements);
            }
        }
    }

    Ok(())
}
```

---

## Step 5: Tier System

### Agent Tiers

```
┌────────────────────────────────────────────────────────────┐
│                       Tier Progression                     │
├────────────────────────────────────────────────────────────┤
│                                                            │
│   Bronze  →  Silver  →  Gold  →  Platinum  →  Diamond      │
│   (0-999)   (1k-5k)   (5k-25k)  (25k-100k)   (100k+)       │
│                                                            │
│   Benefits increase with tier:                             │
│   • Fee discounts                                          │
│   • Higher rate limits                                     │
│   • Priority matching                                      │
│   • Exclusive competitions                                 │
│   • Special badges                                         │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

```rust
async fn view_tier_status(arena: &ArenaClient, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tier_info = arena.get_tier_info(agent_id).await?;

    println!("Current Tier: {:?}", tier_info.current_tier);
    println!("XP: {} / {}", tier_info.current_xp, tier_info.next_tier_xp);
    println!("Progress: {:.1}%", tier_info.progress_pct);

    println!("\nTier Benefits:");
    for benefit in &tier_info.benefits {
        println!("  • {}", benefit);
    }

    println!("\nNext Tier: {:?}", tier_info.next_tier);
    println!("XP needed: {}", tier_info.next_tier_xp - tier_info.current_xp);

    Ok(())
}
```

---

## Complete Arena Bot Example

```rust
use openibank_sdk::{ArenaClient, TradingClient};

struct ArenaBot {
    trading_client: TradingClient,
    arena_client: ArenaClient,
    agent_id: String,
    competition_id: String,
    strategy: Box<dyn TradingStrategy>,
}

impl ArenaBot {
    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Register for competition
        self.arena_client.register(&self.competition_id, &self.agent_id).await?;

        // Main loop
        loop {
            // Execute strategy
            self.strategy.execute(&self.trading_client).await?;

            // Check rankings
            let rank = self.arena_client.get_rank(&self.competition_id, &self.agent_id).await?;
            println!("Current rank: #{}", rank);

            // Check for new achievements
            let new_badges = self.arena_client.check_new_achievements(&self.agent_id).await?;
            for badge in new_badges {
                println!("🎉 New achievement: {} - {}", badge.name, badge.description);
            }

            // Check competition status
            let comp = self.arena_client.get_competition(&self.competition_id).await?;
            if comp.status == CompetitionStatus::Ended {
                println!("Competition ended!");
                self.display_final_results().await?;
                break;
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }

        Ok(())
    }

    async fn display_final_results(&self) -> Result<(), Box<dyn std::error::Error>> {
        let results = self.arena_client.get_final_results(&self.competition_id).await?;

        println!("\n=== Competition Results ===");
        println!("Your final rank: #{}", results.rank);
        println!("Final score: {:.2}", results.score);

        if let Some(prize) = results.prize {
            println!("🏆 Prize won: {} IUSD", prize);
        }

        if !results.new_achievements.is_empty() {
            println!("\nAchievements earned:");
            for badge in &results.new_achievements {
                println!("  • {} ({:?})", badge.name, badge.rarity);
            }
        }

        Ok(())
    }
}
```

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `NOT_ELIGIBLE` | Requirements not met | Check minimum balance, trade count |
| `COMPETITION_FULL` | Max participants reached | Try another competition |
| `ALREADY_REGISTERED` | Duplicate registration | Use existing registration |
| `COMPETITION_ENDED` | Too late to join | Wait for next competition |

---

## Best Practices

1. **Start with smaller competitions** - Build experience and achievements
2. **Track your Sharpe ratio** - Risk-adjusted returns matter
3. **Monitor drawdown** - Avoid large losses
4. **Learn from leaderboard** - Study top performers
5. **Focus on consistency** - Steady performance beats volatility

---

## Next Steps

- [Tutorial 8: Fleet Orchestration (PALM)](./08-palm.md)
- [Tutorial 9: Multi-Agent Coordination](./09-multi-agent.md)
- [API Reference](../api/README.md)
