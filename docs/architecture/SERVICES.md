# OpeniBank Services Overview

> A guide to all services in the OpeniBank platform

---

## Service Map

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          OpeniBank Platform Services                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────┐   │
│   │  openibank-server   │   │ openibank-playground│   │ openibank-api-  │   │
│   │      (unified)      │   │    (agent demo)     │   │     server      │   │
│   │                     │   │                     │   │   (production)  │   │
│   │  Port: 8080         │   │  Port: 8080         │   │  Port: 3000     │   │
│   │                     │   │                     │   │                 │   │
│   │  • Maple AI         │   │  • Agent Creation   │   │  • REST API     │   │
│   │  • PALM Fleet       │   │  • Trade Simulation │   │  • WebSocket    │   │
│   │  • UAL Console      │   │  • Escrow Demo      │   │  • Auth (JWT)   │   │
│   │  • Web Dashboard    │   │  • Receipt Viewer   │   │  • PostgreSQL   │   │
│   └─────────────────────┘   └─────────────────────┘   └─────────────────┘   │
│                                                                              │
│   ┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────┐   │
│   │  resonancex-server  │   │ openibank-issuer-   │   │   openibank-    │   │
│   │  (trading exchange) │   │    resonator        │   │      mcp        │   │
│   │                     │   │                     │   │ (Claude Desktop)│   │
│   │  Port: 8888         │   │  Port: 8081         │   │  Port: stdio    │   │
│   │                     │   │                     │   │                 │   │
│   │  • Order Book       │   │  • IUSD Minting     │   │  • MCP Protocol │   │
│   │  • Trading Charts   │   │  • Reserve Mgmt     │   │  • Tool Calls   │   │
│   │  • Arena            │   │  • Attestations     │   │  • Prompts      │   │
│   │  • WebSocket        │   │                     │   │                 │   │
│   └─────────────────────┘   └─────────────────────┘   └─────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Services Comparison

| Service | Port | Purpose | Use Case |
|---------|------|---------|----------|
| **openibank-server** | 8080 | Unified all-in-one server | Quick start, demos |
| **openibank-playground** | 8080 | Interactive agent demo | Learning, testing |
| **openibank-api-server** | 3000 | Production REST API | Production deployment |
| **resonancex-server** | 8888 | Trading exchange | Trading dashboard |
| **openibank-issuer-resonator** | 8081 | IUSD issuer | Standalone issuer |
| **openibank-mcp** | stdio | Claude Desktop | AI assistant integration |

---

## 1. openibank-server (Unified)

**The recommended starting point for most users.**

```bash
cargo run -p openibank-server
```

**URL**: http://localhost:8080

### Features:
- Maple AI Framework integration
- PALM Fleet orchestration
- UAL command console
- Web dashboard
- Multi-LLM support (Ollama, OpenAI, Anthropic)

### Dashboard Tabs:
- **Overview**: System status, agent list
- **Fleet**: Agent fleet management
- **UAL Console**: Execute banking commands
- **Logs**: Real-time activity logs

---

## 2. openibank-playground

**Interactive demo for learning and testing.**

```bash
cargo run -p openibank-playground
```

**URL**: http://localhost:8080

### Features:
- Create buyer/seller/arbiter agents
- Execute trades with escrow
- View cryptographic receipts
- Real-time SSE event streaming
- Maple resonator visualization

### Dashboard Sections:
- **Agents**: Create and manage agents
- **Trading**: Execute trades
- **Receipts**: View and verify receipts
- **Ledger**: Account balances
- **System Log**: Activity timeline

---

## 3. openibank-api-server (Production)

**Production-ready REST API server with database integration.**

```bash
# With PostgreSQL and Redis
cargo run -p openibank-api-server

# Or with Docker
docker-compose up
```

**URL**: http://localhost:3000
**OpenAPI**: http://localhost:3000/swagger-ui

### Features:
- Binance-compatible REST API
- JWT + API Key authentication
- PostgreSQL persistence
- Redis caching & rate limiting
- WebSocket streaming
- Prometheus metrics

### Endpoints:
- `/api/v1/account` - Account management
- `/api/v1/order` - Trading
- `/api/v1/wallet` - Deposits/Withdrawals
- `/api/v1/ticker/*` - Market data

