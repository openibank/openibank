# Tutorial 5: Escrow Workflows

> **Duration**: 45 minutes
> **Difficulty**: Intermediate
> **Prerequisites**: Completed [Tutorial 2: Making Payments](./02-payments.md) and [Tutorial 4: Building with Permits](./04-permits.md)

In this comprehensive tutorial, you will master OpeniBank's escrow system for multi-party trades. Escrows provide conditional settlement with delivery verification, enabling safe commerce between AI agents that don't trust each other. You will learn the complete escrow lifecycle, arbiter integration, dispute resolution, and implement a full trade from start to finish.

---

## Learning Objectives

By the end of this tutorial, you will be able to:

1. Understand when and why to use escrows
2. Design multi-party trade flows with buyer, seller, and arbiter
3. Implement the complete escrow lifecycle (create, fund, deliver, release)
4. Handle disputes and resolutions
5. Build real-world escrow patterns for AI agent commerce

---

## Why Escrows?

Direct payments work when parties trust each other. But what happens when:

- A buyer pays for a service that's never delivered?
- A seller delivers but never gets paid?
- Both parties disagree about whether delivery was satisfactory?

**Escrows solve this** by introducing a trusted third party (the arbiter) and conditional fund release.

### The Trust Problem

```
Without Escrow:
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  Scenario 1: Buyer pays first                                       │
│  ┌─────────┐    $100    ┌─────────┐                                 │
│  │  Buyer  │ ─────────▶ │ Seller  │                                 │
│  └─────────┘            └─────────┘                                 │
│       │                      │                                       │
│       │                      └─▶ Seller disappears with money       │
│       └─▶ Buyer gets nothing                                        │
│                                                                      │
│  Scenario 2: Seller delivers first                                  │
│  ┌─────────┐   Service  ┌─────────┐                                 │
│  │ Seller  │ ─────────▶ │  Buyer  │                                 │
│  └─────────┘            └─────────┘                                 │
│       │                      │                                       │
│       │                      └─▶ Buyer claims it was bad, no pay    │
│       └─▶ Seller wasted effort                                      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### The Escrow Solution

```
With Escrow:
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  Step 1: Buyer funds escrow                                         │
│  ┌─────────┐    $100    ┌─────────┐                                 │
│  │  Buyer  │ ─────────▶ │ Escrow  │ (money locked)                  │
│  └─────────┘            └─────────┘                                 │
│                                                                      │
│  Step 2: Seller delivers knowing payment is secured                 │
│  ┌─────────┐   Service  ┌─────────┐                                 │
│  │ Seller  │ ─────────▶ │  Buyer  │                                 │
│  └─────────┘            └─────────┘                                 │
│                                                                      │
│  Step 3: Buyer confirms delivery                                    │
│  ┌─────────┐  Confirm   ┌─────────┐                                 │
│  │  Buyer  │ ─────────▶ │ Arbiter │ (verifies)                      │
│  └─────────┘            └─────────┘                                 │
│                                                                      │
│  Step 4: Escrow releases to seller                                  │
│  ┌─────────┐    $100    ┌─────────┐                                 │
│  │ Escrow  │ ─────────▶ │ Seller  │ (everyone happy)                │
│  └─────────┘            └─────────┘                                 │
│                                                                      │
│  Dispute? Arbiter decides who gets the money.                       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## The Escrow State Machine

Every escrow follows a well-defined state machine:

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Escrow State Machine                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│                         ┌──────────┐                                │
│                         │ Created  │                                │
│                         └────┬─────┘                                │
│                              │                                       │
│                      buyer funds                                     │
│                              │                                       │
│                              ▼                                       │
│                         ┌──────────┐                                │
│                         │  Funded  │                                │
│                         └────┬─────┘                                │
│                              │                                       │
│               seller starts delivery                                │
│                              │                                       │
│                              ▼                                       │
│                   ┌──────────────────┐                              │
│                   │ DeliveryPending  │                              │
│                   └────────┬─────────┘                              │
│                            │                                         │
│          ┌─────────────────┼─────────────────┐                      │
│          │                 │                 │                      │
│   buyer confirms    buyer disputes     timeout                      │
│          │                 │                 │                      │
│          ▼                 ▼                 ▼                      │
│  ┌───────────────┐  ┌──────────┐    ┌──────────┐                   │
│  │   Confirmed   │  │ Disputed │    │ Expired  │                   │
│  └───────┬───────┘  └────┬─────┘    └────┬─────┘                   │
│          │               │               │                          │
│    auto-release    arbiter rules    auto-refund                     │
│          │               │               │                          │
│          ▼               ▼               ▼                          │
│  ┌──────────┐    ┌──────────────┐  ┌──────────┐                    │
│  │ Released │    │   Resolved   │  │ Refunded │                    │
│  │ (seller) │    │ (winner gets)│  │ (buyer)  │                    │
│  └──────────┘    └──────────────┘  └──────────┘                    │
│                                                                      │
│  Terminal states: Released, Resolved, Refunded                      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### State Definitions

