//! Market Data DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// =============================================================================
// Exchange Info
// =============================================================================

/// Exchange information response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeInfo {
    /// Timezone
    pub timezone: String,
    /// Server time
    pub server_time: i64,
    /// Rate limits
    pub rate_limits: Vec<RateLimit>,
    /// Exchange filters
    pub exchange_filters: Vec<ExchangeFilter>,
    /// Symbols
    pub symbols: Vec<SymbolInfo>,
}

/// Rate limit info
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    /// Rate limit type (REQUEST_WEIGHT, ORDERS, RAW_REQUESTS)
    pub rate_limit_type: String,
    /// Interval (SECOND, MINUTE, DAY)
    pub interval: String,
    /// Interval num
    pub interval_num: i32,
    /// Limit
    pub limit: i32,
}

/// Exchange filter
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeFilter {
    /// Filter type
    pub filter_type: String,
    /// Max number of orders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_orders: Option<i32>,
    /// Max number of algo orders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_algo_orders: Option<i32>,
}

/// Symbol information
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SymbolInfo {
    /// Symbol
    pub symbol: String,
    /// Status
    pub status: String,
    /// Base asset
    pub base_asset: String,
    /// Base asset precision
    pub base_asset_precision: i32,
    /// Quote asset
    pub quote_asset: String,
    /// Quote precision
    pub quote_precision: i32,
    /// Quote asset precision
    pub quote_asset_precision: i32,
    /// Base commission precision
    pub base_commission_precision: i32,
    /// Quote commission precision
    pub quote_commission_precision: i32,
    /// Order types
    pub order_types: Vec<String>,
    /// Iceberg allowed
    pub iceberg_allowed: bool,
    /// OCO allowed
    pub oco_allowed: bool,
    /// Quote order qty market allowed
    pub quote_order_qty_market_allowed: bool,
    /// Allow trailing stop
    pub allow_trailing_stop: bool,
    /// Cancel replace allowed
    pub cancel_replace_allowed: bool,
    /// Is spot trading allowed
    pub is_spot_trading_allowed: bool,
    /// Is margin trading allowed
    pub is_margin_trading_allowed: bool,
    /// Filters
    pub filters: Vec<SymbolFilter>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Default self trade prevention mode
    pub default_self_trade_prevention_mode: String,
    /// Allowed self trade prevention modes
    pub allowed_self_trade_prevention_modes: Vec<String>,
}

/// Symbol filter
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SymbolFilter {
    /// Filter type
    pub filter_type: String,
    /// Min price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_price: Option<String>,
    /// Max price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<String>,
    /// Tick size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick_size: Option<String>,
    /// Min quantity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_qty: Option<String>,
    /// Max quantity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_qty: Option<String>,
    /// Step size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_size: Option<String>,
    /// Limit (for lot size)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    /// Min notional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_notional: Option<String>,
    /// Apply to market
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_to_market: Option<bool>,
    /// Avg price mins
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_price_mins: Option<i32>,
    /// Max number of orders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_orders: Option<i32>,
    /// Max number of algo orders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_algo_orders: Option<i32>,
    /// Max position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_position: Option<String>,
    /// Multiplier up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier_up: Option<String>,
    /// Multiplier down
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier_down: Option<String>,
    /// Multiplier decimal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier_decimal: Option<String>,
}

// =============================================================================
// Order Book
// =============================================================================

/// Order book depth request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OrderBookQuery {
    /// Symbol
    pub symbol: String,
    /// Limit (5, 10, 20, 50, 100, 500, 1000, 5000)
    #[serde(default)]
    pub limit: Option<i32>,
}

/// Order book response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    /// Last update ID
    pub last_update_id: i64,
    /// Bids [price, quantity]
    pub bids: Vec<[String; 2]>,
    /// Asks [price, quantity]
    pub asks: Vec<[String; 2]>,
}

// =============================================================================
// Recent Trades
// =============================================================================

