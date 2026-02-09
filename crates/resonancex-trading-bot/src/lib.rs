//! ResonanceX Trading Bot - Client Library for Trading Bots
//!
//! This crate provides a client library for building trading bots that
//! interact with the ResonanceX exchange. It includes utilities for
//! order management, market data consumption, and strategy execution.
//!
//! # Features
//!
//! - **REST Client**: HTTP client for the ResonanceX API
//! - **Order Management**: Submit, cancel, and track orders
//! - **Market Data**: Subscribe to real-time market data
//! - **Strategy Framework**: Base traits for implementing trading strategies
//! - **Risk Management**: Position limits, stop-losses, and risk controls
//!
//! # Example
//!
//! ```ignore
//! use resonancex_trading_bot::{TradingBot, Strategy, BotConfig};
//!
//! struct MyStrategy;
//!
//! #[async_trait]
//! impl Strategy for MyStrategy {
//!     async fn on_tick(&mut self, bot: &TradingBot) -> BotResult<()> {
//!         let ticker = bot.get_ticker(&MarketId::new("ETH_IUSD")).await?;
//!
//!         // Simple strategy: buy if price dropped more than 1%
//!         if ticker.change_24h < dec!(-1.0) {
//!             bot.submit_order(Order::market_buy("ETH_IUSD", dec!(0.1))).await?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! let bot = TradingBot::new(config);
//! bot.run(MyStrategy).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export core types
pub use resonancex_types::{
    MarketId, OrderId, Order, Trade, Side, OrderType, OrderStatus,
    Ticker, DepthSnapshot, Candle, CandleInterval,
};

/// Bot errors
#[derive(Debug, Error)]
pub enum BotError {
    #[error("API error: {0}")]
    Api(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Order rejected: {0}")]
    OrderRejected(String),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Risk limit exceeded: {0}")]
    RiskLimit(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type for bot operations
pub type BotResult<T> = Result<T, BotError>;

/// Bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// API endpoint
    pub api_url: String,
    /// WebSocket endpoint
    pub ws_url: String,
    /// API key
    pub api_key: Option<String>,
    /// API secret
    pub api_secret: Option<String>,
    /// Markets to trade
    pub markets: Vec<MarketId>,
    /// Tick interval in milliseconds
    pub tick_interval_ms: u64,
    /// Maximum position size per market
    pub max_position: Decimal,
    /// Maximum orders per market
    pub max_orders: usize,
    /// Enable paper trading (no real orders)
    pub paper_trading: bool,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8888".to_string(),
            ws_url: "ws://localhost:8888/ws".to_string(),
            api_key: None,
            api_secret: None,
            markets: vec![
                MarketId::new("BTC_IUSD"),
                MarketId::new("ETH_IUSD"),
                MarketId::new("SOL_IUSD"),
                MarketId::new("OBK_IUSD"),
            ],
            tick_interval_ms: 1000,
            max_position: dec!(10),
            max_orders: 10,
            paper_trading: true,
        }
    }
}

/// Position tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Market
    pub market: MarketId,
    /// Position size (positive = long, negative = short)
    pub size: Decimal,
    /// Average entry price
    pub entry_price: Decimal,
    /// Unrealized PnL
    pub unrealized_pnl: Decimal,
    /// Realized PnL
    pub realized_pnl: Decimal,
}

impl Position {
    /// Create a new position
    pub fn new(market: MarketId) -> Self {
        Self {
            market,
            size: Decimal::ZERO,
            entry_price: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
        }
    }

