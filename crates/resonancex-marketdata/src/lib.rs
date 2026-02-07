//! ResonanceX Market Data - Real-Time Market Data Distribution
//!
//! This crate provides real-time market data aggregation and distribution
//! for the ResonanceX exchange. It processes trades and orderbook updates
//! to produce tickers, candles, and depth snapshots.
//!
//! # Features
//!
//! - **Ticker Updates**: Real-time price, volume, and 24h statistics
//! - **OHLCV Candles**: Candlestick data at multiple intervals (1m to 1M)
//! - **Depth Snapshots**: Aggregated orderbook depth at configurable levels
//! - **Trade Feed**: Real-time trade stream
//!
//! # Example
//!
//! ```ignore
//! use resonancex_marketdata::{MarketDataService, Subscription};
//!
//! let service = MarketDataService::new();
//!
//! // Subscribe to ticker updates
//! let rx = service.subscribe(Subscription::Ticker(market_id.clone()));
//!
//! while let Ok(update) = rx.recv() {
//!     match update {
//!         MarketDataUpdate::Ticker(ticker) => {
//!             println!("{}: {} bid/ask {}/{}", ticker.market, ticker.last_price, ticker.bid, ticker.ask);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono::{DateTime, Utc};
use thiserror::Error;
use flume::{Sender, Receiver};

// Re-export core types
pub use resonancex_types::{
    MarketId, Trade, Candle, CandleInterval, Ticker, DepthSnapshot, DepthLevel,
};

/// Market data errors
#[derive(Debug, Error)]
pub enum MarketDataError {
    #[error("Market not found: {0}")]
    MarketNotFound(MarketId),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Channel closed")]
    ChannelClosed,
}

/// Result type for market data operations
pub type MarketDataResult<T> = Result<T, MarketDataError>;

/// Subscription types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Subscription {
    /// Subscribe to ticker updates for a market
    Ticker(MarketId),
    /// Subscribe to trades for a market
    Trades(MarketId),
    /// Subscribe to depth updates for a market
    Depth(MarketId),
    /// Subscribe to candles for a market at a specific interval
    Candles(MarketId, CandleInterval),
}

/// Market data update events
#[derive(Debug, Clone)]
pub enum MarketDataUpdate {
    /// Ticker update
    Ticker(Ticker),
    /// New trade
    Trade(Trade),
    /// Depth snapshot
    Depth(DepthSnapshot),
    /// Candle update
    Candle {
        market: MarketId,
        interval: CandleInterval,
        candle: Candle,
    },
}

/// Market statistics aggregator
#[derive(Debug, Clone)]
pub struct MarketStats {
    /// Market ID
    pub market: MarketId,
    /// Last trade price
    pub last_price: Decimal,
    /// 24h high
    pub high_24h: Decimal,
    /// 24h low
    pub low_24h: Decimal,
    /// 24h volume in base currency
    pub volume_24h: Decimal,
    /// 24h volume in quote currency
    pub quote_volume_24h: Decimal,
    /// 24h trade count
    pub trade_count_24h: u64,
    /// Opening price (24h ago)
    pub open_24h: Decimal,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl MarketStats {
    /// Create new market stats
    pub fn new(market: MarketId) -> Self {
        Self {
            market,
            last_price: Decimal::ZERO,
            high_24h: Decimal::ZERO,
            low_24h: Decimal::ZERO,
            volume_24h: Decimal::ZERO,
            quote_volume_24h: Decimal::ZERO,
            trade_count_24h: 0,
            open_24h: Decimal::ZERO,
            updated_at: Utc::now(),
        }
    }

    /// Update stats with a new trade
    pub fn update_with_trade(&mut self, trade: &Trade) {
        self.last_price = trade.price;

        if self.trade_count_24h == 0 {
            self.open_24h = trade.price;
            self.high_24h = trade.price;
            self.low_24h = trade.price;
        } else {
            self.high_24h = self.high_24h.max(trade.price);
            self.low_24h = self.low_24h.min(trade.price);
        }

        self.volume_24h += trade.amount;
        self.quote_volume_24h += trade.quote_amount;
        self.trade_count_24h += 1;
        self.updated_at = Utc::now();
    }

    /// Calculate 24h price change percentage
    pub fn change_24h_percent(&self) -> Decimal {
        if self.open_24h.is_zero() {
            Decimal::ZERO
        } else {
            ((self.last_price - self.open_24h) / self.open_24h) * dec!(100)
        }
    }

    /// Convert to ticker format
    pub fn to_ticker(&self, bid: Decimal, ask: Decimal) -> Ticker {
        Ticker {
            market: self.market.clone(),
            last_price: self.last_price,
            bid,
            ask,
            high_24h: self.high_24h,
            low_24h: self.low_24h,
            volume_24h: self.volume_24h,
            quote_volume_24h: self.quote_volume_24h,
            change_24h: self.change_24h_percent(),
            trade_count_24h: self.trade_count_24h,
            timestamp: self.updated_at,
        }
    }
}

/// Candle aggregator for a single market and interval
pub struct CandleAggregator {
    market: MarketId,
    interval: CandleInterval,
    current_candle: Option<Candle>,
}

impl CandleAggregator {
    /// Create a new candle aggregator
    pub fn new(market: MarketId, interval: CandleInterval) -> Self {
        Self {
            market,
            interval,
            current_candle: None,
        }
    }

