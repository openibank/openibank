//! ResonanceX Types - Trading Domain Types for AI-Native Exchange
//!
//! This crate defines all the core types for the ResonanceX exchange:
//! - Markets and trading pairs
//! - Orders and trades
//! - Market data (candles, depth, tickers)
//! - Arena competitions
//!
//! # Architecture
//!
//! ResonanceX is built on the MAPLE Resonance Architecture:
//! - Every order follows the Resonance Flow: Intent → Commitment → Consequence
//! - Funds are escrowed BEFORE orders hit the orderbook
//! - Every trade produces cryptographic receipts
//!
//! # Example
//!
//! ```ignore
//! use resonancex_types::{Order, Side, OrderType, MarketId};
//!
//! let order = Order::builder()
//!     .market(MarketId::new("ETH_IUSD"))
//!     .side(Side::Buy)
//!     .order_type(OrderType::limit(dec!(3245.00)))
//!     .amount(dec!(1.5))
//!     .build()?;
//! ```

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub use openibank_types::{
    AgentId, Amount, Currency, PermitId, ReceiptId, WalletId,
    CommitmentId, OpeniBankError, TemporalAnchor,
};

// ============================================================================
// ID Types
// ============================================================================

/// Market identifier (e.g., "ETH_IUSD", "BTC_USDC")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

impl MarketId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Parse base and quote currencies from market ID
    pub fn parse_pair(&self) -> Option<(String, String)> {
        let parts: Vec<&str> = self.0.split('_').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

impl fmt::Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Order identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

impl OrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Trade identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradeId(pub Uuid);

impl TradeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TradeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Market Configuration
// ============================================================================

/// Market status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketStatus {
    /// Market is active and accepting orders
    Active,
    /// Market is suspended (no new orders)
    Suspended,
    /// Market is in maintenance
    Maintenance,
    /// Market is closed permanently
    Closed,
}

impl Default for MarketStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Market configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConfig {
    /// Market identifier (e.g., "ETH_IUSD")
    pub id: MarketId,
    /// Base currency (what you're buying/selling)
    pub base: Currency,
    /// Quote currency (what you're pricing in)
    pub quote: Currency,
    /// Decimal places for price
    pub price_precision: u8,
    /// Decimal places for amount
    pub amount_precision: u8,
    /// Minimum order size in base currency
    pub min_amount: Decimal,
    /// Maximum order size in base currency
    pub max_amount: Option<Decimal>,
    /// Maker fee rate (e.g., 0.001 = 0.1%)
    pub maker_fee: Decimal,
    /// Taker fee rate (e.g., 0.002 = 0.2%)
    pub taker_fee: Decimal,
    /// Market status
    pub status: MarketStatus,
    /// Tick size (minimum price increment)
    pub tick_size: Decimal,
    /// Lot size (minimum amount increment)
    pub lot_size: Decimal,
}

impl MarketConfig {
    /// Create a new market configuration with defaults
    pub fn new(id: MarketId, base: Currency, quote: Currency) -> Self {
        Self {
            id,
            base,
            quote,
            price_precision: 2,
            amount_precision: 4,
            min_amount: dec!(0.0001),
            max_amount: None,
            maker_fee: dec!(0.001),
            taker_fee: dec!(0.002),
            status: MarketStatus::Active,
            tick_size: dec!(0.01),
            lot_size: dec!(0.0001),
        }
    }

    /// Check if trading is allowed
    pub fn is_trading_allowed(&self) -> bool {
        self.status == MarketStatus::Active
    }

    /// Validate an order amount
    pub fn validate_amount(&self, amount: Decimal) -> Result<(), ExchangeError> {
        if amount < self.min_amount {
            return Err(ExchangeError::AmountTooSmall {
                min: self.min_amount,
                got: amount,
            });
        }
        if let Some(max) = self.max_amount {
            if amount > max {
                return Err(ExchangeError::AmountTooLarge {
                    max,
                    got: amount,
                });
            }
        }
        Ok(())
    }

    /// Round price to tick size
    pub fn round_price(&self, price: Decimal) -> Decimal {
        (price / self.tick_size).round() * self.tick_size
    }

    /// Round amount to lot size
    pub fn round_amount(&self, amount: Decimal) -> Decimal {
        (amount / self.lot_size).floor() * self.lot_size
    }
}

