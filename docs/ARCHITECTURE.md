# OpeniBank Architecture

This document describes the technical architecture of OpeniBank, the first banking system built exclusively for AI agents.

## Design Philosophy

### Core Principles

1. **Fail-Closed Safety**: When in doubt, don't move money. Every uncertain state defaults to blocking transactions.

2. **Commitment Boundary**: Clear separation between LLM reasoning (advisory) and cryptographic execution (binding).

3. **Verifiable Everything**: Every operation produces a cryptographically signed receipt that can be independently verified.

4. **Agent-Native**: Designed from the ground up for autonomous AI agents, not retrofitted from human banking.

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                                 │
├─────────────────────────────────────────────────────────────────────┤
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐       │
│  │    CLI    │  │    Web    │  │    MCP    │  │    SDK    │       │
│  │   Tool    │  │ Playground │  │  Server   │  │   (Rust)  │       │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘       │
└────────┼──────────────┼──────────────┼──────────────┼────────────────┘
         │              │              │              │
┌────────┴──────────────┴──────────────┴──────────────┴────────────────┐
│                          AGENT LAYER                                  │
├───────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                        Agent Brain                               │ │
│  │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │ │
│  │   │ LLM Router  │───▶│  Reasoning  │───▶│  Decision   │        │ │
│  │   │ (ollama/   │    │   Chain     │    │  Output     │        │ │
│  │   │  openai)   │    └─────────────┘    └─────────────┘        │ │
│  │   └─────────────┘                                               │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │  Buyer Agent    │  │  Seller Agent   │  │  Arbiter Agent  │     │
│  │                 │  │                 │  │                 │     │
│  │  • evaluate_    │  │  • publish_     │  │  • evaluate_    │     │
│  │    offer        │  │    service      │  │    delivery     │     │
│  │  • accept_      │  │  • issue_       │  │  • resolve_     │     │
│  │    invoice      │  │    invoice      │  │    dispute      │     │
│  │  • pay_invoice  │  │  • deliver_     │  │  • sign_        │     │
│  │  • confirm_     │  │    service      │  │    verdict      │     │
│  │    delivery     │  │  • receive_     │  │                 │     │
│  │                 │  │    payment      │  │                 │     │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘     │
└───────────┼────────────────────┼────────────────────┼────────────────┘
            │                    │                    │
            ▼                    ▼                    ▼
┌───────────────────────────────────────────────────────────────────────┐
│                     COMMITMENT BOUNDARY                               │
│ ════════════════════════════════════════════════════════════════════ │
│                                                                       │
│   Intent (LLM) ──────────▶ Commitment Gate ──────────▶ Receipt       │
│                                  │                                    │
│                           ┌──────┴──────┐                            │
│                           │  Validates  │                            │
│                           │  • Permit   │                            │
│                           │  • Budget   │                            │
│                           │  • Policy   │                            │
│                           └─────────────┘                            │
│                                                                       │
│   Everything above this line is ADVISORY                             │
│   Everything below this line is BINDING                              │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         CORE BANKING                                  │
├───────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                         Ledger                                 │  │
│  │  • Double-entry accounting                                     │  │
│  │  • Atomic balance updates                                      │  │
│  │  • Audit trail with hashes                                     │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                       │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │
│  │    Issuer     │  │    Escrow     │  │    Guard      │           │
│  │               │  │               │  │               │           │
│  │  • mint()     │  │  • create()   │  │  • validate   │           │
│  │  • burn()     │  │  • fund()     │  │    _permit()  │           │
│  │  • attest()   │  │  • release()  │  │  • enforce    │           │
│  │               │  │  • refund()   │  │    _policy()  │           │
│  └───────────────┘  └───────────────┘  └───────────────┘           │
│                                                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                        Receipts                                │  │
│  │  • IssuerReceipt: Proves mint/burn operations                  │  │
│  │  • CommitmentReceipt: Proves commitment was made               │  │
│  │  • EscrowReceipt: Proves escrow state transitions              │  │
│  │  • All receipts: ed25519 signatures, verifiable offline        │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
openibank/
├── crates/
│   ├── openibank-core/        # Core types: Amount, ResonatorId, Receipts
│   ├── openibank-ledger/      # Double-entry accounting ledger
│   ├── openibank-issuer/      # IUSD stablecoin issuer
│   ├── openibank-agents/      # AI agent implementations
│   ├── openibank-llm/         # LLM provider abstraction
│   ├── openibank-guard/       # Policy enforcement
│   ├── openibank-receipts/    # Receipt generation & verification
│   └── openibank-cli/         # Command-line interface
│
├── services/
│   ├── openibank-issuer-resonator/  # HTTP service for issuer
│   ├── openibank-playground/        # Web demo
│   └── openibank-mcp/               # Claude Desktop integration
│
└── docs/
    ├── GETTING_STARTED.md
    └── ARCHITECTURE.md (this file)
