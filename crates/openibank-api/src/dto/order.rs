//! Order and Trading DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

// =============================================================================
// Order Types and Enums
// =============================================================================

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    StopLoss,
    StopLossLimit,
    TakeProfit,
    TakeProfitLimit,
    LimitMaker,
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    /// Good Till Cancel
    Gtc,
    /// Immediate Or Cancel
    Ioc,
    /// Fill Or Kill
    Fok,
    /// Good Till Date
    Gtd,
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::Gtc
    }
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    PendingCancel,
    Rejected,
    Expired,
    ExpiredInMatch,
}

/// Self-trade prevention mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SelfTradePreventionMode {
    /// Expire taker order
    ExpireTaker,
    /// Expire maker order
    ExpireMaker,
    /// Expire both orders
    ExpireBoth,
    /// No self-trade prevention
    None,
}

impl Default for SelfTradePreventionMode {
    fn default() -> Self {
        Self::None
    }
}

// =============================================================================
// Create Order
// =============================================================================

/// Create new order request (Binance-compatible)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderRequest {
    /// Trading pair symbol (e.g., "BTCUSDT")
    #[validate(length(min = 1, max = 20))]
    pub symbol: String,

    /// Order side (BUY/SELL)
    pub side: OrderSide,

    /// Order type
    #[serde(rename = "type")]
    pub order_type: OrderType,

    /// Time in force
    #[serde(default)]
    pub time_in_force: Option<TimeInForce>,

    /// Order quantity
    #[serde(default)]
    pub quantity: Option<String>,

    /// Quote order quantity (for MARKET orders)
    #[serde(default)]
    pub quote_order_qty: Option<String>,

    /// Limit price
    #[serde(default)]
    pub price: Option<String>,

    /// Client order ID
    #[serde(default)]
    #[validate(length(max = 36))]
    pub new_client_order_id: Option<String>,

    /// Strategy ID
    #[serde(default)]
    pub strategy_id: Option<i64>,

    /// Strategy type
    #[serde(default)]
    pub strategy_type: Option<i32>,

    /// Stop price (for STOP_LOSS, STOP_LOSS_LIMIT, TAKE_PROFIT, TAKE_PROFIT_LIMIT)
    #[serde(default)]
    pub stop_price: Option<String>,

    /// Trailing delta
    #[serde(default)]
    pub trailing_delta: Option<i64>,

    /// Iceberg quantity
    #[serde(default)]
    pub iceberg_qty: Option<String>,

    /// Response type (ACK, RESULT, FULL)
    #[serde(default)]
    pub new_order_resp_type: Option<OrderResponseType>,

    /// Self-trade prevention mode
    #[serde(default)]
    pub self_trade_prevention_mode: Option<SelfTradePreventionMode>,

    /// Receive window (ms)
    #[serde(default)]
    pub recv_window: Option<i64>,
}

/// Order response type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderResponseType {
    Ack,
    Result,
    Full,
}

impl Default for OrderResponseType {
    fn default() -> Self {
        Self::Ack
    }
}

/// Order response (ACK type - minimal)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderAckResponse {
    /// Symbol
    pub symbol: String,
    /// Order ID
    pub order_id: i64,
    /// Order list ID (-1 for non-OCO)
    pub order_list_id: i64,
    /// Client order ID
    pub client_order_id: String,
    /// Transaction time
    pub transact_time: i64,
}

/// Order response (RESULT type)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderResultResponse {
    /// Symbol
    pub symbol: String,
    /// Order ID
    pub order_id: i64,
    /// Order list ID
    pub order_list_id: i64,
    /// Client order ID
    pub client_order_id: String,
    /// Transaction time
    pub transact_time: i64,
    /// Price
    pub price: String,
    /// Original quantity
    pub orig_qty: String,
    /// Executed quantity
    pub executed_qty: String,
    /// Cumulative quote quantity
    pub cummulative_quote_qty: String,
    /// Status
    pub status: OrderStatus,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Order type
    #[serde(rename = "type")]
    pub order_type: OrderType,
    /// Side
    pub side: OrderSide,
    /// Working time
    pub working_time: i64,
    /// Self trade prevention mode
    pub self_trade_prevention_mode: SelfTradePreventionMode,
}