/// Recent trades query
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecentTradesQuery {
    /// Symbol
    pub symbol: String,
    /// Limit (default 500, max 1000)
    #[serde(default)]
    pub limit: Option<i32>,
}

/// Trade record
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradeRecord {
    /// Trade ID
    pub id: i64,
    /// Price
    pub price: String,
    /// Quantity
    pub qty: String,
    /// Quote quantity
    pub quote_qty: String,
    /// Time
    pub time: i64,
    /// Is buyer maker
    pub is_buyer_maker: bool,
    /// Is best match
    pub is_best_match: bool,
}

/// Historical trades query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalTradesQuery {
    /// Symbol
    pub symbol: String,
    /// Limit
    #[serde(default)]
    pub limit: Option<i32>,
    /// Trade ID to start from
    #[serde(default)]
    pub from_id: Option<i64>,
}

// =============================================================================
// Aggregate Trades
// =============================================================================

/// Aggregate trades query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AggTradesQuery {
    /// Symbol
    pub symbol: String,
    /// From aggregate trade ID
    #[serde(default)]
    pub from_id: Option<i64>,
    /// Start time
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Limit (default 500, max 1000)
    #[serde(default)]
    pub limit: Option<i32>,
}

/// Aggregate trade record
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AggTrade {
    /// Aggregate trade ID
    #[serde(rename = "a")]
    pub agg_trade_id: i64,
    /// Price
    #[serde(rename = "p")]
    pub price: String,
    /// Quantity
    #[serde(rename = "q")]
    pub qty: String,
    /// First trade ID
    #[serde(rename = "f")]
    pub first_id: i64,
    /// Last trade ID
    #[serde(rename = "l")]
    pub last_id: i64,
    /// Timestamp
    #[serde(rename = "T")]
    pub timestamp: i64,
    /// Is buyer maker
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
    /// Is best match (always true)
    #[serde(rename = "M")]
    pub is_best_match: bool,
}

// =============================================================================
// Klines (Candlestick)
// =============================================================================

/// Kline interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum KlineInterval {
    #[serde(rename = "1s")]
    OneSecond,
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "3m")]
    ThreeMinutes,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    TwoHours,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "6h")]
    SixHours,
    #[serde(rename = "8h")]
    EightHours,
    #[serde(rename = "12h")]
    TwelveHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "3d")]
    ThreeDays,
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "1M")]
    OneMonth,
}

/// Klines query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KlinesQuery {
    /// Symbol
    pub symbol: String,
    /// Interval
    pub interval: KlineInterval,
    /// Start time
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Time zone (default: 0 UTC)
    #[serde(default)]
    pub time_zone: Option<String>,
    /// Limit (default 500, max 1000)
    #[serde(default)]
    pub limit: Option<i32>,
}

/// Kline data (returned as array for efficiency)
/// [open_time, open, high, low, close, volume, close_time, quote_volume, trades, taker_buy_base, taker_buy_quote, ignore]
pub type Kline = (i64, String, String, String, String, String, i64, String, i64, String, String, String);

/// UI Klines query (with time zone support)
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UiKlinesQuery {
    /// Symbol
    pub symbol: String,
    /// Interval
    pub interval: KlineInterval,
    /// Start time
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Time zone
    #[serde(default)]
    pub time_zone: Option<String>,
    /// Limit
    #[serde(default)]
    pub limit: Option<i32>,
}

// =============================================================================
// Ticker
// =============================================================================

/// 24hr ticker query
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TickerQuery {
    /// Symbol (optional for all symbols)
    #[serde(default)]
    pub symbol: Option<String>,
    /// Symbols (array)
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    /// Type (FULL or MINI)
    #[serde(default, rename = "type")]
    pub ticker_type: Option<String>,
}

