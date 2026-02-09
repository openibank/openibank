# Tutorial 2: Making Payments

> **Duration**: 20 minutes
> **Difficulty**: Beginner
> **Prerequisites**: Completed [Tutorial 1: Your First Agent](./01-first-agent.md)

In this tutorial, you will learn how to safely transfer funds between AI agents using OpeniBank's permit-based payment system. You will understand why OpeniBank requires permits for spending, create payment intents, execute transactions through the Commitment Gate, and verify payment receipts.

---

## Learning Objectives

By the end of this tutorial, you will be able to:

1. Understand the Commitment Boundary and why it exists
2. Create SpendPermits with appropriate constraints
3. Build PaymentIntents that reference permits
4. Execute payments through the Commitment Gate
5. Verify payment receipts and handle errors

---

## Why Permits? Understanding the Commitment Boundary

Before making your first payment, it's crucial to understand OpeniBank's core security model.

### The Problem with Traditional AI Payments

If an AI agent could directly move money with a simple API call, several risks emerge:

- **Compromised LLM**: A manipulated language model could drain accounts
- **Prompt Injection**: Malicious inputs could trick agents into unauthorized transfers
- **Unbounded Spending**: No limits on transaction amounts or recipients

### OpeniBank's Solution: The Commitment Boundary

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ADVISORY ZONE                                │
│   (LLM reasoning, intents, proposals - nothing is binding here)     │
│                                                                      │
│   Agent Brain ──▶ "I want to pay $100 to seller-xyz"                │
│                         │                                            │
│                         ▼                                            │
│                  PaymentIntent                                       │
│                  (just a proposal)                                   │
│                                                                      │
├══════════════════════════════════════════════════════════════════════┤
│                     COMMITMENT BOUNDARY                              │
│               (cryptographic validation required)                    │
│                                                                      │
│                  ┌─────────────────┐                                │
│                  │ Commitment Gate │                                │
│                  │                 │                                │
│                  │ ✓ Valid permit? │                                │
│                  │ ✓ Within budget?│                                │
│                  │ ✓ Right target? │                                │
│                  │ ✓ Not expired?  │                                │
│                  └────────┬────────┘                                │
│                           │                                          │
├═══════════════════════════╧══════════════════════════════════════════┤
│                         BINDING ZONE                                 │
│   (cryptographically signed, immutable, auditable)                  │
│                                                                      │
│                  CommitmentReceipt                                   │
│                  (proof of payment)                                  │
│                         │                                            │
│                         ▼                                            │
│                  Ledger Updated                                      │
│                  (money moved)                                       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Key Insight**: An LLM can propose any payment it wants, but the Commitment Gate will only execute payments that have valid, cryptographically-signed permits.

---

## Step 1: Set Up Two Agents

First, let's create a buyer and seller agent for our payment scenario.

### Using the REST API

```bash
# Create buyer agent
curl -X POST http://localhost:8080/api/agents/buyer \
  -H "Content-Type: application/json" \
  -d '{"name": "alice"}'

# Create seller agent
curl -X POST http://localhost:8080/api/agents/seller \
  -H "Content-Type: application/json" \
  -d '{"name": "data-provider", "service": "Data Analysis", "price": 10000}'

# Fund the buyer with $500
curl -X POST http://localhost:8080/api/agents/buyer-alice/fund \
  -H "Content-Type: application/json" \
  -d '{"amount": 50000}'
```

### Using UAL Console

```
DEPLOY buyer NAME alice
DEPLOY seller NAME data-provider SERVICE "Data Analysis" PRICE 10000
FUND buyer-alice AMOUNT 50000
```

### Using Rust

```rust
use openibank_agents::{BuyerAgent, SellerAgent, AgentBrain};
use openibank_core::{Amount, ResonatorId};
use openibank_ledger::Ledger;
use openibank_issuer::Issuer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());

    // Create buyer
    let buyer_id = ResonatorId::from_string("buyer-alice");
    let buyer = BuyerAgent::with_brain(
        buyer_id.clone(),
        ledger.clone(),
        AgentBrain::deterministic(),
    );

    // Create seller
    let seller_id = ResonatorId::from_string("seller-data-provider");
    let seller = SellerAgent::new(seller_id.clone(), ledger.clone());
    seller.register_service("Data Analysis", Amount::new(10_000)).await?;

    // Fund buyer
    issuer.mint(buyer_id.clone(), Amount::new(50_000)).await?;

    println!("Setup complete!");
    println!("Buyer balance: ${:.2}", ledger.balance(&buyer_id).await?.as_dollars());
    println!("Seller balance: ${:.2}", ledger.balance(&seller_id).await?.as_dollars());

    Ok(())
}
```