    /// Update position with a fill
    pub fn update(&mut self, side: Side, amount: Decimal, price: Decimal) {
        let signed_amount = match side {
            Side::Buy => amount,
            Side::Sell => -amount,
        };

        let old_size = self.size;
        self.size += signed_amount;

        // Calculate entry price using weighted average
        if old_size.is_zero() {
            self.entry_price = price;
        } else if (old_size > Decimal::ZERO) == (signed_amount > Decimal::ZERO) {
            // Adding to position
            let total_value = old_size * self.entry_price + signed_amount.abs() * price;
            self.entry_price = total_value / self.size.abs();
        } else {
            // Reducing position - realize PnL
            let closed_amount = signed_amount.abs().min(old_size.abs());
            let pnl = closed_amount * (price - self.entry_price) * if old_size > Decimal::ZERO { dec!(1) } else { dec!(-1) };
            self.realized_pnl += pnl;
        }
    }

    /// Calculate unrealized PnL at current price
    pub fn calculate_unrealized_pnl(&mut self, current_price: Decimal) {
        if !self.size.is_zero() {
            self.unrealized_pnl = self.size * (current_price - self.entry_price);
        } else {
            self.unrealized_pnl = Decimal::ZERO;
        }
    }

    /// Get total PnL
    pub fn total_pnl(&self) -> Decimal {
        self.realized_pnl + self.unrealized_pnl
    }

    /// Check if position is long
    pub fn is_long(&self) -> bool {
        self.size > Decimal::ZERO
    }

    /// Check if position is short
    pub fn is_short(&self) -> bool {
        self.size < Decimal::ZERO
    }

    /// Check if position is flat
    pub fn is_flat(&self) -> bool {
        self.size.is_zero()
    }
}

/// Order request builder
#[derive(Debug, Clone)]
pub struct OrderRequest {
    /// Market
    pub market: MarketId,
    /// Side
    pub side: Side,
    /// Order type
    pub order_type: OrderType,
    /// Amount
    pub amount: Decimal,
    /// Client order ID
    pub client_order_id: Option<String>,
}

impl OrderRequest {
    /// Create a limit buy order
    pub fn limit_buy(market: impl Into<String>, price: Decimal, amount: Decimal) -> Self {
        Self {
            market: MarketId::new(market),
            side: Side::Buy,
            order_type: OrderType::limit(price),
            amount,
            client_order_id: None,
        }
    }

    /// Create a limit sell order
    pub fn limit_sell(market: impl Into<String>, price: Decimal, amount: Decimal) -> Self {
        Self {
            market: MarketId::new(market),
            side: Side::Sell,
            order_type: OrderType::limit(price),
            amount,
            client_order_id: None,
        }
    }

    /// Create a market buy order
    pub fn market_buy(market: impl Into<String>, amount: Decimal) -> Self {
        Self {
            market: MarketId::new(market),
            side: Side::Buy,
            order_type: OrderType::Market,
            amount,
            client_order_id: None,
        }
    }

    /// Create a market sell order
    pub fn market_sell(market: impl Into<String>, amount: Decimal) -> Self {
        Self {
            market: MarketId::new(market),
            side: Side::Sell,
            order_type: OrderType::Market,
            amount,
            client_order_id: None,
        }
    }

    /// Set client order ID
    pub fn with_client_id(mut self, id: impl Into<String>) -> Self {
        self.client_order_id = Some(id.into());
        self
    }
}

/// Strategy trait for implementing trading strategies
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Called on each tick
    async fn on_tick(&mut self, context: &StrategyContext) -> BotResult<Vec<OrderRequest>>;

    /// Called when an order is filled
    async fn on_fill(&mut self, _context: &StrategyContext, _trade: &Trade) -> BotResult<()> {
        Ok(())
    }

    /// Called when an order is cancelled
    async fn on_cancel(&mut self, _context: &StrategyContext, _order_id: OrderId) -> BotResult<()> {
        Ok(())
    }

    /// Called on strategy start
    async fn on_start(&mut self, _context: &StrategyContext) -> BotResult<()> {
        Ok(())
    }

    /// Called on strategy stop
    async fn on_stop(&mut self, _context: &StrategyContext) -> BotResult<()> {
        Ok(())
    }
}

