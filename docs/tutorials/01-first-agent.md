# Tutorial 1: Your First AI Agent

> **Duration**: 15 minutes
> **Difficulty**: Beginner
> **Prerequisites**: Rust 1.75+, OpeniBank cloned and built

Welcome to OpeniBank! In this tutorial, you will create your first AI agent, understand how agent identity works through the ResonatorId system, fund your agent with IUSD stablecoin, and verify the balance. By the end, you will have a fully operational agent ready for transactions.

---

## Learning Objectives

By the end of this tutorial, you will be able to:

1. Start the OpeniBank development environment
2. Create an AI agent with a unique identity (ResonatorId)
3. Fund your agent using the IUSD issuer
4. Check agent balances and verify account state
5. Understand the relationship between agents, wallets, and the ledger

---

## Prerequisites

Before starting, ensure you have:

- **Rust 1.75+**: Install via [rustup](https://rustup.rs/)
- **OpeniBank repository**: Cloned and dependencies installed
- **Maple framework**: Cloned as a sibling directory (required)

```bash
# Verify Rust installation
rustc --version  # Should show 1.75.0 or higher

# Clone both required repositories
mkdir -p ~/ClaudeProjects && cd ~/ClaudeProjects
git clone https://github.com/mapleaiorg/maple.git
git clone https://github.com/openibank/openibank.git

# Directory structure should be:
# ~/ClaudeProjects/
# ├── maple/
# └── openibank/
```

---

## Step 1: Start the OpeniBank Server

First, start the unified OpeniBank server that provides all the functionality you need.

```bash
cd ~/ClaudeProjects/openibank

# Build and start the server (first run may take a few minutes)
cargo run --release -p openibank-server
```

You should see output similar to:

```
[INFO] OpeniBank Server starting...
[INFO] Maple Framework initialized
[INFO] IUSD Issuer ready (reserve cap: $1,000,000)
[INFO] Ledger initialized
[INFO] Listening on http://localhost:8080
```

Open your browser to **http://localhost:8080** to access the web dashboard.

---

## Step 2: Understanding Agent Identity (ResonatorId)

Before creating an agent, let's understand how identity works in OpeniBank.

### What is a ResonatorId?

A **ResonatorId** is a unique, cryptographically-derived identifier for every agent in the system. It serves multiple purposes:

- **Identity**: Uniquely identifies the agent across all transactions
- **Wallet Binding**: Links the agent to their financial accounts
- **Signature Authority**: Used for signing transactions and permits
- **Maple Integration**: Connects to the Maple AI Framework's Resonator system

```rust
// ResonatorId structure (from openibank-core)
pub struct ResonatorId {
    pub id: String,           // Unique identifier (e.g., "buyer-abc123")
    pub public_key: String,   // Ed25519 public key for verification
}

// ResonatorIds are derived from the agent's keypair
impl ResonatorId {
    pub fn from_string(s: &str) -> Self {
        // Creates a deterministic ID from the string
        Self {
            id: s.to_string(),
            public_key: derive_public_key(s),
        }
    }

    pub fn generate() -> Self {
        // Generates a new random ID with fresh keypair
        let keypair = Keypair::generate();
        Self {
            id: format!("agent-{}", generate_short_id()),
            public_key: keypair.public_key().to_string(),
        }
    }
}
```

### Identity in Practice

When you create an agent, OpeniBank:

1. Generates a unique ResonatorId
2. Creates an Ed25519 keypair (private key stays with agent)
3. Registers the agent in the Authority & Accountability Service (AAS)
4. Creates a wallet account in the ledger

---

## Step 3: Create Your First Agent

### Option A: Using the Web Dashboard

1. Open http://localhost:8080
2. Click the **"Create Buyer"** button
3. Enter a name for your agent (e.g., "my-first-agent")
4. Click **"Create"**

The dashboard will display your new agent with its ResonatorId.

### Option B: Using the REST API

```bash
# Create a buyer agent via API
curl -X POST http://localhost:8080/api/agents/buyer \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-first-agent"
  }'
```

**Response:**
```json
{
  "agent_id": "buyer-a7f3c2",
  "resonator_id": "buyer-a7f3c2",
  "name": "my-first-agent",
  "agent_type": "buyer",
  "balance": 0,
  "public_key": "ed25519:7KjQ9...",
  "created_at": "2025-02-08T10:30:00Z",
  "status": "active"
}
```

### Option C: Using the UAL Console

The Universal Agent Language (UAL) console provides a command-line interface within the web dashboard.

1. Open http://localhost:8080
2. Navigate to the **UAL Console** tab
3. Enter:

```
DEPLOY buyer NAME my-first-agent
```

**Response:**
```
✓ Agent deployed: buyer-a7f3c2
  Name: my-first-agent
  Type: buyer
  Balance: 0 IUSD
```

### Option D: Using Rust Code

```rust
use openibank_agents::{BuyerAgent, AgentBrain};
use openibank_core::ResonatorId;
use openibank_ledger::Ledger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the ledger
    let ledger = Ledger::new_in_memory();

    // Create the agent with deterministic brain (no LLM required)
    let agent = BuyerAgent::with_brain(
        ResonatorId::from_string("my-first-agent"),
        ledger.clone(),
        AgentBrain::deterministic(),
    );

    println!("Created agent: {}", agent.id());
    println!("Public key: {}", agent.public_key());

    Ok(())
}
```

---

## Step 4: Fund Your Agent

New agents start with zero balance. Let's fund your agent with IUSD (OpeniBank's stablecoin, pegged 1:1 to USD).

