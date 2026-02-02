# OpeniBank

**Programmable Wallets + Receipts for AI Agents**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![GitHub](https://img.shields.io/badge/github-openibank/openibank-blue.svg)](https://github.com/openibank/openibank)

OpeniBank is an **AI-agent-only banking system** built on Resonance architecture. It provides the economic primitives that enable autonomous agents to safely pay each other, escrow funds, and build machine-verifiable trust.

> **OpeniBank is AI-agent-only by design.**
> No human UI. No human assumptions. Just agent economics.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/openibank/openibank.git
cd openibank

# Run the viral demo
cargo run --example asset_cycle
```

That's it. You'll see a complete asset lifecycle:

```
Mint → Budget → Permit → Escrow → Settlement → Receipt → Verification
```

## What You'll See

```
╔══════════════════════════════════════════════════════════════════════╗
║                                                                      ║
║     ██████╗ ██████╗ ███████╗███╗   ██╗██╗██████╗  █████╗ ███╗   ██╗██╗  ██╗║
║    ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██║██╔══██╗██╔══██╗████╗  ██║██║ ██╔╝║
...
║   ✓ Complete Asset Cycle Demonstrated Successfully!                  ║
║                                                                      ║
║   All transactions produced verifiable receipts.                     ║
║   All spending was bounded by budgets and permits.                   ║
║   No direct settlement without commitment.                           ║
╚══════════════════════════════════════════════════════════════════════╝
```

## Core Concepts

### 1. Resonator = Economic Actor

In OpeniBank, the only economic actor is a **Resonator**. AI agents act through Resonators. Wallets, budgets, and permits are bound to Resonator identity.

### 2. SpendPermit = Agent Currency

Agents don't trade raw money. They trade **SpendPermits** - signed, expiring, bounded authorizations that can be verified by third parties.

```
┌─────────────────────────────────────┐
│           SpendPermit               │
├─────────────────────────────────────┤
│ permit_id: permit_abc123            │
│ issuer: buyer_agent                 │
│ max_amount: $100.00                 │
│ remaining: $100.00                  │
│ counterparty: seller_agent          │
│ expires_at: 2024-12-31T23:59:59Z    │
│ signature: ed25519(...)             │
└─────────────────────────────────────┘
```

### 3. Commitment Gate = No Direct Settlement

Every economic action must pass through the **Commitment Gate**:

```
Intent → Commitment → Evidence → Policy → Receipt
```

No exceptions. This is how we achieve auditability and accountability.

### 4. Receipts = Trust Artifacts

Receipts are the social objects of OpeniBank:
- **Shareable** - agents can show them to other agents
- **Stable** - schema won't change incompatibly
- **Verifiable** - cryptographically signed

## Architecture

```
openibank/
├── crates/
│   ├── openibank-core/      # Wallet, permits, commitments
│   ├── openibank-ledger/    # Double-entry accounting
│   ├── openibank-issuer/    # Mock IUSD stablecoin
│   ├── openibank-llm/       # LLM provider abstraction
│   ├── openibank-agents/    # Reference agents
│   ├── openibank-guard/     # LLM output validator
│   └── openibank-receipts/  # Trust artifact tools
├── services/
│   └── openibank-issuer-resonator/  # HTTP issuer service
└── examples/
    └── asset_cycle.rs       # Viral demo
```

## LLM Support (Optional)

OpeniBank works without any LLM. But if you want agent brains:

```bash
# With local Ollama (no API key needed)
OPENIBANK_LLM_PROVIDER=ollama cargo run --example asset_cycle

# With OpenAI
OPENIBANK_LLM_PROVIDER=openai OPENAI_API_KEY=sk-... cargo run --example asset_cycle

# With Anthropic Claude
OPENIBANK_LLM_PROVIDER=anthropic ANTHROPIC_API_KEY=sk-... cargo run --example asset_cycle
```

See [docs/local-llm.md](docs/local-llm.md) for setup instructions.

**Key principle: LLMs may PROPOSE intents, NEVER EXECUTE money.**

## Reference Agents

### BuyerAgent
- Has a wallet with funds and budget
- Issues SpendPermits for purchases
- Pays via escrow

### SellerAgent
- Publishes service offers
- Issues invoices
- Delivers services with proof

### ArbiterAgent
- Receives delivery proofs
- Evaluates disputes
- Makes release/refund decisions

## Issuer Service

Run the IUSD issuer as an HTTP service:

```bash
cargo run -p openibank-issuer-resonator
```

Endpoints:
- `POST /v1/issuer/init` - Initialize issuer
- `POST /v1/issuer/mint` - Mint IUSD
- `POST /v1/issuer/burn` - Burn IUSD
- `GET /v1/issuer/supply` - Get supply info
- `GET /v1/issuer/receipts` - Get receipts

## Receipt CLI

Verify and inspect receipts:

```bash
# Verify a receipt
cargo run -p openibank-receipts -- verify receipt.json

# Inspect a receipt
cargo run -p openibank-receipts -- inspect receipt.json
```

## Architectural Invariants

These are non-negotiable:

1. **Resonator is the only economic actor**
2. **No direct settlement without commitment**
3. **SpendPermits are mandatory**
4. **All money-impacting actions emit verifiable receipts**
5. **Authority is always bounded**
6. **Fail closed**
7. **LLMs may propose, never execute**

## Why OpeniBank?

Traditional banking assumes humans. OpeniBank assumes agents:

| Human Banking | Agent Banking (OpeniBank) |
|--------------|---------------------------|
| Checking/savings | Budget envelopes |
| Cards/loans | SpendPermits |
| KYC/KYB | Capability attestations |
| Statements | Verifiable receipts |
| Trust in institutions | Trust in cryptography |

## Viral Growth

OpeniBank spreads because:

1. **Receipts become trust** - Other agents can verify your history
2. **Permits are portable** - Trade authorization, not money
3. **Escrow is default** - Safe transactions with strangers
4. **No human needed** - Fully autonomous operation

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache 2.0 - see [LICENSE](LICENSE)

## Links

- **Website**: https://www.openibank.com/
- **GitHub**: https://github.com/openibank/openibank/
- **Documentation**: https://docs.openibank.com/

---

**OpeniBank: Where AI agents bank.**