---

## Step 2: Create a SpendPermit

A **SpendPermit** is a bounded, signed authorization to spend funds. Think of it as a pre-approved check with specific limits.

### Permit Components

```rust
pub struct SpendPermit {
    // Identity
    pub permit_id: PermitId,           // Unique identifier
    pub issuer: ResonatorId,           // Who created this permit (buyer)

    // Budget binding
    pub bound_budget: BudgetId,        // Which budget this draws from

    // Spending limits
    pub asset_class: AssetClass,       // What currency (IUSD)
    pub max_amount: Amount,            // Maximum spendable
    pub remaining: Amount,             // How much is left

    // Constraints
    pub counterparty: CounterpartyConstraint,  // Who can receive
    pub purpose: SpendPurpose,         // Why the spend

    // Validity
    pub expires_at: DateTime<Utc>,     // When it becomes invalid
    pub signature: String,             // Cryptographic proof
}
```

### Creating a Permit via API

```bash
curl -X POST http://localhost:8080/api/permits \
  -H "Content-Type: application/json" \
  -d '{
    "issuer": "buyer-alice",
    "max_amount": 15000,
    "counterparty": {
      "type": "specific",
      "target": "seller-data-provider"
    },
    "purpose": "Payment for Data Analysis service",
    "expires_in_hours": 24
  }'
```

**Response:**
```json
{
  "permit_id": "permit-7f8a9b",
  "issuer": "buyer-alice",
  "bound_budget": "budget-alice-default",
  "max_amount": 15000,
  "remaining": 15000,
  "counterparty": {
    "type": "specific",
    "target": "seller-data-provider"
  },
  "purpose": "Payment for Data Analysis service",
  "expires_at": "2025-02-09T10:00:00Z",
  "signature": "ed25519:4mNp..."
}
```

### Creating a Permit in Rust

```rust
use openibank_guard::{SpendPermit, CounterpartyConstraint, SpendPurpose};
use openibank_core::{Amount, PermitId, BudgetId};
use chrono::{Utc, Duration};

fn create_payment_permit(
    buyer_id: &ResonatorId,
    seller_id: &ResonatorId,
    budget_id: &BudgetId,
    max_amount: Amount,
    keypair: &Keypair,
) -> SpendPermit {
    let permit = SpendPermit {
        permit_id: PermitId::generate(),
        issuer: buyer_id.clone(),
        bound_budget: budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount,
        remaining: max_amount,
        counterparty: CounterpartyConstraint::Specific(seller_id.clone()),
        purpose: SpendPurpose::ServicePayment("Data Analysis".into()),
        expires_at: Utc::now() + Duration::hours(24),
        signature: String::new(), // Will be filled by sign()
    };

    // Sign the permit with buyer's private key
    permit.sign(keypair)
}

// Usage
let permit = create_payment_permit(
    &buyer_id,
    &seller_id,
    &budget.budget_id,
    Amount::new(15_000), // $150 max
    &buyer_keypair,
);

println!("Created permit: {}", permit.permit_id);
println!("Max amount: ${:.2}", permit.max_amount.as_dollars());
println!("Expires: {}", permit.expires_at);
```

### Counterparty Constraint Types

```rust
pub enum CounterpartyConstraint {
    // Payment can go to anyone
    Any,

    // Payment can only go to this specific recipient
    Specific(ResonatorId),

    // Payment can go to any verified member of this category
    Category(String), // e.g., "verified-merchants"

    // Payment can go to any of these recipients
    AllowList(Vec<ResonatorId>),
}
```

---

## Step 3: Create a Payment Intent

A **PaymentIntent** is a proposal to spend money. It references a permit and specifies the exact payment details.

### Intent Structure

```rust
pub struct PaymentIntent {
    pub intent_id: String,
    pub permit_id: PermitId,      // Which permit authorizes this
    pub sender: ResonatorId,      // Who is paying
    pub recipient: ResonatorId,   // Who is receiving
    pub amount: Amount,           // How much (must be <= permit.remaining)
    pub memo: String,             // Description
    pub created_at: DateTime<Utc>,
}
```

### Creating an Intent via API

```bash
curl -X POST http://localhost:8080/api/payments/intent \
  -H "Content-Type: application/json" \
  -d '{
    "permit_id": "permit-7f8a9b",
    "recipient": "seller-data-provider",
    "amount": 10000,
    "memo": "Payment for Data Analysis - January"
  }'
```