/// Order response (FULL type - includes fills)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderFullResponse {
    /// Symbol
    pub symbol: String,
    /// Order ID
    pub order_id: i64,
    /// Order list ID
    pub order_list_id: i64,
    /// Client order ID
    pub client_order_id: String,
    /// Transaction time
    pub transact_time: i64,
    /// Price
    pub price: String,
    /// Original quantity
    pub orig_qty: String,
    /// Executed quantity
    pub executed_qty: String,
    /// Cumulative quote quantity
    pub cummulative_quote_qty: String,
    /// Status
    pub status: OrderStatus,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Order type
    #[serde(rename = "type")]
    pub order_type: OrderType,
    /// Side
    pub side: OrderSide,
    /// Working time
    pub working_time: i64,
    /// Self trade prevention mode
    pub self_trade_prevention_mode: SelfTradePreventionMode,
    /// Fill details
    pub fills: Vec<OrderFill>,
}

/// Individual fill in an order
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderFill {
    /// Fill price
    pub price: String,
    /// Fill quantity
    pub qty: String,
    /// Commission amount
    pub commission: String,
    /// Commission asset
    pub commission_asset: String,
    /// Trade ID
    pub trade_id: i64,
}

// =============================================================================
// Query/Cancel Orders
// =============================================================================

/// Query order request
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueryOrderRequest {
    /// Symbol
    pub symbol: String,
    /// Order ID
    #[serde(default)]
    pub order_id: Option<i64>,
    /// Original client order ID
    #[serde(default)]
    pub orig_client_order_id: Option<String>,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

/// Order info response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfo {
    /// Symbol
    pub symbol: String,
    /// Order ID
    pub order_id: i64,
    /// Order list ID
    pub order_list_id: i64,
    /// Client order ID
    pub client_order_id: String,
    /// Price
    pub price: String,
    /// Original quantity
    pub orig_qty: String,
    /// Executed quantity
    pub executed_qty: String,
    /// Cumulative quote quantity
    pub cummulative_quote_qty: String,
    /// Status
    pub status: OrderStatus,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Order type
    #[serde(rename = "type")]
    pub order_type: OrderType,
    /// Side
    pub side: OrderSide,
    /// Stop price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
    /// Iceberg quantity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iceberg_qty: Option<String>,
    /// Time
    pub time: i64,
    /// Update time
    pub update_time: i64,
    /// Is working
    pub is_working: bool,
    /// Working time
    pub working_time: i64,
    /// Original quote order quantity
    pub orig_quote_order_qty: String,
    /// Self trade prevention mode
    pub self_trade_prevention_mode: SelfTradePreventionMode,
}

/// Cancel order request
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    /// Symbol
    pub symbol: String,
    /// Order ID
    #[serde(default)]
    pub order_id: Option<i64>,
    /// Original client order ID
    #[serde(default)]
    pub orig_client_order_id: Option<String>,
    /// New client order ID for the cancel
    #[serde(default)]
    pub new_client_order_id: Option<String>,
    /// Cancel restrictions
    #[serde(default)]
    pub cancel_restrictions: Option<CancelRestrictions>,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

/// Cancel restrictions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CancelRestrictions {
    OnlyNew,
    OnlyPartiallyFilled,
}

/// Cancel order response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderResponse {
    /// Symbol
    pub symbol: String,
    /// Original client order ID
    pub orig_client_order_id: String,
    /// Order ID
    pub order_id: i64,
    /// Order list ID
    pub order_list_id: i64,
    /// Client order ID
    pub client_order_id: String,
    /// Transaction time
    pub transact_time: i64,
    /// Price
    pub price: String,
    /// Original quantity
    pub orig_qty: String,
    /// Executed quantity
    pub executed_qty: String,
    /// Cumulative quote quantity
    pub cummulative_quote_qty: String,
    /// Status
    pub status: OrderStatus,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Order type
    #[serde(rename = "type")]
    pub order_type: OrderType,
    /// Side
    pub side: OrderSide,
    /// Self trade prevention mode
    pub self_trade_prevention_mode: SelfTradePreventionMode,
}

