//! ResonanceX Server - AI-Native Trading Exchange
//!
//! The world's first exchange built for AI agents.
//! "Where AI Agents Trade at the Speed of Thought"
//!
//! # Quick Start
//!
//! ```bash
//! # Start the exchange server
//! cargo run -p resonancex-server
//!
//! # Start with demo mode (simulated trading)
//! cargo run -p resonancex-server -- --demo
//!
//! # View the dashboard
//! open http://localhost:8888
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    extract::ws::{WebSocket, Message},
    http::{StatusCode, header, Method},
    response::{Html, IntoResponse, Json},
    routing::{get, post, delete},
    Router,
};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use resonancex_types::{
    Order, OrderId, Trade, TradeId, Side, OrderType, OrderStatus, TimeInForce,
    MarketConfig, MarketId, MarketStatus, Candle, CandleInterval, DepthSnapshot,
    Ticker, DepthLevel,
};
use resonancex_engine::{MatchingEngine, EngineConfig, SubmitResult};
use openibank_types::{AgentId, WalletId, PermitId, Currency};

// ============================================================================
// CLI
// ============================================================================

#[derive(Parser)]
#[command(name = "resonancex")]
#[command(about = "ResonanceX Exchange - AI-Native Trading Platform")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "8888")]
    port: u16,

    /// Enable demo mode with simulated trading
    #[arg(long)]
    demo: bool,

    /// Number of demo agents
    #[arg(long, default_value = "10")]
    demo_agents: usize,
}

// ============================================================================
// Application State
// ============================================================================

struct AppState {
    /// The matching engine
    engine: MatchingEngine,
    /// Market data (candles, tickers)
    market_data: RwLock<MarketData>,
    /// Recent trades
    trades: RwLock<Vec<Trade>>,
    /// Agent balances (demo mode)
    balances: RwLock<HashMap<AgentId, HashMap<String, Decimal>>>,
    /// WebSocket broadcast channel
    ws_tx: broadcast::Sender<WsMessage>,
    /// Demo mode flag
    demo_mode: bool,
}

struct MarketData {
    /// Tickers by market
    tickers: HashMap<MarketId, Ticker>,
    /// Candles by market and interval
    candles: HashMap<(MarketId, CandleInterval), Vec<Candle>>,
    /// Current building candles
    current_candles: HashMap<(MarketId, CandleInterval), Candle>,
}

impl MarketData {
    fn new() -> Self {
        Self {
            tickers: HashMap::new(),
            candles: HashMap::new(),
            current_candles: HashMap::new(),
        }
    }
}

// ============================================================================
// WebSocket Messages
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "trade")]
    Trade { market: String, trade: TradeInfo },
    #[serde(rename = "depth")]
    Depth { market: String, depth: DepthSnapshot },
    #[serde(rename = "ticker")]
    Ticker { market: String, ticker: TickerInfo },
    #[serde(rename = "candle")]
    Candle { market: String, interval: String, candle: CandleInfo },
}

#[derive(Debug, Clone, Serialize)]
struct TradeInfo {
    id: String,
    price: String,
    amount: String,
    side: String,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
struct TickerInfo {
    last: String,
    bid: String,
    ask: String,
    high_24h: String,
    low_24h: String,
    volume_24h: String,
    change_24h: String,
}

#[derive(Debug, Clone, Serialize)]
struct CandleInfo {
    timestamp: i64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ApiResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ApiResponse {
    fn ok<T: Serialize>(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data: serde_json::to_value(data).ok(),
            error: None,
        })
    }

    fn err(msg: impl Into<String>) -> Json<Self> {
        Json(Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        })
    }
}

#[derive(Debug, Serialize)]
struct MarketInfo {
    id: String,
    base: String,
    quote: String,
    status: String,
    price_precision: u8,
    amount_precision: u8,
    min_amount: String,
    maker_fee: String,
    taker_fee: String,
}