    /// Process a trade and return a candle if one was completed
    pub fn process_trade(&mut self, trade: &Trade) -> Option<Candle> {
        let trade_ts = trade.timestamp.timestamp();
        let candle_start = self.interval.floor(trade_ts);

        let mut completed = None;

        match &mut self.current_candle {
            Some(candle) if candle.timestamp == candle_start => {
                // Update current candle
                candle.update(trade.price, trade.amount, trade.quote_amount);
            }
            Some(candle) => {
                // Candle completed, start new one
                completed = Some(candle.clone());
                let mut new_candle = Candle::new(candle_start, trade.price);
                new_candle.update(trade.price, trade.amount, trade.quote_amount);
                self.current_candle = Some(new_candle);
            }
            None => {
                // First trade
                let mut new_candle = Candle::new(candle_start, trade.price);
                new_candle.update(trade.price, trade.amount, trade.quote_amount);
                self.current_candle = Some(new_candle);
            }
        }

        completed
    }

    /// Get the current (incomplete) candle
    pub fn current(&self) -> Option<&Candle> {
        self.current_candle.as_ref()
    }
}

/// Market data service for aggregation and distribution
pub struct MarketDataService {
    /// Market statistics by market ID
    stats: RwLock<HashMap<MarketId, MarketStats>>,
    /// Candle aggregators by (market, interval)
    candles: RwLock<HashMap<(MarketId, CandleInterval), CandleAggregator>>,
    /// Current depth snapshots
    depth: RwLock<HashMap<MarketId, DepthSnapshot>>,
    /// Subscribers
    subscribers: RwLock<Vec<(Subscription, Sender<MarketDataUpdate>)>>,
}

impl MarketDataService {
    /// Create a new market data service
    pub fn new() -> Self {
        Self {
            stats: RwLock::new(HashMap::new()),
            candles: RwLock::new(HashMap::new()),
            depth: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Register a market for tracking
    pub fn register_market(&self, market_id: MarketId) {
        self.stats.write().entry(market_id.clone()).or_insert_with(|| MarketStats::new(market_id.clone()));

        // Initialize candle aggregators for common intervals
        let intervals = [
            CandleInterval::M1,
            CandleInterval::M5,
            CandleInterval::M15,
            CandleInterval::H1,
            CandleInterval::H4,
            CandleInterval::D1,
        ];

        let mut candles = self.candles.write();
        for interval in intervals {
            candles.entry((market_id.clone(), interval)).or_insert_with(|| {
                CandleAggregator::new(market_id.clone(), interval)
            });
        }
    }

    /// Process a trade
    pub fn process_trade(&self, trade: Trade) {
        let market_id = trade.market.clone();

        // Update market stats
        if let Some(stats) = self.stats.write().get_mut(&market_id) {
            stats.update_with_trade(&trade);
        }

        // Update candles and check for completed candles
        {
            let mut candles = self.candles.write();
            for ((mid, interval), aggregator) in candles.iter_mut() {
                if mid == &market_id {
                    if let Some(completed_candle) = aggregator.process_trade(&trade) {
                        // Notify candle subscribers
                        self.notify(MarketDataUpdate::Candle {
                            market: market_id.clone(),
                            interval: *interval,
                            candle: completed_candle,
                        });
                    }
                }
            }
        }

        // Notify trade subscribers
        self.notify(MarketDataUpdate::Trade(trade));
    }

    /// Update depth snapshot
    pub fn update_depth(&self, depth: DepthSnapshot) {
        let market_id = depth.market.clone();
        self.depth.write().insert(market_id, depth.clone());
        self.notify(MarketDataUpdate::Depth(depth));
    }

    /// Get current ticker for a market
    pub fn get_ticker(&self, market_id: &MarketId) -> Option<Ticker> {
        let stats = self.stats.read();
        let depth = self.depth.read();

        let market_stats = stats.get(market_id)?;
        let market_depth = depth.get(market_id);

        let (bid, ask) = match market_depth {
            Some(d) => (d.best_bid().unwrap_or_default(), d.best_ask().unwrap_or_default()),
            None => (Decimal::ZERO, Decimal::ZERO),
        };

        Some(market_stats.to_ticker(bid, ask))
    }

    /// Get current depth for a market
    pub fn get_depth(&self, market_id: &MarketId) -> Option<DepthSnapshot> {
        self.depth.read().get(market_id).cloned()
    }

    /// Subscribe to market data updates
    pub fn subscribe(&self, subscription: Subscription) -> Receiver<MarketDataUpdate> {
        let (tx, rx) = flume::unbounded();
        self.subscribers.write().push((subscription, tx));
        rx
    }

    /// Notify subscribers of an update
    fn notify(&self, update: MarketDataUpdate) {
        let subs = self.subscribers.read();

        for (sub, tx) in subs.iter() {
            let should_send = match (&update, sub) {
                (MarketDataUpdate::Ticker(t), Subscription::Ticker(m)) => &t.market == m,
                (MarketDataUpdate::Trade(t), Subscription::Trades(m)) => &t.market == m,
                (MarketDataUpdate::Depth(d), Subscription::Depth(m)) => &d.market == m,
                (MarketDataUpdate::Candle { market, interval, .. }, Subscription::Candles(m, i)) => {
                    market == m && interval == i
                }
                _ => false,
            };

            if should_send {
                let _ = tx.try_send(update.clone());
            }
        }
    }
}

impl Default for MarketDataService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resonancex_types::{TradeId, OrderId, Side};
    use openibank_types::AgentId;

    fn test_trade(price: Decimal, amount: Decimal) -> Trade {
        Trade {
            id: TradeId::new(),
            market: MarketId::new("ETH_IUSD"),
            price,
            amount,
            quote_amount: price * amount,
            maker_order_id: OrderId::new(),
            taker_order_id: OrderId::new(),
            maker_agent_id: AgentId::new(),
            taker_agent_id: AgentId::new(),
            maker_fee: dec!(0.001) * price * amount,
            taker_fee: dec!(0.002) * price * amount,
            maker_side: Side::Sell,
            maker_receipt_id: None,
            taker_receipt_id: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_market_stats() {
        let mut stats = MarketStats::new(MarketId::new("ETH_IUSD"));

        let trade1 = test_trade(dec!(3000), dec!(1.0));
        stats.update_with_trade(&trade1);

        assert_eq!(stats.last_price, dec!(3000));
        assert_eq!(stats.high_24h, dec!(3000));
        assert_eq!(stats.low_24h, dec!(3000));
        assert_eq!(stats.volume_24h, dec!(1.0));
        assert_eq!(stats.trade_count_24h, 1);

        let trade2 = test_trade(dec!(3100), dec!(0.5));
        stats.update_with_trade(&trade2);

        assert_eq!(stats.last_price, dec!(3100));
        assert_eq!(stats.high_24h, dec!(3100));
        assert_eq!(stats.low_24h, dec!(3000));
        assert_eq!(stats.volume_24h, dec!(1.5));
    }

    #[test]
    fn test_candle_aggregator() {
        let mut aggregator = CandleAggregator::new(
            MarketId::new("ETH_IUSD"),
            CandleInterval::M1,
        );

        let trade = test_trade(dec!(3000), dec!(1.0));
        aggregator.process_trade(&trade);

        let candle = aggregator.current().unwrap();
        assert_eq!(candle.open, dec!(3000));
        assert_eq!(candle.close, dec!(3000));
        assert_eq!(candle.volume, dec!(1.0));
        assert_eq!(candle.trade_count, 1);
    }

    #[test]
    fn test_market_data_service() {
        let service = MarketDataService::new();
        let market_id = MarketId::new("ETH_IUSD");

        service.register_market(market_id.clone());

        // Process some trades
        service.process_trade(test_trade(dec!(3000), dec!(1.0)));
        service.process_trade(test_trade(dec!(3050), dec!(0.5)));

        // Check ticker
        let ticker = service.get_ticker(&market_id).unwrap();
        assert_eq!(ticker.last_price, dec!(3050));
        assert_eq!(ticker.volume_24h, dec!(1.5));
    }

    #[test]
    fn test_subscription() {
        let service = MarketDataService::new();
        let market_id = MarketId::new("ETH_IUSD");

        service.register_market(market_id.clone());

        let rx = service.subscribe(Subscription::Trades(market_id.clone()));

        service.process_trade(test_trade(dec!(3000), dec!(1.0)));

        // Should receive the trade
        let update = rx.try_recv().unwrap();
        match update {
            MarketDataUpdate::Trade(t) => {
                assert_eq!(t.price, dec!(3000));
            }
            _ => panic!("Expected trade update"),
        }
    }
}
