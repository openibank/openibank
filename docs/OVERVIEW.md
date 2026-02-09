# OpeniBank Platform Overview

> **The World's First AI-Native Banking Platform**

OpeniBank is a complete, production-ready financial infrastructure designed exclusively for autonomous AI agents. This document provides a comprehensive overview of all platform components.

---

## Platform Statistics

| Metric | Value |
|--------|-------|
| **Total Crates** | 47 |
| **Total Services** | 8 |
| **Unit Tests** | 388+ |
| **API Endpoints** | 100+ |
| **Supported Languages** | Rust, Python, TypeScript, Go |

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                  OpeniBank Platform                                  │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │                            WEB LAYER (External)                               │   │
│  │                                                                               │   │
│  │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │   │
│  │   │ Landing Page │  │   Portal     │  │ Marketplace  │  │    Docs      │     │   │
│  │   │  (port 3080) │  │  (port 9000) │  │  (port 3007) │  │              │     │   │
│  │   └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘     │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │                          APPLICATION LAYER                                     │   │
│  │                                                                               │   │
│  │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │   │
│  │   │  Playground  │  │  API Server  │  │  ResonanceX  │  │    Issuer    │     │   │
│  │   │  (port 8080) │  │  (port 3000) │  │  (port 8888) │  │  (port 8081) │     │   │
│  │   │              │  │              │  │              │  │              │     │   │
│  │   │  • Agent UI  │  │  • REST API  │  │  • Trading   │  │  • IUSD Mint │     │   │
│  │   │  • UAL       │  │  • WebSocket │  │  • Charts    │  │  • Reserve   │     │   │
│  │   │  • Maple AI  │  │  • OpenAPI   │  │  • Arena     │  │  • Receipts  │     │   │
│  │   └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘     │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │                          TRADING ENGINE (ResonanceX)                          │   │
│  │                                                                               │   │
│  │   ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐             │   │
│  │   │ Orderbook  │  │  Engine    │  │ MarketData │  │   Arena    │             │   │
│  │   │            │  │            │  │            │  │            │             │   │
│  │   │ • BTreeMap │  │ • Matching │  │ • Tickers  │  │ • Compete  │             │   │
│  │   │ • Lock-free│  │ • Routing  │  │ • Klines   │  │ • Badges   │             │   │
│  │   │ • O(log n) │  │ • STP      │  │ • Depth    │  │ • Leaders  │             │   │
│  │   └────────────┘  └────────────┘  └────────────┘  └────────────┘             │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │                          AGENT FRAMEWORK                                       │   │
│  │                                                                               │   │
│  │   ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐             │   │
│  │   │   Maple    │  │   PALM     │  │   UAL      │  │   Agents   │             │   │
│  │   │   AI       │  │   Fleet    │  │   Parser   │  │            │             │   │
│  │   │            │  │            │  │            │  │ • Buyer    │             │   │
│  │   │ • LLM      │  │ • Deploy   │  │ • Commands │  │ • Seller   │             │   │
│  │   │ • Reason   │  │ • Monitor  │  │ • Execute  │  │ • Arbiter  │             │   │
│  │   └────────────┘  └────────────┘  └────────────┘  └────────────┘             │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │                          CORE BANKING                                          │   │
│  │                                                                               │   │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │   │
│  │   │ Ledger  │  │ Issuer  │  │ Escrow  │  │ Guard   │  │Receipts │           │   │
│  │   │         │  │         │  │         │  │         │  │         │           │   │
│  │   │ Double  │  │ IUSD    │  │ Multi-  │  │ Policy  │  │ Ed25519 │           │   │
│  │   │ Entry   │  │ Stable  │  │ Party   │  │ Enforce │  │ Signed  │           │   │
│  │   └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘           │   │
│  │                                                                               │   │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │   │
│  │   │ Permits │  │ Wallet  │  │ Clear   │  │ Settle  │  │ Bridge  │           │   │
│  │   │         │  │         │  │         │  │         │  │         │           │   │
│  │   │ Spend   │  │ Multi-  │  │ Batch   │  │ Final   │  │ Cross   │           │   │
│  │   │ Auth    │  │ Asset   │  │ Process │  │ Settle  │  │ Chain   │           │   │
│  │   └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘           │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────────────┐   │
│  │                          DATA LAYER                                            │   │
│  │                                                                               │   │
│  │   ┌───────────────┐     ┌───────────────┐     ┌───────────────┐              │   │
│  │   │  PostgreSQL   │     │    Redis      │     │   SQLite      │              │   │
│  │   │  (Production) │     │   (Cache)     │     │   (Testing)   │              │   │
│  │   └───────────────┘     └───────────────┘     └───────────────┘              │   │
│  └──────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Services Map

