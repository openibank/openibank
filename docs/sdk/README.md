# OpeniBank SDK Guide

> Building Applications with the Rust SDK

The OpeniBank SDK provides a type-safe, async-first interface for building applications on the OpeniBank platform.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
openibank-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
```

---

## Quick Start

```rust
use openibank_sdk::{Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = Client::new(Config {
        base_url: "http://localhost:3000".into(),
        api_key: Some("your-api-key".into()),
        api_secret: Some("your-api-secret".into()),
    })?;

    // Get account info
    let account = client.account().get_info().await?;
    println!("Account: {:?}", account);

    // Get balances
    let balances = client.account().get_balances().await?;
    for balance in balances {
        println!("{}: {} available, {} locked",
            balance.asset, balance.free, balance.locked);
    }

    Ok(())
}
```

---

## Authentication

### API Key + Secret

```rust
let client = Client::new(Config {
    base_url: "http://localhost:3000".into(),
    api_key: Some("your-api-key".into()),
    api_secret: Some("your-api-secret".into()),
})?;
```

### JWT Token

```rust
let client = Client::new(Config {
    base_url: "http://localhost:3000".into(),
    bearer_token: Some("your-jwt-token".into()),
    ..Default::default()
})?;

// Or authenticate after creation
let tokens = client.auth().login("email@example.com", "password").await?;
client.set_bearer_token(&tokens.access_token);
```

---

## Market Data API

### Get Exchange Info

```rust
let info = client.market().get_exchange_info().await?;
for symbol in info.symbols {
    println!("{}: {}", symbol.symbol, symbol.status);
}
```

### Get Order Book

```rust
let depth = client.market()
    .get_order_book("BTCUSDT", Some(20))
    .await?;

println!("Best bid: {} @ {}", depth.bids[0].price, depth.bids[0].quantity);
println!("Best ask: {} @ {}", depth.asks[0].price, depth.asks[0].quantity);
```

### Get Klines

```rust
use openibank_sdk::types::KlineInterval;

let klines = client.market()
    .get_klines("BTCUSDT", KlineInterval::OneHour, None, None, Some(100))
    .await?;

for kline in klines {
    println!("O: {} H: {} L: {} C: {}",
        kline.open, kline.high, kline.low, kline.close);
}
```

### Real-time Market Data

```rust
use openibank_sdk::websocket::{WebSocketClient, Stream};

let ws = WebSocketClient::connect("ws://localhost:8080/ws").await?;

// Subscribe to streams
ws.subscribe(&[
    Stream::Trade("btcusdt"),
    Stream::Depth("btcusdt", 100),
    Stream::Kline("btcusdt", KlineInterval::OneMinute),
]).await?;

// Handle messages
while let Some(msg) = ws.next().await {
    match msg {
        Message::Trade(trade) => {
            println!("Trade: {} @ {}", trade.quantity, trade.price);
        }
        Message::DepthUpdate(depth) => {
            println!("Depth update: {} bids, {} asks",
                depth.bids.len(), depth.asks.len());
        }
        Message::Kline(kline) => {
            println!("Kline: {} close", kline.close);
        }
        _ => {}
    }
}
```

---

## Trading API

### Place Limit Order

```rust
use openibank_sdk::types::{Side, OrderType, TimeInForce};

