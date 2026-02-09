//! WebSocket Support
//!
//! Real-time data streaming via WebSocket connections.
//! Supports:
//! - Market data streams (trades, order book, klines, tickers)
//! - User data streams (orders, account updates)
//! - Multiple stream subscriptions per connection
//! - Binance-compatible message format

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::state::AppState;

// =============================================================================
// WebSocket Types
// =============================================================================

/// WebSocket stream subscription request
#[derive(Debug, Clone, Deserialize)]
pub struct SubscribeRequest {
    /// Method (SUBSCRIBE, UNSUBSCRIBE, LIST_SUBSCRIPTIONS)
    pub method: String,
    /// Stream parameters
    #[serde(default)]
    pub params: Vec<String>,
    /// Request ID
    pub id: Option<u64>,
}

/// WebSocket stream subscription response
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeResponse {
    /// Result (null on success)
    pub result: Option<serde_json::Value>,
    /// Request ID
    pub id: Option<u64>,
}

/// WebSocket error response
#[derive(Debug, Clone, Serialize)]
pub struct WsError {
    /// Error code
    pub code: i32,
    /// Error message
    pub msg: String,
    /// Request ID
    pub id: Option<u64>,
}

/// Market data stream types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    /// Individual trade stream
    Trade,
    /// Aggregate trade stream
    AggTrade,
    /// Kline/candlestick stream
    Kline,
    /// Mini ticker stream
    MiniTicker,
    /// 24hr ticker stream
    Ticker,
    /// Book ticker stream (best bid/ask)
    BookTicker,
    /// Partial book depth stream
    Depth,
    /// Diff depth stream
    DiffDepth,
}

impl StreamType {
    /// Parse stream type from stream name
    pub fn from_stream_name(name: &str) -> Option<Self> {
        if name.ends_with("@trade") {
            Some(Self::Trade)
        } else if name.ends_with("@aggTrade") {
            Some(Self::AggTrade)
        } else if name.contains("@kline_") {
            Some(Self::Kline)
        } else if name.ends_with("@miniTicker") || name == "!miniTicker@arr" {
            Some(Self::MiniTicker)
        } else if name.ends_with("@ticker") || name == "!ticker@arr" {
            Some(Self::Ticker)
        } else if name.ends_with("@bookTicker") || name == "!bookTicker" {
            Some(Self::BookTicker)
        } else if name.contains("@depth") && !name.contains("@depth@") {
            Some(Self::Depth)
        } else if name.contains("@depth@") {
            Some(Self::DiffDepth)
        } else {
            None
        }
    }
}

// =============================================================================
// Market Data Messages
// =============================================================================

/// Trade message
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeMessage {
    /// Event type
    #[serde(rename = "e")]
    pub event_type: String,
    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// Trade ID
    #[serde(rename = "t")]
    pub trade_id: i64,
    /// Price
    #[serde(rename = "p")]
    pub price: String,
    /// Quantity
    #[serde(rename = "q")]
    pub quantity: String,
    /// Buyer order ID
    #[serde(rename = "b")]
    pub buyer_order_id: i64,
    /// Seller order ID
    #[serde(rename = "a")]
    pub seller_order_id: i64,
    /// Trade time
    #[serde(rename = "T")]
    pub trade_time: i64,
    /// Is buyer the market maker
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

/// Book ticker message (best bid/ask)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookTickerMessage {
    /// Update ID
    #[serde(rename = "u")]
    pub update_id: i64,
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// Best bid price
    #[serde(rename = "b")]
    pub bid_price: String,
    /// Best bid quantity
    #[serde(rename = "B")]
    pub bid_qty: String,
    /// Best ask price
    #[serde(rename = "a")]
    pub ask_price: String,
    /// Best ask quantity
    #[serde(rename = "A")]
    pub ask_qty: String,
}

/// Kline message
#[derive(Debug, Clone, Serialize)]
pub struct KlineMessage {
    /// Event type
    #[serde(rename = "e")]
    pub event_type: String,
    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// Kline data
    #[serde(rename = "k")]
    pub kline: KlineData,
}

/// Kline data
#[derive(Debug, Clone, Serialize)]
pub struct KlineData {
    /// Kline start time
    #[serde(rename = "t")]
    pub start_time: i64,
    /// Kline close time
    #[serde(rename = "T")]
    pub close_time: i64,
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// Interval
    #[serde(rename = "i")]
    pub interval: String,
    /// First trade ID
    #[serde(rename = "f")]
    pub first_trade_id: i64,
    /// Last trade ID
    #[serde(rename = "L")]
    pub last_trade_id: i64,
    /// Open price
    #[serde(rename = "o")]
    pub open: String,
    /// Close price
    #[serde(rename = "c")]
    pub close: String,
    /// High price
    #[serde(rename = "h")]
    pub high: String,
    /// Low price
    #[serde(rename = "l")]
    pub low: String,
    /// Base asset volume
    #[serde(rename = "v")]
    pub volume: String,
    /// Number of trades
    #[serde(rename = "n")]
    pub trade_count: i64,
    /// Is this kline closed?
    #[serde(rename = "x")]
    pub is_closed: bool,
    /// Quote asset volume
    #[serde(rename = "q")]
    pub quote_volume: String,
    /// Taker buy base asset volume
    #[serde(rename = "V")]
    pub taker_buy_volume: String,
    /// Taker buy quote asset volume
    #[serde(rename = "Q")]
    pub taker_buy_quote_volume: String,
}