// ============================================================================
// Order Types
// ============================================================================

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "buy"),
            Side::Sell => write!(f, "sell"),
        }
    }
}

/// Order type with parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit order at specified price
    Limit {
        price: Decimal,
        /// Post-only: reject if would immediately match
        post_only: bool,
    },
    /// Market order (execute at best available price)
    Market,
    /// Stop-limit order
    StopLimit {
        trigger: Decimal,
        price: Decimal,
    },
    /// Stop-market order
    StopMarket {
        trigger: Decimal,
    },
}

impl OrderType {
    /// Create a limit order
    pub fn limit(price: Decimal) -> Self {
        Self::Limit {
            price,
            post_only: false,
        }
    }

    /// Create a post-only limit order
    pub fn limit_post_only(price: Decimal) -> Self {
        Self::Limit {
            price,
            post_only: true,
        }
    }

    /// Get the price if this is a limit order
    pub fn price(&self) -> Option<Decimal> {
        match self {
            OrderType::Limit { price, .. } => Some(*price),
            OrderType::StopLimit { price, .. } => Some(*price),
            _ => None,
        }
    }

    /// Check if this is a post-only order
    pub fn is_post_only(&self) -> bool {
        matches!(self, OrderType::Limit { post_only: true, .. })
    }
}

/// Time in force
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good 'til cancelled
    GTC,
    /// Immediate or cancel (fill what you can, cancel rest)
    IOC,
    /// Fill or kill (complete fill or cancel)
    FOK,
    /// Good 'til time
    GTT(DateTime<Utc>),
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GTC
    }
}

/// Order status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order submitted, awaiting Resonance flow
    Pending,
    /// Resonator committed, funds escrowed
    Committed,
    /// Order is on the orderbook
    Open,
    /// Order is partially filled
    PartialFill,
    /// Order is completely filled
    Filled,
    /// Order was cancelled by agent
    Cancelled,
    /// Order was rejected
    Rejected(RejectReason),
    /// Order expired (GTT)
    Expired,
}

impl OrderStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            OrderStatus::Filled
                | OrderStatus::Cancelled
                | OrderStatus::Rejected(_)
                | OrderStatus::Expired
        )
    }

    pub fn is_open(&self) -> bool {
        matches!(self, OrderStatus::Open | OrderStatus::PartialFill)
    }
}

/// Order rejection reason
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    InsufficientFunds,
    InvalidPermit,
    MarketClosed,
    InvalidPrice,
    InvalidAmount,
    SelfTrade,
    PostOnlyWouldMatch,
    RateLimited,
    CommitmentFailed,
    Other(String),
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectReason::InsufficientFunds => write!(f, "Insufficient funds"),
            RejectReason::InvalidPermit => write!(f, "Invalid or expired permit"),
            RejectReason::MarketClosed => write!(f, "Market is closed"),
            RejectReason::InvalidPrice => write!(f, "Invalid price"),
            RejectReason::InvalidAmount => write!(f, "Invalid amount"),
            RejectReason::SelfTrade => write!(f, "Self-trade not allowed"),
            RejectReason::PostOnlyWouldMatch => write!(f, "Post-only order would match"),
            RejectReason::RateLimited => write!(f, "Rate limited"),
            RejectReason::CommitmentFailed => write!(f, "Commitment failed"),
            RejectReason::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// A trading order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Order ID
    pub id: OrderId,
    /// Agent placing the order
    pub agent_id: AgentId,
    /// Wallet for funds
    pub wallet_id: WalletId,
    /// Market being traded
    pub market: MarketId,
    /// Buy or sell
    pub side: Side,
    /// Order type and parameters
    pub order_type: OrderType,
    /// Order amount in base currency
    pub amount: Decimal,
    /// Amount already filled
    pub filled: Decimal,
    /// Remaining amount (amount - filled)
    pub remaining: Decimal,
    /// Average fill price
    pub avg_fill_price: Option<Decimal>,
    /// Time in force
    pub tif: TimeInForce,
    /// Spend permit reference
    pub permit_id: PermitId,
    /// Commitment ID from Resonance flow
    pub commitment_id: Option<CommitmentId>,
    /// Current status
    pub status: OrderStatus,
    /// Client order ID (optional)
    pub client_order_id: Option<String>,
    /// When the order was created
    pub created_at: DateTime<Utc>,
    /// When the order was last updated
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// Create a new order builder
    pub fn builder() -> OrderBuilder {
        OrderBuilder::default()
    }

    /// Check if this order can match with another
    pub fn can_match(&self, other: &Order) -> bool {
        if self.side == other.side {
            return false;
        }
        if self.market != other.market {
            return false;
        }
        if self.agent_id == other.agent_id {
            return false; // No self-trade
        }

        match (&self.order_type, &other.order_type) {
            (OrderType::Limit { price: p1, .. }, OrderType::Limit { price: p2, .. }) => {
                if self.side == Side::Buy {
                    *p1 >= *p2 // Buy at p1, sell at p2: matches if p1 >= p2
                } else {
                    *p1 <= *p2 // Sell at p1, buy at p2: matches if p1 <= p2
                }
            }
            (OrderType::Market, _) | (_, OrderType::Market) => true,
            _ => false, // Stop orders don't match until triggered
        }
    }

    /// Calculate the quote amount for this order
    pub fn quote_amount(&self) -> Option<Decimal> {
        self.order_type.price().map(|p| p * self.amount)
    }

    /// Update the order after a fill
    pub fn record_fill(&mut self, fill_amount: Decimal, fill_price: Decimal) {
        let old_filled = self.filled;
        self.filled += fill_amount;
        self.remaining = self.amount - self.filled;

        // Update average fill price
        if old_filled.is_zero() {
            self.avg_fill_price = Some(fill_price);
        } else if let Some(avg) = self.avg_fill_price {
            let total_value = avg * old_filled + fill_price * fill_amount;
            self.avg_fill_price = Some(total_value / self.filled);
        }

        // Update status
        if self.remaining.is_zero() {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartialFill;
        }

        self.updated_at = Utc::now();
    }
}

