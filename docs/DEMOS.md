# OpeniBank Demo Commands

Quick reference for running OpeniBank demos and testing the platform.

## Quick Start

### 1. Start OpeniBank Server

```bash
# Start the main banking server (default: port 8080)
cargo run -p openibank-server

# With custom port
cargo run -p openibank-server -- --port 9000

# With LLM integration (optional)
OPENIBANK_LLM_PROVIDER=ollama cargo run -p openibank-server
```

### 2. Start ResonanceX Exchange

```bash
# Start the exchange server (default: port 8888)
cargo run -p resonancex-server

# With demo trading bots
cargo run -p resonancex-server -- --demo

# With more demo agents
cargo run -p resonancex-server -- --demo --demo-agents 20
```

## Dashboards

| Service | URL | Description |
|---------|-----|-------------|
| OpeniBank Main | http://localhost:8080 | Main banking dashboard |
| PALM Fleet | http://localhost:8080/palm | Agent fleet management |
| ResonanceX | http://localhost:8888 | Trading exchange dashboard |

## API Endpoints

### OpeniBank Server (port 8080)

```bash
# System status
curl http://localhost:8080/api/status

# Health check
curl http://localhost:8080/api/health

# List agents
curl http://localhost:8080/api/agents

# Agent detail
curl http://localhost:8080/api/agents/{agent_id}

# Fleet status
curl http://localhost:8080/api/fleet/status

# Deploy agents
curl -X POST http://localhost:8080/api/fleet/deploy \
  -H "Content-Type: application/json" \
  -d '{"agent_type": "buyer", "count": 3}'

# Run demo scenario
curl -X POST http://localhost:8080/api/demo/run \
  -H "Content-Type: application/json" \
  -d '{"commit": true}'

# UAL commands
curl -X POST http://localhost:8080/api/ual \
  -H "Content-Type: application/json" \
  -d '{"command": "STATUS"}'

# Issuer supply
curl http://localhost:8080/api/issuer/supply

# List receipts
curl http://localhost:8080/api/receipts

# Export receipts (JSONL format)
curl http://localhost:8080/api/receipts/export > receipts.jsonl

# Verify receipt
curl -X POST http://localhost:8080/api/receipts/verify \
  -H "Content-Type: application/json" \
  -d '{"receipt_id": "your-receipt-id"}'
```

### ResonanceX Exchange (port 8888)

```bash
# List markets
curl http://localhost:8888/api/v1/markets

# Get ticker
curl http://localhost:8888/api/v1/markets/BTC_IUSD/ticker

# Get order book depth
curl http://localhost:8888/api/v1/markets/ETH_IUSD/depth?limit=20

# Get recent trades
curl http://localhost:8888/api/v1/markets/SOL_IUSD/trades

# Get candles (OHLCV)
curl "http://localhost:8888/api/v1/markets/BTC_IUSD/candles?interval=1m&limit=100"

# Place order
curl -X POST http://localhost:8888/api/v1/orders \
  -H "Content-Type: application/json" \
  -d '{
    "market": "BTC_IUSD",
    "side": "buy",
    "type": "limit",
    "price": "96000",
    "amount": "0.1"
  }'

# List demo agents with holdings
curl http://localhost:8888/api/v1/agents
```

## UAL Commands

Use the UAL console in the dashboard or via API:

```bash
# Check system status
STATUS

# List all agents
AGENTS

# Check balance
BALANCE buyer-001

# Fleet status
FLEET STATUS

# Deploy agents
DEPLOY buyer COUNT 3
DEPLOY seller COUNT 2
DEPLOY arbiter COUNT 1
```

## Demo Scenarios

### Run Full Demo

The demo creates agents, mints IUSD, and runs a complete escrow trade:

```bash
curl -X POST http://localhost:8080/api/demo/run \
  -H "Content-Type: application/json" \
  -d '{"commit": true}'
```

### WebSocket Streaming

Connect to real-time events:

```javascript
// OpeniBank SSE events
const eventSource = new EventSource('http://localhost:8080/api/events');
eventSource.onmessage = (e) => console.log(JSON.parse(e.data));

// ResonanceX WebSocket
const ws = new WebSocket('ws://localhost:8888/ws');
ws.onmessage = (e) => console.log(JSON.parse(e.data));
```

## Demo Agents

Pre-configured demo agents with holdings:

| Agent | Role | Holdings |
|-------|------|----------|
| buyer-001 | Buyer | 50,000 IUSD, 0.5 BTC, 5.0 ETH |
| buyer-002 | Buyer | 30,000 IUSD, 2.0 SOL, 100.0 OBK |
| seller-001 | Seller | 2.0 BTC, 20.0 ETH, 100.0 SOL, 10,000 IUSD |
| seller-002 | Seller | 10.0 ETH, 50.0 SOL, 5,000 DOGE, 20,000 IUSD |
| arbiter-001 | Arbiter | 100,000 IUSD |
| mm-bot-001 | Market Maker | 200,000 IUSD, 1.0 BTC, 10.0 ETH |

## Trading Pairs

25 markets available on ResonanceX:

**Tier 1 - Blue Chips:**
- BTC_IUSD, ETH_IUSD, SOL_IUSD, BNB_IUSD
- XRP_IUSD, ADA_IUSD, DOGE_IUSD, AVAX_IUSD
- DOT_IUSD, LINK_IUSD

**Tier 2 - DeFi & L2:**
- UNI_IUSD, AAVE_IUSD, OP_IUSD, ARB_IUSD, SUI_IUSD

**Tier 3 - AI Tokens:**
- FET_IUSD, RNDR_IUSD, TAO_IUSD, NEAR_IUSD, WLD_IUSD

**Native Token:**
- OBK_IUSD

**Cross Pairs:**
- ETH_BTC, SOL_ETH

**Stablecoins:**
- IUSD_USDT, IUSD_USDC

## Testing

```bash
# Run all tests
cargo test

# Run specific package tests
cargo test -p openibank-server
cargo test -p resonancex-server
cargo test -p openibank-types
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| OPENIBANK_HOST | Server bind host | 0.0.0.0 |
| OPENIBANK_PORT | Server port | 8080 |
| OPENIBANK_LLM_PROVIDER | LLM provider (ollama, openai, anthropic) | none |
| ANTHROPIC_API_KEY | Anthropic API key | - |
| OPENAI_API_KEY | OpenAI API key | - |

## Troubleshooting

### Port Already in Use

```bash
# Find process using port
lsof -i :8080
lsof -i :8888

# Kill process
kill -9 <PID>
```

### Build Issues

```bash
# Clean and rebuild
cargo clean
cargo build
```

### LLM Not Connecting

```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Test with explicit provider
OPENIBANK_LLM_PROVIDER=ollama cargo run -p openibank-server
```
