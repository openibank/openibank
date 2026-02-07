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
            api_url: "http://localhost:8080".to_string(),
            ws_url: "ws://localhost:8081".to_string(),
            api_key: None,
            api_secret: None,
            markets: vec![MarketId::new("ETH_IUSD")],
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
}