/// Strategy context providing access to market data and state
#[derive(Debug, Clone)]
pub struct StrategyContext {
    /// Current tickers
    pub tickers: HashMap<MarketId, Ticker>,
    /// Current positions
    pub positions: HashMap<MarketId, Position>,
    /// Open orders
    pub open_orders: HashMap<OrderId, Order>,
    /// Available balance
    pub available_balance: Decimal,
    /// Current time
    pub timestamp: DateTime<Utc>,
}

impl StrategyContext {
    /// Create a new strategy context
    pub fn new() -> Self {
        Self {
            tickers: HashMap::new(),
            positions: HashMap::new(),
            open_orders: HashMap::new(),
            available_balance: Decimal::ZERO,
            timestamp: Utc::now(),
        }
    }

    /// Get ticker for a market
    pub fn ticker(&self, market: &MarketId) -> Option<&Ticker> {
        self.tickers.get(market)
    }

    /// Get position for a market
    pub fn position(&self, market: &MarketId) -> Option<&Position> {
        self.positions.get(market)
    }

    /// Get last price for a market
    pub fn last_price(&self, market: &MarketId) -> Option<Decimal> {
        self.tickers.get(market).map(|t| t.last_price)
    }

    /// Get bid price for a market
    pub fn bid(&self, market: &MarketId) -> Option<Decimal> {
        self.tickers.get(market).map(|t| t.bid)
    }

    /// Get ask price for a market
    pub fn ask(&self, market: &MarketId) -> Option<Decimal> {
        self.tickers.get(market).map(|t| t.ask)
    }

    /// Get mid price for a market
    pub fn mid_price(&self, market: &MarketId) -> Option<Decimal> {
        self.tickers.get(market).map(|t| (t.bid + t.ask) / dec!(2))
    }

    /// Get total unrealized PnL
    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    /// Get total realized PnL
    pub fn total_realized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.realized_pnl).sum()
    }
}

impl Default for StrategyContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple grid trading strategy example
pub struct GridStrategy {
    /// Number of grid levels
    pub levels: usize,
    /// Grid spacing percentage
    pub spacing: Decimal,
    /// Order size
    pub order_size: Decimal,
    /// Center price (set on start)
    center_price: Option<Decimal>,
}

impl GridStrategy {
    /// Create a new grid strategy
    pub fn new(levels: usize, spacing: Decimal, order_size: Decimal) -> Self {
        Self {
            levels,
            spacing,
            order_size,
            center_price: None,
        }
    }
}

#[async_trait]
impl Strategy for GridStrategy {
    async fn on_start(&mut self, context: &StrategyContext) -> BotResult<()> {
        // Set center price from current market price
        if let Some(ticker) = context.tickers.values().next() {
            self.center_price = Some(ticker.last_price);
        }
        Ok(())
    }

    async fn on_tick(&mut self, context: &StrategyContext) -> BotResult<Vec<OrderRequest>> {
        let mut orders = Vec::new();

        // Skip if no center price set
        let center = match self.center_price {
            Some(p) => p,
            None => return Ok(orders),
        };

        // For each market, place grid orders if none exist
        for (market, _ticker) in &context.tickers {
            // Skip if we already have orders
            if context.open_orders.values().any(|o| &o.market == market) {
                continue;
            }

            // Place buy orders below center
            for i in 1..=self.levels {
                let price = center * (Decimal::ONE - self.spacing * Decimal::from(i as u32));
                orders.push(OrderRequest::limit_buy(market.0.clone(), price, self.order_size));
            }

            // Place sell orders above center
            for i in 1..=self.levels {
                let price = center * (Decimal::ONE + self.spacing * Decimal::from(i as u32));
                orders.push(OrderRequest::limit_sell(market.0.clone(), price, self.order_size));
            }
        }

        Ok(orders)
    }

    async fn on_fill(&mut self, _context: &StrategyContext, trade: &Trade) -> BotResult<()> {
        // Update center price to trade price
        self.center_price = Some(trade.price);
        Ok(())
    }
}

