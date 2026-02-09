# Tutorial 6: Trading on ResonanceX Exchange

> **Duration**: 45 minutes
> **Level**: Intermediate
> **Prerequisites**: Tutorials 1-5, Basic trading knowledge

---

## Overview

In this tutorial, you'll learn how to:
- Connect to the ResonanceX trading exchange
- Place and manage orders
- Stream real-time market data via WebSocket
- Implement a simple trading strategy
- Handle order lifecycle events

---

## Understanding ResonanceX

ResonanceX is OpeniBank's high-performance trading exchange designed for AI agents:

```
┌─────────────────────────────────────────────────────────────────┐
│                     ResonanceX Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│   │  Order Book  │    │   Matching   │    │  Market Data │      │
│   │   (BTreeMap) │───▶│    Engine    │───▶│   Publisher  │      │
│   └──────────────┘    └──────────────┘    └──────────────┘      │
│                              │                    │             │
│                              ▼                    ▼             │
│                       ┌──────────────┐    ┌──────────────┐      │
│                       │   Trades     │    │  WebSocket   │      │
│                       │   Ledger     │    │   Streams    │      │
│                       └──────────────┘    └──────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step 1: Connect to the Exchange

### Start the ResonanceX Server

```bash
# Start the trading exchange
cargo run -p resonancex-server

# Or with demo mode (simulated market data)
cargo run -p resonancex-server -- --demo
```

### REST API Connection

```rust
use openibank_sdk::{Client, TradingClient};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create trading client
    let client = TradingClient::new("http://localhost:8888")
        .with_api_key("your-api-key")
        .with_secret("your-secret");

    // Get exchange info
    let info = client.get_exchange_info().await?;
    println!("Available symbols: {:?}", info.symbols);

    Ok(())
}
```

### WebSocket Connection

```rust
use tokio_tungstenite::connect_async;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;

