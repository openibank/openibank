# Getting Started with OpeniBank

OpeniBank is the **first banking system built exclusively for AI agents**. This guide will help you get up and running in minutes.

## Quick Start (30 seconds)

```bash
# Install OpeniBank CLI
curl -sSL https://openibank.com/install.sh | bash

# Run the viral demo
openibank demo full
```

That's it! You'll see AI agents trading with verifiable receipts.

## What You Just Saw

The demo shows the complete agent commerce cycle:

1. **Agent Creation**: Buyer and seller AI agents with wallets
2. **IUSD Minting**: Stablecoin creation with cryptographic receipts
3. **Service Discovery**: Seller publishes a service, buyer evaluates it
4. **Invoice & Escrow**: Payment locked in escrow, protected by spend permits
5. **Delivery Verification**: Service delivered, arbiter confirms
6. **Settlement**: Escrow releases, seller receives payment

Every step produces a **verifiable cryptographic receipt** that proves the transaction happened.

## Installation Options

### Option 1: Pre-built Binaries (Recommended)

```bash
# macOS / Linux
curl -sSL https://openibank.com/install.sh | bash

# Windows (PowerShell)
iwr -useb https://openibank.com/install.ps1 | iex
```

### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/openibank/openibank.git
cd openibank

# Build everything
cargo build --release

# Add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

### Option 3: Cargo Install (Coming Soon)

```bash
# Not yet published to crates.io
# For now, use Option 2 (build from source)
cargo install openibank-cli  # Coming soon!
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
cargo run -p openibank-playground

# Open in browser
open http://localhost:8080
```

The playground provides:
- **Live Agent Trading**: Watch agents trade in real-time
- **Visual Feedback**: See reasoning and decisions
- **Interactive Controls**: Create agents, trigger trades
- **Real-time Updates**: Server-sent events for live state

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

1. **Explore the Playground**: `cargo run -p openibank-playground`
2. **Try Claude Integration**: Set up the MCP server
3. **Build Custom Agents**: Check the examples in `crates/openibank-agents/examples/`
4. **Read the Architecture Docs**: See `docs/ARCHITECTURE.md`

## Community

- **GitHub**: https://github.com/openibank/openibank
- **Discord**: https://discord.gg/openibank
- **Twitter**: @openibank

---

**OpeniBank**: AI agents need banks too. This is how they'll pay each other.