/// Order builder
#[derive(Default)]
pub struct OrderBuilder {
    agent_id: Option<AgentId>,
    wallet_id: Option<WalletId>,
    market: Option<MarketId>,
    side: Option<Side>,
    order_type: Option<OrderType>,
    amount: Option<Decimal>,
    tif: TimeInForce,
    permit_id: Option<PermitId>,
    client_order_id: Option<String>,
}

impl OrderBuilder {
    pub fn agent(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn wallet(mut self, wallet_id: WalletId) -> Self {
        self.wallet_id = Some(wallet_id);
        self
    }

    pub fn market(mut self, market: MarketId) -> Self {
        self.market = Some(market);
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    pub fn amount(mut self, amount: Decimal) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn tif(mut self, tif: TimeInForce) -> Self {
        self.tif = tif;
        self
    }

    pub fn permit(mut self, permit_id: PermitId) -> Self {
        self.permit_id = Some(permit_id);
        self
    }

    pub fn client_order_id(mut self, id: impl Into<String>) -> Self {
        self.client_order_id = Some(id.into());
        self
    }

    pub fn build(self) -> Result<Order, ExchangeError> {
        let now = Utc::now();
        let amount = self.amount.ok_or(ExchangeError::MissingField("amount"))?;

        Ok(Order {
            id: OrderId::new(),
            agent_id: self.agent_id.ok_or(ExchangeError::MissingField("agent_id"))?,
            wallet_id: self.wallet_id.ok_or(ExchangeError::MissingField("wallet_id"))?,
            market: self.market.ok_or(ExchangeError::MissingField("market"))?,
            side: self.side.ok_or(ExchangeError::MissingField("side"))?,
            order_type: self.order_type.ok_or(ExchangeError::MissingField("order_type"))?,
            amount,
            filled: Decimal::ZERO,
            remaining: amount,
            avg_fill_price: None,
            tif: self.tif,
            permit_id: self.permit_id.ok_or(ExchangeError::MissingField("permit_id"))?,
            commitment_id: None,
            status: OrderStatus::Pending,
            client_order_id: self.client_order_id,
            created_at: now,
            updated_at: now,
        })
    }
}

// ============================================================================
// Trade Types
// ============================================================================

/// A completed trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Trade ID
    pub id: TradeId,
    /// Market
    pub market: MarketId,
    /// Execution price
    pub price: Decimal,
    /// Amount in base currency
    pub amount: Decimal,
    /// Amount in quote currency (price * amount)
    pub quote_amount: Decimal,
    /// Maker (passive) order ID
    pub maker_order_id: OrderId,
    /// Taker (aggressive) order ID
    pub taker_order_id: OrderId,
    /// Maker agent ID
    pub maker_agent_id: AgentId,
    /// Taker agent ID
    pub taker_agent_id: AgentId,
    /// Maker fee
    pub maker_fee: Decimal,
    /// Taker fee
    pub taker_fee: Decimal,
    /// Maker side (the taker side is opposite)
    pub maker_side: Side,
    /// Maker receipt ID
    pub maker_receipt_id: Option<ReceiptId>,
    /// Taker receipt ID
    pub taker_receipt_id: Option<ReceiptId>,
    /// Trade timestamp
    pub timestamp: DateTime<Utc>,
}

impl Trade {
    /// Create a new trade
    pub fn new(
        market: MarketId,
        price: Decimal,
        amount: Decimal,
        maker_order: &Order,
        taker_order: &Order,
        maker_fee: Decimal,
        taker_fee: Decimal,
    ) -> Self {
        Self {
            id: TradeId::new(),
            market,
            price,
            amount,
            quote_amount: price * amount,
            maker_order_id: maker_order.id,
            taker_order_id: taker_order.id,
            maker_agent_id: maker_order.agent_id.clone(),
            taker_agent_id: taker_order.agent_id.clone(),
            maker_fee,
            taker_fee,
            maker_side: maker_order.side,
            maker_receipt_id: None,
            taker_receipt_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Get the taker side
    pub fn taker_side(&self) -> Side {
        self.maker_side.opposite()
    }
}

// ============================================================================
// Market Data Types
// ============================================================================

/// OHLCV candle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    /// Unix timestamp (seconds)
    pub timestamp: i64,
    /// Open price
    pub open: Decimal,
    /// High price
    pub high: Decimal,
    /// Low price
    pub low: Decimal,
    /// Close price
    pub close: Decimal,
    /// Volume in base currency
    pub volume: Decimal,
    /// Volume in quote currency
    pub quote_volume: Decimal,
    /// Number of trades
    pub trade_count: u64,
}

impl Candle {
    /// Create a new candle starting at timestamp with initial price
    pub fn new(timestamp: i64, price: Decimal) -> Self {
        Self {
            timestamp,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: Decimal::ZERO,
            quote_volume: Decimal::ZERO,
            trade_count: 0,
        }
    }