let order = client.trading()
    .create_order(CreateOrderRequest {
        symbol: "BTCUSDT".into(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        quantity: dec!(0.01),
        price: Some(dec!(50000)),
        time_in_force: Some(TimeInForce::GTC),
        client_order_id: Some("my-order-001".into()),
        ..Default::default()
    })
    .await?;

println!("Order created: {}", order.order_id);
```

### Place Market Order

```rust
let order = client.trading()
    .create_order(CreateOrderRequest {
        symbol: "BTCUSDT".into(),
        side: Side::Buy,
        order_type: OrderType::Market,
        quantity: dec!(0.01),
        ..Default::default()
    })
    .await?;
```

### Query Order

```rust
let order = client.trading()
    .get_order("BTCUSDT", Some(12345), None)
    .await?;

println!("Status: {:?}, Filled: {}/{}",
    order.status, order.executed_qty, order.orig_qty);
```

### Cancel Order

```rust
let cancelled = client.trading()
    .cancel_order("BTCUSDT", Some(12345), None)
    .await?;
```

### Get Open Orders

```rust
let orders = client.trading()
    .get_open_orders(Some("BTCUSDT"))
    .await?;
```

### Get Trade History

```rust
let trades = client.trading()
    .get_my_trades("BTCUSDT", None, None, Some(100))
    .await?;
```

---

## Wallet API

### Get Deposit Address

```rust
let address = client.wallet()
    .get_deposit_address("BTC", Some("BTC"))
    .await?;

println!("Deposit to: {}", address.address);
```

### Get Deposit History

```rust
let deposits = client.wallet()
    .get_deposit_history(Some("BTC"), None, None, Some(100))
    .await?;
```

### Submit Withdrawal

```rust
let withdrawal = client.wallet()
    .submit_withdrawal(WithdrawalRequest {
        coin: "BTC".into(),
        network: Some("BTC".into()),
        address: "bc1q...".into(),
        amount: dec!(0.01),
        memo: None,
    })
    .await?;

println!("Withdrawal ID: {}", withdrawal.id);
```

### Internal Transfer

```rust
client.wallet()
    .internal_transfer(InternalTransferRequest {
        asset: "USDT".into(),
        amount: dec!(100),
        from_account: "spot".into(),
        to_account: "margin".into(),
    })
    .await?;
```

---

## Agent API

### Create Agent

```rust
use openibank_sdk::agent::{AgentBuilder, AgentType};

let agent = AgentBuilder::new()
    .agent_type(AgentType::Buyer)
    .name("my-trading-agent")
    .initial_balance(dec!(10000))
    .build(&client)
    .await?;

println!("Agent created: {}", agent.agent_id);
```

### Fund Agent

```rust
client.agent()
    .fund(&agent.agent_id, dec!(5000))
    .await?;
```

### Create Permit

```rust
use openibank_sdk::permits::{PermitBuilder, CounterpartyConstraint};

let permit = PermitBuilder::new()
    .max_amount(dec!(1000))
    .counterparty(CounterpartyConstraint::Specific(seller_id))
    .purpose("API subscription")
    .expires_in(Duration::days(30))
    .build(&agent)
    .await?;
```

### Execute Payment

```rust
let receipt = agent
    .pay(&permit, seller_id, dec!(100), "Monthly subscription")
    .await?;

println!("Payment receipt: {}", receipt.receipt_id);
```

---

## Escrow API

### Create Escrow

```rust
use openibank_sdk::escrow::EscrowBuilder;

let escrow = EscrowBuilder::new()
    .buyer(buyer_id)
    .seller(seller_id)
    .arbiter(arbiter_id)
    .amount(dec!(1000))
    .currency("IUSD")
    .deadline(Utc::now() + Duration::hours(24))
    .add_condition("Deliver API access")
    .add_condition("Provide documentation")
    .build(&client)
    .await?;
```

### Fund Escrow

```rust
client.escrow()
    .fund(&escrow.escrow_id)
    .await?;
```

### Confirm Delivery

```rust
client.escrow()
    .confirm_delivery(&escrow.escrow_id, Some("Delivery verified"))
    .await?;
```

### Release Funds

```rust
client.escrow()
    .release(&escrow.escrow_id)
    .await?;
```

---

## Receipt API

### Get Receipt

```rust
let receipt = client.receipts()
    .get("rcpt_xxx")
    .await?;
```

### Verify Receipt

```rust
let is_valid = client.receipts()
    .verify(&receipt)
    .await?;

if is_valid {
    println!("Receipt signature is valid");
}
```

### List Receipts

```rust
let receipts = client.receipts()
    .list(ReceiptFilter {
        agent_id: Some(my_agent_id),
        from_date: Some(Utc::now() - Duration::days(7)),
        ..Default::default()
    })
    .await?;
```

---

## Error Handling

```rust
use openibank_sdk::error::{SdkError, ApiError};

match client.trading().create_order(order).await {
    Ok(result) => println!("Order: {}", result.order_id),
    Err(SdkError::Api(ApiError { code, msg })) => {
        match code {
            -2010 => println!("Insufficient balance"),
            -1121 => println!("Invalid symbol"),
            _ => println!("API error {}: {}", code, msg),
        }
    }
    Err(SdkError::Network(e)) => println!("Network error: {}", e),
    Err(SdkError::Serialization(e)) => println!("Parse error: {}", e),
}
```

---

## Advanced Configuration

### Custom HTTP Client

```rust
use reqwest::ClientBuilder;

let http_client = ClientBuilder::new()
    .timeout(Duration::from_secs(30))
    .pool_max_idle_per_host(10)
    .build()?;

let client = Client::with_http_client(config, http_client)?;
```

### Retry Configuration

```rust
let client = Client::new(Config {
    base_url: "http://localhost:3000".into(),
    retry: RetryConfig {
        max_retries: 3,
        backoff_base: Duration::from_millis(100),
        backoff_max: Duration::from_secs(10),
    },
    ..Default::default()
})?;
```

### Rate Limiting

```rust
// The SDK automatically handles rate limits
// Headers are parsed and requests are throttled

let client = Client::new(Config {
    rate_limit_behavior: RateLimitBehavior::Wait,  // or Fail
    ..Default::default()
})?;
```

---

## Examples

### Market Maker Bot

```rust
async fn market_maker(client: &Client, symbol: &str) -> Result<()> {
    let depth = client.market().get_order_book(symbol, Some(5)).await?;

    let mid_price = (depth.bids[0].price + depth.asks[0].price) / dec!(2);
    let spread = dec!(0.001); // 0.1%

    // Place bid
    client.trading().create_order(CreateOrderRequest {
        symbol: symbol.into(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: Some(mid_price * (dec!(1) - spread)),
        quantity: dec!(0.1),
        ..Default::default()
    }).await?;

    // Place ask
    client.trading().create_order(CreateOrderRequest {
        symbol: symbol.into(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        price: Some(mid_price * (dec!(1) + spread)),
        quantity: dec!(0.1),
        ..Default::default()
    }).await?;

    Ok(())
}
```

### Price Alert Monitor

```rust
async fn price_alert(client: &Client, symbol: &str, target: Decimal) -> Result<()> {
    let ws = WebSocketClient::connect("ws://localhost:8080/ws").await?;
    ws.subscribe(&[Stream::Trade(symbol)]).await?;

    while let Some(Message::Trade(trade)) = ws.next().await {
        if trade.price >= target {
            println!("ALERT: {} reached {}", symbol, trade.price);
            break;
        }
    }

    Ok(())
}
```

---

## Next Steps

- [API Reference](../api/README.md) - Complete API documentation
- [Tutorials](../tutorials/README.md) - Step-by-step guides
- [Deployment](../deployment/README.md) - Production deployment