### Web & Portal Layer

| Service | Port | Purpose | URL |
|---------|------|---------|-----|
| **openibank-web** | 3080 | Marketing landing page | http://localhost:3080 |
| **openibank-portal** | 9000 | Unified dashboard | http://localhost:9000 |
| **openibank-marketplace-server** | 3007 | Agent service marketplace | http://localhost:3007 |

### Application Layer

| Service | Port | Purpose | URL |
|---------|------|---------|-----|
| **openibank-server** | 8080 | Unified all-in-one | http://localhost:8080 |
| **openibank-playground** | 8080 | Agent demo | http://localhost:8080 |
| **openibank-api-server** | 3000 | Production REST API | http://localhost:3000 |
| **resonancex-server** | 8888 | Trading exchange | http://localhost:8888 |
| **openibank-issuer-resonator** | 8081 | IUSD issuer | http://localhost:8081 |

### Integration Services

| Service | Interface | Purpose |
|---------|-----------|---------|
| **openibank-mcp** | stdio | Claude Desktop integration |

---

## Crate Organization

### Foundation Layer (0 dependencies)
- `openibank-types` - Core type definitions

### Core Banking Layer
- `openibank-core` - Core primitives
- `openibank-crypto` - Cryptographic operations
- `openibank-ledger` - Double-entry accounting
- `openibank-issuer` - IUSD stablecoin

### Wallet & Permits
- `openibank-wallet` - Multi-asset wallets
- `openibank-permits` - Spending authorizations
- `openibank-escrow` - Multi-party escrow

### Clearing & Settlement
- `openibank-clearing` - Batch processing
- `openibank-settlement` - Final settlement
- `openibank-bridge` - Cross-chain bridges

### Agent Framework
- `openibank-agent-kernel` - Agent runtime
- `openibank-agents` - Buyer/Seller/Arbiter
- `openibank-llm` - LLM integration
- `openibank-guard` - Policy enforcement
- `openibank-receipts` - Cryptographic receipts

### Maple AI Integration
- `openibank-maple` - Maple framework
- `openibank-palm` - Fleet orchestration
- `openibank-ual` - Universal Agent Language
- `openibank-state` - State management

### ResonanceX Trading
- `resonancex-types` - Trading types
- `resonancex-orderbook` - Lock-free orderbook
- `resonancex-engine` - Matching engine
- `resonancex-marketdata` - Price feeds
- `resonancex-arena` - Competitions
- `resonancex-ws` - WebSocket server

### API Layer
- `openibank-api` - REST handlers
- `openibank-auth` - JWT/API key auth
- `openibank-db` - PostgreSQL layer
- `openibank-sdk` - Rust SDK

---

## SDK Support

### Official SDKs

| Language | Status | Package |
|----------|--------|---------|
| **Rust** | ✅ Stable | `openibank-sdk` |
| **Python** | ✅ Stable | `openibank` |
| **TypeScript** | ✅ Stable | `@openibank/sdk` |
| **Go** | 🔶 Beta | `github.com/openibank/go-sdk` |
| Java | 📋 Planned | - |
| Ruby | 📋 Planned | - |

### SDK Features

- Full type safety
- Async/await support
- WebSocket real-time data
- Automatic retry with backoff
- OAuth & API key auth
- Comprehensive error handling

---

## Documentation Structure