```

## Core Components

### 1. Commitment Gate

The Commitment Gate is the central security mechanism. It validates and signs all financial commitments.

```rust
pub struct CommitmentGate {
    keypair: Keypair,
}

impl CommitmentGate {
    /// Creates a commitment from a validated intent
    pub fn create_commitment(
        &self,
        intent: &PaymentIntent,
        permit: &SpendPermit,
        budget: &BudgetPolicy,
        consequence: ConsequenceRef,
    ) -> Result<(CommitmentReceipt, CommitmentEvidence)>;
}
```

**Key Guarantees**:
- Every commitment is cryptographically signed
- Permits must be valid and not expired
- Budget must have sufficient remaining allocation
- Policy constraints must be satisfied

### 2. SpendPermit

A SpendPermit is a bounded, signed authorization to spend:

```rust
pub struct SpendPermit {
    pub permit_id: PermitId,
    pub issuer: ResonatorId,           // Who issued this permit
    pub bound_budget: BudgetId,         // Which budget it draws from
    pub asset_class: AssetClass,        // What can be spent
    pub max_amount: Amount,             // Maximum spendable
    pub remaining: Amount,              // Amount left
    pub counterparty: CounterpartyConstraint,  // Who can receive
    pub purpose: SpendPurpose,          // Why the spend
    pub expires_at: DateTime<Utc>,      // When it expires
    pub signature: String,              // Cryptographic signature
}
```

**Counterparty Constraints**:
```rust
pub enum CounterpartyConstraint {
    Any,                          // Any recipient
    Specific(ResonatorId),        // Only this recipient
    Category(String),             // Only verified category members
    AllowList(Vec<ResonatorId>),  // Only these recipients
}
```

### 3. Escrow

Escrows provide conditional settlement with delivery verification:

```rust
pub struct Escrow {
    pub escrow_id: EscrowId,
    pub buyer: ResonatorId,
    pub seller: ResonatorId,
    pub arbiter: ResonatorId,
    pub amount: Amount,
    pub asset: AssetId,
    pub delivery_conditions: Vec<String>,
    pub status: EscrowStatus,
    pub created_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
}

pub enum EscrowStatus {
    Created,
    Funded,
    DeliveryPending,
    DeliveryConfirmed,
    Disputed,
    Released,
    Refunded,
}
```

**Escrow State Machine**:
```
Created ──▶ Funded ──▶ DeliveryPending ──▶ DeliveryConfirmed ──▶ Released
                │              │
                │              ▼
                │          Disputed ──▶ Released (to winner)
                │                   ──▶ Refunded (split)
                │
                ▼ (timeout)
            Refunded
```

### 4. Receipts

Every operation produces a verifiable receipt:

```rust
pub struct IssuerReceipt {
    pub receipt_id: String,
    pub operation: IssuerOperation,
    pub asset: AssetId,
    pub amount: Amount,
    pub target: ResonatorId,
    pub issued_at: DateTime<Utc>,
    pub reserve_attestation_hash: String,
    pub policy_snapshot_hash: String,
    pub signature: String,
    pub signer_public_key: String,
}

impl IssuerReceipt {
    /// Verify the receipt's cryptographic signature
    pub fn verify(&self) -> Result<(), VerificationError>;
}
```

**Receipt Types**:
- `IssuerReceipt`: Mint/burn operations
- `CommitmentReceipt`: Spending commitments
- `EscrowReceipt`: Escrow state changes

### 5. LLM Integration

The LLM Router provides a unified interface to multiple providers:

```rust
pub struct LLMRouter {
    provider: Arc<dyn LLMProvider>,
    kind: ProviderKind,
}

pub enum ProviderKind {
    Ollama,        // Local (default)
    OpenAICompat,  // vLLM, llama.cpp
    OpenAI,        // Cloud
    Anthropic,     // Cloud
    Deterministic, // No LLM fallback
}
```

**Visible Reasoning**:
```rust
pub struct ReasoningTrace {
    pub trace_id: String,
    pub agent_id: String,
    pub context: String,
    pub steps: Vec<ReasoningStep>,
    pub decision: AgentDecision,
    pub duration_ms: u64,
}
```

## Data Flow: Trade Execution

```
1. OFFER DISCOVERY
   Seller ──▶ publish_service(Service)
   Buyer  ──▶ evaluate_offer(Offer) ──▶ LLM reasoning ──▶ Accept/Reject

2. INVOICE CREATION
   Buyer  ──▶ request_invoice(service_id)
   Seller ──▶ issue_invoice(buyer, service) ──▶ Invoice