| State | Description |
|-------|-------------|
| **Created** | Escrow exists but has no funds |
| **Funded** | Buyer has deposited funds |
| **DeliveryPending** | Seller is working on delivery |
| **Confirmed** | Buyer confirmed satisfactory delivery |
| **Disputed** | Parties disagree, arbiter must decide |
| **Expired** | Deadline passed without resolution |
| **Released** | Funds sent to seller (success) |
| **Refunded** | Funds returned to buyer |
| **Resolved** | Arbiter decided the outcome |

---

## Part 1: Creating an Escrow

### Escrow Structure

```rust
pub struct Escrow {
    // Identity
    pub escrow_id: EscrowId,

    // Parties
    pub buyer: ResonatorId,
    pub seller: ResonatorId,
    pub arbiter: ResonatorId,

    // Financial
    pub amount: Amount,
    pub asset: AssetId,

    // Conditions
    pub delivery_conditions: Vec<DeliveryCondition>,
    pub deadline: DateTime<Utc>,

    // State
    pub status: EscrowStatus,
    pub created_at: DateTime<Utc>,
    pub funded_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,

    // Evidence
    pub delivery_proof: Option<DeliveryProof>,
    pub dispute_reason: Option<String>,
    pub arbiter_ruling: Option<ArbiterRuling>,
}

pub struct DeliveryCondition {
    pub condition_id: String,
    pub description: String,
    pub verification_method: VerificationMethod,
    pub required: bool,
}

pub enum VerificationMethod {
    BuyerConfirmation,      // Buyer says it's done
    ArbiterVerification,    // Arbiter checks
    AutomaticCheck(String), // Automated verification URL/method
    ProofOfDelivery(String),// Seller provides proof type
}
```

### Creating an Escrow via API

```bash
curl -X POST http://localhost:8080/api/escrow \
  -H "Content-Type: application/json" \
  -d '{
    "buyer": "buyer-alice",
    "seller": "seller-bob",
    "arbiter": "arbiter-charlie",
    "amount": 100000,
    "asset": "IUSD",
    "deadline": "2025-02-09T12:00:00Z",
    "delivery_conditions": [
      {
        "description": "Provide working API endpoint",
        "verification_method": "proof_of_delivery",
        "required": true
      },
      {
        "description": "Include documentation",
        "verification_method": "buyer_confirmation",
        "required": true
      },
      {
        "description": "Pass integration tests",
        "verification_method": "automatic_check",
        "check_url": "https://api.example.com/verify",
        "required": false
      }
    ]
  }'
```

**Response:**
```json
{
  "escrow_id": "escrow-x1y2z3",
  "buyer": "buyer-alice",
  "seller": "seller-bob",
  "arbiter": "arbiter-charlie",
  "amount": 100000,
  "asset": "IUSD",
  "status": "Created",
  "deadline": "2025-02-09T12:00:00Z",
  "created_at": "2025-02-08T10:00:00Z",
  "delivery_conditions": [
    {
      "condition_id": "cond-001",
      "description": "Provide working API endpoint",
      "verification_method": "proof_of_delivery",
      "required": true,
      "satisfied": false
    },
    {
      "condition_id": "cond-002",
      "description": "Include documentation",
      "verification_method": "buyer_confirmation",
      "required": true,
      "satisfied": false
    }
  ]
}
```

### Creating an Escrow in Rust

```rust
use openibank_escrow::{EscrowBuilder, DeliveryCondition, VerificationMethod};
use openibank_core::{Amount, ResonatorId, EscrowId};
use chrono::{Utc, Duration};

async fn create_service_escrow(
    escrow_service: &EscrowService,
    buyer: &ResonatorId,
    seller: &ResonatorId,
    arbiter: &ResonatorId,
    amount: Amount,
    service_description: &str,
) -> Result<Escrow, EscrowError> {

    let escrow = EscrowBuilder::new()
        .buyer(buyer.clone())
        .seller(seller.clone())
        .arbiter(arbiter.clone())
        .amount(amount)
        .asset("IUSD")
        .deadline(Utc::now() + Duration::hours(48))
        .add_condition(DeliveryCondition {
            condition_id: "main-delivery".into(),
            description: service_description.to_string(),
            verification_method: VerificationMethod::BuyerConfirmation,
            required: true,
        })
        .add_condition(DeliveryCondition {
            condition_id: "quality-check".into(),
            description: "Service meets quality standards".to_string(),
            verification_method: VerificationMethod::ArbiterVerification,
            required: true,
        })
        .build()?;

    let created = escrow_service.create(escrow).await?;

    println!("Created escrow: {}", created.escrow_id);
    println!("  Amount: ${:.2}", created.amount.as_dollars());
    println!("  Deadline: {}", created.deadline);
    println!("  Conditions: {}", created.delivery_conditions.len());

    Ok(created)
}
```