### Understanding IUSD

**IUSD** (Issuer USD) is OpeniBank's native stablecoin:

- **1 IUSD = 1 USD** (pegged stablecoin)
- Represented in **cents** internally (e.g., $100 = 10000 cents)
- Backed by cryptographic receipts
- Can only be minted by the authorized Issuer

### Funding via REST API

```bash
# Fund the agent with $500 (50000 cents)
curl -X POST http://localhost:8080/api/agents/buyer-a7f3c2/fund \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 50000
  }'
```

**Response:**
```json
{
  "success": true,
  "agent_id": "buyer-a7f3c2",
  "amount": 50000,
  "new_balance": 50000,
  "receipt": {
    "receipt_id": "rcpt-mint-b8c4d5",
    "operation": "MINT",
    "amount": 50000,
    "asset": "IUSD",
    "target": "buyer-a7f3c2",
    "issued_at": "2025-02-08T10:35:00Z",
    "signature": "ed25519:3nKp..."
  }
}
```

### Funding via UAL Console

```
FUND buyer-a7f3c2 AMOUNT 50000
```

**Response:**
```
✓ Funded buyer-a7f3c2
  Amount: 50000 cents ($500.00 IUSD)
  New Balance: 50000 cents ($500.00 IUSD)
  Receipt: rcpt-mint-b8c4d5
```

### Funding via Rust Code

```rust
use openibank_issuer::{Issuer, MintRequest};
use openibank_core::{Amount, AssetId};

async fn fund_agent(
    issuer: &Issuer,
    agent_id: &ResonatorId,
    amount_cents: u64,
) -> Result<IssuerReceipt, IssuerError> {
    // Create mint request
    let request = MintRequest {
        target: agent_id.clone(),
        amount: Amount::new(amount_cents),
        asset: AssetId::iusd(),
        reason: "Initial funding".to_string(),
    };

    // Execute mint (returns cryptographic receipt)
    let receipt = issuer.mint(request).await?;

    println!("Minted {} cents to {}", amount_cents, agent_id);
    println!("Receipt ID: {}", receipt.receipt_id);

    Ok(receipt)
}
```

---

## Step 5: Check Agent Balance

### Via REST API

```bash
# Get all ledger accounts
curl http://localhost:8080/api/ledger/accounts

# Get specific agent balance
curl http://localhost:8080/api/agents/buyer-a7f3c2
```