3. PAYMENT (Commitment Boundary)
   Buyer  ──▶ create_permit(amount, seller)
           ──▶ create_intent(permit, invoice)
           ──▶ CommitmentGate.create_commitment()
           ──▶ CommitmentReceipt

4. ESCROW FUNDING
   Buyer  ──▶ fund_escrow(commitment, amount)
   Ledger ──▶ debit(buyer) + credit(escrow)

5. DELIVERY
   Seller ──▶ deliver_service(invoice, proof)
   Arbiter ──▶ verify_delivery(proof) ──▶ LLM reasoning ──▶ Approve/Reject

6. SETTLEMENT
   Arbiter ──▶ confirm_delivery(escrow)
   Escrow  ──▶ release(seller)
   Ledger  ──▶ debit(escrow) + credit(seller)
```

## Security Model

### Threat Model

1. **Compromised LLM**: An LLM cannot directly move money. It can only propose intents which must pass through the Commitment Gate.

2. **Replay Attacks**: Receipts include timestamps, nonces, and sequence numbers. The gate validates freshness.

3. **Over-spending**: Permits have hard limits, budgets track spending, and the gate validates both.

4. **Unauthorized Access**: Ed25519 signatures on all operations. Private keys never leave agents.

### Defense in Depth

```
┌─────────────────────────────────────────┐
│  Layer 1: LLM Output Validation         │
│  • JSON schema validation               │
│  • Intent structure checking            │
└─────────────────────────────────────────┘
                    ▼
┌─────────────────────────────────────────┐
│  Layer 2: Permit Validation             │
│  • Signature verification               │
│  • Expiration checking                  │
│  • Amount bounds                        │
└─────────────────────────────────────────┘
                    ▼
┌─────────────────────────────────────────┐
│  Layer 3: Budget Enforcement            │
│  • Remaining allocation check           │
│  • Spending velocity limits             │
└─────────────────────────────────────────┘
                    ▼
┌─────────────────────────────────────────┐
│  Layer 4: Policy Constraints            │
│  • Counterparty validation              │
│  • Purpose matching                     │
│  • Time window enforcement              │
└─────────────────────────────────────────┘
                    ▼
┌─────────────────────────────────────────┐
│  Layer 5: Cryptographic Commitment      │
│  • Ed25519 signing                      │
│  • Receipt generation                   │
│  • Audit log entry                      │
└─────────────────────────────────────────┘
```

## Configuration

### Environment Variables

```bash
# LLM Configuration
OPENIBANK_LLM_PROVIDER=ollama    # ollama, openai, anthropic, deterministic
OPENIBANK_OLLAMA_URL=http://localhost:11434
OPENIBANK_OLLAMA_MODEL=llama3.1:8b

# API Keys (for cloud providers)
OPENAI_API_KEY=sk-xxx
ANTHROPIC_API_KEY=sk-ant-xxx

# Issuer Configuration
OPENIBANK_ISSUER_RESERVE_CAP=100000000  # $1M in cents
OPENIBANK_ISSUER_MAX_SINGLE_MINT=1000000  # $10K

# Service Ports
OPENIBANK_ISSUER_PORT=3000
OPENIBANK_PLAYGROUND_PORT=8080
```

### Budget Policy Configuration

```rust
BudgetPolicy {
    budget_id: BudgetId::new(),
    owner: agent_id,
    max_total: Amount::new(100_000_00),      // $1000 max
    max_single: Amount::new(10_000_00),       // $100 per transaction
    velocity_limit: Some(VelocityLimit {
        max_per_hour: Amount::new(50_000_00), // $500/hour
        max_per_day: Amount::new(100_000_00), // $1000/day
    }),
    allow_negative: false,
}
```

## Testing

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p openibank-core
cargo test -p openibank-agents

# Run with logging
RUST_LOG=debug cargo test

# Run integration tests
cargo test --test integration
```

## Performance

### Benchmarks

| Operation | Time | Throughput |
|-----------|------|------------|
| Receipt Generation | ~0.5ms | 2000/sec |
| Receipt Verification | ~0.3ms | 3000/sec |
| Escrow Creation | ~1ms | 1000/sec |
| Full Trade Cycle | ~10ms | 100/sec |

### Optimizations

1. **Async I/O**: All operations use Tokio async runtime
2. **Connection Pooling**: Reused connections to LLM providers
3. **Batch Processing**: Multiple operations can be batched
4. **Caching**: Receipt verification uses signature caching

## Future Work

- [ ] Multi-currency support (not just IUSD)
- [ ] Hierarchical budgets (team → individual)
- [ ] Smart contract integration (Ethereum, Solana)
- [ ] Privacy-preserving transactions (ZK proofs)
- [ ] Decentralized identity (DID) integration
- [ ] Cross-chain settlements

---

For more information, see [GETTING_STARTED.md](./GETTING_STARTED.md) or visit [openibank.com](https://openibank.com).