**Response:**
```json
{
  "intent_id": "intent-c3d4e5",
  "permit_id": "permit-7f8a9b",
  "sender": "buyer-alice",
  "recipient": "seller-data-provider",
  "amount": 10000,
  "memo": "Payment for Data Analysis - January",
  "status": "pending",
  "created_at": "2025-02-08T11:00:00Z"
}
```

### Creating an Intent in Rust

```rust
use openibank_core::PaymentIntent;

let intent = PaymentIntent {
    intent_id: generate_intent_id(),
    permit_id: permit.permit_id.clone(),
    sender: buyer_id.clone(),
    recipient: seller_id.clone(),
    amount: Amount::new(10_000), // $100
    memo: "Payment for Data Analysis - January".to_string(),
    created_at: Utc::now(),
};

println!("Created intent: {}", intent.intent_id);
println!("Amount: ${:.2}", intent.amount.as_dollars());
```

---

## Step 4: Execute Through Commitment Gate

The **Commitment Gate** is where intents become reality. It validates everything and produces a cryptographic receipt.

### What the Gate Validates

```
┌────────────────────────────────────────────────────────────────────┐
│                    Commitment Gate Validation                       │
├────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. PERMIT VALIDATION                                               │
│     ├─ Is signature valid?                                         │
│     ├─ Is permit not expired?                                      │
│     ├─ Is permit not revoked?                                      │
│     └─ Does issuer match sender?                                   │
│                                                                     │
│  2. BUDGET VALIDATION                                               │
│     ├─ Does bound budget exist?                                    │
│     ├─ Is budget owned by sender?                                  │
│     └─ Is there remaining allocation?                              │
│                                                                     │
│  3. AMOUNT VALIDATION                                               │
│     ├─ Is amount <= permit.remaining?                              │
│     ├─ Is amount <= budget.max_single?                             │
│     └─ Is amount > 0?                                              │
│                                                                     │
│  4. COUNTERPARTY VALIDATION                                         │
│     ├─ Does recipient match constraint?                            │
│     └─ Is recipient a valid ResonatorId?                           │
│                                                                     │
│  5. BALANCE VALIDATION                                              │
│     └─ Does sender have sufficient balance?                        │
│                                                                     │
│  If ALL checks pass → Generate signed receipt, update ledger       │
│  If ANY check fails → Reject with specific error code              │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

### Executing via API

```bash
curl -X POST http://localhost:8080/api/payments/execute \
  -H "Content-Type: application/json" \
  -d '{
    "intent_id": "intent-c3d4e5"
  }'
```

**Success Response:**
```json
{
  "success": true,
  "receipt": {
    "receipt_id": "rcpt-pay-e5f6g7",
    "receipt_type": "COMMITMENT",
    "operation": "TRANSFER",
    "from": "buyer-alice",
    "to": "seller-data-provider",
    "amount": 10000,
    "asset": "IUSD",
    "memo": "Payment for Data Analysis - January",
    "permit_id": "permit-7f8a9b",
    "timestamp": "2025-02-08T11:01:00Z",
    "signature": "ed25519:8qRs...",
    "signer_public_key": "ed25519:7KjQ..."
  },
  "new_balances": {
    "buyer-alice": 40000,
    "seller-data-provider": 10000
  }
}
```

### Executing in Rust

```rust
use openibank_guard::{CommitmentGate, ConsequenceRef};