**Response:**
```json
{
  "agent_id": "buyer-a7f3c2",
  "name": "my-first-agent",
  "agent_type": "buyer",
  "balance": 50000,
  "balance_formatted": "$500.00",
  "currency": "IUSD",
  "status": "active",
  "created_at": "2025-02-08T10:30:00Z",
  "last_activity": "2025-02-08T10:35:00Z"
}
```

### Via UAL Console

```
STATUS buyer-a7f3c2
```

**Response:**
```
Agent: buyer-a7f3c2 (my-first-agent)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Type:     buyer
Status:   active
Balance:  50000 cents ($500.00 IUSD)
Created:  2025-02-08T10:30:00Z

Recent Activity:
  [10:35:00] MINT +50000 cents (rcpt-mint-b8c4d5)
```

### Via Rust Code

```rust
use openibank_ledger::Ledger;

async fn check_balance(
    ledger: &Ledger,
    agent_id: &ResonatorId,
) -> Result<Amount, LedgerError> {
    let account = ledger.get_account(agent_id).await?;

    println!("Agent: {}", agent_id);
    println!("Balance: {} cents (${:.2})",
        account.balance.cents(),
        account.balance.as_dollars()
    );

    Ok(account.balance)
}
```

---

## Complete Working Example

Here's a complete example that creates an agent, funds it, and checks the balance:

```rust
//! Complete First Agent Example
//!
//! Run with: cargo run --example first_agent

use openibank_agents::{BuyerAgent, AgentBrain};
use openibank_core::{Amount, ResonatorId};
use openibank_issuer::Issuer;
use openibank_ledger::Ledger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Initialize core components
    println!("=== OpeniBank: Your First Agent ===\n");

    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());

    // Step 2: Create the agent
    println!("Creating agent...");
    let agent_id = ResonatorId::from_string("tutorial-agent-001");

    let agent = BuyerAgent::with_brain(
        agent_id.clone(),
        ledger.clone(),
        AgentBrain::deterministic(),
    );

    println!("✓ Agent created: {}", agent.id());
    println!("  Public key: {}", agent.public_key());

    // Step 3: Fund the agent with $500
    println!("\nFunding agent with $500...");
    let mint_receipt = issuer.mint(
        agent_id.clone(),
        Amount::new(50_000), // 50000 cents = $500
    ).await?;

    println!("✓ Funding complete");
    println!("  Receipt ID: {}", mint_receipt.receipt_id);
    println!("  Signature: {}...", &mint_receipt.signature[..20]);

    // Step 4: Verify the balance
    println!("\nVerifying balance...");
    let account = ledger.get_account(&agent_id).await?;

    println!("✓ Balance verified");
    println!("  Balance: {} cents (${:.2})",
        account.balance.cents(),
        account.balance.as_dollars()
    );

    // Step 5: Verify the receipt cryptographically
    println!("\nVerifying receipt signature...");
    match mint_receipt.verify() {
        Ok(_) => println!("✓ Receipt signature is VALID"),
        Err(e) => println!("✗ Receipt verification failed: {}", e),
    }

    println!("\n=== Tutorial Complete ===");
    println!("Your agent is ready for transactions!");

    Ok(())
}
```

**Expected Output:**
```
=== OpeniBank: Your First Agent ===

Creating agent...
✓ Agent created: tutorial-agent-001
  Public key: ed25519:7KjQ9vBn...

Funding agent with $500...
✓ Funding complete
  Receipt ID: rcpt-mint-f3a8c1
  Signature: ed25519:3nKp8mWq...

Verifying balance...
✓ Balance verified
  Balance: 50000 cents ($500.00)

Verifying receipt signature...
✓ Receipt signature is VALID

=== Tutorial Complete ===
Your agent is ready for transactions!
```

---

## Understanding What Happened

Let's trace through what OpeniBank did when you created and funded your agent:

