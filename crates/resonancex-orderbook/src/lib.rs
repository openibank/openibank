//! ResonanceX Orderbook - High-Performance In-Memory Orderbook
//!
//! This crate implements a lock-free, in-memory orderbook using BTreeMap
//! for price-time priority matching. Designed for single-threaded operation
//! within the matching engine.
//!
//! # Features
//!
//! - **Price-Time Priority**: Orders sorted by price, then by timestamp
//! - **O(log n) Operations**: Insert, cancel, and lookup are all O(log n)
//! - **Immediate Matching**: Limit orders match against opposite side on insert
//! - **Market Orders**: Execute at best available prices
//!
//! # Example
//!
//! ```ignore
//! use resonancex_orderbook::OrderBook;
//!
//! let mut book = OrderBook::new(market_config);
//!
//! // Insert a limit order
//! let trades = book.insert_limit(order);
//!
//! // Get best bid/ask
//! let (bid, ask) = book.spread();
//!
//! // Get depth snapshot
//! let depth = book.depth(20);
//! ```

use std::collections::BTreeMap;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono::Utc;

use resonancex_types::{
    Order, OrderId, Trade, TradeId, Side, OrderType, OrderStatus, RejectReason,
    MarketConfig, MarketId, OrderBookKey, OrderBookEntry, DepthSnapshot, DepthLevel,
    ExchangeError,
};
use openibank_types::AgentId;

// ============================================================================
// Match Result
// ============================================================================

/// Result of inserting an order into the orderbook
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Trades produced by the match
    pub trades: Vec<Trade>,
    /// The order after matching (may be partially filled)
    pub order: Order,
    /// Whether the order was placed on the book (not fully filled)
    pub placed_on_book: bool,
    /// Remaining amount after matching
    pub remaining: Decimal,
}

impl MatchResult {
    pub fn no_match(order: Order) -> Self {
        let remaining = order.remaining;
        Self {
            trades: Vec::new(),
            order,
            placed_on_book: true,
            remaining,
        }
    }

    pub fn rejected(mut order: Order, reason: RejectReason) -> Self {
        order.status = OrderStatus::Rejected(reason);
        Self {
            trades: Vec::new(),
            order,
            placed_on_book: false,
            remaining: Decimal::ZERO,
        }
    }
}

// ============================================================================
// Cancel Result
// ============================================================================

/// Result of cancelling an order
#[derive(Debug, Clone)]
pub struct CancelResult {
    pub order_id: OrderId,
    pub cancelled: bool,
    pub remaining: Decimal,
    pub price: Option<Decimal>,
}

// ============================================================================
// OrderBook
// ============================================================================

/// In-memory orderbook with price-time priority
///
/// Uses BTreeMap for efficient sorted access:
/// - Bids: sorted by price descending (best bid first)
/// - Asks: sorted by price ascending (best ask first)
pub struct OrderBook {
    /// Market configuration
    config: MarketConfig,
    /// Bid orders (buy side) - sorted by negated price for descending order
    bids: BTreeMap<OrderBookKey, OrderBookEntry>,
    /// Ask orders (sell side) - sorted by price ascending
    asks: BTreeMap<OrderBookKey, OrderBookEntry>,
    /// Order lookup by ID
    orders: BTreeMap<OrderId, (Side, OrderBookKey)>,
    /// Current microsecond timestamp (monotonic within engine)
    timestamp_us: u64,
}

impl OrderBook {
    /// Create a new orderbook for a market
    pub fn new(config: MarketConfig) -> Self {
        Self {
            config,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: BTreeMap::new(),
            timestamp_us: 0,
        }
    }

    /// Get the market ID
    pub fn market_id(&self) -> &MarketId {
        &self.config.id
    }

    /// Get the market configuration
    pub fn config(&self) -> &MarketConfig {
        &self.config
    }