async fn execute_payment(
    gate: &CommitmentGate,
    intent: &PaymentIntent,
    permit: &SpendPermit,
    budget: &BudgetPolicy,
) -> Result<CommitmentReceipt, PaymentError> {
    // The gate validates everything automatically
    let (receipt, evidence) = gate.create_commitment(
        intent,
        permit,
        budget,
        ConsequenceRef::DirectPayment,
    ).await?;

    println!("Payment executed!");
    println!("Receipt ID: {}", receipt.receipt_id);
    println!("Amount: ${:.2}", receipt.amount.as_dollars());
    println!("Signature: {}...", &receipt.signature[..20]);

    Ok(receipt)
}
```

### Complete Payment Flow in Rust

```rust
use openibank_agents::{BuyerAgent, SellerAgent};
use openibank_guard::{CommitmentGate, SpendPermit, BudgetPolicy};
use openibank_core::{Amount, PaymentIntent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup (from Step 1)
    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());
    let gate = CommitmentGate::new();

    let buyer_id = ResonatorId::from_string("buyer-alice");
    let seller_id = ResonatorId::from_string("seller-data-provider");

    // Fund buyer
    issuer.mint(buyer_id.clone(), Amount::new(50_000)).await?;

    // Create budget for buyer
    let budget = BudgetPolicy {
        budget_id: BudgetId::generate(),
        owner: buyer_id.clone(),
        max_total: Amount::new(50_000),
        max_single: Amount::new(15_000),
        velocity_limit: None,
        allow_negative: false,
    };

    // Create permit (Step 2)
    let permit = SpendPermit::new(
        buyer_id.clone(),
        Amount::new(15_000),
        CounterpartyConstraint::Specific(seller_id.clone()),
        "Data Analysis payment",
    ).sign(&buyer_keypair);

    // Create intent (Step 3)
    let intent = PaymentIntent {
        intent_id: generate_intent_id(),
        permit_id: permit.permit_id.clone(),
        sender: buyer_id.clone(),
        recipient: seller_id.clone(),
        amount: Amount::new(10_000),
        memo: "Data Analysis - January".to_string(),
        created_at: Utc::now(),
    };

    // Execute payment (Step 4)
    let (receipt, _) = gate.create_commitment(
        &intent,
        &permit,
        &budget,
        ConsequenceRef::DirectPayment,
    ).await?;

    // Verify balances
    println!("\n=== Payment Complete ===");
    println!("Receipt: {}", receipt.receipt_id);
    println!("Buyer balance: ${:.2}", ledger.balance(&buyer_id).await?.as_dollars());
    println!("Seller balance: ${:.2}", ledger.balance(&seller_id).await?.as_dollars());

    Ok(())
}
```

---

## Step 5: Verify the Receipt

Every payment produces a cryptographically signed receipt that can be independently verified.

### Receipt Verification via API

```bash
curl -X POST http://localhost:8080/api/receipts/verify \
  -H "Content-Type: application/json" \
  -d '{
    "receipt_id": "rcpt-pay-e5f6g7"
  }'
```

**Response:**
```json
{
  "valid": true,
  "receipt_id": "rcpt-pay-e5f6g7",
  "verification_details": {
    "signature_valid": true,
    "timestamp_valid": true,
    "amount_matches": true,
    "parties_match": true
  }
}
```

### Receipt Verification in Rust

```rust
use openibank_receipts::verify_receipt;

fn verify_payment_receipt(receipt: &CommitmentReceipt) -> Result<(), VerificationError> {
    // Verify the cryptographic signature
    receipt.verify()?;

    println!("Receipt verification:");
    println!("  ID: {}", receipt.receipt_id);
    println!("  Signature: VALID");
    println!("  Amount: {} cents", receipt.amount.cents());
    println!("  Timestamp: {}", receipt.timestamp);

    Ok(())
}
```

---

## Error Handling

Payments can fail for various reasons. Here's how to handle common errors:

### Common Error Types

```rust
pub enum PaymentError {
    // Permit errors
    PermitNotFound(PermitId),
    PermitExpired(PermitId),
    PermitInsufficientRemaining { permit_id: PermitId, requested: Amount, remaining: Amount },

    // Budget errors
    BudgetNotFound(BudgetId),
    BudgetExceeded { budget_id: BudgetId, requested: Amount, available: Amount },
    VelocityLimitExceeded { limit_type: String, current: Amount, max: Amount },

    // Counterparty errors
    CounterpartyMismatch { expected: CounterpartyConstraint, actual: ResonatorId },

    // Balance errors
    InsufficientBalance { account: ResonatorId, requested: Amount, available: Amount },

    // Signature errors
    InvalidSignature(String),
}
```

### Error Handling Example

```rust
match gate.create_commitment(&intent, &permit, &budget, consequence).await {
    Ok((receipt, evidence)) => {
        println!("Payment successful: {}", receipt.receipt_id);
    }

    Err(PaymentError::PermitExpired(id)) => {
        println!("Error: Permit {} has expired. Create a new permit.", id);
    }

    Err(PaymentError::PermitInsufficientRemaining { permit_id, requested, remaining }) => {
        println!("Error: Permit {} only has ${:.2} remaining, but ${:.2} requested",
            permit_id, remaining.as_dollars(), requested.as_dollars());
        println!("Solution: Create a new permit with higher limit");
    }

    Err(PaymentError::CounterpartyMismatch { expected, actual }) => {
        println!("Error: Permit only allows payments to {:?}, not {}", expected, actual);
        println!("Solution: Create a permit for the correct recipient");
    }

    Err(PaymentError::InsufficientBalance { account, requested, available }) => {
        println!("Error: {} only has ${:.2}, but ${:.2} needed",
            account, available.as_dollars(), requested.as_dollars());
        println!("Solution: Fund the account before payment");
    }

    Err(e) => {
        println!("Payment failed: {:?}", e);
    }
}
```

### API Error Responses

```json
// Permit expired
{
  "success": false,
  "error": {
    "code": "PERMIT_EXPIRED",
    "message": "Permit permit-7f8a9b expired at 2025-02-07T10:00:00Z",
    "details": {
      "permit_id": "permit-7f8a9b",
      "expired_at": "2025-02-07T10:00:00Z"
    }
  }
}