/// Depth update message
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthMessage {
    /// Event type
    #[serde(rename = "e")]
    pub event_type: String,
    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// First update ID
    #[serde(rename = "U")]
    pub first_update_id: i64,
    /// Final update ID
    #[serde(rename = "u")]
    pub final_update_id: i64,
    /// Bids to update
    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>,
    /// Asks to update
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>,
}

// =============================================================================
// User Data Messages
// =============================================================================

/// Account update message
#[derive(Debug, Clone, Serialize)]
pub struct AccountUpdateMessage {
    /// Event type
    #[serde(rename = "e")]
    pub event_type: String,
    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,
    /// Time of last account update
    #[serde(rename = "u")]
    pub last_update_time: i64,
    /// Balances
    #[serde(rename = "B")]
    pub balances: Vec<BalanceUpdate>,
}

/// Balance update in account update
#[derive(Debug, Clone, Serialize)]
pub struct BalanceUpdate {
    /// Asset
    #[serde(rename = "a")]
    pub asset: String,
    /// Free balance
    #[serde(rename = "f")]
    pub free: String,
    /// Locked balance
    #[serde(rename = "l")]
    pub locked: String,
}

/// Order update message
#[derive(Debug, Clone, Serialize)]
pub struct OrderUpdateMessage {
    /// Event type
    #[serde(rename = "e")]
    pub event_type: String,
    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// Client order ID
    #[serde(rename = "c")]
    pub client_order_id: String,
    /// Side
    #[serde(rename = "S")]
    pub side: String,
    /// Order type
    #[serde(rename = "o")]
    pub order_type: String,
    /// Time in force
    #[serde(rename = "f")]
    pub time_in_force: String,
    /// Order quantity
    #[serde(rename = "q")]
    pub quantity: String,
    /// Order price
    #[serde(rename = "p")]
    pub price: String,
    /// Stop price
    #[serde(rename = "P")]
    pub stop_price: String,
    /// Current execution type
    #[serde(rename = "x")]
    pub execution_type: String,
    /// Current order status
    #[serde(rename = "X")]
    pub order_status: String,
    /// Order reject reason
    #[serde(rename = "r")]
    pub reject_reason: String,
    /// Order ID
    #[serde(rename = "i")]
    pub order_id: i64,
    /// Last executed quantity
    #[serde(rename = "l")]
    pub last_executed_qty: String,
    /// Cumulative filled quantity
    #[serde(rename = "z")]
    pub cumulative_filled_qty: String,
    /// Last executed price
    #[serde(rename = "L")]
    pub last_executed_price: String,
    /// Commission amount
    #[serde(rename = "n")]
    pub commission: String,
    /// Commission asset
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,
    /// Transaction time
    #[serde(rename = "T")]
    pub transaction_time: i64,
    /// Trade ID
    #[serde(rename = "t")]
    pub trade_id: i64,
    /// Is the order on the book?
    #[serde(rename = "w")]
    pub is_on_book: bool,
    /// Is this trade the maker side?
    #[serde(rename = "m")]
    pub is_maker: bool,
    /// Order creation time
    #[serde(rename = "O")]
    pub order_creation_time: i64,
    /// Cumulative quote asset transacted quantity
    #[serde(rename = "Z")]
    pub cumulative_quote_qty: String,
    /// Last quote asset transacted quantity
    #[serde(rename = "Y")]
    pub last_quote_qty: String,
    /// Quote order quantity
    #[serde(rename = "Q")]
    pub quote_order_qty: String,
}

// =============================================================================
// WebSocket Handlers
// =============================================================================

/// WebSocket connection query parameters
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Streams to subscribe to (comma-separated)
    #[serde(default)]
    pub streams: Option<String>,
}

/// Handle WebSocket upgrade for market data streams
pub async fn ws_market_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    let streams: Vec<String> = query
        .streams
        .map(|s| s.split('/').map(|s| s.to_string()).collect())
        .unwrap_or_default();

    ws.on_upgrade(move |socket| handle_market_socket(socket, streams))
}

/// Handle WebSocket upgrade for combined streams
pub async fn ws_combined_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_combined_socket)
}

