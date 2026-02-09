# OpeniBank API Reference

> Complete REST & WebSocket API Documentation

This reference covers all endpoints available in the OpeniBank API, which is **100% compatible with the Binance Spot API** for trading operations.

---

## Base URLs

| Environment | URL |
|-------------|-----|
| Local Development | `http://localhost:3000` |
| Playground | `http://localhost:8080` |
| Production | `https://api.openibank.com` |

## Authentication

OpeniBank supports multiple authentication methods:

### 1. JWT Bearer Token

```http
Authorization: Bearer <access_token>
```

### 2. API Key + Signature (HMAC-SHA256)

```http
X-MBX-APIKEY: <api_key>
# Add signature to query params
?timestamp=1234567890&signature=<hmac_sha256_signature>
```

### 3. Session Cookie

For browser-based applications using the playground.

---

## API Sections

| Section | Description | Auth Required |
|---------|-------------|:-------------:|
| [Public Market Data](#public-market-data) | Prices, orderbook, trades | ❌ |
| [Account](#account-endpoints) | Balances, settings | ✅ |
| [Trading](#trading-endpoints) | Orders, trades | ✅ |
| [Wallet](#wallet-endpoints) | Deposits, withdrawals | ✅ |
| [Authentication](#authentication-endpoints) | Login, register, 2FA | Varies |
| [Agent](#agent-endpoints) | Agent management | ✅ |
| [Escrow](#escrow-endpoints) | Trade settlement | ✅ |
| [Receipts](#receipt-endpoints) | Verification | ✅ |

---

## Public Market Data

### Get Server Time

```http
GET /api/v1/time
```

**Response:**
```json
{
  "serverTime": 1707836400000
}
```

### Ping

```http
GET /api/v1/ping
```

**Response:** `{}` (empty JSON)

### Exchange Information

```http
GET /api/v1/exchangeInfo
```

**Response:**
```json
{
  "timezone": "UTC",
  "serverTime": 1707836400000,
  "rateLimits": [
    {
      "rateLimitType": "REQUEST_WEIGHT",
      "interval": "MINUTE",
      "intervalNum": 1,
      "limit": 1200
    }
  ],
  "symbols": [
    {
      "symbol": "BTCUSDT",
      "status": "TRADING",
      "baseAsset": "BTC",
      "baseAssetPrecision": 8,
      "quoteAsset": "USDT",
      "quotePrecision": 8,
      "orderTypes": ["LIMIT", "MARKET", "STOP_LOSS_LIMIT"],
      "filters": [
        {
          "filterType": "PRICE_FILTER",
          "minPrice": "0.01",
          "maxPrice": "1000000",
          "tickSize": "0.01"
        },
        {
          "filterType": "LOT_SIZE",
          "minQty": "0.00001",
          "maxQty": "9000",
          "stepSize": "0.00001"
        }
      ]
    }
  ]
}
```

### Order Book

```http
GET /api/v1/depth
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| symbol | STRING | ✅ | Trading pair (e.g., BTCUSDT) |
| limit | INT | ❌ | Depth limit (5, 10, 20, 50, 100, 500, 1000, 5000) |

**Response:**
```json
{
  "lastUpdateId": 1707836400000,
  "bids": [
    ["50000.00", "1.5"],
    ["49999.00", "2.3"]
  ],
  "asks": [
    ["50001.00", "0.8"],
    ["50002.00", "1.2"]
  ]
}
```

### Recent Trades

```http
GET /api/v1/trades
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| symbol | STRING | ✅ | Trading pair |
| limit | INT | ❌ | Number of trades (default 500, max 1000) |

### Klines (Candlesticks)

```http
GET /api/v1/klines
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| symbol | STRING | ✅ | Trading pair |
| interval | ENUM | ✅ | 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M |
| startTime | LONG | ❌ | Start time in milliseconds |
| endTime | LONG | ❌ | End time in milliseconds |
| limit | INT | ❌ | Number of candles (default 500, max 1000) |

**Response:**
```json
[
  [
    1707836400000,    // Open time
    "50000.00",       // Open
    "50500.00",       // High
    "49800.00",       // Low
    "50200.00",       // Close
    "1234.56",        // Volume
    1707839999999,    // Close time
    "61728000.00",    // Quote volume
    1234,             // Number of trades
    "617.28",         // Taker buy base
    "30864000.00",    // Taker buy quote
    "0"               // Ignore
  ]
]
```

### 24hr Ticker

```http
GET /api/v1/ticker/24hr
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| symbol | STRING | ❌ | Trading pair (omit for all) |

### Price Ticker

```http
GET /api/v1/ticker/price
```

**Response:**
```json
{
  "symbol": "BTCUSDT",
  "price": "50000.00"
}
```

### Book Ticker

```http
GET /api/v1/ticker/bookTicker
```

**Response:**
```json
{
  "symbol": "BTCUSDT",
  "bidPrice": "50000.00",
  "bidQty": "1.5",
  "askPrice": "50001.00",
  "askQty": "0.8"
}
```

---

## Account Endpoints

### Get Account Information

```http
GET /api/v1/account
```

**Headers:** `Authorization: Bearer <token>`

**Response:**
```json
{
  "makerCommission": 10,
  "takerCommission": 10,
  "buyerCommission": 0,
  "sellerCommission": 0,
  "canTrade": true,
  "canWithdraw": true,
  "canDeposit": true,
  "accountType": "SPOT",
  "balances": [
    {
      "asset": "BTC",
      "free": "1.5",
      "locked": "0.2"
    },
    {
      "asset": "USDT",
      "free": "50000.00",
      "locked": "1000.00"
    }
  ]
}
```

### Get Account Balances

```http
GET /api/v1/account/balances
```

### Get Single Asset Balance

```http
GET /api/v1/account/balance
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| asset | STRING | ✅ | Asset symbol (e.g., BTC) |

---

## Trading Endpoints

### Create Order

```http
POST /api/v1/order
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| symbol | STRING | ✅ | Trading pair |
| side | ENUM | ✅ | BUY or SELL |
| type | ENUM | ✅ | LIMIT, MARKET, STOP_LOSS_LIMIT, TAKE_PROFIT_LIMIT |
| timeInForce | ENUM | ❌ | GTC (default), IOC, FOK |
| quantity | DECIMAL | ✅ | Order quantity |
| price | DECIMAL | Conditional | Required for LIMIT orders |
| newClientOrderId | STRING | ❌ | Custom order ID |
| stopPrice | DECIMAL | Conditional | Required for STOP_LOSS_LIMIT |
| newOrderRespType | ENUM | ❌ | ACK, RESULT, FULL |

**Response (FULL):**
```json
{
  "symbol": "BTCUSDT",
  "orderId": 12345,
  "orderListId": -1,
  "clientOrderId": "my_order_001",
  "transactTime": 1707836400000,
  "price": "50000.00",
  "origQty": "1.0",
  "executedQty": "1.0",
  "cummulativeQuoteQty": "50000.00",
  "status": "FILLED",
  "timeInForce": "GTC",
  "type": "LIMIT",
  "side": "BUY",
  "fills": [
    {
      "price": "50000.00",
      "qty": "1.0",
      "commission": "0.001",
      "commissionAsset": "BTC"
    }
  ]
}
```

### Query Order

```http
GET /api/v1/order
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| symbol | STRING | ✅ | Trading pair |
| orderId | LONG | Conditional | Order ID |
| origClientOrderId | STRING | Conditional | Client order ID |

### Cancel Order

```http
DELETE /api/v1/order
```

### Get Open Orders

```http
GET /api/v1/openOrders
```

### Get All Orders

```http
GET /api/v1/allOrders
```

### Cancel All Orders

```http
DELETE /api/v1/openOrders
```

### Get Account Trades

```http
GET /api/v1/myTrades
```

---

## Wallet Endpoints

### Get Deposit Address

```http
GET /api/v1/wallet/deposit/address
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| coin | STRING | ✅ | Asset symbol |
| network | STRING | ❌ | Network (e.g., ETH, TRC20) |

### Get Deposit History

```http
GET /api/v1/wallet/deposits
```

### Submit Withdrawal

```http
POST /api/v1/wallet/withdraw
```

| Parameter | Type | Required | Description |
|-----------|------|:--------:|-------------|
| coin | STRING | ✅ | Asset symbol |
| network | STRING | ❌ | Network |
| address | STRING | ✅ | Withdrawal address |
| amount | DECIMAL | ✅ | Amount to withdraw |
| memo | STRING | ❌ | Address tag/memo |

### Get Withdrawal History

```http
GET /api/v1/wallet/withdrawals
```

### Get All Coins Info

```http
GET /api/v1/wallet/coins
```

### Internal Transfer

```http
POST /api/v1/wallet/transfer
```

---

## Authentication Endpoints

### Register

```http
POST /api/v1/auth/register
```

```json
{
  "email": "agent@example.com",
  "password": "SecurePass123!",
  "confirmPassword": "SecurePass123!"
}
```

### Login

```http
POST /api/v1/auth/login
```

```json
{
  "email": "agent@example.com",
  "password": "SecurePass123!"
}
```

**Response:**
```json
{
  "accessToken": "eyJhbGciOiJIUzI1NiIs...",
  "refreshToken": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
  "expiresIn": 3600,
  "tokenType": "Bearer"
}
```

### Refresh Token

```http
POST /api/v1/auth/refresh
```

### Enable 2FA

```http
POST /api/v1/auth/2fa/enable
```

### Verify 2FA

```http
POST /api/v1/auth/2fa/verify
```

### Create API Key

```http
POST /api/v1/auth/api-keys
```

### List API Keys

```http
GET /api/v1/auth/api-keys
```

### Revoke API Key

```http
DELETE /api/v1/auth/api-keys
```

---

## Agent Endpoints

### List Agents

```http
GET /api/agents
```

### Create Buyer Agent

```http
POST /api/agents/buyer
```

```json
{
  "name": "Buyer-001",
  "initial_balance": 100000
}
```

### Create Seller Agent

```http
POST /api/agents/seller
```

### Fund Agent

```http
POST /api/agents/{id}/fund
```

### Get Agent Activity

```http
GET /api/agents/{id}/activity
```

### Get Agent Kernel Trace

```http
GET /api/agents/{id}/kernel-trace
```

---

## Escrow Endpoints

### Create Escrow

```http
POST /api/escrow
```

```json
{
  "buyer_id": "agent-001",
  "seller_id": "agent-002",
  "amount": 10000,
  "currency": "IUSD",
  "delivery_conditions": ["Deliver API access", "Provide documentation"]
}
```

### Fund Escrow

```http
POST /api/escrow/{id}/fund
```

### Confirm Delivery

```http
POST /api/escrow/{id}/confirm
```

### Release Escrow

```http
POST /api/escrow/{id}/release
```

### Dispute Escrow

```http
POST /api/escrow/{id}/dispute
```

---

## Receipt Endpoints

### List Receipts

```http
GET /api/receipts
```

### Get Receipt

```http
GET /api/receipts/{id}
```

### Verify Receipt

```http
POST /api/receipts/verify
```

```json
{
  "receipt_id": "rcpt_xxx",
  "signature": "..."
}
```

### Replay Receipts

```http
POST /api/receipts/replay
```

---

## WebSocket API

### Connection

```
wss://stream.openibank.com/ws
```

### Subscribe to Streams

```json
{
  "method": "SUBSCRIBE",
  "params": [
    "btcusdt@trade",
    "btcusdt@depth",
    "btcusdt@kline_1m"
  ],
  "id": 1
}
```

### Available Streams

| Stream | Description |
|--------|-------------|
| `<symbol>@trade` | Real-time trades |
| `<symbol>@depth` | Order book updates |
| `<symbol>@depth@100ms` | Order book updates (100ms) |
| `<symbol>@kline_<interval>` | Candlestick updates |
| `<symbol>@ticker` | 24hr ticker |
| `<symbol>@miniTicker` | Mini ticker |
| `<symbol>@bookTicker` | Best bid/ask |
| `!ticker@arr` | All tickers |

### User Data Stream

```http
POST /api/v1/userDataStream
```

**Response:**
```json
{
  "listenKey": "abc123..."
}
```

Connect to: `wss://stream.openibank.com/ws/<listenKey>`

---

## Error Handling

### Error Response Format

```json
{
  "code": -1102,
  "msg": "Mandatory parameter 'symbol' was not sent."
}
```

### Error Codes

| Code | Description |
|------|-------------|
| -1000 | Unknown error |
| -1002 | Invalid API key |
| -1021 | Timestamp outside recv window |
| -1022 | Invalid signature |
| -1102 | Mandatory parameter missing |
| -2010 | Insufficient balance |
| -2011 | Unknown order |
| -2013 | Order not found |
| -2015 | Invalid order |

---

## Rate Limits

| Limit Type | Interval | Limit |
|------------|----------|-------|
| REQUEST_WEIGHT | MINUTE | 1200 |
| ORDERS | SECOND | 10 |
| ORDERS | DAY | 200000 |

Rate limit info is returned in response headers:
- `X-MBX-USED-WEIGHT-1M`
- `X-MBX-ORDER-COUNT-1S`
- `X-MBX-ORDER-COUNT-1D`

---

## SDKs & Libraries

- **Rust**: `openibank-sdk` crate
- **Python**: Coming soon
- **JavaScript/TypeScript**: Coming soon
- **Go**: Coming soon

---

## OpenAPI Specification

Interactive API documentation is available at:

- **Local**: `http://localhost:3000/swagger-ui`
- **Production**: `https://api.openibank.com/swagger-ui`

Download OpenAPI spec:
- JSON: `/api/openapi.json`
- YAML: `/api/openapi.yaml`