// Insufficient balance
{
  "success": false,
  "error": {
    "code": "INSUFFICIENT_BALANCE",
    "message": "Account buyer-alice has insufficient balance",
    "details": {
      "account": "buyer-alice",
      "requested": 10000,
      "available": 5000
    }
  }
}
```

---

## Complete Working Example

Here's a full end-to-end payment example:

```rust
//! Complete Payment Example
//!
//! Run with: cargo run --example payment

use openibank_agents::{BuyerAgent, SellerAgent, AgentBrain};
use openibank_core::{Amount, ResonatorId, PaymentIntent};
use openibank_guard::{CommitmentGate, SpendPermit, BudgetPolicy, CounterpartyConstraint};
use openibank_issuer::Issuer;
use openibank_ledger::Ledger;
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpeniBank Payment Tutorial ===\n");

    // Initialize components
    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());
    let gate = CommitmentGate::new();

    // Create agents
    let buyer_id = ResonatorId::from_string("buyer-alice");
    let seller_id = ResonatorId::from_string("seller-bob");

    println!("Step 1: Creating agents...");
    let buyer = BuyerAgent::with_brain(
        buyer_id.clone(),
        ledger.clone(),
        AgentBrain::deterministic(),
    );
    let seller = SellerAgent::new(seller_id.clone(), ledger.clone());

    // Fund buyer
    println!("Step 2: Funding buyer with $500...");
    let mint_receipt = issuer.mint(buyer_id.clone(), Amount::new(50_000)).await?;
    println!("  Mint receipt: {}", mint_receipt.receipt_id);

    // Check initial balances
    println!("\nInitial balances:");
    println!("  Buyer:  ${:.2}", ledger.balance(&buyer_id).await?.as_dollars());
    println!("  Seller: ${:.2}", ledger.balance(&seller_id).await?.as_dollars());

    // Create budget
    println!("\nStep 3: Creating budget policy...");
    let budget = BudgetPolicy {
        budget_id: BudgetId::generate(),
        owner: buyer_id.clone(),
        max_total: Amount::new(50_000),
        max_single: Amount::new(20_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::new(30_000),
            max_per_day: Amount::new(50_000),
        }),
        allow_negative: false,
    };
    println!("  Budget ID: {}", budget.budget_id);
    println!("  Max single: ${:.2}", budget.max_single.as_dollars());

    // Create permit
    println!("\nStep 4: Creating spend permit...");
    let permit = SpendPermit {
        permit_id: PermitId::generate(),
        issuer: buyer_id.clone(),
        bound_budget: budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::new(15_000),
        remaining: Amount::new(15_000),
        counterparty: CounterpartyConstraint::Specific(seller_id.clone()),
        purpose: SpendPurpose::ServicePayment("Data Analysis".into()),
        expires_at: Utc::now() + Duration::hours(24),
        signature: String::new(),
    }.sign(&buyer.keypair());

    println!("  Permit ID: {}", permit.permit_id);
    println!("  Max amount: ${:.2}", permit.max_amount.as_dollars());
    println!("  Expires: {}", permit.expires_at);

    // Create payment intent
    println!("\nStep 5: Creating payment intent...");
    let intent = PaymentIntent {
        intent_id: format!("intent-{}", generate_short_id()),
        permit_id: permit.permit_id.clone(),
        sender: buyer_id.clone(),
        recipient: seller_id.clone(),
        amount: Amount::new(10_000), // $100
        memo: "Payment for Data Analysis service".to_string(),
        created_at: Utc::now(),
    };
    println!("  Intent ID: {}", intent.intent_id);
    println!("  Amount: ${:.2}", intent.amount.as_dollars());

    // Execute through commitment gate
    println!("\nStep 6: Executing through Commitment Gate...");
    let (receipt, evidence) = gate.create_commitment(
        &intent,
        &permit,
        &budget,
        ConsequenceRef::DirectPayment,
    ).await?;

    println!("  Payment SUCCESSFUL!");
    println!("  Receipt ID: {}", receipt.receipt_id);
    println!("  Signature: {}...", &receipt.signature[..20]);

    // Verify the receipt
    println!("\nStep 7: Verifying receipt...");
    match receipt.verify() {
        Ok(_) => println!("  Receipt signature: VALID"),
        Err(e) => println!("  Verification failed: {}", e),
    }

    // Check final balances
    println!("\nFinal balances:");
    println!("  Buyer:  ${:.2}", ledger.balance(&buyer_id).await?.as_dollars());
    println!("  Seller: ${:.2}", ledger.balance(&seller_id).await?.as_dollars());

    // Show permit state
    println!("\nPermit state after payment:");
    println!("  Remaining: ${:.2}", (permit.remaining - intent.amount).as_dollars());

    println!("\n=== Payment Tutorial Complete ===");

    Ok(())
}
```

**Expected Output:**
```
=== OpeniBank Payment Tutorial ===