// ============================================================================
// Market Maker Strategy
// ============================================================================

/// Professional market making strategy with inventory management
pub struct MarketMakerStrategy {
    /// Target spread (as a percentage, e.g., 0.002 = 0.2%)
    pub target_spread: Decimal,
    /// Order size per level
    pub order_size: Decimal,
    /// Number of price levels on each side
    pub levels: usize,
    /// Level spacing (as percentage of mid price)
    pub level_spacing: Decimal,
    /// Maximum inventory (absolute value)
    pub max_inventory: Decimal,
    /// Inventory skew factor (how much to adjust quotes based on inventory)
    pub skew_factor: Decimal,
    /// Minimum edge required to quote
    pub min_edge: Decimal,
}

impl MarketMakerStrategy {
    /// Create a new market maker strategy
    pub fn new() -> Self {
        Self {
            target_spread: dec!(0.002),      // 0.2% spread
            order_size: dec!(0.1),           // 0.1 ETH per order
            levels: 3,                        // 3 levels on each side
            level_spacing: dec!(0.0005),     // 0.05% between levels
            max_inventory: dec!(2.0),         // Max 2 ETH inventory
            skew_factor: dec!(0.0001),       // Skew quotes by 0.01% per unit inventory
            min_edge: dec!(0.0001),          // Minimum 0.01% edge to quote
        }
    }

    /// Create with custom spread
    pub fn with_spread(mut self, spread: Decimal) -> Self {
        self.target_spread = spread;
        self
    }

    /// Create with custom order size
    pub fn with_order_size(mut self, size: Decimal) -> Self {
        self.order_size = size;
        self
    }

    /// Calculate inventory skew
    fn calculate_skew(&self, inventory: Decimal) -> Decimal {
        inventory * self.skew_factor
    }
}

impl Default for MarketMakerStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Strategy for MarketMakerStrategy {
    async fn on_tick(&mut self, context: &StrategyContext) -> BotResult<Vec<OrderRequest>> {
        let mut orders = Vec::new();

        for (market, ticker) in &context.tickers {
            // Get current inventory
            let inventory = context.position(market)
                .map(|p| p.size)
                .unwrap_or(Decimal::ZERO);

            // Check inventory limits
            if inventory.abs() >= self.max_inventory {
                // Only quote on the reducing side
                let mid = (ticker.bid + ticker.ask) / dec!(2);
                if inventory > Decimal::ZERO {
                    // Long: only sell
                    for i in 0..self.levels {
                        let price = mid * (Decimal::ONE + self.target_spread / dec!(2) + self.level_spacing * Decimal::from(i as u32));
                        orders.push(OrderRequest::limit_sell(market.0.clone(), price.round_dp(2), self.order_size));
                    }
                } else {
                    // Short: only buy
                    for i in 0..self.levels {
                        let price = mid * (Decimal::ONE - self.target_spread / dec!(2) - self.level_spacing * Decimal::from(i as u32));
                        orders.push(OrderRequest::limit_buy(market.0.clone(), price.round_dp(2), self.order_size));
                    }
                }
                continue;
            }

            // Calculate skewed mid price based on inventory
            let raw_mid = (ticker.bid + ticker.ask) / dec!(2);
            let skew = self.calculate_skew(inventory);
            let skewed_mid = raw_mid * (Decimal::ONE - skew);

            // Calculate bid and ask prices
            let half_spread = self.target_spread / dec!(2);

            // Place bids
            for i in 0..self.levels {
                let level_offset = self.level_spacing * Decimal::from(i as u32);
                let price = skewed_mid * (Decimal::ONE - half_spread - level_offset);
                orders.push(OrderRequest::limit_buy(market.0.clone(), price.round_dp(2), self.order_size));
            }

            // Place asks
            for i in 0..self.levels {
                let level_offset = self.level_spacing * Decimal::from(i as u32);
                let price = skewed_mid * (Decimal::ONE + half_spread + level_offset);
                orders.push(OrderRequest::limit_sell(market.0.clone(), price.round_dp(2), self.order_size));
            }
        }

        Ok(orders)
    }
}

