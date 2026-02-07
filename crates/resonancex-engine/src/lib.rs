//! ResonanceX Engine - Matching Engine Core
//!
//! This crate implements the core matching engine for the ResonanceX exchange,
//! integrating with the MAPLE Resonance Architecture for secure, atomic trading.
//!
//! # Architecture
//!
//! The engine follows the Resonance Flow for every order:
//! 1. **Intent**: Agent submits an order with a spend permit
//! 2. **Commitment**: Funds are escrowed via Resonator commitment
//! 3. **Matching**: Order is matched against the orderbook
//! 4. **Consequence**: Trade receipts are generated and funds are settled
//!
//! # Components
//!
//! - `MatchingEngine`: The main engine managing multiple orderbooks
//! - `OrderProcessor`: Handles order validation and Resonance flow
//! - `TradeSettler`: Settles trades and releases escrowed funds
//!
//! # Example
//!
//! ```ignore
//! use resonancex_engine::MatchingEngine;
//!
//! let engine = MatchingEngine::new(config).await?;
//!
//! // Submit an order
//! let result = engine.submit_order(order).await?;
//!
//! // Process trades
//! for trade in result.trades {
//!     println!("Trade executed: {} @ {}", trade.amount, trade.price);
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use thiserror::Error;

// Re-export core types from dependencies
pub use resonancex_types::{
    Order, OrderId, Trade, TradeId, Side, OrderType, OrderStatus,
    MarketConfig, MarketId, MarketStatus, ExchangeError,
};
pub use resonancex_orderbook::{OrderBook, MatchResult, CancelResult};

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum orders per agent per market
    pub max_orders_per_agent: usize,
    /// Enable Resonance flow (escrow before matching)
    pub resonance_enabled: bool,
    /// Enable receipt generation
    pub receipts_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_orders_per_agent: 100,
            resonance_enabled: true,
            receipts_enabled: true,
        }
    }
}