    /// Get next timestamp (monotonic)
    fn next_timestamp(&mut self) -> u64 {
        self.timestamp_us += 1;
        self.timestamp_us
    }

    // ========================================================================
    // Insert Operations
    // ========================================================================

    /// Insert a limit order
    ///
    /// Attempts to match against the opposite side first, then places
    /// any remaining amount on the book.
    pub fn insert_limit(&mut self, mut order: Order) -> MatchResult {
        let price = match &order.order_type {
            OrderType::Limit { price, .. } => *price,
            _ => return MatchResult::rejected(order, RejectReason::InvalidPrice),
        };

        // Validate price
        if price <= Decimal::ZERO {
            return MatchResult::rejected(order, RejectReason::InvalidPrice);
        }

        // Check post-only
        let is_post_only = order.order_type.is_post_only();
        if is_post_only && self.would_match(&order) {
            return MatchResult::rejected(order, RejectReason::PostOnlyWouldMatch);
        }

        // Try to match against opposite side
        let trades = self.match_order(&mut order, Some(price));

        // If fully filled, we're done
        if order.remaining.is_zero() {
            order.status = OrderStatus::Filled;
            return MatchResult {
                trades,
                order,
                placed_on_book: false,
                remaining: Decimal::ZERO,
            };
        }

        // Place remaining on book
        let timestamp = self.next_timestamp();
        let key = match order.side {
            Side::Buy => OrderBookKey::bid(price, timestamp, order.id),
            Side::Sell => OrderBookKey::ask(price, timestamp, order.id),
        };

        let entry = OrderBookEntry {
            order_id: order.id,
            agent_id: order.agent_id.clone(),
            price,
            remaining: order.remaining,
            timestamp_us: timestamp,
        };

        // Insert into appropriate side
        match order.side {
            Side::Buy => self.bids.insert(key.clone(), entry),
            Side::Sell => self.asks.insert(key.clone(), entry),
        };

        // Track for lookup
        self.orders.insert(order.id, (order.side, key));

        // Update status
        if order.filled > Decimal::ZERO {
            order.status = OrderStatus::PartialFill;
        } else {
            order.status = OrderStatus::Open;
        }

        let remaining = order.remaining;
        MatchResult {
            trades,
            order,
            placed_on_book: true,
            remaining,
        }
    }

    /// Insert a market order
    ///
    /// Attempts to fill at best available prices. Any unfilled portion
    /// is cancelled (IOC behavior).
    pub fn insert_market(&mut self, mut order: Order) -> MatchResult {
        if !matches!(order.order_type, OrderType::Market) {
            return MatchResult::rejected(order, RejectReason::Other("Not a market order".into()));
        }

        // Match against opposite side with no price limit
        let trades = self.match_order(&mut order, None);

        // Market orders don't rest on the book
        if order.remaining.is_zero() {
            order.status = OrderStatus::Filled;
        } else {
            // Unfilled portion is cancelled
            order.status = OrderStatus::Cancelled;
        }

        MatchResult {
            trades,
            order,
            placed_on_book: false,
            remaining: Decimal::ZERO,
        }
    }

    /// Check if an order would immediately match
    fn would_match(&self, order: &Order) -> bool {
        let price = match order.order_type.price() {
            Some(p) => p,
            None => return false,
        };

        match order.side {
            Side::Buy => {
                // Buy order matches if best ask <= buy price
                self.best_ask().map(|ask| ask <= price).unwrap_or(false)
            }
            Side::Sell => {
                // Sell order matches if best bid >= sell price
                self.best_bid().map(|bid| bid >= price).unwrap_or(false)
            }
        }
    }

    /// Match an incoming order against the opposite side
    fn match_order(&mut self, order: &mut Order, limit_price: Option<Decimal>) -> Vec<Trade> {
        let mut trades = Vec::new();

        // Get reference to opposite side
        let opposite_side = match order.side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };

        // Track keys to remove after matching
        let mut keys_to_remove = Vec::new();