    /// Update candle with a trade
    pub fn update(&mut self, price: Decimal, amount: Decimal, quote_amount: Decimal) {
        if self.trade_count == 0 {
            self.open = price;
            self.high = price;
            self.low = price;
        } else {
            self.high = self.high.max(price);
            self.low = self.low.min(price);
        }
        self.close = price;
        self.volume += amount;
        self.quote_volume += quote_amount;
        self.trade_count += 1;
    }

    /// Check if candle is complete (has trades)
    pub fn is_complete(&self) -> bool {
        self.trade_count > 0
    }
}

/// Candle interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandleInterval {
    M1,   // 1 minute
    M3,   // 3 minutes
    M5,   // 5 minutes
    M15,  // 15 minutes
    M30,  // 30 minutes
    H1,   // 1 hour
    H2,   // 2 hours
    H4,   // 4 hours
    H6,   // 6 hours
    H8,   // 8 hours
    H12,  // 12 hours
    D1,   // 1 day
    D3,   // 3 days
    W1,   // 1 week
    MN1,  // 1 month
}

impl CandleInterval {
    /// Get interval duration in seconds
    pub fn seconds(&self) -> i64 {
        match self {
            CandleInterval::M1 => 60,
            CandleInterval::M3 => 180,
            CandleInterval::M5 => 300,
            CandleInterval::M15 => 900,
            CandleInterval::M30 => 1800,
            CandleInterval::H1 => 3600,
            CandleInterval::H2 => 7200,
            CandleInterval::H4 => 14400,
            CandleInterval::H6 => 21600,
            CandleInterval::H8 => 28800,
            CandleInterval::H12 => 43200,
            CandleInterval::D1 => 86400,
            CandleInterval::D3 => 259200,
            CandleInterval::W1 => 604800,
            CandleInterval::MN1 => 2592000, // 30 days
        }
    }