// ============================================================================
// Trend Following Strategy
// ============================================================================

/// Trend following strategy using simple moving averages
pub struct TrendFollowingStrategy {
    /// Fast EMA period
    pub fast_period: usize,
    /// Slow EMA period
    pub slow_period: usize,
    /// Position size
    pub position_size: Decimal,
    /// Historical prices (circular buffer)
    prices: Vec<Decimal>,
    /// Fast EMA value
    fast_ema: Option<Decimal>,
    /// Slow EMA value
    slow_ema: Option<Decimal>,
    /// Current signal (1 = long, -1 = short, 0 = flat)
    current_signal: i8,
}

impl TrendFollowingStrategy {
    /// Create a new trend following strategy
    pub fn new(fast_period: usize, slow_period: usize, position_size: Decimal) -> Self {
        Self {
            fast_period,
            slow_period,
            position_size,
            prices: Vec::with_capacity(slow_period),
            fast_ema: None,
            slow_ema: None,
            current_signal: 0,
        }
    }

    /// Update EMAs with a new price
    fn update_emas(&mut self, price: Decimal) {
        // Update fast EMA
        let fast_alpha = dec!(2) / (Decimal::from(self.fast_period as u32) + Decimal::ONE);
        self.fast_ema = Some(match self.fast_ema {
            Some(prev) => fast_alpha * price + (Decimal::ONE - fast_alpha) * prev,
            None => price,
        });

        // Update slow EMA
        let slow_alpha = dec!(2) / (Decimal::from(self.slow_period as u32) + Decimal::ONE);
        self.slow_ema = Some(match self.slow_ema {
            Some(prev) => slow_alpha * price + (Decimal::ONE - slow_alpha) * prev,
            None => price,
        });

        // Store price history
        if self.prices.len() >= self.slow_period {
            self.prices.remove(0);
        }
        self.prices.push(price);
    }