---

## Part 2: Funding the Escrow

Once created, the buyer must fund the escrow to lock in the payment.

### Funding via API

```bash
# Buyer funds the escrow
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/fund \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <buyer_token>" \
  -d '{
    "from_account": "buyer-alice"
  }'
```

**Response:**
```json
{
  "escrow_id": "escrow-x1y2z3",
  "status": "Funded",
  "funded_at": "2025-02-08T10:30:00Z",
  "receipt": {
    "receipt_id": "rcpt-escrow-fund-a1b2",
    "operation": "ESCROW_FUND",
    "amount": 100000,
    "from": "buyer-alice",
    "to": "escrow-x1y2z3",
    "signature": "ed25519:..."
  }
}
```

### Funding in Rust

```rust
async fn fund_escrow(
    escrow_service: &EscrowService,
    ledger: &Ledger,
    escrow_id: &EscrowId,
    buyer: &BuyerAgent,
) -> Result<EscrowReceipt, EscrowError> {

    // Verify escrow is in Created state
    let escrow = escrow_service.get(escrow_id).await?;
    if escrow.status != EscrowStatus::Created {
        return Err(EscrowError::InvalidState {
            current: escrow.status,
            expected: EscrowStatus::Created,
        });
    }

    // Verify buyer has sufficient balance
    let balance = ledger.balance(&buyer.id()).await?;
    if balance < escrow.amount {
        return Err(EscrowError::InsufficientFunds {
            required: escrow.amount,
            available: balance,
        });
    }

    // Create permit for escrow funding
    let permit = buyer.create_escrow_funding_permit(
        escrow_id,
        escrow.amount,
    )?;

    // Execute funding (atomic: debit buyer, credit escrow)
    let receipt = escrow_service.fund(escrow_id, &permit).await?;

    println!("Escrow funded!");
    println!("  Receipt: {}", receipt.receipt_id);
    println!("  New status: {:?}", receipt.to_status);

    Ok(receipt)
}
```

### What Happens During Funding

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Escrow Funding Process                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. VALIDATE                                                        │
│     ├─ Escrow status == Created?                                    │
│     ├─ Buyer has sufficient balance?                                │
│     ├─ Permit is valid?                                             │
│     └─ Amount matches escrow.amount?                                │
│                                                                      │
│  2. LOCK FUNDS (Atomic Transaction)                                 │
│     ├─ Debit buyer account: -$1,000                                 │
│     └─ Credit escrow holding account: +$1,000                       │
│                                                                      │
│  3. UPDATE STATE                                                    │
│     ├─ escrow.status = Funded                                       │
│     └─ escrow.funded_at = now()                                     │
│                                                                      │
│  4. GENERATE RECEIPT                                                │
│     ├─ Sign with escrow service key                                 │
│     └─ Include all transaction details                              │
│                                                                      │
│  5. NOTIFY PARTIES                                                  │
│     ├─ Seller: "Escrow funded, you may begin delivery"              │
│     └─ Arbiter: "Escrow active, may need your attention"            │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Part 3: Delivery and Confirmation

### Seller Delivers Service

```rust
async fn deliver_service(
    escrow_service: &EscrowService,
    escrow_id: &EscrowId,
    seller: &SellerAgent,
    delivery_proof: DeliveryProof,
) -> Result<EscrowReceipt, EscrowError> {

    // Mark delivery started
    let receipt = escrow_service.start_delivery(escrow_id, &seller.id()).await?;

    println!("Delivery started for escrow: {}", escrow_id);

    // ... seller performs the actual service ...

    // Submit delivery proof
    let proof_receipt = escrow_service.submit_proof(
        escrow_id,
        &seller.id(),
        delivery_proof,
    ).await?;

    println!("Delivery proof submitted");
    println!("  Proof type: {}", proof_receipt.proof_type);
    println!("  Status: {:?}", proof_receipt.to_status);

    Ok(proof_receipt)
}

// DeliveryProof structure
pub struct DeliveryProof {
    pub proof_type: String,           // "api_endpoint", "document", "code", etc.
    pub proof_data: String,           // The actual proof content
    pub proof_hash: String,           // Hash of proof for verification
    pub delivered_at: DateTime<Utc>,
    pub seller_signature: String,     // Seller signs the proof
}
```

### Delivery via API

```bash
# Seller submits delivery proof
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/deliver \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <seller_token>" \
  -d '{
    "proof_type": "api_endpoint",
    "proof_data": "https://api.seller-service.com/v1",
    "notes": "API is live and tested. Documentation at /docs"
  }'
```

### Buyer Confirms Delivery

```bash
# Buyer confirms satisfactory delivery
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/confirm \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <buyer_token>" \
  -d '{
    "satisfied": true,
    "feedback": "API works as expected, documentation is comprehensive"
  }'
```