    /// Get the interval start timestamp for a given timestamp
    pub fn floor(&self, timestamp: i64) -> i64 {
        let secs = self.seconds();
        (timestamp / secs) * secs
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "1M" | "1MIN" | "M1" => Some(CandleInterval::M1),
            "3M" | "3MIN" | "M3" => Some(CandleInterval::M3),
            "5M" | "5MIN" | "M5" => Some(CandleInterval::M5),
            "15M" | "15MIN" | "M15" => Some(CandleInterval::M15),
            "30M" | "30MIN" | "M30" => Some(CandleInterval::M30),
            "1H" | "H1" => Some(CandleInterval::H1),
            "2H" | "H2" => Some(CandleInterval::H2),
            "4H" | "H4" => Some(CandleInterval::H4),
            "6H" | "H6" => Some(CandleInterval::H6),
            "8H" | "H8" => Some(CandleInterval::H8),
            "12H" | "H12" => Some(CandleInterval::H12),
            "1D" | "D1" => Some(CandleInterval::D1),
            "3D" | "D3" => Some(CandleInterval::D3),
            "1W" | "W1" => Some(CandleInterval::W1),
            "1MN" | "MN1" => Some(CandleInterval::MN1),
            _ => None,
        }
    }
}

impl fmt::Display for CandleInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CandleInterval::M1 => "1m",
            CandleInterval::M3 => "3m",
            CandleInterval::M5 => "5m",
            CandleInterval::M15 => "15m",
            CandleInterval::M30 => "30m",
            CandleInterval::H1 => "1h",
            CandleInterval::H2 => "2h",
            CandleInterval::H4 => "4h",
            CandleInterval::H6 => "6h",
            CandleInterval::H8 => "8h",
            CandleInterval::H12 => "12h",
            CandleInterval::D1 => "1d",
            CandleInterval::D3 => "3d",
            CandleInterval::W1 => "1w",
            CandleInterval::MN1 => "1M",
        };
        write!(f, "{}", s)
    }
}

/// Order book depth level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthLevel {
    pub price: Decimal,
    pub amount: Decimal,
}

/// Order book depth snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthSnapshot {
    pub market: MarketId,
    /// Bids (buy orders) sorted by price descending
    pub bids: Vec<DepthLevel>,
    /// Asks (sell orders) sorted by price ascending
    pub asks: Vec<DepthLevel>,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
}

impl DepthSnapshot {
    pub fn new(market: MarketId) -> Self {
        Self {
            market,
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Get the best bid price
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.first().map(|l| l.price)
    }

    /// Get the best ask price
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.first().map(|l| l.price)
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
}

/// Market ticker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    pub market: MarketId,
    /// Last trade price
    pub last_price: Decimal,
    /// Best bid price
    pub bid: Decimal,
    /// Best ask price
    pub ask: Decimal,
    /// 24h high
    pub high_24h: Decimal,
    /// 24h low
    pub low_24h: Decimal,
    /// 24h volume in base currency
    pub volume_24h: Decimal,
    /// 24h volume in quote currency
    pub quote_volume_24h: Decimal,
    /// 24h price change percentage
    pub change_24h: Decimal,
    /// 24h trade count
    pub trade_count_24h: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl Ticker {
    pub fn new(market: MarketId) -> Self {
        Self {
            market,
            last_price: Decimal::ZERO,
            bid: Decimal::ZERO,
            ask: Decimal::ZERO,
            high_24h: Decimal::ZERO,
            low_24h: Decimal::ZERO,
            volume_24h: Decimal::ZERO,
            quote_volume_24h: Decimal::ZERO,
            change_24h: Decimal::ZERO,
            trade_count_24h: 0,
            timestamp: Utc::now(),
        }
    }
}

// ============================================================================
// Orderbook Key (for BTreeMap sorting)
// ============================================================================

/// Key for orderbook entries - sorts by (price, timestamp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookKey {
    /// Price (negated for bids to sort descending)
    pub price: Decimal,
    /// Timestamp in microseconds for time priority
    pub timestamp_us: u64,
    /// Order ID for uniqueness
    pub order_id: OrderId,
}

impl OrderBookKey {
    pub fn new(price: Decimal, timestamp_us: u64, order_id: OrderId) -> Self {
        Self {
            price,
            timestamp_us,
            order_id,
        }
    }

    /// Create a bid key (price negated for descending sort)
    pub fn bid(price: Decimal, timestamp_us: u64, order_id: OrderId) -> Self {
        Self {
            price: -price, // Negate for descending sort
            timestamp_us,
            order_id,
        }
    }

