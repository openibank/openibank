//! ResonanceX History - Historical Data Storage
//!
//! This crate provides storage and retrieval of historical market data,
//! trades, and receipts for the ResonanceX exchange.
//!
//! # Features
//!
//! - **Trade History**: Store and query historical trades
//! - **Candle History**: Pre-aggregated OHLCV data at multiple intervals
//! - **Receipt Archive**: Cryptographic receipt storage and verification
//! - **Order History**: Historical order records
//!
//! # Example
//!
//! ```ignore
//! use resonancex_history::{HistoryService, TradeQuery};
//!
//! let service = HistoryService::new(config).await?;
//!
//! // Query recent trades
//! let trades = service.get_trades(TradeQuery {
//!     market: MarketId::new("ETH_IUSD"),
//!     start: Some(Utc::now() - Duration::hours(24)),
//!     end: None,
//!     limit: Some(100),
//! }).await?;
//!
//! // Get historical candles
//! let candles = service.get_candles(
//!     MarketId::new("ETH_IUSD"),
//!     CandleInterval::H1,
//!     start,
//!     end,
//! ).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export core types
pub use resonancex_types::{
    MarketId, OrderId, TradeId, Trade, Order, Candle, CandleInterval,
};
pub use openibank_types::ReceiptId;

/// History service errors
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("Market not found: {0}")]
    MarketNotFound(MarketId),

    #[error("Trade not found: {0}")]
    TradeNotFound(TradeId),

    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),
}

/// Result type for history operations
pub type HistoryResult<T> = Result<T, HistoryError>;

