# Getting Started with OpeniBank

OpeniBank is the **first banking system built exclusively for AI agents**. This guide will help you get up and running in minutes.

## Quick Start (2 minutes)

### Step 1: Clone both repos

OpeniBank requires the **maple** framework as a sibling directory:

```bash
# Create a project directory
mkdir ~/ClaudeProjects && cd ~/ClaudeProjects

# Clone both repos side by side
git clone https://github.com/mapleaiorg/maple.git
git clone https://github.com/openibank/openibank.git

# You should have:
#   ~/ClaudeProjects/
#   ├── maple/        # Maple AI Framework
#   └── openibank/    # OpeniBank
```

### Step 2: Build and run

```bash
cd openibank

# Start the unified server (builds everything)
cargo run --release -p openibank-server

# Open: http://localhost:8080
```

That's it! You're running an AI Agent Bank.

### Step 3: Try some UAL commands

Open http://localhost:8080 and type in the UAL console:

```
STATUS
FLEET STATUS
DEPLOY buyer COUNT 3
```

Or via curl:
```bash
curl -s http://localhost:8080/api/status | jq .
curl -X POST http://localhost:8080/api/ual -H "Content-Type: application/json" -d '{"command":"STATUS"}'
```

## All Services

OpeniBank provides multiple binaries. Here's how to run each:

| Service | Command | Port | Description |
|---------|---------|------|-------------|
| **openibank-server** | `cargo run -p openibank-server` | 8080 | All-in-one server (recommended) |
| **openibank-playground** | `cargo run -p openibank-playground` | 8080 | Interactive trading playground |
| **openibank-cli** | `cargo run -p openibank-cli -- demo full` | - | Command-line interface |
| **openibank-issuer-resonator** | `cargo run -p openibank-issuer-resonator` | 8081 | Standalone IUSD issuer |
| **openibank-mcp** | `cargo run -p openibank-mcp` | stdio | Claude Desktop MCP server |

### Running the Playground with the CLI

```bash
# Terminal 1: Start the playground
cargo run --release -p openibank-playground

# Terminal 2: Use the CLI against it
cargo run --release -p openibank-cli -- status
cargo run --release -p openibank-cli -- agents list
cargo run --release -p openibank-cli -- demo full
```

## What You'll See

The demo shows the complete agent commerce cycle:

1. **Agent Creation**: Buyer and seller AI agents with wallets and Maple Resonator identities
2. **IUSD Minting**: Stablecoin creation with cryptographic receipts
3. **AAS Registration**: Identity + capability grants via Authority & Accountability Service
4. **Coupling**: Buyer-Seller resonance coupling for the trade
5. **RCF Commitment**: Formal commitment submitted and adjudicated
6. **Invoice & Escrow**: Payment locked in escrow, protected by spend permits
7. **Delivery & Settlement**: Service delivered, escrow released
8. **Receipts**: Ed25519 cryptographic receipts for every step

## Installation via Script

```bash
# macOS / Linux
curl -sSL https://openibank.com/install.sh | bash
```

The install script will clone both `maple` and `openibank`, build all binaries, and add them to your PATH.

### Build from Source (Manual)

```bash
# Clone both repos side by side
git clone https://github.com/mapleaiorg/maple.git
git clone https://github.com/openibank/openibank.git
cd openibank

# Build everything
cargo build --release

# Add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

## CLI Commands

### Demo Commands

```bash
# Full agent commerce demo
openibank demo full

# Safety features demo (fail-closed behavior)
openibank demo safety

# Interactive multi-agent trading
openibank demo interactive
```

### Wallet Commands

```bash
# Create a wallet for an agent
openibank wallet create --name my-agent --funding 50000 --budget 25000

# Check wallet info
openibank wallet info --name my-agent

# List all wallets
openibank wallet list

# Transfer between wallets
openibank wallet transfer --from alice --to bob --amount 1000
```

### Issuer Commands

```bash
# Start the issuer service
openibank issuer start

# Initialize the issuer
openibank issuer init --reserve-cap 10000000

# Mint IUSD to an agent
openibank issuer mint --to agent_alice --amount 50000

# Check supply
openibank issuer supply
```

### Agent Commands

```bash
# Run a buyer agent
openibank agent buyer --name alice --funding 50000

# Run a seller agent
openibank agent seller --name datacorp --service "Data Analysis" --price 10000

# Run a marketplace simulation
openibank agent marketplace --buyers 5 --sellers 3
```

### Receipt Commands

```bash
# Verify a receipt
openibank receipt verify receipt.json

# Inspect receipt details
openibank receipt inspect receipt.json

# Compare two receipts
openibank receipt diff receipt1.json receipt2.json
```

## Web Playground

Start the interactive web playground:

```bash
# Start the playground server
cargo run --release -p openibank-playground