/// Engine errors
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Exchange error: {0}")]
    Exchange(#[from] ExchangeError),

    #[error("Market not found: {0}")]
    MarketNotFound(MarketId),

    #[error("Market already exists: {0}")]
    MarketAlreadyExists(MarketId),

    #[error("Order limit exceeded")]
    OrderLimitExceeded,

    #[error("Escrow failed: {0}")]
    EscrowFailed(String),

    #[error("Settlement failed: {0}")]
    SettlementFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for engine operations
pub type EngineResult<T> = Result<T, EngineError>;

/// Order submission result
#[derive(Debug, Clone)]
pub struct SubmitResult {
    /// The order after processing
    pub order: Order,
    /// Trades produced by the order
    pub trades: Vec<Trade>,
    /// Whether the order was placed on the book
    pub placed_on_book: bool,
}

/// The matching engine managing multiple orderbooks
pub struct MatchingEngine {
    /// Engine configuration
    config: EngineConfig,
    /// Orderbooks by market ID
    orderbooks: RwLock<HashMap<MarketId, OrderBook>>,
}

impl MatchingEngine {
    /// Create a new matching engine
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            orderbooks: RwLock::new(HashMap::new()),
        }
    }

    /// Add a new market
    pub fn add_market(&self, market_config: MarketConfig) -> EngineResult<()> {
        let mut books = self.orderbooks.write();
        let market_id = market_config.id.clone();

        if books.contains_key(&market_id) {
            return Err(EngineError::MarketAlreadyExists(market_id));
        }

        let orderbook = OrderBook::new(market_config);
        books.insert(market_id, orderbook);
        Ok(())
    }

    /// Get market configuration
    pub fn get_market(&self, market_id: &MarketId) -> Option<MarketConfig> {
        self.orderbooks.read().get(market_id).map(|book| book.config().clone())
    }

    /// List all markets
    pub fn list_markets(&self) -> Vec<MarketId> {
        self.orderbooks.read().keys().cloned().collect()
    }

    /// Submit an order for matching
    pub fn submit_order(&self, order: Order) -> EngineResult<SubmitResult> {
        let mut books = self.orderbooks.write();

        let book = books
            .get_mut(&order.market)
            .ok_or_else(|| EngineError::MarketNotFound(order.market.clone()))?;

        // Match based on order type
        let result = match &order.order_type {
            OrderType::Limit { .. } => book.insert_limit(order),
            OrderType::Market => book.insert_market(order),
            OrderType::StopLimit { .. } | OrderType::StopMarket { .. } => {
                // Stop orders are not yet implemented
                return Err(EngineError::Internal("Stop orders not yet supported".into()));
            }
        };

        Ok(SubmitResult {
            order: result.order,
            trades: result.trades,
            placed_on_book: result.placed_on_book,
        })
    }

    /// Cancel an order
    pub fn cancel_order(&self, market_id: &MarketId, order_id: OrderId) -> EngineResult<CancelResult> {
        let mut books = self.orderbooks.write();

        let book = books
            .get_mut(market_id)
            .ok_or_else(|| EngineError::MarketNotFound(market_id.clone()))?;

        Ok(book.cancel(order_id))
    }

    /// Get orderbook depth
    pub fn get_depth(&self, market_id: &MarketId, levels: usize) -> EngineResult<resonancex_types::DepthSnapshot> {
        let books = self.orderbooks.read();

        let book = books
            .get(market_id)
            .ok_or_else(|| EngineError::MarketNotFound(market_id.clone()))?;

        Ok(book.depth(levels))
    }

    /// Get best bid and ask
    pub fn get_bbo(&self, market_id: &MarketId) -> EngineResult<(Option<Decimal>, Option<Decimal>)> {
        let books = self.orderbooks.read();

        let book = books
            .get(market_id)
            .ok_or_else(|| EngineError::MarketNotFound(market_id.clone()))?;

        Ok((book.best_bid(), book.best_ask()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openibank_types::{AgentId, WalletId, PermitId, Currency};
    use rust_decimal_macros::dec;

    fn test_market_config() -> MarketConfig {
        MarketConfig::new(
            MarketId::new("ETH_IUSD"),
            Currency::eth(),
            Currency::iusd(),
        )
    }

    fn test_order(side: Side, price: Decimal, amount: Decimal) -> Order {
        Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(side)
            .order_type(OrderType::limit(price))
            .amount(amount)
            .permit(PermitId::new())
            .build()
            .unwrap()
    }

    #[test]
    fn test_engine_creation() {
        let engine = MatchingEngine::new(EngineConfig::default());
        assert!(engine.list_markets().is_empty());
    }

    #[test]
    fn test_add_market() {
        let engine = MatchingEngine::new(EngineConfig::default());
        engine.add_market(test_market_config()).unwrap();

        let markets = engine.list_markets();
        assert_eq!(markets.len(), 1);
        assert_eq!(markets[0].0, "ETH_IUSD");
    }

    #[test]
    fn test_submit_order() {
        let engine = MatchingEngine::new(EngineConfig::default());
        engine.add_market(test_market_config()).unwrap();

        let order = test_order(Side::Buy, dec!(3000), dec!(1.0));
        let result = engine.submit_order(order).unwrap();

        assert!(result.trades.is_empty());
        assert!(result.placed_on_book);
    }

    #[test]
    fn test_order_matching() {
        let engine = MatchingEngine::new(EngineConfig::default());
        engine.add_market(test_market_config()).unwrap();

        // Place a sell order
        let sell = test_order(Side::Sell, dec!(3000), dec!(1.0));
        engine.submit_order(sell).unwrap();

        // Place a matching buy order
        let buy = test_order(Side::Buy, dec!(3000), dec!(0.5));
        let result = engine.submit_order(buy).unwrap();

        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].price, dec!(3000));
        assert_eq!(result.trades[0].amount, dec!(0.5));
    }

    #[test]
    fn test_get_depth() {
        let engine = MatchingEngine::new(EngineConfig::default());
        engine.add_market(test_market_config()).unwrap();

        engine.submit_order(test_order(Side::Buy, dec!(2999), dec!(1.0))).unwrap();
        engine.submit_order(test_order(Side::Buy, dec!(3000), dec!(2.0))).unwrap();
        engine.submit_order(test_order(Side::Sell, dec!(3001), dec!(1.5))).unwrap();

        let depth = engine.get_depth(&MarketId::new("ETH_IUSD"), 10).unwrap();

        assert_eq!(depth.bids.len(), 2);
        assert_eq!(depth.asks.len(), 1);
        assert_eq!(depth.best_bid(), Some(dec!(3000)));
        assert_eq!(depth.best_ask(), Some(dec!(3001)));
    }
}