    /// Generate trading signal
    fn generate_signal(&self) -> i8 {
        match (self.fast_ema, self.slow_ema) {
            (Some(fast), Some(slow)) => {
                if fast > slow {
                    1  // Bullish
                } else if fast < slow {
                    -1 // Bearish
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

#[async_trait]
impl Strategy for TrendFollowingStrategy {
    async fn on_tick(&mut self, context: &StrategyContext) -> BotResult<Vec<OrderRequest>> {
        let mut orders = Vec::new();

        for (market, ticker) in &context.tickers {
            // Update EMAs
            self.update_emas(ticker.last_price);

            // Generate signal
            let new_signal = self.generate_signal();

            // Only trade on signal change
            if new_signal != self.current_signal && new_signal != 0 {
                let position = context.position(market);
                let current_size = position.map(|p| p.size).unwrap_or(Decimal::ZERO);

                match new_signal {
                    1 => {
                        // Go long
                        if current_size < Decimal::ZERO {
                            // Close short first
                            orders.push(OrderRequest::market_buy(market.0.clone(), current_size.abs()));
                        }
                        if current_size <= Decimal::ZERO {
                            // Open long
                            orders.push(OrderRequest::market_buy(market.0.clone(), self.position_size));
                        }
                    }
                    -1 => {
                        // Go short
                        if current_size > Decimal::ZERO {
                            // Close long first
                            orders.push(OrderRequest::market_sell(market.0.clone(), current_size));
                        }
                        if current_size >= Decimal::ZERO {
                            // Open short
                            orders.push(OrderRequest::market_sell(market.0.clone(), self.position_size));
                        }
                    }
                    _ => {}
                }

                self.current_signal = new_signal;
            }
        }

        Ok(orders)
    }
}

// ============================================================================
// Arbitrage Bot (for cross-market arbitrage)
// ============================================================================

/// Statistical arbitrage strategy for correlated pairs
pub struct StatArbStrategy {
    /// Trading pair A
    pub pair_a: MarketId,
    /// Trading pair B
    pub pair_b: MarketId,
    /// Hedge ratio
    pub hedge_ratio: Decimal,
    /// Entry threshold (z-score)
    pub entry_threshold: Decimal,
    /// Exit threshold (z-score)
    pub exit_threshold: Decimal,
    /// Lookback period for calculating spread statistics
    pub lookback: usize,
    /// Position size
    pub position_size: Decimal,
    /// Spread history
    spread_history: Vec<Decimal>,
    /// Current position (-1, 0, 1)
    position: i8,
}

impl StatArbStrategy {
    /// Create a new stat arb strategy
    pub fn new(pair_a: impl Into<String>, pair_b: impl Into<String>) -> Self {
        Self {
            pair_a: MarketId::new(pair_a),
            pair_b: MarketId::new(pair_b),
            hedge_ratio: dec!(1),
            entry_threshold: dec!(2),
            exit_threshold: dec!(0.5),
            lookback: 100,
            position_size: dec!(0.1),
            spread_history: Vec::with_capacity(100),
            position: 0,
        }
    }

    /// Calculate z-score of current spread
    fn calculate_zscore(&self, current_spread: Decimal) -> Option<Decimal> {
        if self.spread_history.len() < 20 {
            return None;
        }

        let n = Decimal::from(self.spread_history.len() as u32);
        let sum: Decimal = self.spread_history.iter().sum();
        let mean = sum / n;

        let variance: Decimal = self.spread_history.iter()
            .map(|x| (*x - mean) * (*x - mean))
            .sum::<Decimal>() / n;

        // Avoid division by zero
        if variance.is_zero() {
            return None;
        }

        // Approximate sqrt using Newton's method
        let std_dev = self.approximate_sqrt(variance)?;

        Some((current_spread - mean) / std_dev)
    }

    /// Approximate square root
    fn approximate_sqrt(&self, x: Decimal) -> Option<Decimal> {
        if x < Decimal::ZERO {
            return None;
        }
        if x.is_zero() {
            return Some(Decimal::ZERO);
        }

        let mut guess = x / dec!(2);
        for _ in 0..10 {
            guess = (guess + x / guess) / dec!(2);
        }
        Some(guess)
    }
}

#[async_trait]
impl Strategy for StatArbStrategy {
    async fn on_tick(&mut self, context: &StrategyContext) -> BotResult<Vec<OrderRequest>> {
        let mut orders = Vec::new();

        // Get prices for both pairs
        let price_a = match context.last_price(&self.pair_a) {
            Some(p) => p,
            None => return Ok(orders),
        };

        let price_b = match context.last_price(&self.pair_b) {
            Some(p) => p,
            None => return Ok(orders),
        };

        // Calculate spread
        let spread = price_a - self.hedge_ratio * price_b;

        // Update history
        if self.spread_history.len() >= self.lookback {
            self.spread_history.remove(0);
        }
        self.spread_history.push(spread);

        // Calculate z-score
        let zscore = match self.calculate_zscore(spread) {
            Some(z) => z,
            None => return Ok(orders),
        };

        // Trading logic
        match self.position {
            0 => {
                // No position - look for entry
                if zscore > self.entry_threshold {
                    // Spread too high: sell A, buy B
                    orders.push(OrderRequest::market_sell(self.pair_a.0.clone(), self.position_size));
                    orders.push(OrderRequest::market_buy(self.pair_b.0.clone(), self.position_size * self.hedge_ratio));
                    self.position = -1;
                } else if zscore < -self.entry_threshold {
                    // Spread too low: buy A, sell B
                    orders.push(OrderRequest::market_buy(self.pair_a.0.clone(), self.position_size));
                    orders.push(OrderRequest::market_sell(self.pair_b.0.clone(), self.position_size * self.hedge_ratio));
                    self.position = 1;
                }
            }
            1 => {
                // Long spread - exit when zscore reverts
                if zscore > -self.exit_threshold {
                    orders.push(OrderRequest::market_sell(self.pair_a.0.clone(), self.position_size));
                    orders.push(OrderRequest::market_buy(self.pair_b.0.clone(), self.position_size * self.hedge_ratio));
                    self.position = 0;
                }
            }
            -1 => {
                // Short spread - exit when zscore reverts
                if zscore < self.exit_threshold {
                    orders.push(OrderRequest::market_buy(self.pair_a.0.clone(), self.position_size));
                    orders.push(OrderRequest::market_sell(self.pair_b.0.clone(), self.position_size * self.hedge_ratio));
                    self.position = 0;
                }
            }
            _ => {}
        }

        Ok(orders)
    }
}

// ============================================================================
// Random Walk Bot (for demo/testing)
// ============================================================================

/// Random walk bot for generating realistic market activity
pub struct RandomWalkBot {
    /// Base order size
    pub order_size: Decimal,
    /// Price volatility (as percentage)
    pub volatility: Decimal,
    /// Probability of placing a market order vs limit
    pub market_order_prob: f32,
    /// Counter for deterministic randomness
    tick_count: u64,
}

impl RandomWalkBot {
    /// Create a new random walk bot
    pub fn new(order_size: Decimal, volatility: Decimal) -> Self {
        Self {
            order_size,
            volatility,
            market_order_prob: 0.3,
            tick_count: 0,
        }
    }

    /// Simple pseudo-random using tick count
    fn pseudo_random(&mut self) -> u64 {
        self.tick_count = self.tick_count.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.tick_count
    }
}

#[async_trait]
impl Strategy for RandomWalkBot {
    async fn on_tick(&mut self, context: &StrategyContext) -> BotResult<Vec<OrderRequest>> {
        let mut orders = Vec::new();

        for (market, ticker) in &context.tickers {
            // Generate pseudo-random values
            let rand1 = self.pseudo_random();
            let rand2 = self.pseudo_random();

            // Determine side (50/50)
            let side = if rand1 % 2 == 0 { Side::Buy } else { Side::Sell };

            // Determine if market or limit order
            let is_market = (rand1 as f32 / u64::MAX as f32) < self.market_order_prob;

            // Calculate price offset (-1 to +1) * volatility
            let offset_factor = ((rand2 % 1000) as i64 - 500) as f32 / 500.0;
            let price_offset = ticker.last_price * self.volatility * Decimal::try_from(offset_factor).unwrap_or(Decimal::ZERO);

            let mid_price = if ticker.bid.is_zero() || ticker.ask.is_zero() {
                ticker.last_price
            } else {
                (ticker.bid + ticker.ask) / dec!(2)
            };

            // If mid_price is zero, use a default
            let base_price = if mid_price.is_zero() { dec!(3000) } else { mid_price };

            if is_market {
                match side {
                    Side::Buy => orders.push(OrderRequest::market_buy(market.0.clone(), self.order_size)),
                    Side::Sell => orders.push(OrderRequest::market_sell(market.0.clone(), self.order_size)),
                }
            } else {
                let price = match side {
                    Side::Buy => (base_price + price_offset).max(dec!(0.01)),
                    Side::Sell => (base_price + price_offset).max(dec!(0.01)),
                };
                match side {
                    Side::Buy => orders.push(OrderRequest::limit_buy(market.0.clone(), price.round_dp(2), self.order_size)),
                    Side::Sell => orders.push(OrderRequest::limit_sell(market.0.clone(), price.round_dp(2), self.order_size)),
                }
            }
        }

        Ok(orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_config_default() {
        let config = BotConfig::default();
        assert!(config.paper_trading);
        assert_eq!(config.tick_interval_ms, 1000);
    }

    #[test]
    fn test_position_update_long() {
        let mut position = Position::new(MarketId::new("ETH_IUSD"));

        // Open long position
        position.update(Side::Buy, dec!(1.0), dec!(3000));
        assert_eq!(position.size, dec!(1.0));
        assert_eq!(position.entry_price, dec!(3000));
        assert!(position.is_long());

        // Add to position
        position.update(Side::Buy, dec!(1.0), dec!(3100));
        assert_eq!(position.size, dec!(2.0));
        assert_eq!(position.entry_price, dec!(3050)); // Weighted average

        // Calculate unrealized PnL
        position.calculate_unrealized_pnl(dec!(3200));
        assert_eq!(position.unrealized_pnl, dec!(300)); // 2.0 * (3200 - 3050)
    }

    #[test]
    fn test_position_close() {
        let mut position = Position::new(MarketId::new("ETH_IUSD"));

        // Open and close position
        position.update(Side::Buy, dec!(1.0), dec!(3000));
        position.update(Side::Sell, dec!(1.0), dec!(3100));

        assert!(position.is_flat());
        assert_eq!(position.realized_pnl, dec!(100));
    }

    #[test]
    fn test_order_request_builders() {
        let buy = OrderRequest::limit_buy("ETH_IUSD", dec!(3000), dec!(1.0));
        assert_eq!(buy.side, Side::Buy);
        assert!(matches!(buy.order_type, OrderType::Limit { price, .. } if price == dec!(3000)));

        let sell = OrderRequest::market_sell("ETH_IUSD", dec!(0.5));
        assert_eq!(sell.side, Side::Sell);
        assert!(matches!(sell.order_type, OrderType::Market));
    }

    #[test]
    fn test_strategy_context() {
        let mut context = StrategyContext::new();

        context.tickers.insert(
            MarketId::new("ETH_IUSD"),
            Ticker::new(MarketId::new("ETH_IUSD")),
        );

        assert!(context.ticker(&MarketId::new("ETH_IUSD")).is_some());
        assert!(context.ticker(&MarketId::new("BTC_IUSD")).is_none());
    }

    #[test]
    fn test_grid_strategy_creation() {
        let strategy = GridStrategy::new(5, dec!(0.01), dec!(0.1));
        assert_eq!(strategy.levels, 5);
        assert_eq!(strategy.spacing, dec!(0.01));
        assert_eq!(strategy.order_size, dec!(0.1));
    }

    #[test]
    fn test_market_maker_strategy() {
        let mm = MarketMakerStrategy::new()
            .with_spread(dec!(0.003))
            .with_order_size(dec!(0.5));

        assert_eq!(mm.target_spread, dec!(0.003));
        assert_eq!(mm.order_size, dec!(0.5));
        assert_eq!(mm.levels, 3);
    }

    #[test]
    fn test_trend_following_strategy() {
        let mut tf = TrendFollowingStrategy::new(10, 20, dec!(1.0));

        // Simulate uptrend
        for i in 0..30 {
            tf.update_emas(dec!(100) + Decimal::from(i));
        }

        // Fast EMA should be above slow EMA in uptrend
        assert!(tf.fast_ema.unwrap() > tf.slow_ema.unwrap());
        assert_eq!(tf.generate_signal(), 1);
    }

    #[test]
    fn test_stat_arb_strategy() {
        let arb = StatArbStrategy::new("ETH_IUSD", "BTC_IUSD");
        assert_eq!(arb.pair_a.0, "ETH_IUSD");
        assert_eq!(arb.pair_b.0, "BTC_IUSD");
        assert_eq!(arb.entry_threshold, dec!(2));
    }

    #[test]
    fn test_random_walk_bot() {
        let mut bot = RandomWalkBot::new(dec!(0.1), dec!(0.005));

        // Generate some random numbers and ensure they're different
        let r1 = bot.pseudo_random();
        let r2 = bot.pseudo_random();
        assert_ne!(r1, r2);
    }
}