        // Iterate through opposite side in price-time priority
        for (key, entry) in opposite_side.iter_mut() {
            if order.remaining.is_zero() {
                break;
            }

            // Check price limit
            let entry_price = entry.price;
            if let Some(limit) = limit_price {
                match order.side {
                    Side::Buy => {
                        if entry_price > limit {
                            break; // Ask price too high
                        }
                    }
                    Side::Sell => {
                        if entry_price < limit {
                            break; // Bid price too low
                        }
                    }
                }
            }

            // Check for self-trade
            if entry.agent_id == order.agent_id {
                // Skip self-trades (could also reject entire order)
                continue;
            }

            // Calculate match amount
            let match_amount = order.remaining.min(entry.remaining);
            let match_price = entry_price; // Trade at maker price

            // Calculate fees
            let quote_amount = match_price * match_amount;
            let maker_fee = quote_amount * self.config.maker_fee;
            let taker_fee = quote_amount * self.config.taker_fee;

            // Create trade
            let trade = Trade {
                id: TradeId::new(),
                market: self.config.id.clone(),
                price: match_price,
                amount: match_amount,
                quote_amount,
                maker_order_id: entry.order_id,
                taker_order_id: order.id,
                maker_agent_id: entry.agent_id.clone(),
                taker_agent_id: order.agent_id.clone(),
                maker_fee,
                taker_fee,
                maker_side: order.side.opposite(), // Maker is opposite of taker
                maker_receipt_id: None,
                taker_receipt_id: None,
                timestamp: Utc::now(),
            };

            // Update the incoming order
            order.record_fill(match_amount, match_price);

            // Update the resting order
            entry.remaining -= match_amount;
            if entry.remaining.is_zero() {
                keys_to_remove.push(key.clone());
            }

            trades.push(trade);
        }

        // Remove fully filled orders
        for key in keys_to_remove {
            opposite_side.remove(&key);
            // Also remove from order lookup
            // We need to find the order_id from the key
            let order_id = key.order_id;
            self.orders.remove(&order_id);
        }