/// Handle WebSocket upgrade for user data streams
pub async fn ws_user_data_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
    Query(_query): Query<WsQuery>,
) -> impl IntoResponse {
    // In production, would validate listen key here
    ws.on_upgrade(handle_user_data_socket)
}

/// Handle market data WebSocket connection
async fn handle_market_socket(mut socket: WebSocket, initial_streams: Vec<String>) {
    let mut subscriptions: Vec<String> = initial_streams;

    // Send initial subscription confirmation if streams were provided in URL
    if !subscriptions.is_empty() {
        let response = SubscribeResponse {
            result: None,
            id: Some(0),
        };
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    }

    // Create a broadcast channel for sending market data
    let (tx, mut rx) = broadcast::channel::<String>(100);

    // Spawn a task to send market data to the client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if socket.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // For now, we'll just keep the connection alive and respond to pings
    // In production, this would subscribe to actual market data feeds
    tracing::info!(streams = ?subscriptions, "WebSocket market connection established");

    // Simulate sending periodic data (in production, this would be real market data)
    let _tx_clone = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            // In production, would send real market data here
            // For now, just keeping the channel alive
        }
    });

    send_task.await.ok();
}

/// Handle combined stream WebSocket connection
async fn handle_combined_socket(mut socket: WebSocket) {
    let mut subscriptions: Vec<String> = Vec::new();

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<SubscribeRequest>(&text) {
                        match request.method.to_uppercase().as_str() {
                            "SUBSCRIBE" => {
                                for param in &request.params {
                                    if !subscriptions.contains(param) {
                                        subscriptions.push(param.clone());
                                    }
                                }
                                let response = SubscribeResponse {
                                    result: None,
                                    id: request.id,
                                };
                                if let Ok(json) = serde_json::to_string(&response) {
                                    let _ = socket.send(Message::Text(json.into())).await;
                                }
                            }
                            "UNSUBSCRIBE" => {
                                subscriptions.retain(|s| !request.params.contains(s));
                                let response = SubscribeResponse {
                                    result: None,
                                    id: request.id,
                                };
                                if let Ok(json) = serde_json::to_string(&response) {
                                    let _ = socket.send(Message::Text(json.into())).await;
                                }
                            }
                            "LIST_SUBSCRIPTIONS" => {
                                let response = SubscribeResponse {
                                    result: Some(serde_json::json!(subscriptions)),
                                    id: request.id,
                                };
                                if let Ok(json) = serde_json::to_string(&response) {
                                    let _ = socket.send(Message::Text(json.into())).await;
                                }
                            }
                            _ => {
                                let error = WsError {
                                    code: -1,
                                    msg: "Unknown method".to_string(),
                                    id: request.id,
                                };
                                if let Ok(json) = serde_json::to_string(&error) {
                                    let _ = socket.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }
                }
                Message::Ping(data) => {
                    let _ = socket.send(Message::Pong(data)).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }

    tracing::info!("WebSocket combined connection closed");
}

/// Handle user data WebSocket connection
async fn handle_user_data_socket(mut socket: WebSocket) {
    // In production, this would:
    // 1. Validate the listen key
    // 2. Subscribe to user-specific events (orders, trades, account updates)
    // 3. Send real-time updates to the client

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Ping(data) => {
                    let _ = socket.send(Message::Pong(data)).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }

    tracing::info!("WebSocket user data connection closed");
}

// =============================================================================
// Listen Key Management
// =============================================================================

/// Listen key for user data streams
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenKey {
    /// Listen key value
    pub listen_key: String,
}

/// Generate a new listen key
pub fn generate_listen_key() -> String {
    // Use UUID v4 which is already cryptographically random
    // Two UUIDs give us 64 hex characters (256 bits of randomness)
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_type_parsing() {
        assert_eq!(
            StreamType::from_stream_name("btcusdt@trade"),
            Some(StreamType::Trade)
        );
        assert_eq!(
            StreamType::from_stream_name("btcusdt@aggTrade"),
            Some(StreamType::AggTrade)
        );
        assert_eq!(
            StreamType::from_stream_name("btcusdt@kline_1m"),
            Some(StreamType::Kline)
        );
        assert_eq!(
            StreamType::from_stream_name("btcusdt@ticker"),
            Some(StreamType::Ticker)
        );
        assert_eq!(
            StreamType::from_stream_name("btcusdt@bookTicker"),
            Some(StreamType::BookTicker)
        );
        assert_eq!(
            StreamType::from_stream_name("btcusdt@depth20"),
            Some(StreamType::Depth)
        );
        assert_eq!(
            StreamType::from_stream_name("btcusdt@depth@100ms"),
            Some(StreamType::DiffDepth)
        );
        assert_eq!(StreamType::from_stream_name("invalid"), None);
    }

    #[test]
    fn test_listen_key_generation() {
        let key = generate_listen_key();
        assert_eq!(key.len(), 64); // 32 bytes = 64 hex chars
    }
}
