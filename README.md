# OpeniBank

**The Open AI Agent Banking Server**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![GitHub](https://img.shields.io/badge/github-openibank/openibank-blue.svg)](https://github.com/openibank/openibank)

OpeniBank is a **complete AI-agent banking infrastructure** you can run on your own server, Mac Mini, or Linux box. Powered by the [Maple AI Framework](https://github.com/mapleaiorg/maple) and its Resonance Architecture, it provides everything AI agents need to safely transact: wallets, escrow, stablecoin issuance, fleet orchestration, and verifiable cryptographic receipts.

> **AI agents need banks too. This is how they'll pay each other.**

## Quick Start

### Prerequisites

OpeniBank requires the **maple** framework as a sibling directory:

```bash
# Clone both repos side by side
git clone https://github.com/mapleaiorg/maple.git
git clone https://github.com/openibank/openibank.git

# Directory layout:
#   parent_dir/
#   ├── maple/        # Maple AI Framework
#   └── openibank/    # OpeniBank (this project)
```

### Build & Run

```bash
cd openibank

# Build everything
cargo build --release

# Start the unified server
cargo run --release -p openibank-server

# Open dashboard: http://localhost:8080
```

That's it. You're running an AI Agent Bank.

### 5-Minute Demo (Default Deterministic Mode)

```bash
# Terminal 1
cargo run -p openibank-server

# Terminal 2 (headless summary + receipt bundle export URL)
./scripts/demo.sh
```

Or open `http://localhost:8080` and click `RUN DEMO`.
The default demo does not require LLM credentials.

## Services

OpeniBank provides multiple services. Here's how to run each one:

### OpeniBank Server (Recommended - All-in-One)

The unified server combines everything into a single binary:

```bash
# Default (localhost:8080)
cargo run --release -p openibank-server

# Custom port
cargo run --release -p openibank-server -- --port 9090

# With LLM support
OPENIBANK_LLM_PROVIDER=ollama cargo run --release -p openibank-server
OPENIBANK_LLM_PROVIDER=anthropic ANTHROPIC_API_KEY=sk-... cargo run --release -p openibank-server
```

**What's included:**
- Maple iBank Runtime (8 invariants, Resonance Architecture)
- PALM Fleet Orchestration (deploy/scale/health/discover financial agents)
- UAL Command Console (SQL-like banking commands)
- AAS Accountability (identity, capabilities, commitments)
- IUSD Stablecoin Issuer
- REST API + Web Dashboard
- Multi-LLM support (Ollama, OpenAI, Anthropic, Gemini, Grok)

**Endpoints:**
| Endpoint | Description |
|----------|-------------|
| `GET /` | Web dashboard |
| `GET /api/status` | System status |
| `GET /api/health` | Health check |
| `GET /api/events` | SSE event stream |
| `POST /api/demo/run` | Run deterministic demo scenario (requires `{ "commit": true }`) |
| `GET /api/info` | System info and capabilities |
| `POST /api/ual` | Execute UAL commands |
| `GET /api/fleet/status` | Fleet orchestration status |
| `GET /api/fleet/specs` | Registered agent specs |
| `POST /api/fleet/deploy` | Deploy agent instances |
| `GET /api/agents` | List agents |
| `GET /api/ledger/accounts` | Live account balances |
| `GET /api/transactions` | Transaction history |
| `GET /api/receipts` | Receipt records |
| `GET /api/receipts/{id}` | Receipt by id |
| `POST /api/receipts/verify` | Verify receipt signature/integrity |
| `GET /api/receipts/export` | Export receipt bundle (`jsonl`) |
| `GET /api/issuer/supply` | IUSD supply info |

### Playground (Interactive Web UI)

Full-featured interactive playground with live trading:

```bash
cargo run --release -p openibank-playground
# Open: http://localhost:8080
```

**Features:**
- Create buyer/seller/arbiter agents
- Execute trades with escrow settlement
- Real-time SSE event streaming
- Maple resonator coupling visualization
- AAS commitment pipeline dashboard
- UAL command endpoint

### CLI

Command-line interface for all operations:

```bash
cargo run --release -p openibank-cli -- demo full
cargo run --release -p openibank-cli -- wallet create --name alice --funding 50000
cargo run --release -p openibank-cli -- agent marketplace --buyers 5 --sellers 3
```

### Issuer Resonator (HTTP Stablecoin Service)

Standalone IUSD issuer with HTTP API:

```bash
cargo run --release -p openibank-issuer-resonator
```

**Endpoints:**
- `POST /v1/issuer/init` - Initialize issuer
- `POST /v1/issuer/mint` - Mint IUSD
- `POST /v1/issuer/burn` - Burn IUSD
- `GET /v1/issuer/supply` - Supply info

### MCP Server (Claude Desktop Integration)

Integrate with Claude Desktop:

```bash
cargo build --release -p openibank-mcp
```

Add to `~/.config/claude/claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "openibank": {
      "command": "/path/to/target/release/openibank-mcp"
    }
  }
}
```

## UAL Commands

The Universal Agent Language (UAL) provides SQL-like commands for banking:

```sql
-- Check system status
STATUS

-- Check agent balance
BALANCE "buyer-001"

-- Deploy a fleet of agents
DEPLOY buyer COUNT 3

-- Transfer funds
TRANSFER 5000 IUSD FROM "buyer-001" TO "seller-001"

-- Mint stablecoin
MINT 10000 IUSD TO "reserve"

-- Standard UAL commitment (compiles to RCF)
COMMIT BY "agent-001"
    DOMAIN Finance
    OUTCOME "Purchase data analysis service for $100"
    SCOPE GLOBAL
    TAG trade TAG payment
    REVERSIBLE;

-- PALM fleet operations
CREATE DEPLOYMENT "buyer-agent" REPLICAS 5;
SCALE DEPLOYMENT "deploy-001" REPLICAS 10;
HEALTH CHECK "instance-001";
```

Send commands via API:
```bash
curl -X POST http://localhost:8080/api/ual \
  -H "Content-Type: application/json" \
  -d '{"command": "STATUS"}'
```

## Architecture

```
openibank/
├── crates/
│   ├── openibank-core/       # Wallet, permits, commitments, escrow
│   ├── openibank-ledger/     # Double-entry immutable accounting
│   ├── openibank-issuer/     # IUSD stablecoin with reserve management
│   ├── openibank-llm/        # Multi-provider LLM abstraction
│   ├── openibank-agents/     # Buyer, Seller, Arbiter agent implementations
│   ├── openibank-guard/      # LLM output validation and safety
│   ├── openibank-receipts/   # Ed25519 cryptographic receipt toolkit
│   ├── openibank-maple/      # Maple bridge (Resonators, AAS, Couplings)
│   ├── openibank-palm/       # PALM fleet orchestration integration
│   ├── openibank-ual/        # UAL banking command parser/compiler
│   ├── openibank-state/      # Shared system state and SSE events
│   └── openibank-cli/        # Command-line interface
├── services/
│   ├── openibank-server/     # Unified all-in-one server
│   ├── openibank-playground/ # Interactive web playground
│   ├── openibank-mcp/        # Claude Desktop MCP integration
│   └── openibank-issuer-resonator/ # Standalone IUSD issuer
├── Dockerfile                # Multi-stage Docker build
├── docker-compose.yml        # Docker Compose with optional Ollama
└── scripts/
    ├── demo.sh               # Headless deterministic demo runner
    └── install.sh            # One-line installer
```

### System Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  OpeniBank Server                                            │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Buyer Agent │  │ Seller Agent│  │  Arbiter Agent     │  │
│  │ (Resonator) │  │ (Resonator) │  │  (Resonator)       │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────────────┘  │
│         │    Coupling     │                │                  │
│  ═══════╪════════════════╪════════════════╪═════════════     │
│         │  COMMITMENT BOUNDARY            │                  │
│  ═══════╪════════════════╪════════════════╪═════════════     │
│         │                │                │                  │
│  ┌──────┴────────────────┴────────────────┴──────────────┐  │
│  │  AAS (Authority & Accountability Service)              │  │
│  │  Identity → Capability → Policy → Adjudication         │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────┐  │
│  │  Ledger  │ │  Issuer  │ │  Escrow  │ │ PALM Fleet    │  │
│  │  (D.E.)  │ │  (IUSD)  │ │          │ │ Orchestrator  │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  Maple Runtime (8 Invariants, Resonance Architecture)  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## LLM Support

OpeniBank works **without any LLM** (deterministic mode). Add LLM for intelligent agent reasoning:

```bash
# Local LLM with Ollama (recommended for privacy)
curl -fsSL https://ollama.com/install.sh | sh
ollama pull llama3.1:8b
OPENIBANK_LLM_PROVIDER=ollama cargo run -p openibank-server

# Anthropic Claude
OPENIBANK_LLM_PROVIDER=anthropic ANTHROPIC_API_KEY=sk-ant-... cargo run -p openibank-server

# OpenAI
OPENIBANK_LLM_PROVIDER=openai OPENAI_API_KEY=sk-... cargo run -p openibank-server

# Deterministic (no LLM)
cargo run -p openibank-server
```

**Key principle: LLMs may PROPOSE intents, NEVER EXECUTE money.**

## Docker

```bash
# Build and run
docker compose up

# With local LLM (Ollama)
docker compose --profile llm up
```

## Architectural Invariants

These are non-negotiable, enforced by the Maple runtime:

1. **Presence precedes meaning** - agents must be present before reasoning
2. **Meaning precedes intent** - understanding before action
3. **Intent precedes commitment** - proposal before binding
4. **Commitment precedes consequence** - signed before executed
5. **Coupling bounded by attention** - finite capacity, no runaway
6. **Safety overrides optimization** - always
7. **Human agency cannot be bypassed** - architectural guarantee
8. **Failure must be explicit** - fail closed, never silent

## Why OpeniBank?

| Human Banking | Agent Banking (OpeniBank) |
|--------------|---------------------------|
| Checking/savings | Budget envelopes with attention bounds |
| Credit cards | SpendPermits (signed, expiring, bounded) |
| KYC/KYB | Capability attestations via AAS |
| Statements | Verifiable cryptographic receipts |
| Branch network | PALM fleet orchestration |
| SQL queries | UAL (Universal Agent Language) |
| Trust in institutions | Trust in 8 architectural invariants |

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache 2.0 - see [LICENSE](LICENSE)

## Links

- **Website**: https://www.openibank.com/
- **GitHub**: https://github.com/openibank/openibank
- **Documentation**: https://docs.openibank.com/

---

**OpeniBank**: AI agents need banks too. This is how they'll pay each other.