**Response:**
```json
{
  "escrow_id": "escrow-x1y2z3",
  "status": "Released",
  "released_at": "2025-02-08T14:00:00Z",
  "released_to": "seller-bob",
  "release_receipt": {
    "receipt_id": "rcpt-escrow-release-c3d4",
    "operation": "ESCROW_RELEASE",
    "amount": 100000,
    "from": "escrow-x1y2z3",
    "to": "seller-bob",
    "signature": "ed25519:..."
  }
}
```

### Confirmation in Rust

```rust
async fn confirm_delivery(
    escrow_service: &EscrowService,
    escrow_id: &EscrowId,
    buyer: &BuyerAgent,
    feedback: Option<String>,
) -> Result<EscrowReceipt, EscrowError> {

    // Verify escrow state
    let escrow = escrow_service.get(escrow_id).await?;
    if escrow.status != EscrowStatus::DeliveryPending {
        return Err(EscrowError::InvalidState {
            current: escrow.status,
            expected: EscrowStatus::DeliveryPending,
        });
    }

    // Verify caller is the buyer
    if escrow.buyer != buyer.id() {
        return Err(EscrowError::Unauthorized {
            action: "confirm_delivery".into(),
            caller: buyer.id().clone(),
            required_role: "buyer".into(),
        });
    }

    // Confirm delivery
    let confirmation = DeliveryConfirmation {
        escrow_id: escrow_id.clone(),
        confirmed_by: buyer.id().clone(),
        satisfied: true,
        feedback,
        confirmed_at: Utc::now(),
        signature: buyer.sign_confirmation(escrow_id)?,
    };

    // This triggers automatic release to seller
    let receipt = escrow_service.confirm_delivery(confirmation).await?;

    println!("Delivery confirmed!");
    println!("  Funds released to: {}", escrow.seller);
    println!("  Amount: ${:.2}", escrow.amount.as_dollars());
    println!("  Receipt: {}", receipt.receipt_id);

    Ok(receipt)
}
```

---

## Part 4: Arbiter Integration

The arbiter is a neutral third party that:
- Verifies delivery when required
- Resolves disputes between buyer and seller
- Can be an AI agent or human

### Arbiter Agent Setup

```rust
use openibank_agents::{ArbiterAgent, AgentBrain};

async fn create_arbiter(
    ledger: Ledger,
    llm_router: LLMRouter,
) -> ArbiterAgent {
    ArbiterAgent::with_brain(
        ResonatorId::from_string("arbiter-main"),
        ledger,
        AgentBrain::new(llm_router), // LLM for reasoning
    )
}

impl ArbiterAgent {
    /// Evaluate if delivery conditions are met
    pub async fn evaluate_delivery(
        &self,
        escrow: &Escrow,
        proof: &DeliveryProof,
    ) -> Result<EvaluationResult, ArbiterError> {

        // Build evaluation context
        let context = EvaluationContext {
            escrow_id: escrow.escrow_id.clone(),
            conditions: escrow.delivery_conditions.clone(),
            proof: proof.clone(),
            buyer_claim: None,
            seller_claim: None,
        };

        // Use LLM to reason about delivery
        let reasoning = self.brain.evaluate_delivery(&context).await?;

        println!("Arbiter evaluation:");
        println!("  Reasoning: {}", reasoning.summary);
        println!("  Conditions met: {}/{}",
            reasoning.conditions_satisfied,
            escrow.delivery_conditions.len()
        );

        Ok(EvaluationResult {
            approved: reasoning.all_required_met,
            reasoning: reasoning.trace,
            conditions_status: reasoning.per_condition_status,
        })
    }
}
```

### Arbiter Verification via API

```bash
# Arbiter verifies delivery
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/arbiter/verify \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <arbiter_token>" \
  -d '{
    "approved": true,
    "reasoning": "API endpoint is functional, documentation is complete, integration tests pass",
    "conditions_verified": ["cond-001", "cond-002"]
  }'
```

---

## Part 5: Dispute Resolution

When buyer and seller disagree, the arbiter must decide.

### Raising a Dispute

```bash
# Buyer disputes delivery
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/dispute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <buyer_token>" \
  -d '{
    "reason": "API returns errors for 50% of requests",
    "evidence": [
      {
        "type": "error_log",
        "data": "Error logs showing 500 responses...",
        "timestamp": "2025-02-08T13:00:00Z"
      }
    ]
  }'
```

**Response:**
```json
{
  "escrow_id": "escrow-x1y2z3",
  "status": "Disputed",
  "disputed_at": "2025-02-08T15:00:00Z",
  "disputed_by": "buyer-alice",
  "dispute_reason": "API returns errors for 50% of requests",
  "arbiter_notified": true,
  "resolution_deadline": "2025-02-09T15:00:00Z"
}
```

### Dispute in Rust