### 1. Agent Creation

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Creation                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  You: "Create agent"                                     │
│         │                                                │
│         ▼                                                │
│  ┌──────────────────┐                                   │
│  │ Generate Keypair │  Ed25519 keypair created          │
│  └────────┬─────────┘                                   │
│           │                                              │
│           ▼                                              │
│  ┌──────────────────┐                                   │
│  │ Create Resonator │  Unique ID assigned               │
│  │        ID        │  Public key registered            │
│  └────────┬─────────┘                                   │
│           │                                              │
│           ▼                                              │
│  ┌──────────────────┐                                   │
│  │  Register in AAS │  Authority & Accountability       │
│  │                  │  Service records the agent        │
│  └────────┬─────────┘                                   │
│           │                                              │
│           ▼                                              │
│  ┌──────────────────┐                                   │
│  │ Create Ledger    │  Empty account created            │
│  │    Account       │  Balance = 0                      │
│  └──────────────────┘                                   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 2. Funding Operation

```
┌─────────────────────────────────────────────────────────┐
│                    Funding (Mint)                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  You: "Fund $500"                                        │
│         │                                                │
│         ▼                                                │
│  ┌──────────────────┐                                   │
│  │ Validate Request │  Check amount within limits       │
│  │                  │  Check reserve capacity           │
│  └────────┬─────────┘                                   │
│           │                                              │
│           ▼                                              │
│  ┌──────────────────┐                                   │
│  │   Mint IUSD      │  Create new currency units        │
│  │                  │  Update total supply              │
│  └────────┬─────────┘                                   │
│           │                                              │
│           ▼                                              │
│  ┌──────────────────┐                                   │
│  │  Credit Ledger   │  Agent balance += 50000           │
│  │                  │  Double-entry accounting          │
│  └────────┬─────────┘                                   │
│           │                                              │
│           ▼                                              │
│  ┌──────────────────┐                                   │
│  │ Generate Receipt │  Sign with issuer's private key   │
│  │                  │  Include operation details        │
│  └──────────────────┘                                   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## Key Concepts Recap

| Concept | Description |
|---------|-------------|
| **ResonatorId** | Unique cryptographic identity for each agent |
| **Keypair** | Ed25519 public/private key pair for signing |
| **IUSD** | Stablecoin currency (1 IUSD = 1 USD) |
| **Ledger** | Double-entry accounting system for balances |
| **Receipt** | Cryptographic proof of every operation |
| **Issuer** | Authority that can mint/burn IUSD |

---

## Troubleshooting

### Server Won't Start

```bash
# Check if another process is using port 8080
lsof -i :8080

# Try a different port
cargo run -p openibank-server -- --port 8081
```

### Build Errors

```bash
# Ensure Maple is in the correct location
ls ~/ClaudeProjects/maple  # Should exist

# Clean and rebuild
cargo clean
cargo build --release
```

### Agent Creation Fails

```bash
# Check server logs for details
RUST_LOG=debug cargo run -p openibank-server

# Verify API is responding
curl http://localhost:8080/api/status
```

### Funding Fails with "Insufficient Reserve"

```bash
# Check issuer status
curl http://localhost:8080/api/issuer/supply

# The default reserve cap is $1,000,000
# Individual mints are limited to $10,000
```

---

## Next Steps

Congratulations! You have successfully created and funded your first AI agent. Here's what to explore next:

1. **[Making Payments](./02-payments.md)** - Learn how to transfer funds between agents using the permit system
2. **[Understanding Receipts](./03-receipts.md)** - Deep dive into cryptographic verification
3. **[Building with Permits](./04-permits.md)** - Create bounded spending permissions

---

## Quick Reference

### UAL Commands

| Command | Description |
|---------|-------------|
| `DEPLOY buyer NAME <name>` | Create a buyer agent |
| `DEPLOY seller NAME <name>` | Create a seller agent |
| `FUND <agent_id> AMOUNT <cents>` | Add funds to an agent |
| `STATUS <agent_id>` | Check agent status |
| `FLEET STATUS` | View all agents |

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/agents/buyer` | POST | Create buyer agent |
| `/api/agents/seller` | POST | Create seller agent |
| `/api/agents/{id}/fund` | POST | Fund an agent |
| `/api/agents/{id}` | GET | Get agent details |
| `/api/ledger/accounts` | GET | List all accounts |

---

**Next Tutorial**: [Making Payments](./02-payments.md) - Learn how to send money between agents safely using permits.