/// Cancel all orders request
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelAllOrdersRequest {
    /// Symbol
    pub symbol: String,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

// =============================================================================
// Open Orders
// =============================================================================

/// Open orders query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrdersQuery {
    /// Symbol (optional - if not provided, returns all symbols)
    #[serde(default)]
    pub symbol: Option<String>,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

// =============================================================================
// All Orders History
// =============================================================================

/// All orders query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AllOrdersQuery {
    /// Symbol
    pub symbol: String,
    /// Order ID to start from
    #[serde(default)]
    pub order_id: Option<i64>,
    /// Start time
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Limit (default 500, max 1000)
    #[serde(default)]
    pub limit: Option<i32>,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

// =============================================================================
// Trade History
// =============================================================================

/// Account trades query
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountTradesQuery {
    /// Symbol
    pub symbol: String,
    /// Order ID
    #[serde(default)]
    pub order_id: Option<i64>,
    /// Start time
    #[serde(default)]
    pub start_time: Option<i64>,
    /// End time
    #[serde(default)]
    pub end_time: Option<i64>,
    /// Trade ID to start from
    #[serde(default)]
    pub from_id: Option<i64>,
    /// Limit (default 500, max 1000)
    #[serde(default)]
    pub limit: Option<i32>,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

/// Account trade record
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountTrade {
    /// Symbol
    pub symbol: String,
    /// Trade ID
    pub id: i64,
    /// Order ID
    pub order_id: i64,
    /// Order list ID
    pub order_list_id: i64,
    /// Price
    pub price: String,
    /// Quantity
    pub qty: String,
    /// Quote quantity
    pub quote_qty: String,
    /// Commission
    pub commission: String,
    /// Commission asset
    pub commission_asset: String,
    /// Trade time
    pub time: i64,
    /// Is buyer
    pub is_buyer: bool,
    /// Is maker
    pub is_maker: bool,
    /// Is best match
    pub is_best_match: bool,
}

// =============================================================================
// OCO Orders
// =============================================================================

/// OCO (One-Cancels-Other) order request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateOcoOrderRequest {
    /// Symbol
    pub symbol: String,
    /// List client order ID
    #[serde(default)]
    pub list_client_order_id: Option<String>,
    /// Side
    pub side: OrderSide,
    /// Quantity
    pub quantity: String,
    /// Limit client order ID
    #[serde(default)]
    pub limit_client_order_id: Option<String>,
    /// Limit price
    pub price: String,
    /// Limit iceberg quantity
    #[serde(default)]
    pub limit_iceberg_qty: Option<String>,
    /// Trailing delta
    #[serde(default)]
    pub trailing_delta: Option<i64>,
    /// Stop client order ID
    #[serde(default)]
    pub stop_client_order_id: Option<String>,
    /// Stop price
    pub stop_price: String,
    /// Stop limit price
    #[serde(default)]
    pub stop_limit_price: Option<String>,
    /// Stop iceberg quantity
    #[serde(default)]
    pub stop_iceberg_qty: Option<String>,
    /// Stop limit time in force
    #[serde(default)]
    pub stop_limit_time_in_force: Option<TimeInForce>,
    /// Response type
    #[serde(default)]
    pub new_order_resp_type: Option<OrderResponseType>,
    /// Self trade prevention mode
    #[serde(default)]
    pub self_trade_prevention_mode: Option<SelfTradePreventionMode>,
    /// Receive window
    #[serde(default)]
    pub recv_window: Option<i64>,
}

/// OCO order response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OcoOrderResponse {
    /// Order list ID
    pub order_list_id: i64,
    /// Contingency type
    pub contingency_type: String,
    /// List status type
    pub list_status_type: String,
    /// List order status
    pub list_order_status: String,
    /// List client order ID
    pub list_client_order_id: String,
    /// Transaction time
    pub transaction_time: i64,
    /// Symbol
    pub symbol: String,
    /// Orders in the OCO
    pub orders: Vec<OcoOrderInfo>,
    /// Order reports (detailed)
    pub order_reports: Vec<OrderResultResponse>,
}

/// OCO order info (summary)
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OcoOrderInfo {
    /// Symbol
    pub symbol: String,
    /// Order ID
    pub order_id: i64,
    /// Client order ID
    pub client_order_id: String,
}

// =============================================================================
// Order Count
// =============================================================================

/// Rate limit info response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitInfo {
    /// Rate limit type
    pub rate_limit_type: String,
    /// Interval
    pub interval: String,
    /// Interval num
    pub interval_num: i32,
    /// Limit
    pub limit: i32,
    /// Current count
    pub count: i32,
}