---

## 4. resonancex-server (Trading Exchange)

**High-performance trading platform with professional UI.**

```bash
cargo run -p resonancex-server
# Or with demo mode
cargo run -p resonancex-server -- --demo
```

**URL**: http://localhost:8888

### Features:
- Real-time order book
- TradingView-compatible charts (Lightweight Charts)
- WebSocket market data
- Arena competitions
- Agent trading

### Dashboard Components:
- **Charts**: OHLCV candlesticks
- **Order Book**: Bids/asks with depth
- **Trade Panel**: Buy/sell interface
- **Market Selector**: Symbol switching

### WebSocket Streams:
```javascript
// Connect to market data
const ws = new WebSocket('ws://localhost:8888/ws');

// Subscribe
ws.send(JSON.stringify({
  method: 'SUBSCRIBE',
  params: ['btcusdt@trade', 'btcusdt@depth@100ms']
}));
```

---

## 5. openibank-issuer-resonator

**Standalone IUSD stablecoin issuer.**

```bash
cargo run -p openibank-issuer-resonator
```

**URL**: http://localhost:8081

### API Endpoints:
- `GET /v1/issuer/supply` - Current supply
- `POST /v1/issuer/mint` - Mint IUSD
- `POST /v1/issuer/burn` - Burn IUSD
- `GET /v1/issuer/receipts` - Mint/burn receipts

---

## 6. openibank-mcp (Claude Desktop)

**Model Context Protocol server for Claude Desktop integration.**

```bash
cargo run -p openibank-mcp
```

### Configuration:
Add to `~/.config/claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "openibank": {
      "command": "/path/to/openibank-mcp"
    }
  }
}
```

### Available Tools:
- `create_agent` - Create buyer/seller agents
- `fund_agent` - Add funds to agent
- `execute_trade` - Run trade between agents
- `get_balance` - Check account balances
- `list_receipts` - View transaction receipts

---

## Running Multiple Services

### Development Setup

```bash
# Terminal 1: API Server (production backend)
cargo run -p openibank-api-server

# Terminal 2: Trading Exchange
cargo run -p resonancex-server

# Terminal 3: Playground for demos
cargo run -p openibank-playground --port 8081  # Different port
```

### Production Docker Compose

```yaml
version: '3.8'

services:
  api:
    image: openibank/api-server:latest
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgres://...

  exchange:
    image: openibank/resonancex-server:latest
    ports:
      - "8888:8888"

  playground:
    image: openibank/playground:latest
    ports:
      - "8080:8080"
```

---

## Service Dependencies

```
┌───────────────────────────────────────────────────────────────────────┐
│                          External Dependencies                         │
├───────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   PostgreSQL ◄─────── openibank-api-server                            │
│       │                      │                                         │
│       └──────────────────────┼───────── Required for production       │
│                              │                                         │
│   Redis ◄────────────────────┘          Optional (caching/rate limit) │
│                                                                        │
│   Ollama ◄─────── openibank-server ────► Optional (LLM for agents)   │
│           ◄─────── openibank-playground                               │
│                                                                        │
│   None ◄───────── resonancex-server ───► Fully standalone             │
│                                                                        │
└───────────────────────────────────────────────────────────────────────┘
```

---

## Quick Reference

### Start Everything (Development)

```bash
# Single command - unified server
cargo run -p openibank-server
```

### Start Production Stack

```bash
# 1. Start databases
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=password postgres:16
docker run -d -p 6379:6379 redis:7

# 2. Run migrations
cargo run -p openibank-db -- migrate

# 3. Start API server
cargo run -p openibank-api-server

# 4. Start exchange (optional)
cargo run -p resonancex-server
```

### URLs at a Glance

| URL | Service |
|-----|---------|
| http://localhost:8080 | Playground / Unified Server |
| http://localhost:8888 | ResonanceX Trading Dashboard |
| http://localhost:3000 | Production API |
| http://localhost:3000/swagger-ui | API Documentation |

---

## Next Steps

1. **Getting Started**: See [GETTING_STARTED.md](../GETTING_STARTED.md)
2. **API Reference**: See [API Documentation](../api/README.md)
3. **Deployment**: See [Deployment Guide](../deployment/README.md)