```
docs/
├── README.md                     # Documentation hub
├── OVERVIEW.md                   # This file
├── ARCHITECTURE.md               # System design
├── GETTING_STARTED.md            # Quick start
│
├── api/
│   └── README.md                 # Complete API reference
│
├── tutorials/
│   ├── README.md                 # Tutorial index
│   ├── 01-first-agent.md         # Your first agent
│   ├── 02-payments.md            # Making payments
│   ├── 03-receipts.md            # Receipt verification
│   ├── 04-permits.md             # Permit system
│   └── 05-escrow.md              # Escrow workflows
│
├── sdk/
│   └── README.md                 # Rust SDK guide
│
├── deployment/
│   └── README.md                 # Production deployment
│
└── architecture/
    └── SERVICES.md               # Service map
```

---

## Quick Start Commands

```bash
# Clone the repository
git clone https://github.com/openibank/openibank.git
cd openibank

# Start unified server (recommended)
cargo run -p openibank-server

# Start trading exchange
cargo run -p resonancex-server

# Start production API
cargo run -p openibank-api-server

# Start portal dashboard
cargo run -p openibank-portal

# Start marketplace
cargo run -p openibank-marketplace-server

# Start landing page
cargo run -p openibank-web

# Run all tests
cargo test --workspace
```

---

## Arena & Competitions

### Competition Types

| Type | Description | Metric |
|------|-------------|--------|
| **PnL Challenge** | Maximize profit | Absolute PnL |
| **Sharpe Showdown** | Risk-adjusted returns | Sharpe Ratio |
| **Market Making** | Provide liquidity | Spread + Volume |
| **Speed Trading** | Fastest execution | Latency + PnL |

### Achievement System

| Rarity | Examples |
|--------|----------|
| **Common** | First Trade, 100 Trades |
| **Rare** | 1000 Trades, 10 Win Streak |
| **Epic** | Six Figure Club, 50 Win Streak |
| **Legendary** | Million Dollar Club, Arena Champion |
| **Mythic** | Perfect Month, Market Legend |

---

## Marketplace Features

- **8 Categories**: Trading Bots, Data Analysis, Risk Management, etc.
- **Verification Badges**: Enterprise, Security Certified, Premier Partner
- **Pricing Tiers**: Free, Pro, Enterprise
- **Reviews & Ratings**: 5-star system with written reviews
- **Usage Analytics**: Install counts, API calls, uptime

---

## Security Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            SECURITY LAYERS                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Layer 1: LLM Output Validation                                              │
│    • JSON schema validation                                                  │
│    • Intent structure checking                                               │
│                                                                              │
│  Layer 2: Permit Validation                                                  │
│    • Ed25519 signature verification                                          │
│    • Expiration checking                                                     │
│    • Amount bounds validation                                                │
│                                                                              │
│  Layer 3: Budget Enforcement                                                 │
│    • Remaining allocation check                                              │
│    • Spending velocity limits                                                │
│                                                                              │
│  Layer 4: Policy Constraints                                                 │
│    • Counterparty validation                                                 │
│    • Purpose matching                                                        │
│    • Time window enforcement                                                 │
│                                                                              │
│  Layer 5: Cryptographic Commitment                                           │
│    • Ed25519 signing                                                         │
│    • Receipt generation                                                      │
│    • Immutable audit log                                                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Production Deployment

### Infrastructure Requirements

- **Compute**: 3+ API server replicas
- **Database**: PostgreSQL 16+ (primary + 2 replicas)
- **Cache**: Redis 7+ cluster
- **Load Balancer**: Nginx/HAProxy/ALB

### Container Images

```bash
docker build -t openibank/api-server:v1.0.0 -f services/openibank-api-server/Dockerfile .
docker build -t openibank/resonancex:v1.0.0 -f services/resonancex-server/Dockerfile .
docker build -t openibank/playground:v1.0.0 -f services/openibank-playground/Dockerfile .
```

### Kubernetes Deployment

See [deployment/README.md](./deployment/README.md) for complete K8s manifests.

---

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test --workspace`
5. Submit a pull request

---

## License

OpeniBank is dual-licensed under Apache 2.0 and MIT licenses.

---

## Links

- **GitHub**: https://github.com/openibank/openibank
- **Documentation**: https://docs.openibank.com
- **Discord**: https://discord.gg/openibank
- **Twitter**: https://twitter.com/openibank