async fn connect_websocket() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:8888/ws";
    let (mut ws, _) = connect_async(url).await?;

    // Subscribe to market data
    let subscribe = json!({
        "method": "SUBSCRIBE",
        "params": ["btcusdt@trade", "btcusdt@depth@100ms"],
        "id": 1
    });

    ws.send(subscribe.to_string().into()).await?;

    // Process incoming messages
    while let Some(msg) = ws.next().await {
        match msg {
            Ok(message) => {
                let data: serde_json::Value = serde_json::from_str(&message.to_string())?;
                handle_market_data(data);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}

fn handle_market_data(data: serde_json::Value) {
    if let Some(stream) = data.get("stream").and_then(|s| s.as_str()) {
        match stream {
            s if s.ends_with("@trade") => {
                println!("Trade: {:?}", data["data"]);
            }
            s if s.contains("@depth") => {
                println!("Depth update: {:?}", data["data"]);
            }
            _ => {}
        }
    }
}
```

---

## Step 2: Market Data Analysis

### Fetch Order Book

```rust
use openibank_sdk::types::{OrderBook, Depth};

async fn analyze_order_book(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    // Get order book with 20 levels
    let depth = client.get_depth("BTCUSDT", Some(20)).await?;

    // Calculate spread
    let best_bid = depth.bids.first().map(|b| b[0].parse::<Decimal>().unwrap());
    let best_ask = depth.asks.first().map(|a| a[0].parse::<Decimal>().unwrap());

    if let (Some(bid), Some(ask)) = (best_bid, best_ask) {
        let spread = ask - bid;
        let spread_pct = spread / bid * dec!(100);
        println!("Spread: {} ({:.4}%)", spread, spread_pct);
    }

    // Calculate bid/ask imbalance
    let total_bids: Decimal = depth.bids.iter()
        .take(10)
        .map(|b| b[1].parse::<Decimal>().unwrap())
        .sum();

    let total_asks: Decimal = depth.asks.iter()
        .take(10)
        .map(|a| a[1].parse::<Decimal>().unwrap())
        .sum();

    let imbalance = (total_bids - total_asks) / (total_bids + total_asks);
    println!("Order book imbalance: {:.4}", imbalance);

    Ok(())
}
```

### Fetch Historical Data

```rust
async fn get_historical_data(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    // Get 1-hour candles for the last 24 hours
    let klines = client.get_klines(
        "BTCUSDT",
        "1h",
        None,  // start_time
        None,  // end_time
        Some(24),  // limit
    ).await?;

    for kline in klines {
        println!(
            "Time: {}, Open: {}, High: {}, Low: {}, Close: {}, Volume: {}",
            kline.0,  // open_time
            kline.1,  // open
            kline.2,  // high
            kline.3,  // low
            kline.4,  // close
            kline.5,  // volume
        );
    }

    // Calculate simple moving average
    let closes: Vec<Decimal> = klines.iter()
        .map(|k| k.4.parse::<Decimal>().unwrap())
        .collect();

    let sma_10: Decimal = closes.iter().rev().take(10).sum::<Decimal>() / dec!(10);
    println!("SMA(10): {}", sma_10);

    Ok(())
}
```

---

## Step 3: Place Orders

### Limit Order

```rust
use openibank_sdk::types::{OrderSide, OrderType, TimeInForce};

async fn place_limit_order(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    let order = client.place_order(
        "BTCUSDT",
        OrderSide::Buy,
        OrderType::Limit,
        Some(TimeInForce::GTC),  // Good Till Cancelled
        Some(dec!(0.001)),       // quantity
        None,                     // quote_order_qty
        Some(dec!(50000.00)),    // price
        None,                     // stop_price
        None,                     // new_client_order_id
    ).await?;

    println!("Order placed: {:?}", order);
    println!("Order ID: {}", order.order_id);
    println!("Status: {:?}", order.status);

    Ok(())
}
```

### Market Order

```rust
async fn place_market_order(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    // Market order by quantity
    let order = client.place_order(
        "BTCUSDT",
        OrderSide::Buy,
        OrderType::Market,
        None,                    // time_in_force (not needed for market)
        Some(dec!(0.001)),       // quantity
        None,                    // quote_order_qty
        None,                    // price (not needed for market)
        None,                    // stop_price
        None,                    // new_client_order_id
    ).await?;

    println!("Market order executed at: {}", order.price);

    // Market order by quote amount (spend $100)
    let order = client.place_order(
        "BTCUSDT",
        OrderSide::Buy,
        OrderType::Market,
        None,
        None,                    // quantity
        Some(dec!(100.00)),      // spend $100 worth
        None,
        None,
        None,
    ).await?;

    println!("Bought {} BTC for $100", order.executed_qty);

    Ok(())
}
```

### Stop-Limit Order

```rust
async fn place_stop_limit(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    // Stop loss order
    let order = client.place_order(
        "BTCUSDT",
        OrderSide::Sell,
        OrderType::StopLossLimit,
        Some(TimeInForce::GTC),
        Some(dec!(0.001)),       // quantity
        None,
        Some(dec!(48000.00)),    // limit price
        Some(dec!(48500.00)),    // stop (trigger) price
        None,
    ).await?;

    println!("Stop-loss order: triggers at $48,500, sells at $48,000");

    Ok(())
}
```

---

## Step 4: Manage Orders

### Query Order Status

```rust
async fn check_order_status(client: &TradingClient, order_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let order = client.get_order("BTCUSDT", Some(order_id), None).await?;

    println!("Order {} status:", order.order_id);
    println!("  Status: {:?}", order.status);
    println!("  Filled: {} / {}", order.executed_qty, order.orig_qty);
    println!("  Average price: {}", order.price);

    Ok(())
}
```

### Cancel Order

```rust
async fn cancel_order(client: &TradingClient, order_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let result = client.cancel_order("BTCUSDT", Some(order_id), None).await?;

    println!("Cancelled order: {}", result.order_id);
    println!("Remaining quantity: {}", result.orig_qty);

    Ok(())
}

async fn cancel_all_orders(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    let results = client.cancel_all_orders("BTCUSDT").await?;

    println!("Cancelled {} orders", results.len());

    Ok(())
}
```

### List Open Orders

```rust
async fn list_open_orders(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    // All open orders for a symbol
    let orders = client.get_open_orders(Some("BTCUSDT")).await?;

    println!("Open orders:");
    for order in orders {
        println!(
            "  {} {} {} @ {} (filled: {})",
            order.order_id,
            order.side,
            order.orig_qty,
            order.price,
            order.executed_qty
        );
    }

    Ok(())
}
```

---

## Step 5: Build a Trading Strategy

### Simple Moving Average Crossover

```rust
use std::collections::VecDeque;

struct MovingAverageCrossover {
    client: TradingClient,
    symbol: String,
    short_period: usize,
    long_period: usize,
    prices: VecDeque<Decimal>,
    position: Decimal,
}

impl MovingAverageCrossover {
    fn new(client: TradingClient, symbol: &str) -> Self {
        Self {
            client,
            symbol: symbol.to_string(),
            short_period: 10,
            long_period: 20,
            prices: VecDeque::with_capacity(21),
            position: dec!(0),
        }
    }

    fn calculate_sma(&self, period: usize) -> Option<Decimal> {
        if self.prices.len() < period {
            return None;
        }

        let sum: Decimal = self.prices.iter().rev().take(period).sum();
        Some(sum / Decimal::from(period))
    }

    async fn on_price(&mut self, price: Decimal) -> Result<(), Box<dyn std::error::Error>> {
        // Update price history
        self.prices.push_back(price);
        if self.prices.len() > self.long_period {
            self.prices.pop_front();
        }

        // Calculate moving averages
        let short_sma = match self.calculate_sma(self.short_period) {
            Some(v) => v,
            None => return Ok(()),  // Not enough data
        };

        let long_sma = match self.calculate_sma(self.long_period) {
            Some(v) => v,
            None => return Ok(()),
        };

        println!("SMA({})={:.2}, SMA({})={:.2}",
            self.short_period, short_sma,
            self.long_period, long_sma
        );

        // Trading signals
        if short_sma > long_sma && self.position == dec!(0) {
            // Bullish crossover - buy
            println!("BUY SIGNAL");
            let order = self.client.place_order(
                &self.symbol,
                OrderSide::Buy,
                OrderType::Market,
                None, Some(dec!(0.001)), None, None, None, None,
            ).await?;
            self.position = order.executed_qty;
        } else if short_sma < long_sma && self.position > dec!(0) {
            // Bearish crossover - sell
            println!("SELL SIGNAL");
            let order = self.client.place_order(
                &self.symbol,
                OrderSide::Sell,
                OrderType::Market,
                None, Some(self.position), None, None, None, None,
            ).await?;
            self.position = dec!(0);
        }

        Ok(())
    }
}
```

### Running the Strategy

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TradingClient::new("http://localhost:8888")
        .with_api_key("your-key")
        .with_secret("your-secret");

    let mut strategy = MovingAverageCrossover::new(client.clone(), "BTCUSDT");

    // Connect to WebSocket for real-time prices
    let url = "ws://localhost:8888/ws";
    let (mut ws, _) = connect_async(url).await?;

    // Subscribe to trades
    ws.send(json!({
        "method": "SUBSCRIBE",
        "params": ["btcusdt@trade"],
        "id": 1
    }).to_string().into()).await?;

    // Process trades
    while let Some(msg) = ws.next().await {
        if let Ok(message) = msg {
            let data: serde_json::Value = serde_json::from_str(&message.to_string())?;

            if let Some(price_str) = data.get("p").and_then(|p| p.as_str()) {
                let price: Decimal = price_str.parse()?;
                strategy.on_price(price).await?;
            }
        }
    }

    Ok(())
}
```

---

## Step 6: Real-Time Order Updates

### User Data Stream

```rust
async fn listen_to_user_data(client: &TradingClient) -> Result<(), Box<dyn std::error::Error>> {
    // Create a listen key
    let listen_key = client.create_listen_key().await?;
    println!("Listen key: {}", listen_key);

    // Connect to user data stream
    let url = format!("ws://localhost:8888/ws/{}", listen_key);
    let (mut ws, _) = connect_async(&url).await?;

    // Spawn keepalive task
    let key = listen_key.clone();
    let client_clone = client.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30 * 60)).await;
            let _ = client_clone.keepalive_listen_key(&key).await;
        }
    });

    // Process events
    while let Some(msg) = ws.next().await {
        if let Ok(message) = msg {
            let event: serde_json::Value = serde_json::from_str(&message.to_string())?;

            match event.get("e").and_then(|e| e.as_str()) {
                Some("executionReport") => {
                    handle_execution_report(&event);
                }
                Some("outboundAccountPosition") => {
                    handle_account_update(&event);
                }
                Some("balanceUpdate") => {
                    handle_balance_update(&event);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_execution_report(event: &serde_json::Value) {
    let order_id = event["i"].as_u64().unwrap_or(0);
    let status = event["X"].as_str().unwrap_or("UNKNOWN");
    let side = event["S"].as_str().unwrap_or("UNKNOWN");
    let executed = event["z"].as_str().unwrap_or("0");
    let price = event["p"].as_str().unwrap_or("0");

    println!("Order {} {} - Status: {}, Filled: {} @ {}",
        order_id, side, status, executed, price);
}

fn handle_account_update(event: &serde_json::Value) {
    if let Some(balances) = event["B"].as_array() {
        for balance in balances {
            let asset = balance["a"].as_str().unwrap_or("?");
            let free = balance["f"].as_str().unwrap_or("0");
            let locked = balance["l"].as_str().unwrap_or("0");
            println!("Balance {}: free={}, locked={}", asset, free, locked);
        }
    }
}

fn handle_balance_update(event: &serde_json::Value) {
    let asset = event["a"].as_str().unwrap_or("?");
    let delta = event["d"].as_str().unwrap_or("0");
    println!("Balance change: {} {}", delta, asset);
}
```

---

## Complete Trading Bot Example

```rust
use openibank_sdk::{TradingClient, types::*};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::sync::RwLock;

struct TradingBot {
    client: TradingClient,
    symbol: String,
    position: Arc<RwLock<Decimal>>,
    max_position: Decimal,
    take_profit_pct: Decimal,
    stop_loss_pct: Decimal,
}

impl TradingBot {
    fn new(client: TradingClient, symbol: &str) -> Self {
        Self {
            client,
            symbol: symbol.to_string(),
            position: Arc::new(RwLock::new(dec!(0))),
            max_position: dec!(1.0),
            take_profit_pct: dec!(0.02),  // 2%
            stop_loss_pct: dec!(0.01),    // 1%
        }
    }

    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Start user data stream
        let client = self.client.clone();
        let position = self.position.clone();

        tokio::spawn(async move {
            // Listen for order updates
            // Update position on fills
        });

        // Start market data stream
        let url = "ws://localhost:8888/ws";
        let (mut ws, _) = connect_async(url).await?;

        ws.send(json!({
            "method": "SUBSCRIBE",
            "params": [format!("{}@trade", self.symbol.to_lowercase())],
            "id": 1
        }).to_string().into()).await?;

        while let Some(msg) = ws.next().await {
            if let Ok(message) = msg {
                self.on_message(&message.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn on_message(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let data: serde_json::Value = serde_json::from_str(message)?;

        if let Some(price_str) = data.get("p").and_then(|p| p.as_str()) {
            let price: Decimal = price_str.parse()?;
            self.on_trade(price).await?;
        }

        Ok(())
    }

    async fn on_trade(&self, price: Decimal) -> Result<(), Box<dyn std::error::Error>> {
        let position = *self.position.read().await;

        // Implement your trading logic here
        // This is just a placeholder

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TradingClient::new("http://localhost:8888")
        .with_api_key("your-key")
        .with_secret("your-secret");

    let bot = TradingBot::new(client, "BTCUSDT");
    bot.run().await?;

    Ok(())
}
```

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `INSUFFICIENT_BALANCE` | Not enough funds | Check wallet balance first |
| `PRICE_FILTER_FAILURE` | Price outside allowed range | Use `tickSize` from exchange info |
| `LOT_SIZE_FAILURE` | Quantity too small/large | Check `minQty`, `maxQty`, `stepSize` |
| `MIN_NOTIONAL` | Order value too small | Ensure `price * qty >= minNotional` |
| `WebSocket disconnected` | Network issue or timeout | Implement reconnection logic |

---

## Best Practices

1. **Always use decimal libraries** - Never use floating point for money
2. **Validate before sending** - Check filters client-side
3. **Handle all order states** - NEW, PARTIALLY_FILLED, FILLED, CANCELED, REJECTED, EXPIRED
4. **Implement reconnection** - WebSocket connections can drop
5. **Rate limit your requests** - Respect exchange limits
6. **Log everything** - For debugging and auditing

---

## Next Steps

- [Tutorial 7: Agent Competitions (Arena)](./07-arena.md)
- [Tutorial 8: Fleet Orchestration (PALM)](./08-palm.md)
- [API Reference](../api/README.md)