    /// Create an ask key (price as-is for ascending sort)
    pub fn ask(price: Decimal, timestamp_us: u64, order_id: OrderId) -> Self {
        Self {
            price,
            timestamp_us,
            order_id,
        }
    }

    /// Get the actual price (un-negated)
    pub fn actual_price(&self) -> Decimal {
        self.price.abs()
    }
}

impl PartialEq for OrderBookKey {
    fn eq(&self, other: &Self) -> bool {
        self.order_id == other.order_id
    }
}

impl Eq for OrderBookKey {}

impl PartialOrd for OrderBookKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderBookKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by price
        match self.price.cmp(&other.price) {
            Ordering::Equal => {
                // Then by timestamp (earlier = better)
                match self.timestamp_us.cmp(&other.timestamp_us) {
                    Ordering::Equal => self.order_id.0.cmp(&other.order_id.0),
                    other => other,
                }
            }
            other => other,
        }
    }
}

/// Orderbook entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub order_id: OrderId,
    pub agent_id: AgentId,
    pub price: Decimal,
    pub remaining: Decimal,
    pub timestamp_us: u64,
}

// ============================================================================
// Events
// ============================================================================

/// Engine events for event sourcing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineEvent {
    /// Order placed on book
    OrderPlaced {
        order: Order,
        timestamp: DateTime<Utc>,
    },
    /// Order cancelled
    OrderCancelled {
        order_id: OrderId,
        remaining: Decimal,
        timestamp: DateTime<Utc>,
    },
    /// Trade executed
    TradeExecuted {
        trade: Trade,
    },
    /// Order expired
    OrderExpired {
        order_id: OrderId,
        timestamp: DateTime<Utc>,
    },
    /// Market status changed
    MarketStatusChanged {
        market: MarketId,
        old_status: MarketStatus,
        new_status: MarketStatus,
        timestamp: DateTime<Utc>,
    },
}

// ============================================================================
// Error Types
// ============================================================================