```rust
async fn raise_dispute(
    escrow_service: &EscrowService,
    escrow_id: &EscrowId,
    disputing_party: &ResonatorId,
    reason: String,
    evidence: Vec<DisputeEvidence>,
) -> Result<EscrowReceipt, EscrowError> {

    let dispute = DisputeRequest {
        escrow_id: escrow_id.clone(),
        raised_by: disputing_party.clone(),
        reason,
        evidence,
        raised_at: Utc::now(),
    };

    let receipt = escrow_service.raise_dispute(dispute).await?;

    println!("Dispute raised on escrow: {}", escrow_id);
    println!("  By: {}", disputing_party);
    println!("  Status: {:?}", receipt.to_status);
    println!("  Arbiter will be notified");

    Ok(receipt)
}

pub struct DisputeEvidence {
    pub evidence_type: String,
    pub description: String,
    pub data: String,
    pub data_hash: String,
    pub timestamp: DateTime<Utc>,
}
```

### Seller Response to Dispute

```bash
# Seller responds to dispute
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/dispute/respond \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <seller_token>" \
  -d '{
    "response": "The errors were due to rate limiting, not bugs. Here are the correct usage instructions.",
    "counter_evidence": [
      {
        "type": "documentation",
        "data": "Rate limit documentation showing proper usage...",
        "timestamp": "2025-02-08T15:30:00Z"
      }
    ]
  }'
```

### Arbiter Resolution

```rust
async fn resolve_dispute(
    escrow_service: &EscrowService,
    arbiter: &ArbiterAgent,
    escrow_id: &EscrowId,
) -> Result<EscrowReceipt, EscrowError> {

    // Get full dispute context
    let escrow = escrow_service.get(escrow_id).await?;
    let dispute = escrow_service.get_dispute(escrow_id).await?;

    // Arbiter evaluates both sides
    let ruling = arbiter.resolve_dispute(
        &escrow,
        &dispute.buyer_claim,
        &dispute.seller_response,
    ).await?;

    println!("Arbiter ruling:");
    println!("  Winner: {:?}", ruling.winner);
    println!("  Reasoning: {}", ruling.reasoning);
    println!("  Amount to winner: ${:.2}", ruling.amount_to_winner.as_dollars());
    if let Some(refund) = ruling.amount_refunded {
        println!("  Amount refunded: ${:.2}", refund.as_dollars());
    }

    // Execute the ruling
    let receipt = escrow_service.execute_ruling(escrow_id, ruling).await?;

    Ok(receipt)
}

pub struct ArbiterRuling {
    pub escrow_id: EscrowId,
    pub arbiter: ResonatorId,
    pub winner: DisputeWinner,
    pub reasoning: String,
    pub reasoning_trace: ReasoningTrace,
    pub amount_to_winner: Amount,
    pub amount_refunded: Option<Amount>,  // Partial refund possible
    pub ruled_at: DateTime<Utc>,
    pub signature: String,
}

pub enum DisputeWinner {
    Buyer,           // Full refund to buyer
    Seller,          // Full release to seller
    Split(u8, u8),   // Percentage split (e.g., 70% seller, 30% buyer)
}
```

### Resolution via API

```bash
# Arbiter issues ruling
curl -X POST http://localhost:8080/api/escrow/escrow-x1y2z3/arbiter/resolve \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <arbiter_token>" \
  -d '{
    "winner": "seller",
    "reasoning": "After reviewing the evidence, the API functions correctly. The buyer did not follow the documented rate limits. Seller fulfilled their obligations.",
    "amount_to_seller": 100000,
    "amount_to_buyer": 0
  }'
```

---

## Part 6: Complete Trade Lifecycle

Here's a complete example implementing a full trade from start to finish:

```rust
//! Complete Escrow Trade Example
//!
//! Run with: cargo run --example escrow_trade

use openibank_agents::{BuyerAgent, SellerAgent, ArbiterAgent, AgentBrain};
use openibank_core::{Amount, ResonatorId};
use openibank_escrow::{EscrowBuilder, EscrowService, DeliveryCondition, DeliveryProof};
use openibank_issuer::Issuer;
use openibank_ledger::Ledger;
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpeniBank Escrow Trade Tutorial ===\n");

    // ========================================
    // Setup
    // ========================================
    println!("Setting up the system...\n");

    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());
    let escrow_service = EscrowService::new(ledger.clone());

    // Create agents
    let buyer_id = ResonatorId::from_string("buyer-alice");
    let seller_id = ResonatorId::from_string("seller-bob");
    let arbiter_id = ResonatorId::from_string("arbiter-charlie");

    let buyer = BuyerAgent::with_brain(
        buyer_id.clone(),
        ledger.clone(),
        AgentBrain::deterministic(),
    );

    let seller = SellerAgent::new(seller_id.clone(), ledger.clone());
    let arbiter = ArbiterAgent::with_brain(
        arbiter_id.clone(),
        ledger.clone(),
        AgentBrain::deterministic(),
    );

    // Fund buyer
    println!("Funding buyer with $1,000...");
    issuer.mint(buyer_id.clone(), Amount::from_dollars(1_000)).await?;
    println!("  Buyer balance: ${:.2}\n", ledger.balance(&buyer_id).await?.as_dollars());

    // ========================================
    // Step 1: Create Escrow
    // ========================================
    println!("Step 1: Creating Escrow\n");

    let escrow = EscrowBuilder::new()
        .buyer(buyer_id.clone())
        .seller(seller_id.clone())
        .arbiter(arbiter_id.clone())
        .amount(Amount::from_dollars(500))
        .asset("IUSD")
        .deadline(Utc::now() + Duration::hours(24))
        .add_condition(DeliveryCondition {
            condition_id: "api-delivery".into(),
            description: "Provide working data analysis API endpoint".to_string(),
            verification_method: VerificationMethod::BuyerConfirmation,
            required: true,
        })
        .add_condition(DeliveryCondition {
            condition_id: "documentation".into(),
            description: "Include API documentation".to_string(),
            verification_method: VerificationMethod::BuyerConfirmation,
            required: true,
        })
        .build()?;

    let created_escrow = escrow_service.create(escrow).await?;
    println!("  Escrow ID: {}", created_escrow.escrow_id);
    println!("  Amount: ${:.2}", created_escrow.amount.as_dollars());
    println!("  Status: {:?}", created_escrow.status);
    println!("  Deadline: {}", created_escrow.deadline);

    // ========================================
    // Step 2: Fund Escrow
    // ========================================
    println!("\nStep 2: Buyer Funds Escrow\n");

    let fund_receipt = escrow_service.fund(
        &created_escrow.escrow_id,
        &buyer_id,
    ).await?;

    println!("  Fund receipt: {}", fund_receipt.receipt_id);
    println!("  New status: {:?}", fund_receipt.to_status);
    println!("  Buyer balance after: ${:.2}", ledger.balance(&buyer_id).await?.as_dollars());

    // ========================================
    // Step 3: Seller Delivers
    // ========================================
    println!("\nStep 3: Seller Delivers Service\n");

    // Seller starts delivery
    let start_receipt = escrow_service.start_delivery(
        &created_escrow.escrow_id,
        &seller_id,
    ).await?;
    println!("  Delivery started: {:?}", start_receipt.to_status);

    // Simulate seller doing work
    println!("  [Seller performing data analysis service...]");
    println!("  [Creating API endpoint...]");
    println!("  [Writing documentation...]");

    // Seller submits proof
    let proof = DeliveryProof {
        proof_type: "api_endpoint".into(),
        proof_data: "https://api.seller-service.com/v1/analysis".into(),
        proof_hash: "sha256:abc123...".into(),
        delivered_at: Utc::now(),
        notes: "API is live. Documentation at /docs endpoint.".into(),
        seller_signature: seller.sign("delivery-proof")?,
    };

    let proof_receipt = escrow_service.submit_proof(
        &created_escrow.escrow_id,
        &seller_id,
        proof,
    ).await?;
    println!("  Delivery proof submitted: {}", proof_receipt.receipt_id);

    // ========================================
    // Step 4: Buyer Confirms
    // ========================================
    println!("\nStep 4: Buyer Confirms Delivery\n");

    // Buyer evaluates the delivery
    println!("  [Buyer testing API endpoint...]");
    println!("  [Buyer reviewing documentation...]");
    println!("  [Buyer: Looks good!]");

    let confirm_receipt = escrow_service.confirm_delivery(
        &created_escrow.escrow_id,
        &buyer_id,
        Some("Excellent service, API works perfectly!".into()),
    ).await?;

    println!("  Confirmation receipt: {}", confirm_receipt.receipt_id);
    println!("  New status: {:?}", confirm_receipt.to_status);

    // ========================================
    // Step 5: Automatic Release
    // ========================================
    println!("\nStep 5: Funds Released to Seller\n");

    // Check final balances
    let buyer_final = ledger.balance(&buyer_id).await?;
    let seller_final = ledger.balance(&seller_id).await?;

    println!("  Final balances:");
    println!("    Buyer:  ${:.2} (started with $1,000)", buyer_final.as_dollars());
    println!("    Seller: ${:.2} (received $500)", seller_final.as_dollars());

    // Get final escrow state
    let final_escrow = escrow_service.get(&created_escrow.escrow_id).await?;
    println!("\n  Escrow final state: {:?}", final_escrow.status);
    println!("  Released at: {:?}", final_escrow.resolved_at);

    // ========================================
    // Step 6: Verify All Receipts
    // ========================================
    println!("\nStep 6: Verifying Receipts\n");

    let receipts = escrow_service.get_receipts(&created_escrow.escrow_id).await?;
    println!("  Total receipts generated: {}", receipts.len());

    for receipt in &receipts {
        let verified = receipt.verify().is_ok();
        println!("    {} | {} | {}",
            receipt.receipt_id,
            receipt.state_transition,
            if verified { "VALID" } else { "INVALID" }
        );
    }

    // ========================================
    // Summary
    // ========================================
    println!("\n=== Trade Complete ===\n");
    println!("Summary:");
    println!("  Escrow ID:    {}", created_escrow.escrow_id);
    println!("  Trade Amount: ${:.2}", created_escrow.amount.as_dollars());
    println!("  Buyer:        {} (${:.2} remaining)", buyer_id, buyer_final.as_dollars());
    println!("  Seller:       {} (${:.2} earned)", seller_id, seller_final.as_dollars());
    println!("  Arbiter:      {} (no intervention needed)", arbiter_id);
    println!("  Status:       Successfully completed");
    println!("  Receipts:     {} cryptographic proofs generated", receipts.len());

    Ok(())
}
```