/// Trade query parameters
#[derive(Debug, Clone, Default)]
pub struct TradeQuery {
    /// Market to query
    pub market: Option<MarketId>,
    /// Start time (inclusive)
    pub start: Option<DateTime<Utc>>,
    /// End time (exclusive)
    pub end: Option<DateTime<Utc>>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl TradeQuery {
    /// Create a new trade query for a market
    pub fn for_market(market: MarketId) -> Self {
        Self {
            market: Some(market),
            ..Default::default()
        }
    }

    /// Set time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self.end = Some(end);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set pagination
    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

/// Order query parameters
#[derive(Debug, Clone, Default)]
pub struct OrderQuery {
    /// Market to query
    pub market: Option<MarketId>,
    /// Agent ID filter
    pub agent_id: Option<String>,
    /// Status filter
    pub status: Option<Vec<String>>,
    /// Start time (inclusive)
    pub start: Option<DateTime<Utc>>,
    /// End time (exclusive)
    pub end: Option<DateTime<Utc>>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Candle query parameters
#[derive(Debug, Clone)]
pub struct CandleQuery {
    /// Market to query
    pub market: MarketId,
    /// Candle interval
    pub interval: CandleInterval,
    /// Start time (inclusive)
    pub start: Option<DateTime<Utc>>,
    /// End time (exclusive)
    pub end: Option<DateTime<Utc>>,
    /// Maximum number of candles
    pub limit: Option<usize>,
}

impl CandleQuery {
    /// Create a new candle query
    pub fn new(market: MarketId, interval: CandleInterval) -> Self {
        Self {
            market,
            interval,
            start: None,
            end: None,
            limit: None,
        }
    }

    /// Set time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self.end = Some(end);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Historical trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    /// Trade data
    pub trade: Trade,
    /// Receipt ID if available
    pub receipt_id: Option<ReceiptId>,
    /// Block height (if on-chain)
    pub block_height: Option<u64>,
}

/// Historical order record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRecord {
    /// Order data
    pub order: Order,
    /// Associated trade IDs
    pub trade_ids: Vec<TradeId>,
    /// Receipt IDs for trades
    pub receipt_ids: Vec<ReceiptId>,
}

/// Storage backend trait
#[async_trait]
pub trait HistoryStorage: Send + Sync {
    /// Store a trade
    async fn store_trade(&self, trade: &Trade, receipt_id: Option<ReceiptId>) -> HistoryResult<()>;

    /// Store an order
    async fn store_order(&self, order: &Order) -> HistoryResult<()>;

    /// Store a candle
    async fn store_candle(&self, market: &MarketId, interval: CandleInterval, candle: &Candle) -> HistoryResult<()>;

    /// Query trades
    async fn query_trades(&self, query: TradeQuery) -> HistoryResult<Vec<TradeRecord>>;

    /// Query orders
    async fn query_orders(&self, query: OrderQuery) -> HistoryResult<Vec<OrderRecord>>;

    /// Query candles
    async fn query_candles(&self, query: CandleQuery) -> HistoryResult<Vec<Candle>>;

    /// Get trade by ID
    async fn get_trade(&self, id: TradeId) -> HistoryResult<Option<TradeRecord>>;

    /// Get order by ID
    async fn get_order(&self, id: OrderId) -> HistoryResult<Option<OrderRecord>>;
}

/// In-memory history storage for testing
pub struct InMemoryStorage {
    trades: tokio::sync::RwLock<Vec<TradeRecord>>,
    orders: tokio::sync::RwLock<Vec<OrderRecord>>,
    candles: tokio::sync::RwLock<HashMap<(MarketId, CandleInterval), Vec<Candle>>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            trades: tokio::sync::RwLock::new(Vec::new()),
            orders: tokio::sync::RwLock::new(Vec::new()),
            candles: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HistoryStorage for InMemoryStorage {
    async fn store_trade(&self, trade: &Trade, receipt_id: Option<ReceiptId>) -> HistoryResult<()> {
        let record = TradeRecord {
            trade: trade.clone(),
            receipt_id,
            block_height: None,
        };
        self.trades.write().await.push(record);
        Ok(())
    }

    async fn store_order(&self, order: &Order) -> HistoryResult<()> {
        let record = OrderRecord {
            order: order.clone(),
            trade_ids: Vec::new(),
            receipt_ids: Vec::new(),
        };
        self.orders.write().await.push(record);
        Ok(())
    }

    async fn store_candle(&self, market: &MarketId, interval: CandleInterval, candle: &Candle) -> HistoryResult<()> {
        let mut candles = self.candles.write().await;
        candles
            .entry((market.clone(), interval))
            .or_insert_with(Vec::new)
            .push(candle.clone());
        Ok(())
    }

    async fn query_trades(&self, query: TradeQuery) -> HistoryResult<Vec<TradeRecord>> {
        let trades = self.trades.read().await;
        let mut results: Vec<_> = trades
            .iter()
            .filter(|r| {
                if let Some(ref market) = query.market {
                    if &r.trade.market != market {
                        return false;
                    }
                }
                if let Some(start) = query.start {
                    if r.trade.timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = query.end {
                    if r.trade.timestamp >= end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by timestamp descending
        results.sort_by(|a, b| b.trade.timestamp.cmp(&a.trade.timestamp));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);
        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn query_orders(&self, query: OrderQuery) -> HistoryResult<Vec<OrderRecord>> {
        let orders = self.orders.read().await;
        let mut results: Vec<_> = orders
            .iter()
            .filter(|r| {
                if let Some(ref market) = query.market {
                    if &r.order.market != market {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);
        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    async fn query_candles(&self, query: CandleQuery) -> HistoryResult<Vec<Candle>> {
        let candles = self.candles.read().await;
        let key = (query.market.clone(), query.interval);

        let results = candles
            .get(&key)
            .map(|v| {
                v.iter()
                    .filter(|c| {
                        if let Some(start) = query.start {
                            if c.timestamp < start.timestamp() {
                                return false;
                            }
                        }
                        if let Some(end) = query.end {
                            if c.timestamp >= end.timestamp() {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .take(query.limit.unwrap_or(1000))
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn get_trade(&self, id: TradeId) -> HistoryResult<Option<TradeRecord>> {
        let trades = self.trades.read().await;
        Ok(trades.iter().find(|r| r.trade.id == id).cloned())
    }

    async fn get_order(&self, id: OrderId) -> HistoryResult<Option<OrderRecord>> {
        let orders = self.orders.read().await;
        Ok(orders.iter().find(|r| r.order.id == id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resonancex_types::{Side, OrderType};
    use openibank_types::{AgentId, WalletId, PermitId};
    use rust_decimal_macros::dec;

    fn test_trade() -> Trade {
        Trade {
            id: TradeId::new(),
            market: MarketId::new("ETH_IUSD"),
            price: dec!(3000),
            amount: dec!(1.0),
            quote_amount: dec!(3000),
            maker_order_id: OrderId::new(),
            taker_order_id: OrderId::new(),
            maker_agent_id: AgentId::new(),
            taker_agent_id: AgentId::new(),
            maker_fee: dec!(3.0),
            taker_fee: dec!(6.0),
            maker_side: Side::Sell,
            maker_receipt_id: None,
            taker_receipt_id: None,
            timestamp: Utc::now(),
        }
    }

    fn test_order() -> Order {
        Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Buy)
            .order_type(OrderType::limit(dec!(3000)))
            .amount(dec!(1.0))
            .permit(PermitId::new())
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();

        // Store a trade
        let trade = test_trade();
        storage.store_trade(&trade, None).await.unwrap();

        // Query trades
        let query = TradeQuery::for_market(MarketId::new("ETH_IUSD"));
        let results = storage.query_trades(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trade.id, trade.id);
    }

    #[tokio::test]
    async fn test_trade_query() {
        let storage = InMemoryStorage::new();

        // Store multiple trades
        for _ in 0..5 {
            storage.store_trade(&test_trade(), None).await.unwrap();
        }

        // Query with limit
        let query = TradeQuery::for_market(MarketId::new("ETH_IUSD")).with_limit(3);
        let results = storage.query_trades(query).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_candle_storage() {
        let storage = InMemoryStorage::new();
        let market = MarketId::new("ETH_IUSD");

        // Store candles
        let candle = Candle::new(1700000000, dec!(3000));
        storage.store_candle(&market, CandleInterval::H1, &candle).await.unwrap();

        // Query candles
        let query = CandleQuery::new(market, CandleInterval::H1);
        let results = storage.query_candles(query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_trade_query_builder() {
        let query = TradeQuery::for_market(MarketId::new("ETH_IUSD"))
            .with_limit(50)
            .with_pagination(50, 100);

        assert_eq!(query.market.unwrap().0, "ETH_IUSD");
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(100));
    }
}