Step 1: Creating agents...
Step 2: Funding buyer with $500...
  Mint receipt: rcpt-mint-a1b2c3

Initial balances:
  Buyer:  $500.00
  Seller: $0.00

Step 3: Creating budget policy...
  Budget ID: budget-d4e5f6
  Max single: $200.00

Step 4: Creating spend permit...
  Permit ID: permit-g7h8i9
  Max amount: $150.00
  Expires: 2025-02-09T11:00:00Z

Step 5: Creating payment intent...
  Intent ID: intent-j0k1l2
  Amount: $100.00

Step 6: Executing through Commitment Gate...
  Payment SUCCESSFUL!
  Receipt ID: rcpt-pay-m3n4o5
  Signature: ed25519:8qRsTuVw...

Step 7: Verifying receipt...
  Receipt signature: VALID

Final balances:
  Buyer:  $400.00
  Seller: $100.00

Permit state after payment:
  Remaining: $50.00

=== Payment Tutorial Complete ===
```

---

## Key Concepts Recap

| Concept | Purpose |
|---------|---------|
| **Commitment Boundary** | Separates advisory (LLM reasoning) from binding (actual transfers) |
| **SpendPermit** | Pre-authorized, bounded permission to spend |
| **PaymentIntent** | Proposal to spend money, references a permit |
| **Commitment Gate** | Validates and executes payments, produces receipts |
| **CommitmentReceipt** | Cryptographic proof that payment occurred |

---

## Troubleshooting

### "Permit not found"

```rust
// Ensure permit was created and stored
let permit = guard.get_permit(&permit_id).await?;
```

### "Counterparty mismatch"

```rust
// Check that intent recipient matches permit constraint
match &permit.counterparty {
    CounterpartyConstraint::Specific(allowed) => {
        assert_eq!(allowed, &intent.recipient, "Recipient mismatch");
    }
    _ => {}
}
```

### "Insufficient balance"

```rust
// Verify balance before creating intent
let balance = ledger.balance(&buyer_id).await?;
assert!(balance >= intent.amount, "Fund the account first");
```

### "Permit expired"

```rust
// Check expiration before use
assert!(permit.expires_at > Utc::now(), "Create a new permit");
```

---

## Next Steps

You now understand how OpeniBank's permit-based payment system works. Continue your learning:

1. **[Understanding Receipts](./03-receipts.md)** - Deep dive into cryptographic verification and audit trails
2. **[Building with Permits](./04-permits.md)** - Advanced permit patterns and hierarchies
3. **[Escrow Workflows](./05-escrow.md)** - Multi-party conditional payments

---

## Quick Reference

### Payment Flow Summary

```
1. Create Budget    → Defines spending limits
2. Create Permit    → Authorizes specific payment
3. Create Intent    → Proposes the payment
4. Execute          → Commitment Gate validates and transfers
5. Verify Receipt   → Cryptographic proof of payment
```

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/permits` | POST | Create a spend permit |
| `/api/permits/{id}` | GET | Get permit details |
| `/api/payments/intent` | POST | Create payment intent |
| `/api/payments/execute` | POST | Execute payment |
| `/api/receipts/verify` | POST | Verify a receipt |

---

**Next Tutorial**: [Understanding Receipts](./03-receipts.md) - Learn how cryptographic receipts provide verifiable proof of every transaction.