**Expected Output:**
```
=== OpeniBank Escrow Trade Tutorial ===

Setting up the system...

Funding buyer with $1,000...
  Buyer balance: $1000.00

Step 1: Creating Escrow

  Escrow ID: escrow-a1b2c3
  Amount: $500.00
  Status: Created
  Deadline: 2025-02-09T10:00:00Z

Step 2: Buyer Funds Escrow

  Fund receipt: rcpt-escrow-fund-d4e5
  New status: Funded
  Buyer balance after: $500.00

Step 3: Seller Delivers Service

  Delivery started: DeliveryPending
  [Seller performing data analysis service...]
  [Creating API endpoint...]
  [Writing documentation...]
  Delivery proof submitted: rcpt-escrow-proof-f6g7

Step 4: Buyer Confirms Delivery

  [Buyer testing API endpoint...]
  [Buyer reviewing documentation...]
  [Buyer: Looks good!]
  Confirmation receipt: rcpt-escrow-confirm-h8i9
  New status: Released

Step 5: Funds Released to Seller

  Final balances:
    Buyer:  $500.00 (started with $1,000)
    Seller: $500.00 (received $500)

  Escrow final state: Released
  Released at: Some(2025-02-08T10:30:00Z)

Step 6: Verifying Receipts

  Total receipts generated: 4
    rcpt-escrow-create-a1b2 | Created | VALID
    rcpt-escrow-fund-d4e5 | Funded | VALID
    rcpt-escrow-proof-f6g7 | DeliveryPending | VALID
    rcpt-escrow-confirm-h8i9 | Released | VALID

=== Trade Complete ===

Summary:
  Escrow ID:    escrow-a1b2c3
  Trade Amount: $500.00
  Buyer:        buyer-alice ($500.00 remaining)
  Seller:       seller-bob ($500.00 earned)
  Arbiter:      arbiter-charlie (no intervention needed)
  Status:       Successfully completed
  Receipts:     4 cryptographic proofs generated
```

---

## Real-World Escrow Patterns

### Pattern 1: Milestone-Based Escrow

For large projects with multiple deliverables:

```rust
/// Create milestone-based escrow for a project
async fn create_milestone_escrow(
    escrow_service: &EscrowService,
    project: &Project,
) -> Result<Vec<Escrow>, EscrowError> {
    let mut escrows = Vec::new();

    for milestone in &project.milestones {
        let escrow = EscrowBuilder::new()
            .buyer(project.buyer.clone())
            .seller(project.seller.clone())
            .arbiter(project.arbiter.clone())
            .amount(milestone.payment)
            .deadline(milestone.deadline)
            .add_condition(DeliveryCondition {
                condition_id: milestone.id.clone(),
                description: milestone.description.clone(),
                verification_method: VerificationMethod::ArbiterVerification,
                required: true,
            })
            .metadata("milestone_number", &milestone.number.to_string())
            .metadata("project_id", &project.id)
            .build()?;

        let created = escrow_service.create(escrow).await?;
        escrows.push(created);
    }

    println!("Created {} milestone escrows for project {}", escrows.len(), project.id);
    Ok(escrows)
}
```

### Pattern 2: Recurring Service Escrow

For subscription-like services:

```rust
/// Create recurring escrow that auto-renews
async fn create_recurring_escrow(
    escrow_service: &EscrowService,
    subscription: &Subscription,
) -> Result<RecurringEscrow, EscrowError> {
    let escrow = EscrowBuilder::new()
        .buyer(subscription.subscriber.clone())
        .seller(subscription.provider.clone())
        .arbiter(subscription.arbiter.clone())
        .amount(subscription.monthly_fee)
        .deadline(Utc::now() + Duration::days(30))
        .add_condition(DeliveryCondition {
            condition_id: "service-availability".into(),
            description: "Service available 99.9% of the time".into(),
            verification_method: VerificationMethod::AutomaticCheck(
                subscription.uptime_monitor_url.clone()
            ),
            required: true,
        })
        .auto_release_on_deadline(true) // Release if no dispute by deadline
        .auto_renew(true)
        .build()?;

    let created = escrow_service.create(escrow).await?;

    Ok(RecurringEscrow {
        escrow: created,
        next_renewal: Utc::now() + Duration::days(30),
        total_payments: 0,
    })
}
```

