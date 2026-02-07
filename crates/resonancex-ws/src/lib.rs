//! ResonanceX WebSocket - Real-Time Market Data Streaming
//!
//! This crate provides a WebSocket server for streaming real-time market data
//! to clients, including tickers, trades, depth updates, and candles.
//!
//! # Protocol
//!
//! The WebSocket protocol uses JSON messages for both requests and responses.
//!
//! ## Subscribe
//! ```json
//! {
//!     "type": "subscribe",
//!     "channels": ["ticker:ETH_IUSD", "trades:ETH_IUSD", "depth:ETH_IUSD@20"]
//! }
//! ```
//!
//! ## Unsubscribe
//! ```json
//! {
//!     "type": "unsubscribe",
//!     "channels": ["ticker:ETH_IUSD"]
//! }
//! ```
//!
//! ## Data Messages
//! ```json
//! {
//!     "type": "ticker",
//!     "market": "ETH_IUSD",
//!     "data": { ... }
//! }
//! ```
//!
//! # Example
//!
//! ```ignore
//! use resonancex_ws::{WebSocketServer, WsConfig};
//!
//! let config = WsConfig {
//!     bind_addr: "0.0.0.0:8081".parse().unwrap(),
//!     ..Default::default()
//! };
//!
//! let server = WebSocketServer::new(config, market_data);
//! server.run().await?;
//! ```

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use flume::{Sender, Receiver};

// Re-export core types
pub use resonancex_types::{
    MarketId, Trade, Ticker, DepthSnapshot, Candle, CandleInterval,
};
pub use resonancex_marketdata::{MarketDataUpdate, Subscription};

/// WebSocket server configuration
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Address to bind to
    pub bind_addr: SocketAddr,
    /// Maximum connections per IP
    pub max_connections_per_ip: usize,
    /// Maximum subscriptions per connection
    pub max_subscriptions: usize,
    /// Ping interval in seconds
    pub ping_interval_secs: u64,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8081".parse().unwrap(),
            max_connections_per_ip: 10,
            max_subscriptions: 50,
            ping_interval_secs: 30,
            connection_timeout_secs: 60,
        }
    }
}

/// WebSocket errors
#[derive(Debug, Error)]
pub enum WsError {
    #[error("Connection limit exceeded")]
    ConnectionLimitExceeded,

    #[error("Subscription limit exceeded")]
    SubscriptionLimitExceeded,

    #[error("Invalid channel: {0}")]
    InvalidChannel(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for WebSocket operations
pub type WsResult<T> = Result<T, WsError>;

/// Client message types
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to channels
    Subscribe { channels: Vec<String> },
    /// Unsubscribe from channels
    Unsubscribe { channels: Vec<String> },
    /// Ping message
    Ping { id: Option<u64> },
}

/// Server message types
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Subscription confirmed
    Subscribed { channels: Vec<String> },
    /// Unsubscription confirmed
    Unsubscribed { channels: Vec<String> },
    /// Pong response
    Pong { id: Option<u64> },
    /// Ticker update
    Ticker { market: String, data: TickerData },
    /// Trade update
    Trade { market: String, data: TradeData },
    /// Depth update
    Depth { market: String, data: DepthData },
    /// Candle update
    Candle { market: String, interval: String, data: CandleData },
    /// Error message
    Error { code: i32, message: String },
}

/// Ticker data for WebSocket
#[derive(Debug, Serialize)]
pub struct TickerData {
    pub last_price: String,
    pub bid: String,
    pub ask: String,
    pub high_24h: String,
    pub low_24h: String,
    pub volume_24h: String,
    pub change_24h: String,
    pub timestamp: i64,
}

impl From<Ticker> for TickerData {
    fn from(t: Ticker) -> Self {
        Self {
            last_price: t.last_price.to_string(),
            bid: t.bid.to_string(),
            ask: t.ask.to_string(),
            high_24h: t.high_24h.to_string(),
            low_24h: t.low_24h.to_string(),
            volume_24h: t.volume_24h.to_string(),
            change_24h: t.change_24h.to_string(),
            timestamp: t.timestamp.timestamp_millis(),
        }
    }
}

/// Trade data for WebSocket
#[derive(Debug, Serialize)]
pub struct TradeData {
    pub id: String,
    pub price: String,
    pub amount: String,
    pub side: String,
    pub timestamp: i64,
}

impl From<Trade> for TradeData {
    fn from(t: Trade) -> Self {
        Self {
            id: t.id.to_string(),
            price: t.price.to_string(),
            amount: t.amount.to_string(),
            side: t.maker_side.opposite().to_string(), // Taker side
            timestamp: t.timestamp.timestamp_millis(),
        }
    }
}

/// Depth data for WebSocket
#[derive(Debug, Serialize)]
pub struct DepthData {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
    pub timestamp: i64,
}

impl From<DepthSnapshot> for DepthData {
    fn from(d: DepthSnapshot) -> Self {
        Self {
            bids: d.bids.iter().map(|l| [l.price.to_string(), l.amount.to_string()]).collect(),
            asks: d.asks.iter().map(|l| [l.price.to_string(), l.amount.to_string()]).collect(),
            timestamp: d.timestamp.timestamp_millis(),
        }
    }
}

