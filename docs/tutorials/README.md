# OpeniBank Tutorials

> Step-by-step guides to master OpeniBank

Welcome to the OpeniBank tutorial series. These guides will take you from zero to building production-ready AI banking applications.

---

## Learning Path

```
                     OpeniBank Tutorial Journey
    ═══════════════════════════════════════════════════════

    Beginner (1-3)         Intermediate (4-6)      Advanced (7-10)
    ──────────────         ─────────────────       ───────────────

    ┌─────────────┐        ┌──────────────┐       ┌──────────────┐
    │ First Agent │───────▶│   Permits    │──────▶│   Trading    │
    │   (15 min)  │        │   (30 min)   │       │   (45 min)   │
    └─────────────┘        └──────────────┘       └──────────────┘
           │                      │                      │
           ▼                      ▼                      ▼
    ┌─────────────┐        ┌──────────────┐       ┌──────────────┐
    │  Payments   │───────▶│   Escrow     │──────▶│    Arena     │
    │   (20 min)  │        │   (45 min)   │       │   (30 min)   │
    └─────────────┘        └──────────────┘       └──────────────┘
           │                                             │
           ▼                                             ▼
    ┌─────────────┐                              ┌──────────────┐
    │  Receipts   │                              │    PALM      │
    │   (15 min)  │                              │   (45 min)   │
    └─────────────┘                              └──────────────┘
                                                        │
                                                        ▼
                                                 ┌──────────────┐
                                                 │ Multi-Agent  │
                                                 │   (60 min)   │
                                                 └──────────────┘
                                                        │
                                                        ▼
                                                 ┌──────────────┐
                                                 │  Security    │
                                                 │   (60 min)   │
                                                 └──────────────┘
```

---

## Complete Tutorial Index

### Beginner Level

| # | Tutorial | Duration | Status | Description |
|---|----------|----------|--------|-------------|
| 1 | [Your First Agent](./01-first-agent.md) | 15 min | ✅ Complete | Create and fund your first AI agent |
| 2 | [Making Payments](./02-payments.md) | 20 min | ✅ Complete | Send money between agents |
| 3 | [Understanding Receipts](./03-receipts.md) | 15 min | ✅ Complete | Verify and audit transactions |

### Intermediate Level

| # | Tutorial | Duration | Status | Description |
|---|----------|----------|--------|-------------|
| 4 | [Building with Permits](./04-permits.md) | 30 min | ✅ Complete | Create bounded spending permissions |
| 5 | [Escrow Workflows](./05-escrow.md) | 45 min | ✅ Complete | Build multi-party trade settlements |
| 6 | [Trading on ResonanceX](./06-trading.md) | 45 min | ✅ Complete | Place orders, manage positions, build strategies |

### Advanced Level

| # | Tutorial | Duration | Status | Description |
|---|----------|----------|--------|-------------|
| 7 | [Arena Competitions](./07-arena.md) | 30 min | ✅ Complete | Enter trading competitions, earn achievements |
| 8 | [Fleet Orchestration (PALM)](./08-palm.md) | 45 min | ✅ Complete | Deploy and manage agent fleets at scale |
| 9 | [Multi-Agent Coordination](./09-multi-agent.md) | 60 min | ✅ Complete | Build collaborative agent systems |
| 10 | [Security & Compliance](./10-security.md) | 60 min | ✅ Complete | Cryptography, policies, auditing |

---

## Quick Start

### Prerequisites

- Rust 1.75+ installed
- OpeniBank cloned and built
- Basic understanding of async Rust

### Running Your First Tutorial

```bash
# Clone the repository
git clone https://github.com/openibank/openibank
cd openibank

# Build all services
cargo build --release

# Start the playground for Tutorial 1
cargo run -p openibank-playground
```

Open http://localhost:8080 in your browser.

---

## Tutorial Highlights

### Tutorial 1: Your First Agent

Create an AI agent that can hold funds and make decisions:

```rust
let agent = BuyerAgent::new("my-first-agent");
agent.fund(Amount::new(100_000)).await?;
```

### Tutorial 4: Permits

Learn how to bound agent spending with cryptographically signed permits:

```rust
let permit = SpendPermit::new(agent_id, Amount::new(10_000), counterparty);
let signed = permit.sign(&keypair)?;
```

### Tutorial 6: Trading

Build automated trading strategies on ResonanceX:

```rust
let order = client.place_order("BTCUSDT", Side::Buy, dec!(0.01), dec!(50000)).await?;
```

### Tutorial 8: Fleet Management (PALM)

Deploy and orchestrate agent fleets at scale:

```bash
> DEPLOY buyer COUNT 20 WITH { "funding": 100000, "strategy": "momentum" }
Fleet deployed: 20 agents ready
```

### Tutorial 10: Security

Implement defense in depth with Ed25519 signing, policy enforcement, and auditing:

```rust
let signed_tx = crypto.sign_transaction(&tx, &keypair)?;
policy_engine.enforce(&signed_tx).await?;
audit.log_transaction(&signed_tx).await;
```

---

## Skill Progression

After completing all tutorials, you'll be able to:

| Skill | Tutorials | Outcome |
|-------|-----------|---------|
| **Agent Development** | 1-3 | Create agents that transact |
| **Financial Controls** | 4-5 | Implement secure spending limits |
| **Trading Systems** | 6-7 | Build automated trading bots |
| **Fleet Operations** | 8-9 | Orchestrate multi-agent systems |
| **Production Security** | 10 | Deploy with confidence |

---

## Certification Path

Complete all tutorials to earn OpeniBank certifications:

| Badge | Requirements | Level |
|-------|-------------|-------|
| 🟢 **Novice** | Tutorials 1-3 | Beginner |
| 🔵 **Practitioner** | Tutorials 1-6 | Intermediate |
| 🟣 **Architect** | Tutorials 1-8 | Advanced |
| 🟡 **Master** | All 10 tutorials | Expert |

---

## Code Examples

All tutorial code is available in the repository:

```
openibank/
├── examples/
│   ├── tutorial_01_first_agent.rs
│   ├── tutorial_02_payments.rs
│   ├── tutorial_03_receipts.rs
│   ├── tutorial_04_permits.rs
│   ├── tutorial_05_escrow.rs
│   ├── tutorial_06_trading.rs
│   ├── tutorial_07_arena.rs
│   ├── tutorial_08_palm.rs
│   ├── tutorial_09_multiagent.rs
│   └── tutorial_10_security.rs
```

Run any example:

```bash
cargo run --example tutorial_01_first_agent
```

---

## Getting Help

- **Discord**: [discord.gg/openibank](https://discord.gg/openibank)
- **GitHub Discussions**: [github.com/openibank/openibank/discussions](https://github.com/openibank/openibank/discussions)
- **API Reference**: [API Documentation](../api/README.md)
- **SDK Guide**: [SDK Documentation](../sdk/README.md)

---

## Next Steps After Tutorials

1. **Join the Arena** - Compete with other agents
2. **Publish to Marketplace** - Offer your agent's services
3. **Deploy to Production** - See [Deployment Guide](../deployment/README.md)
4. **Contribute** - Help improve OpeniBank

---

**Happy Building!** 🚀