### Pattern 3: Multi-Party Escrow

For trades involving multiple sellers:

```rust
/// Create escrow with multiple sellers (e.g., marketplace)
async fn create_multi_seller_escrow(
    escrow_service: &EscrowService,
    order: &MarketplaceOrder,
) -> Result<MultiSellerEscrow, EscrowError> {
    let mut sub_escrows = Vec::new();

    for item in &order.items {
        let escrow = EscrowBuilder::new()
            .buyer(order.buyer.clone())
            .seller(item.seller.clone())
            .arbiter(order.marketplace_arbiter.clone())
            .amount(item.price)
            .deadline(order.delivery_deadline)
            .add_condition(DeliveryCondition {
                condition_id: format!("item-{}", item.id),
                description: format!("Deliver: {}", item.name),
                verification_method: VerificationMethod::BuyerConfirmation,
                required: true,
            })
            .metadata("order_id", &order.id)
            .metadata("item_id", &item.id)
            .build()?;

        let created = escrow_service.create(escrow).await?;
        sub_escrows.push(created);
    }

    Ok(MultiSellerEscrow {
        order_id: order.id.clone(),
        sub_escrows,
        total_amount: order.total(),
    })
}
```

---

## Key Concepts Recap

| Concept | Purpose |
|---------|---------|
| **Escrow** | Holds funds conditionally until delivery |
| **Arbiter** | Neutral party that verifies and resolves disputes |
| **DeliveryCondition** | Specific requirement for release |
| **DeliveryProof** | Evidence that conditions are met |
| **Dispute** | Formal disagreement requiring arbiter |
| **ArbiterRuling** | Final decision on disputed escrow |

---

## Troubleshooting

### "Escrow not found"

```rust
// Verify escrow ID format
println!("Looking for escrow: {}", escrow_id);
let all_escrows = escrow_service.list_all().await?;
for e in &all_escrows {
    println!("  Found: {}", e.escrow_id);
}
```

### "Invalid state transition"

```rust
// Check current state before operations
let escrow = escrow_service.get(&escrow_id).await?;
println!("Current status: {:?}", escrow.status);

// Valid transitions:
// Created -> Funded (by buyer)
// Funded -> DeliveryPending (by seller)
// DeliveryPending -> Confirmed/Disputed (by buyer)
// Disputed -> Resolved (by arbiter)
```

### "Unauthorized action"

```rust
// Verify caller is correct party
match action {
    "fund" => assert_eq!(caller, escrow.buyer),
    "deliver" => assert_eq!(caller, escrow.seller),
    "confirm" | "dispute" => assert_eq!(caller, escrow.buyer),
    "resolve" => assert_eq!(caller, escrow.arbiter),
    _ => {}
}
```

### "Deadline passed"

```rust
// Check deadline before actions
if Utc::now() > escrow.deadline {
    println!("Escrow deadline passed: {}", escrow.deadline);
    // Escrow may auto-refund or require manual resolution
}
```

---

## Next Steps

Congratulations! You have mastered OpeniBank's escrow system. You now have all the tools to build safe, multi-party AI agent commerce.

### Continue Learning

- **[Trading on ResonanceX](./06-trading.md)** - Place orders and manage positions
- **[Building Trading Bots](./07-trading-bots.md)** - Automated trading strategies
- **[Production Deployment](./10-deployment.md)** - Deploy to production

### Build Something

Try implementing:
- A marketplace with multiple sellers
- A freelance service platform
- An API subscription service
- A data trading platform

---

## Quick Reference

### Escrow Lifecycle

```
Create -> Fund -> Deliver -> Confirm -> Release
                    |
                    └-> Dispute -> Resolve
```

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/escrow` | POST | Create escrow |
| `/api/escrow/{id}` | GET | Get escrow details |
| `/api/escrow/{id}/fund` | POST | Fund escrow (buyer) |
| `/api/escrow/{id}/deliver` | POST | Submit delivery (seller) |
| `/api/escrow/{id}/confirm` | POST | Confirm delivery (buyer) |
| `/api/escrow/{id}/dispute` | POST | Raise dispute |
| `/api/escrow/{id}/arbiter/resolve` | POST | Arbiter ruling |

### CLI Commands

| Command | Description |
|---------|-------------|
| `openibank escrow create` | Create new escrow |
| `openibank escrow fund <id>` | Fund an escrow |
| `openibank escrow status <id>` | Check escrow status |
| `openibank escrow deliver <id>` | Submit delivery proof |
| `openibank escrow confirm <id>` | Confirm delivery |
| `openibank escrow dispute <id>` | Raise a dispute |

---

**Congratulations!** You have completed the OpeniBank core tutorial series. You now understand agents, payments, receipts, permits, and escrows - everything you need to build AI agent commerce applications.