#[derive(Debug, Deserialize)]
struct PlaceOrderRequest {
    market: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    price: Option<String>,
    amount: String,
    agent_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct OrderInfo {
    id: String,
    market: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    price: Option<String>,
    amount: String,
    filled: String,
    remaining: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CandleQuery {
    interval: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DepthQuery {
    limit: Option<usize>,
}

// ============================================================================
// Routes
// ============================================================================

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "resonancex",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn get_markets(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let markets: Vec<MarketInfo> = state
        .engine
        .list_markets()
        .into_iter()
        .filter_map(|id| {
            state.engine.get_market(&id).map(|config| MarketInfo {
                id: config.id.0.clone(),
                base: config.base.to_string(),
                quote: config.quote.to_string(),
                status: format!("{:?}", config.status),
                price_precision: config.price_precision,
                amount_precision: config.amount_precision,
                min_amount: config.min_amount.to_string(),
                maker_fee: config.maker_fee.to_string(),
                taker_fee: config.taker_fee.to_string(),
            })
        })
        .collect();

    ApiResponse::ok(markets)
}

async fn get_ticker(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
) -> impl IntoResponse {
    let market_id = MarketId::new(&market);
    let data = state.market_data.read();

    if let Some(ticker) = data.tickers.get(&market_id) {
        let info = TickerInfo {
            last: ticker.last_price.to_string(),
            bid: ticker.bid.to_string(),
            ask: ticker.ask.to_string(),
            high_24h: ticker.high_24h.to_string(),
            low_24h: ticker.low_24h.to_string(),
            volume_24h: ticker.volume_24h.to_string(),
            change_24h: format!("{:.2}", ticker.change_24h),
        };
        ApiResponse::ok(info)
    } else {
        // Return default ticker if market exists
        if state.engine.get_market(&market_id).is_some() {
            let (bid, ask) = state.engine.get_bbo(&market_id).unwrap_or((None, None));
            let info = TickerInfo {
                last: "0".to_string(),
                bid: bid.map(|d| d.to_string()).unwrap_or_else(|| "0".to_string()),
                ask: ask.map(|d| d.to_string()).unwrap_or_else(|| "0".to_string()),
                high_24h: "0".to_string(),
                low_24h: "0".to_string(),
                volume_24h: "0".to_string(),
                change_24h: "0.00".to_string(),
            };
            ApiResponse::ok(info)
        } else {
            ApiResponse::err("Market not found")
        }
    }
}

async fn get_depth(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(params): Query<DepthQuery>,
) -> impl IntoResponse {
    let market_id = MarketId::new(&market);
    let levels = params.limit.unwrap_or(20);

    match state.engine.get_depth(&market_id, levels) {
        Ok(depth) => {
            let response = serde_json::json!({
                "bids": depth.bids.iter().map(|l| [l.price.to_string(), l.amount.to_string()]).collect::<Vec<_>>(),
                "asks": depth.asks.iter().map(|l| [l.price.to_string(), l.amount.to_string()]).collect::<Vec<_>>(),
                "timestamp": depth.timestamp.timestamp_millis(),
            });
            ApiResponse::ok(response)
        }
        Err(e) => ApiResponse::err(format!("{}", e)),
    }
}

async fn get_trades(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
) -> impl IntoResponse {
    let market_id = MarketId::new(&market);
    let trades = state.trades.read();

    let market_trades: Vec<TradeInfo> = trades
        .iter()
        .filter(|t| t.market == market_id)
        .take(100)
        .map(|t| TradeInfo {
            id: t.id.0.to_string(),
            price: t.price.to_string(),
            amount: t.amount.to_string(),
            side: format!("{}", t.taker_side()),
            timestamp: t.timestamp.timestamp_millis(),
        })
        .collect();

    ApiResponse::ok(market_trades)
}

async fn get_candles(
    State(state): State<Arc<AppState>>,
    Path(market): Path<String>,
    Query(params): Query<CandleQuery>,
) -> impl IntoResponse {
    let market_id = MarketId::new(&market);
    let interval = params
        .interval
        .as_deref()
        .and_then(CandleInterval::from_str)
        .unwrap_or(CandleInterval::M1);
    let limit = params.limit.unwrap_or(100);

    let data = state.market_data.read();
    let key = (market_id, interval);

    let candles: Vec<CandleInfo> = data
        .candles
        .get(&key)
        .map(|c| {
            c.iter()
                .rev()
                .take(limit)
                .rev()
                .map(|c| CandleInfo {
                    timestamp: c.timestamp,
                    open: c.open.to_string(),
                    high: c.high.to_string(),
                    low: c.low.to_string(),
                    close: c.close.to_string(),
                    volume: c.volume.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    ApiResponse::ok(candles)
}

async fn place_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PlaceOrderRequest>,
) -> impl IntoResponse {
    // Parse request
    let market_id = MarketId::new(&req.market);
    let side = match req.side.to_lowercase().as_str() {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        _ => return ApiResponse::err("Invalid side"),
    };

    let amount: Decimal = match req.amount.parse() {
        Ok(a) => a,
        Err(_) => return ApiResponse::err("Invalid amount"),
    };

    let order_type = match req.order_type.to_lowercase().as_str() {
        "market" => OrderType::Market,
        "limit" => {
            let price: Decimal = match req.price.as_ref().and_then(|p| p.parse().ok()) {
                Some(p) => p,
                None => return ApiResponse::err("Price required for limit order"),
            };
            OrderType::limit(price)
        }
        _ => return ApiResponse::err("Invalid order type"),
    };

    // Create agent ID (use provided or generate new)
    let agent_id = req
        .agent_id
        .map(|id| AgentId(uuid::Uuid::parse_str(&id).unwrap_or_else(|_| uuid::Uuid::new_v4())))
        .unwrap_or_else(AgentId::new);

    // Build order
    let order = match Order::builder()
        .agent(agent_id)
        .wallet(WalletId::new())
        .market(market_id.clone())
        .side(side)
        .order_type(order_type.clone())
        .amount(amount)
        .permit(PermitId::new())
        .build()
    {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(format!("{}", e)),
    };

    // Submit to engine
    match state.engine.submit_order(order) {
        Ok(result) => {
            // Process trades
            for trade in &result.trades {
                // Add to recent trades
                state.trades.write().insert(0, trade.clone());

                // Broadcast trade
                let _ = state.ws_tx.send(WsMessage::Trade {
                    market: market_id.0.clone(),
                    trade: TradeInfo {
                        id: trade.id.0.to_string(),
                        price: trade.price.to_string(),
                        amount: trade.amount.to_string(),
                        side: format!("{}", trade.taker_side()),
                        timestamp: trade.timestamp.timestamp_millis(),
                    },
                });

                // Update market data
                update_market_data(&state, &trade);
            }

            // Broadcast depth update
            if let Ok(depth) = state.engine.get_depth(&market_id, 20) {
                let _ = state.ws_tx.send(WsMessage::Depth {
                    market: market_id.0.clone(),
                    depth,
                });
            }

            let order_info = OrderInfo {
                id: result.order.id.0.to_string(),
                market: result.order.market.0.clone(),
                side: format!("{}", result.order.side),
                order_type: match &result.order.order_type {
                    OrderType::Limit { .. } => "limit".to_string(),
                    OrderType::Market => "market".to_string(),
                    _ => "other".to_string(),
                },
                price: result.order.order_type.price().map(|p| p.to_string()),
                amount: result.order.amount.to_string(),
                filled: result.order.filled.to_string(),
                remaining: result.order.remaining.to_string(),
                status: format!("{:?}", result.order.status),
                created_at: result.order.created_at.to_rfc3339(),
            };

            ApiResponse::ok(serde_json::json!({
                "order": order_info,
                "trades": result.trades.len(),
            }))
        }
        Err(e) => ApiResponse::err(format!("{}", e)),
    }
}

fn update_market_data(state: &AppState, trade: &Trade) {
    let mut data = state.market_data.write();
    let market_id = &trade.market;

    // Update ticker
    let ticker = data.tickers.entry(market_id.clone()).or_insert_with(|| Ticker::new(market_id.clone()));
    ticker.last_price = trade.price;
    ticker.volume_24h += trade.amount;
    ticker.quote_volume_24h += trade.quote_amount;
    ticker.trade_count_24h += 1;
    if ticker.high_24h < trade.price {
        ticker.high_24h = trade.price;
    }
    if ticker.low_24h.is_zero() || ticker.low_24h > trade.price {
        ticker.low_24h = trade.price;
    }

    // Update candles (1m)
    let now = chrono::Utc::now().timestamp();
    let interval = CandleInterval::M1;
    let candle_ts = interval.floor(now);
    let key = (market_id.clone(), interval);

    // Check if we need to roll over the candle
    let needs_rollover = data.current_candles.get(&key)
        .map(|c| c.timestamp != candle_ts)
        .unwrap_or(false);

    if needs_rollover {
        // Save completed candle
        if let Some(completed) = data.current_candles.remove(&key) {
            data.candles.entry(key.clone()).or_insert_with(Vec::new).push(completed);
        }
    }

    // Get or create current candle
    let candle = data.current_candles.entry(key).or_insert_with(|| Candle::new(candle_ts, trade.price));
    candle.update(trade.price, trade.amount, trade.quote_amount);
}

async fn cancel_order(
    State(state): State<Arc<AppState>>,
    Path((market, order_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let market_id = MarketId::new(&market);
    let order_id = match uuid::Uuid::parse_str(&order_id) {
        Ok(id) => OrderId(id),
        Err(_) => return ApiResponse::err("Invalid order ID"),
    };

    match state.engine.cancel_order(&market_id, order_id) {
        Ok(result) => {
            if result.cancelled {
                // Broadcast depth update
                if let Ok(depth) = state.engine.get_depth(&market_id, 20) {
                    let _ = state.ws_tx.send(WsMessage::Depth {
                        market: market_id.0.clone(),
                        depth,
                    });
                }

                ApiResponse::ok(serde_json::json!({
                    "cancelled": true,
                    "order_id": order_id.0.to_string(),
                    "remaining": result.remaining.to_string(),
                }))
            } else {
                ApiResponse::err("Order not found")
            }
        }
        Err(e) => ApiResponse::err(format!("{}", e)),
    }
}

// ============================================================================
// WebSocket Handler
// ============================================================================

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();

    // Spawn task to forward broadcast messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages (subscriptions, etc.)
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                // Handle subscription requests
                if let Ok(req) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Log subscription request
                    info!("WS subscription: {:?}", req);
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}

// ============================================================================
// Dashboard HTML
// ============================================================================

async fn dashboard() -> impl IntoResponse {
    Html(include_str!("dashboard.html"))
}

// ============================================================================
// Demo Mode
// ============================================================================

async fn run_demo_trading(state: Arc<AppState>, num_agents: usize) {
    info!("Starting demo trading with {} agents", num_agents);

    // Create demo agents
    let agents: Vec<AgentId> = (0..num_agents).map(|_| AgentId::new()).collect();

    // Trading loop
    let mut interval = tokio::time::interval(Duration::from_millis(500));

    loop {
        interval.tick().await;

        // Pick random agent
        let agent = &agents[rand::random::<usize>() % agents.len()];

        // Pick random market
        let markets = state.engine.list_markets();
        if markets.is_empty() {
            continue;
        }
        let market = &markets[rand::random::<usize>() % markets.len()];

        // Get current BBO
        let (bid, ask) = state.engine.get_bbo(market).unwrap_or((None, None));

        let mid_price = match (bid, ask) {
            (Some(b), Some(a)) => (b + a) / dec!(2),
            (Some(b), None) => b,
            (None, Some(a)) => a,
            (None, None) => dec!(3000), // Default starting price
        };

        // Random side
        let side = if rand::random::<bool>() { Side::Buy } else { Side::Sell };

        // Random order type (70% limit, 30% market)
        let is_limit = rand::random::<f32>() < 0.7;

        let amount = dec!(0.1) + Decimal::from(rand::random::<u32>() % 10) / dec!(10);

        let order_type = if is_limit {
            // Price within 0.5% of mid
            let offset = mid_price * dec!(0.005) * Decimal::from((rand::random::<i32>() % 200) - 100) / dec!(100);
            let price = match side {
                Side::Buy => mid_price - offset.abs(),
                Side::Sell => mid_price + offset.abs(),
            };
            OrderType::limit(price.round_dp(2))
        } else {
            OrderType::Market
        };

        // Build and submit order
        if let Ok(order) = Order::builder()
            .agent(agent.clone())
            .wallet(WalletId::new())
            .market(market.clone())
            .side(side)
            .order_type(order_type)
            .amount(amount)
            .permit(PermitId::new())
            .build()
        {
            if let Ok(result) = state.engine.submit_order(order) {
                for trade in &result.trades {
                    state.trades.write().insert(0, trade.clone());

                    let _ = state.ws_tx.send(WsMessage::Trade {
                        market: market.0.clone(),
                        trade: TradeInfo {
                            id: trade.id.0.to_string(),
                            price: trade.price.to_string(),
                            amount: trade.amount.to_string(),
                            side: format!("{}", trade.taker_side()),
                            timestamp: trade.timestamp.timestamp_millis(),
                        },
                    });

                    update_market_data(&state, &trade);
                }

                // Broadcast depth
                if let Ok(depth) = state.engine.get_depth(market, 20) {
                    let _ = state.ws_tx.send(WsMessage::Depth {
                        market: market.0.clone(),
                        depth,
                    });
                }
            }
        }
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    info!("Starting ResonanceX Exchange Server");
    info!("======================================");
    info!("  Port: {}", cli.port);
    info!("  Demo Mode: {}", cli.demo);
    if cli.demo {
        info!("  Demo Agents: {}", cli.demo_agents);
    }
    info!("======================================");

    // Create broadcast channel for WebSocket
    let (ws_tx, _) = broadcast::channel(1000);

    // Create application state
    let state = Arc::new(AppState {
        engine: MatchingEngine::new(EngineConfig::default()),
        market_data: RwLock::new(MarketData::new()),
        trades: RwLock::new(Vec::new()),
        balances: RwLock::new(HashMap::new()),
        ws_tx,
        demo_mode: cli.demo,
    });

    // Add default markets
    let markets = vec![
        ("ETH_IUSD", Currency::eth(), Currency::iusd()),
        ("BTC_IUSD", Currency::btc(), Currency::iusd()),
        ("SOL_IUSD", Currency::Crypto(openibank_types::CryptoCurrency::SOL), Currency::iusd()),
    ];

    for (id, base, quote) in markets {
        let config = MarketConfig::new(MarketId::new(id), base, quote);
        state.engine.add_market(config)?;
        info!("Added market: {}", id);
    }

    // Start demo trading if enabled
    if cli.demo {
        let demo_state = state.clone();
        tokio::spawn(async move {
            run_demo_trading(demo_state, cli.demo_agents).await;
        });
    }

    // Build router
    let app = Router::new()
        // Dashboard
        .route("/", get(dashboard))
        // Health
        .route("/health", get(health))
        // REST API
        .route("/api/v1/markets", get(get_markets))
        .route("/api/v1/markets/:market/ticker", get(get_ticker))
        .route("/api/v1/markets/:market/depth", get(get_depth))
        .route("/api/v1/markets/:market/trades", get(get_trades))
        .route("/api/v1/markets/:market/candles", get(get_candles))
        .route("/api/v1/orders", post(place_order))
        .route("/api/v1/orders/:market/:order_id", delete(cancel_order))
        // WebSocket
        .route("/ws", get(ws_handler))
        // CORS
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE]))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    info!("Listening on http://{}", addr);
    info!("Dashboard: http://localhost:{}", cli.port);
    info!("WebSocket: ws://localhost:{}/ws", cli.port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