# Open in browser
open http://localhost:8080
```

The playground provides:
- **Live Agent Trading**: Watch agents trade in real-time with Maple Resonators
- **Maple Deep Integration**: AAS commitments, coupling visualization, attention budgets
- **UAL Command Console**: Execute banking commands from the browser
- **Interactive Controls**: Create agents, trigger trades, simulate marketplaces
- **Real-time Updates**: Server-sent events for all 27+ event types
- **Fleet Orchestration**: Deploy and monitor financial agent fleets

## Claude Desktop Integration (MCP)

OpeniBank integrates with Claude Desktop via MCP:

1. **Build the MCP server**:
```bash
cargo build --release -p openibank-mcp
```

2. **Configure Claude Desktop** (`~/.config/claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "openibank": {
      "command": "/path/to/target/release/openibank-mcp"
    }
  }
}
```

3. **Ask Claude** to interact with OpeniBank:
   - "Create a buyer agent named Alice with $500"
   - "Create a seller agent offering Data Analysis for $100"
   - "Execute a trade between Alice and the seller"

## LLM Integration

OpeniBank works with multiple LLM providers:

### Local LLMs (Ollama - Recommended for Privacy)

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model
ollama pull llama3.1:8b

# Run OpeniBank with Ollama
OPENIBANK_LLM_PROVIDER=ollama openibank demo full
```

### Cloud LLMs

```bash
# OpenAI
OPENAI_API_KEY=sk-xxx openibank demo full

# Anthropic Claude
ANTHROPIC_API_KEY=sk-ant-xxx openibank demo full
```

### Deterministic Mode

When no LLM is available, agents use deterministic decision-making:

```bash
OPENIBANK_LLM_PROVIDER=deterministic openibank demo full
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      AI Agents                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   Buyer     │  │   Seller    │  │   Arbiter   │      │
│  │   Agent     │  │   Agent     │  │   Agent     │      │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
└─────────┼────────────────┼────────────────┼─────────────┘
          │                │                │
┌─────────┴────────────────┴────────────────┴─────────────┐
│              Commitment Boundary                        │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Commitment Gate                    │    │
│  │   Intent → Validation → Signing → Receipt       │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
          │
┌─────────┴───────────────────────────────────────────────┐
│                     Core Banking                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Ledger     │  │   Issuer     │  │   Escrow     │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Key Concepts

### 1. Commitment Boundary
The line between **intent** (what an agent wants) and **commitment** (what is cryptographically bound). Agents can propose anything, but execution requires valid permits and signatures.

### 2. SpendPermit
A bounded, signed authorization to spend funds:
```rust
SpendPermit {
    max_amount: $100,
    counterparty: specific_seller,
    purpose: "Data Analysis service",
    expires_at: 1 hour from now,
}
```

### 3. Escrow
Conditional settlement with delivery verification:
```rust
Escrow {
    buyer: alice,
    seller: datacorp,
    amount: $100,
    arbiter: verification_agent,
    delivery_conditions: ["Service completed", "Quality verified"],
}
```

### 4. Verifiable Receipts
Every operation produces a cryptographically signed receipt that can be independently verified:
```bash
openibank receipt verify '{"receipt_id":"abc123","signature":"..."}'
# Output: ✓ Signature is VALID
```

## Examples

### Example 1: Simple Trade

```bash
# Terminal 1: Create buyer
openibank agent buyer --name alice --funding 50000

# Terminal 2: Create seller
openibank agent seller --name datacorp --service "Data Analysis" --price 10000

# Terminal 3: Execute trade
openibank demo interactive
```

### Example 2: Full Marketplace

```bash
openibank agent marketplace --buyers 10 --sellers 5 --llm ollama
```

### Example 3: Programmatic Usage

```rust
use openibank_agents::{BuyerAgent, SellerAgent, AgentBrain};
use openibank_core::{Amount, ResonatorId};

// Create agents
let buyer = BuyerAgent::with_brain(
    ResonatorId::from_string("buyer_1"),
    ledger.clone(),
    AgentBrain::deterministic(),
);

let seller = SellerAgent::new(
    ResonatorId::from_string("seller_1"),
    ledger.clone(),
);

// Execute trade
let offer = seller.get_offer("Data Analysis").unwrap();
if buyer.evaluate_offer(&offer).await {
    let invoice = seller.issue_invoice(buyer.id(), "Data Analysis").await?;
    buyer.accept_invoice(invoice)?;
    let (_, escrow) = buyer.pay_invoice(&invoice.invoice_id).await?;
    seller.deliver_service(&invoice.invoice_id, "Completed")?;
    let amount = buyer.confirm_delivery(&escrow.escrow_id)?;
    seller.receive_payment(amount)?;
}
```

## Troubleshooting

### "LLM not available"
- Install Ollama: `curl -fsSL https://ollama.com/install.sh | sh`
- Pull a model: `ollama pull llama3.1:8b`
- Or use deterministic mode: `OPENIBANK_LLM_PROVIDER=deterministic`

### "Issuer not initialized"
- Run: `openibank issuer init --reserve-cap 10000000`

### "Insufficient balance"
- Mint more funds: `openibank issuer mint --to agent_name --amount 100000`

## Next Steps

1. **Start the Server**: `cargo run --release -p openibank-server` (all-in-one)
2. **Explore the Playground**: `cargo run --release -p openibank-playground`
3. **Try UAL Commands**: `STATUS`, `DEPLOY buyer COUNT 3`, `FLEET STATUS`
4. **Try Claude Integration**: Set up the MCP server for Claude Desktop
5. **Build Custom Agents**: Check `crates/openibank-agents/`
6. **Deploy with Docker**: `docker compose up`
7. **Read the Architecture Docs**: See `docs/ARCHITECTURE.md`

## Community

- **GitHub**: https://github.com/openibank/openibank
- **Discord**: https://discord.gg/openibank
- **Twitter**: @openibank

---

**OpeniBank**: AI agents need banks too. This is how they'll pay each other.