/// Candle data for WebSocket
#[derive(Debug, Serialize)]
pub struct CandleData {
    pub timestamp: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

impl From<Candle> for CandleData {
    fn from(c: Candle) -> Self {
        Self {
            timestamp: c.timestamp,
            open: c.open.to_string(),
            high: c.high.to_string(),
            low: c.low.to_string(),
            close: c.close.to_string(),
            volume: c.volume.to_string(),
        }
    }
}

/// Channel specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Ticker channel: ticker:MARKET
    Ticker(MarketId),
    /// Trades channel: trades:MARKET
    Trades(MarketId),
    /// Depth channel: depth:MARKET@LEVELS
    Depth(MarketId, usize),
    /// Candle channel: candles:MARKET@INTERVAL
    Candles(MarketId, CandleInterval),
}

impl Channel {
    /// Parse a channel string
    pub fn parse(s: &str) -> WsResult<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(WsError::InvalidChannel(s.to_string()));
        }

        let channel_type = parts[0];
        let params: Vec<&str> = parts[1].split('@').collect();
        let market = MarketId::new(params[0]);

        match channel_type {
            "ticker" => Ok(Channel::Ticker(market)),
            "trades" => Ok(Channel::Trades(market)),
            "depth" => {
                let levels = params.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(20);
                Ok(Channel::Depth(market, levels))
            }
            "candles" => {
                let interval = params.get(1)
                    .and_then(|s| CandleInterval::from_str(s))
                    .unwrap_or(CandleInterval::M1);
                Ok(Channel::Candles(market, interval))
            }
            _ => Err(WsError::InvalidChannel(s.to_string())),
        }
    }

    /// Convert to channel string
    pub fn to_string(&self) -> String {
        match self {
            Channel::Ticker(m) => format!("ticker:{}", m),
            Channel::Trades(m) => format!("trades:{}", m),
            Channel::Depth(m, levels) => format!("depth:{}@{}", m, levels),
            Channel::Candles(m, interval) => format!("candles:{}@{}", m, interval),
        }
    }
}

/// Connection state
pub struct ConnectionState {
    /// Connection ID
    pub id: u64,
    /// Subscribed channels
    pub channels: HashSet<Channel>,
    /// Client IP address
    pub ip: SocketAddr,
    /// Connected at timestamp
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

impl ConnectionState {
    /// Create new connection state
    pub fn new(id: u64, ip: SocketAddr) -> Self {
        Self {
            id,
            channels: HashSet::new(),
            ip,
            connected_at: chrono::Utc::now(),
        }
    }

    /// Subscribe to channels
    pub fn subscribe(&mut self, channels: Vec<Channel>, max: usize) -> WsResult<Vec<Channel>> {
        let mut subscribed = Vec::new();
        for channel in channels {
            if self.channels.len() >= max {
                return Err(WsError::SubscriptionLimitExceeded);
            }
            if self.channels.insert(channel.clone()) {
                subscribed.push(channel);
            }
        }
        Ok(subscribed)
    }

    /// Unsubscribe from channels
    pub fn unsubscribe(&mut self, channels: Vec<Channel>) -> Vec<Channel> {
        let mut unsubscribed = Vec::new();
        for channel in channels {
            if self.channels.remove(&channel) {
                unsubscribed.push(channel);
            }
        }
        unsubscribed
    }

    /// Check if subscribed to a channel
    pub fn is_subscribed(&self, channel: &Channel) -> bool {
        self.channels.contains(channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.bind_addr.port(), 8081);
        assert_eq!(config.max_subscriptions, 50);
    }

    #[test]
    fn test_channel_parse_ticker() {
        let channel = Channel::parse("ticker:ETH_IUSD").unwrap();
        assert!(matches!(channel, Channel::Ticker(m) if m.0 == "ETH_IUSD"));
    }

    #[test]
    fn test_channel_parse_depth() {
        let channel = Channel::parse("depth:ETH_IUSD@50").unwrap();
        assert!(matches!(channel, Channel::Depth(m, 50) if m.0 == "ETH_IUSD"));
    }

    #[test]
    fn test_channel_parse_candles() {
        let channel = Channel::parse("candles:ETH_IUSD@1h").unwrap();
        assert!(matches!(channel, Channel::Candles(m, CandleInterval::H1) if m.0 == "ETH_IUSD"));
    }

    #[test]
    fn test_channel_to_string() {
        let channel = Channel::Ticker(MarketId::new("ETH_IUSD"));
        assert_eq!(channel.to_string(), "ticker:ETH_IUSD");
    }

    #[test]
    fn test_connection_state() {
        let mut state = ConnectionState::new(1, "127.0.0.1:12345".parse().unwrap());

        let channels = vec![
            Channel::Ticker(MarketId::new("ETH_IUSD")),
            Channel::Trades(MarketId::new("ETH_IUSD")),
        ];

        let subscribed = state.subscribe(channels.clone(), 50).unwrap();
        assert_eq!(subscribed.len(), 2);
        assert!(state.is_subscribed(&Channel::Ticker(MarketId::new("ETH_IUSD"))));

        let unsubscribed = state.unsubscribe(vec![Channel::Ticker(MarketId::new("ETH_IUSD"))]);
        assert_eq!(unsubscribed.len(), 1);
        assert!(!state.is_subscribed(&Channel::Ticker(MarketId::new("ETH_IUSD"))));
    }

    #[test]
    fn test_client_message_deserialize() {
        let json = r#"{"type": "subscribe", "channels": ["ticker:ETH_IUSD"]}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Subscribe { channels } if channels.len() == 1));
    }

    #[test]
    fn test_server_message_serialize() {
        let msg = ServerMessage::Subscribed {
            channels: vec!["ticker:ETH_IUSD".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribed"));
        assert!(json.contains("ticker:ETH_IUSD"));
    }
}