/// 24hr ticker statistics (full)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ticker24hr {
    /// Symbol
    pub symbol: String,
    /// Price change
    pub price_change: String,
    /// Price change percent
    pub price_change_percent: String,
    /// Weighted average price
    pub weighted_avg_price: String,
    /// Previous close price
    pub prev_close_price: String,
    /// Last price
    pub last_price: String,
    /// Last quantity
    pub last_qty: String,
    /// Bid price
    pub bid_price: String,
    /// Bid quantity
    pub bid_qty: String,
    /// Ask price
    pub ask_price: String,
    /// Ask quantity
    pub ask_qty: String,
    /// Open price
    pub open_price: String,
    /// High price
    pub high_price: String,
    /// Low price
    pub low_price: String,
    /// Volume
    pub volume: String,
    /// Quote volume
    pub quote_volume: String,
    /// Open time
    pub open_time: i64,
    /// Close time
    pub close_time: i64,
    /// First trade ID
    pub first_id: i64,
    /// Last trade ID
    pub last_id: i64,
    /// Trade count
    pub count: i64,
}

/// 24hr ticker statistics (mini)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ticker24hrMini {
    /// Symbol
    pub symbol: String,
    /// Open price
    pub open_price: String,
    /// High price
    pub high_price: String,
    /// Low price
    pub low_price: String,
    /// Last price
    pub last_price: String,
    /// Volume
    pub volume: String,
    /// Quote volume
    pub quote_volume: String,
    /// Open time
    pub open_time: i64,
    /// Close time
    pub close_time: i64,
    /// First trade ID
    pub first_id: i64,
    /// Last trade ID
    pub last_id: i64,
    /// Trade count
    pub count: i64,
}

/// Trading day ticker query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TradingDayTickerQuery {
    /// Symbol
    #[serde(default)]
    pub symbol: Option<String>,
    /// Symbols
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    /// Time zone
    #[serde(default)]
    pub time_zone: Option<String>,
    /// Type (FULL or MINI)
    #[serde(default, rename = "type")]
    pub ticker_type: Option<String>,
}

/// Price ticker
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PriceTicker {
    /// Symbol
    pub symbol: String,
    /// Price
    pub price: String,
}

/// Book ticker (best bid/ask)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookTicker {
    /// Symbol
    pub symbol: String,
    /// Bid price
    pub bid_price: String,
    /// Bid quantity
    pub bid_qty: String,
    /// Ask price
    pub ask_price: String,
    /// Ask quantity
    pub ask_qty: String,
}

// =============================================================================
// Rolling Window Statistics
// =============================================================================

/// Rolling window ticker query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RollingWindowQuery {
    /// Symbol
    #[serde(default)]
    pub symbol: Option<String>,
    /// Symbols
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    /// Window size (e.g., "1h", "4h", "1d")
    #[serde(default)]
    pub window_size: Option<String>,
    /// Type (FULL or MINI)
    #[serde(default, rename = "type")]
    pub ticker_type: Option<String>,
}

/// Rolling window ticker
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RollingWindowTicker {
    /// Symbol
    pub symbol: String,
    /// Price change
    pub price_change: String,
    /// Price change percent
    pub price_change_percent: String,
    /// Weighted average price
    pub weighted_avg_price: String,
    /// Open price
    pub open_price: String,
    /// High price
    pub high_price: String,
    /// Low price
    pub low_price: String,
    /// Last price
    pub last_price: String,
    /// Volume
    pub volume: String,
    /// Quote volume
    pub quote_volume: String,
    /// Open time
    pub open_time: i64,
    /// Close time
    pub close_time: i64,
    /// First trade ID
    pub first_id: i64,
    /// Last trade ID
    pub last_id: i64,
    /// Trade count
    pub count: i64,
}

// =============================================================================
// Average Price
// =============================================================================

/// Average price query
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AvgPriceQuery {
    /// Symbol
    pub symbol: String,
}

/// Average price response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvgPrice {
    /// Minutes (interval)
    pub mins: i32,
    /// Price
    pub price: String,
    /// Close time
    #[serde(rename = "closeTime")]
    pub close_time: i64,
}