/// Exchange errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExchangeError {
    #[error("Market not found: {0}")]
    MarketNotFound(MarketId),

    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    #[error("Market is closed: {0}")]
    MarketClosed(MarketId),

    #[error("Amount too small: min {min}, got {got}")]
    AmountTooSmall { min: Decimal, got: Decimal },

    #[error("Amount too large: max {max}, got {got}")]
    AmountTooLarge { max: Decimal, got: Decimal },

    #[error("Invalid price: {0}")]
    InvalidPrice(Decimal),

    #[error("Insufficient funds: needed {needed}, available {available}")]
    InsufficientFunds { needed: Decimal, available: Decimal },

    #[error("Invalid permit: {0}")]
    InvalidPermit(String),

    #[error("Self-trade not allowed")]
    SelfTrade,

    #[error("Post-only order would match")]
    PostOnlyWouldMatch,

    #[error("Order rejected: {0}")]
    OrderRejected(RejectReason),

    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Commitment failed: {0}")]
    CommitmentFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_id_parse() {
        let market = MarketId::new("ETH_IUSD");
        let (base, quote) = market.parse_pair().unwrap();
        assert_eq!(base, "ETH");
        assert_eq!(quote, "IUSD");
    }

    #[test]
    fn test_order_builder() {
        let order = Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Buy)
            .order_type(OrderType::limit(dec!(3245.00)))
            .amount(dec!(1.5))
            .permit(PermitId::new())
            .build()
            .unwrap();

        assert_eq!(order.market.0, "ETH_IUSD");
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.amount, dec!(1.5));
        assert_eq!(order.remaining, dec!(1.5));
        assert_eq!(order.filled, dec!(0));
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Buy)
            .order_type(OrderType::limit(dec!(3245.00)))
            .amount(dec!(10.0))
            .permit(PermitId::new())
            .build()
            .unwrap();

        order.record_fill(dec!(3.0), dec!(3245.00));
        assert_eq!(order.filled, dec!(3.0));
        assert_eq!(order.remaining, dec!(7.0));
        assert_eq!(order.status, OrderStatus::PartialFill);

        order.record_fill(dec!(7.0), dec!(3246.00));
        assert_eq!(order.filled, dec!(10.0));
        assert_eq!(order.remaining, dec!(0));
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_candle_update() {
        let mut candle = Candle::new(1700000000, dec!(3245.00));
        assert_eq!(candle.trade_count, 0);

        // First trade sets OHLC to the trade price (overrides initial)
        candle.update(dec!(3250.00), dec!(1.0), dec!(3250.00));
        assert_eq!(candle.open, dec!(3250.00)); // First trade becomes open
        assert_eq!(candle.high, dec!(3250.00));
        assert_eq!(candle.low, dec!(3250.00));  // Low is also first trade price
        assert_eq!(candle.close, dec!(3250.00));
        assert_eq!(candle.volume, dec!(1.0));
        assert_eq!(candle.trade_count, 1);

        // Second trade updates high/low/close
        candle.update(dec!(3240.00), dec!(0.5), dec!(1620.00));
        assert_eq!(candle.open, dec!(3250.00)); // Open unchanged
        assert_eq!(candle.high, dec!(3250.00));
        assert_eq!(candle.low, dec!(3240.00));  // New low
        assert_eq!(candle.close, dec!(3240.00));
        assert_eq!(candle.volume, dec!(1.5));
        assert_eq!(candle.trade_count, 2);
    }

    #[test]
    fn test_candle_interval() {
        assert_eq!(CandleInterval::M1.seconds(), 60);
        assert_eq!(CandleInterval::H1.seconds(), 3600);
        assert_eq!(CandleInterval::D1.seconds(), 86400);

        // Floor test
        let ts = 1700000123;
        assert_eq!(CandleInterval::M1.floor(ts), 1700000100); // 1700000100 = 1700000123 - 23
    }

    #[test]
    fn test_orderbook_key_sorting() {
        let mut keys = vec![
            OrderBookKey::ask(dec!(100), 2, OrderId::new()),
            OrderBookKey::ask(dec!(100), 1, OrderId::new()),
            OrderBookKey::ask(dec!(99), 3, OrderId::new()),
        ];
        keys.sort();

        // Should be: 99 first (lowest), then 100 with ts=1, then 100 with ts=2
        assert_eq!(keys[0].actual_price(), dec!(99));
        assert_eq!(keys[1].actual_price(), dec!(100));
        assert_eq!(keys[1].timestamp_us, 1);
        assert_eq!(keys[2].actual_price(), dec!(100));
        assert_eq!(keys[2].timestamp_us, 2);
    }

    #[test]
    fn test_depth_snapshot() {
        let mut depth = DepthSnapshot::new(MarketId::new("ETH_IUSD"));
        depth.bids.push(DepthLevel { price: dec!(3244), amount: dec!(1.0) });
        depth.bids.push(DepthLevel { price: dec!(3243), amount: dec!(2.0) });
        depth.asks.push(DepthLevel { price: dec!(3246), amount: dec!(0.5) });
        depth.asks.push(DepthLevel { price: dec!(3247), amount: dec!(1.5) });

        assert_eq!(depth.best_bid(), Some(dec!(3244)));
        assert_eq!(depth.best_ask(), Some(dec!(3246)));
        assert_eq!(depth.spread(), Some(dec!(2)));
        assert_eq!(depth.mid_price(), Some(dec!(3245)));
    }

    #[test]
    fn test_trade_creation() {
        let maker = Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Sell)
            .order_type(OrderType::limit(dec!(3245.00)))
            .amount(dec!(1.0))
            .permit(PermitId::new())
            .build()
            .unwrap();

        let taker = Order::builder()
            .agent(AgentId::new())
            .wallet(WalletId::new())
            .market(MarketId::new("ETH_IUSD"))
            .side(Side::Buy)
            .order_type(OrderType::Market)
            .amount(dec!(1.0))
            .permit(PermitId::new())
            .build()
            .unwrap();

        let trade = Trade::new(
            MarketId::new("ETH_IUSD"),
            dec!(3245.00),
            dec!(1.0),
            &maker,
            &taker,
            dec!(3.245),  // 0.1% maker fee
            dec!(6.49),   // 0.2% taker fee
        );

        assert_eq!(trade.price, dec!(3245.00));
        assert_eq!(trade.amount, dec!(1.0));
        assert_eq!(trade.quote_amount, dec!(3245.00));
        assert_eq!(trade.maker_side, Side::Sell);
        assert_eq!(trade.taker_side(), Side::Buy);
    }
}