        trades
    }

    // ========================================================================
    // Cancel Operations
    // ========================================================================

    /// Cancel an order by ID
    pub fn cancel(&mut self, order_id: OrderId) -> CancelResult {
        // Look up the order
        if let Some((side, key)) = self.orders.remove(&order_id) {
            let book = match side {
                Side::Buy => &mut self.bids,
                Side::Sell => &mut self.asks,
            };

            if let Some(entry) = book.remove(&key) {
                return CancelResult {
                    order_id,
                    cancelled: true,
                    remaining: entry.remaining,
                    price: Some(entry.price),
                };
            }
        }

        CancelResult {
            order_id,
            cancelled: false,
            remaining: Decimal::ZERO,
            price: None,
        }
    }

    /// Cancel all orders for an agent
    pub fn cancel_all_for_agent(&mut self, agent_id: &AgentId) -> Vec<CancelResult> {
        let mut results = Vec::new();

        // Find all orders for this agent
        let orders_to_cancel: Vec<OrderId> = self
            .orders
            .iter()
            .filter_map(|(order_id, (side, key))| {
                let book = match side {
                    Side::Buy => &self.bids,
                    Side::Sell => &self.asks,
                };
                book.get(key)
                    .filter(|e| &e.agent_id == agent_id)
                    .map(|_| *order_id)
            })
            .collect();

        for order_id in orders_to_cancel {
            results.push(self.cancel(order_id));
        }

        results
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get the best bid price
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.first_key_value().map(|(k, _)| k.actual_price())
    }

    /// Get the best ask price
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.first_key_value().map(|(k, _)| k.price)
    }

    /// Get the spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get the mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / dec!(2)),
            _ => None,
        }
    }

    /// Get depth snapshot with specified number of levels
    pub fn depth(&self, levels: usize) -> DepthSnapshot {
        let mut snapshot = DepthSnapshot::new(self.config.id.clone());

        // Aggregate bids by price level
        let mut bid_levels: BTreeMap<Decimal, Decimal> = BTreeMap::new();
        for (key, entry) in self.bids.iter() {
            let price = key.actual_price();
            *bid_levels.entry(price).or_insert(Decimal::ZERO) += entry.remaining;
        }
        // Sorted descending by price (best bid first)
        for (price, amount) in bid_levels.iter().rev().take(levels) {
            snapshot.bids.push(DepthLevel {
                price: *price,
                amount: *amount,
            });
        }

        // Aggregate asks by price level
        let mut ask_levels: BTreeMap<Decimal, Decimal> = BTreeMap::new();
        for (key, entry) in self.asks.iter() {
            *ask_levels.entry(key.price).or_insert(Decimal::ZERO) += entry.remaining;
        }
        // Sorted ascending by price (best ask first)
        for (price, amount) in ask_levels.iter().take(levels) {
            snapshot.asks.push(DepthLevel {
                price: *price,
                amount: *amount,
            });
        }

        snapshot.timestamp = Utc::now();
        snapshot
    }

    /// Get order count
    pub fn order_count(&self) -> (usize, usize) {
        (self.bids.len(), self.asks.len())
    }

    /// Get total volume on each side
    pub fn total_volume(&self) -> (Decimal, Decimal) {
        let bid_vol: Decimal = self.bids.values().map(|e| e.remaining).sum();
        let ask_vol: Decimal = self.asks.values().map(|e| e.remaining).sum();
        (bid_vol, ask_vol)
    }

    /// Check if an order exists
    pub fn has_order(&self, order_id: &OrderId) -> bool {
        self.orders.contains_key(order_id)
    }

    /// Get order entry by ID
    pub fn get_order(&self, order_id: &OrderId) -> Option<&OrderBookEntry> {
        self.orders.get(order_id).and_then(|(side, key)| {
            match side {
                Side::Buy => self.bids.get(key),
                Side::Sell => self.asks.get(key),
            }
        })
    }

    /// Clear all orders (for testing/reset)
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.orders.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use openibank_types::{Currency, WalletId, PermitId};

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

    fn make_market_order(side: Side, amount: Decimal) -> Order {
        Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(side)
            .order_type(OrderType::Market)
            .amount(amount)
            .permit(PermitId::new())
            .build()
            .unwrap()
    }

    #[test]
    fn test_empty_orderbook() {
        let book = OrderBook::new(test_market_config());
        assert_eq!(book.best_bid(), None);
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.spread(), None);
        assert_eq!(book.order_count(), (0, 0));
    }

    #[test]
    fn test_insert_limit_no_match() {
        let mut book = OrderBook::new(test_market_config());

        // Insert a buy order
        let buy = test_order(Side::Buy, dec!(3000), dec!(1.0));
        let result = book.insert_limit(buy);

        assert!(result.trades.is_empty());
        assert!(result.placed_on_book);
        assert_eq!(book.best_bid(), Some(dec!(3000)));
        assert_eq!(book.best_ask(), None);

        // Insert a sell order above the bid (no match)
        let sell = test_order(Side::Sell, dec!(3100), dec!(0.5));
        let result = book.insert_limit(sell);

        assert!(result.trades.is_empty());
        assert!(result.placed_on_book);
        assert_eq!(book.best_bid(), Some(dec!(3000)));
        assert_eq!(book.best_ask(), Some(dec!(3100)));
        assert_eq!(book.spread(), Some(dec!(100)));
    }

    #[test]
    fn test_limit_order_matching() {
        let mut book = OrderBook::new(test_market_config());

        // Insert a sell order at 3000
        let sell = test_order(Side::Sell, dec!(3000), dec!(1.0));
        book.insert_limit(sell);

        // Insert a buy order at 3000 - should match
        let buy = test_order(Side::Buy, dec!(3000), dec!(0.5));
        let result = book.insert_limit(buy);

        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].price, dec!(3000));
        assert_eq!(result.trades[0].amount, dec!(0.5));
        assert!(!result.placed_on_book); // Fully filled

        // Remaining sell order should have 0.5 left
        assert_eq!(book.best_ask(), Some(dec!(3000)));
        let depth = book.depth(10);
        assert_eq!(depth.asks[0].amount, dec!(0.5));
    }

    #[test]
    fn test_limit_order_partial_fill() {
        let mut book = OrderBook::new(test_market_config());

        // Insert a small sell order
        let sell = test_order(Side::Sell, dec!(3000), dec!(0.3));
        book.insert_limit(sell);

        // Insert a larger buy order - partial fill
        let buy = test_order(Side::Buy, dec!(3000), dec!(1.0));
        let result = book.insert_limit(buy);

        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].amount, dec!(0.3));
        assert!(result.placed_on_book); // 0.7 remaining on book
        assert_eq!(result.remaining, dec!(0.7));

        // Buy order should now be on the book
        assert_eq!(book.best_bid(), Some(dec!(3000)));
        assert_eq!(book.best_ask(), None); // Sell order fully consumed
    }

    #[test]
    fn test_market_order_matching() {
        let mut book = OrderBook::new(test_market_config());

        // Add some liquidity
        book.insert_limit(test_order(Side::Sell, dec!(3000), dec!(0.5)));
        book.insert_limit(test_order(Side::Sell, dec!(3001), dec!(0.5)));
        book.insert_limit(test_order(Side::Sell, dec!(3002), dec!(0.5)));

        // Market buy
        let market_buy = make_market_order(Side::Buy, dec!(1.0));
        let result = book.insert_market(market_buy);

        // Should fill across multiple price levels
        assert_eq!(result.trades.len(), 2);
        assert_eq!(result.trades[0].price, dec!(3000)); // Best price first
        assert_eq!(result.trades[0].amount, dec!(0.5));
        assert_eq!(result.trades[1].price, dec!(3001));
        assert_eq!(result.trades[1].amount, dec!(0.5));

        // 0.5 remaining on book at 3002
        assert_eq!(book.best_ask(), Some(dec!(3002)));
    }

    #[test]
    fn test_cancel_order() {
        let mut book = OrderBook::new(test_market_config());

        let order = test_order(Side::Buy, dec!(3000), dec!(1.0));
        let order_id = order.id;
        book.insert_limit(order);

        assert!(book.has_order(&order_id));
        assert_eq!(book.order_count(), (1, 0));

        let result = book.cancel(order_id);
        assert!(result.cancelled);
        assert_eq!(result.remaining, dec!(1.0));

        assert!(!book.has_order(&order_id));
        assert_eq!(book.order_count(), (0, 0));
    }

    #[test]
    fn test_price_time_priority() {
        let mut book = OrderBook::new(test_market_config());

        // Insert multiple orders at same price
        let order1 = test_order(Side::Sell, dec!(3000), dec!(1.0));
        let order2 = test_order(Side::Sell, dec!(3000), dec!(1.0));
        let order3 = test_order(Side::Sell, dec!(3000), dec!(1.0));

        let id1 = order1.id;
        let id2 = order2.id;
        let id3 = order3.id;

        book.insert_limit(order1);
        book.insert_limit(order2);
        book.insert_limit(order3);

        // Market buy should match in FIFO order
        let result = book.insert_market(make_market_order(Side::Buy, dec!(1.5)));

        assert_eq!(result.trades.len(), 2);
        assert_eq!(result.trades[0].maker_order_id, id1); // First order matched first
        assert_eq!(result.trades[0].amount, dec!(1.0));
        assert_eq!(result.trades[1].maker_order_id, id2); // Second order
        assert_eq!(result.trades[1].amount, dec!(0.5));

        // order3 should still be on book with full amount
        let entry = book.get_order(&id3).unwrap();
        assert_eq!(entry.remaining, dec!(1.0));
    }

    #[test]
    fn test_depth_snapshot() {
        let mut book = OrderBook::new(test_market_config());

        // Add orders at various prices
        book.insert_limit(test_order(Side::Buy, dec!(2998), dec!(1.0)));
        book.insert_limit(test_order(Side::Buy, dec!(2999), dec!(2.0)));
        book.insert_limit(test_order(Side::Buy, dec!(3000), dec!(3.0)));

        book.insert_limit(test_order(Side::Sell, dec!(3001), dec!(1.5)));
        book.insert_limit(test_order(Side::Sell, dec!(3002), dec!(2.5)));

        let depth = book.depth(10);

        // Bids sorted descending (best first)
        assert_eq!(depth.bids.len(), 3);
        assert_eq!(depth.bids[0].price, dec!(3000));
        assert_eq!(depth.bids[0].amount, dec!(3.0));
        assert_eq!(depth.bids[1].price, dec!(2999));
        assert_eq!(depth.bids[2].price, dec!(2998));

        // Asks sorted ascending (best first)
        assert_eq!(depth.asks.len(), 2);
        assert_eq!(depth.asks[0].price, dec!(3001));
        assert_eq!(depth.asks[0].amount, dec!(1.5));
        assert_eq!(depth.asks[1].price, dec!(3002));

        assert_eq!(depth.spread(), Some(dec!(1)));
    }

    #[test]
    fn test_self_trade_prevention() {
        let mut book = OrderBook::new(test_market_config());

        let agent_id = AgentId::new();

        // Create buy and sell orders for the same agent
        let mut buy = Order::builder()
            .agent(agent_id.clone())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Buy)
            .order_type(OrderType::limit(dec!(3000)))
            .amount(dec!(1.0))
            .permit(PermitId::new())
            .build()
            .unwrap();

        let sell = Order::builder()
            .agent(agent_id.clone())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Sell)
            .order_type(OrderType::limit(dec!(3000)))
            .amount(dec!(1.0))
            .permit(PermitId::new())
            .build()
            .unwrap();

        // Insert sell first
        book.insert_limit(sell);

        // Insert buy - should not match (self-trade)
        let result = book.insert_limit(buy);

        // No trades should occur
        assert!(result.trades.is_empty());
        // Both orders should be on book
        assert_eq!(book.order_count(), (1, 1));
    }

    #[test]
    fn test_post_only_rejection() {
        let mut book = OrderBook::new(test_market_config());

        // Add an ask at 3000
        book.insert_limit(test_order(Side::Sell, dec!(3000), dec!(1.0)));

        // Try to place a post-only buy at 3000 (would match)
        let mut post_only = Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Buy)
            .order_type(OrderType::limit_post_only(dec!(3000)))
            .amount(dec!(1.0))
            .permit(PermitId::new())
            .build()
            .unwrap();

        let result = book.insert_limit(post_only);

        assert!(result.trades.is_empty());
        assert!(!result.placed_on_book);
        assert!(matches!(
            result.order.status,
            OrderStatus::Rejected(RejectReason::PostOnlyWouldMatch)
        ));
    }

    #[test]
    fn test_high_volume_insertions() {
        let mut book = OrderBook::new(test_market_config());

        // Insert 10000 orders
        for i in 0..5000 {
            let price = dec!(3000) + Decimal::from(i % 100);
            book.insert_limit(test_order(Side::Buy, price, dec!(0.1)));
            book.insert_limit(test_order(Side::Sell, dec!(3100) + Decimal::from(i % 100), dec!(0.1)));
        }

        assert_eq!(book.order_count(), (5000, 5000));

        // Verify depth is correct
        let depth = book.depth(20);
        assert_eq!(depth.bids.len(), 20);
        assert_eq!(depth.asks.len(), 20);
    }
}
