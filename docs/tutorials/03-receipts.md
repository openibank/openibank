# Tutorial 3: Understanding Receipts

> **Duration**: 15 minutes
> **Difficulty**: Beginner
> **Prerequisites**: Completed [Tutorial 2: Making Payments](./02-payments.md)

In this tutorial, you will learn about OpeniBank's receipt system, which provides cryptographic proof for every financial operation. Receipts are the foundation of trust and auditability in the system, enabling independent verification without relying on any central authority.

---

## Learning Objectives

By the end of this tutorial, you will be able to:

1. Understand the three types of receipts in OpeniBank
2. Read and interpret receipt data structures
3. Verify receipt signatures cryptographically
4. Build audit trails using receipt chains
5. Perform offline verification without network access

---

## Why Receipts Matter

In traditional banking, you trust the bank to maintain accurate records. In AI agent banking, we need something stronger:

- **Agents operate autonomously** - They need proof of actions for their own records
- **Multiple parties interact** - Buyers, sellers, and arbiters need shared truth
- **Disputes may arise** - Cryptographic proof settles disagreements
- **Audits are required** - Regulators and humans need verifiable trails

OpeniBank solves this with **cryptographic receipts** - immutable, verifiable proof of every operation.

---

## Receipt Types Overview

OpeniBank produces three types of receipts:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Receipt Types                                 │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  IssuerReceipt                                                │   │
│  │  ─────────────                                                │   │
│  │  Proves: IUSD minting and burning operations                  │   │
│  │  Issued by: The IUSD Issuer                                   │   │
│  │  Use case: Track money supply, verify funding                 │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  CommitmentReceipt                                            │   │
│  │  ─────────────────                                            │   │
│  │  Proves: Spending commitments passed the Commitment Gate      │   │
│  │  Issued by: The Commitment Gate                               │   │
│  │  Use case: Verify payments, track spending                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  EscrowReceipt                                                │   │
│  │  ─────────────                                                │   │
│  │  Proves: Escrow state transitions (fund, release, refund)     │   │
│  │  Issued by: The Escrow service                                │   │
│  │  Use case: Track multi-party trades, verify settlements       │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## IssuerReceipt: Tracking Money Supply

Every time IUSD is created (minted) or destroyed (burned), an IssuerReceipt is produced.

### IssuerReceipt Structure

```rust
pub struct IssuerReceipt {
    // Identity
    pub receipt_id: String,              // Unique receipt identifier

    // Operation details
    pub operation: IssuerOperation,       // MINT or BURN
    pub asset: AssetId,                   // Always "IUSD"
    pub amount: Amount,                   // How much was minted/burned
    pub target: ResonatorId,              // Who received/lost the funds

    // Timestamp
    pub issued_at: DateTime<Utc>,         // When this occurred

    // Attestations
    pub reserve_attestation_hash: String, // Hash of reserve proof
    pub policy_snapshot_hash: String,     // Hash of policy at time of issuance

    // Cryptographic proof
    pub signature: String,                // Ed25519 signature
    pub signer_public_key: String,        // Issuer's public key
}

pub enum IssuerOperation {
    Mint,   // New money created
    Burn,   // Money destroyed
}
```

### Example IssuerReceipt

```json
{
  "receipt_id": "rcpt-mint-a1b2c3d4",
  "operation": "MINT",
  "asset": "IUSD",
  "amount": 50000,
  "target": "buyer-alice",
  "issued_at": "2025-02-08T10:30:00Z",
  "reserve_attestation_hash": "sha256:7f8a9b0c...",
  "policy_snapshot_hash": "sha256:1d2e3f4a...",
  "signature": "ed25519:3nKp8mWq...",
  "signer_public_key": "ed25519:9aXb7cYd..."
}
```

### Retrieving IssuerReceipts

```bash
# Get all mint/burn receipts
curl http://localhost:8080/api/issuer/receipts

# Get receipts for a specific agent
curl http://localhost:8080/api/issuer/receipts?target=buyer-alice

# Get a specific receipt
curl http://localhost:8080/api/receipts/rcpt-mint-a1b2c3d4
```

---

## CommitmentReceipt: Proving Payments

When a payment passes through the Commitment Gate, a CommitmentReceipt is produced.

### CommitmentReceipt Structure

```rust
pub struct CommitmentReceipt {
    // Identity
    pub receipt_id: String,              // Unique receipt identifier
    pub receipt_type: String,            // Always "COMMITMENT"

    // Transaction details
    pub operation: String,               // "TRANSFER"
    pub from: ResonatorId,               // Sender
    pub to: ResonatorId,                 // Recipient
    pub amount: Amount,                  // Transfer amount
    pub asset: AssetId,                  // Currency (IUSD)
    pub memo: String,                    // Payment description

    // Authorization chain
    pub permit_id: PermitId,             // Permit that authorized this
    pub budget_id: BudgetId,             // Budget that was charged
    pub intent_id: String,               // Original intent reference

    // Timing
    pub timestamp: DateTime<Utc>,        // When committed

    // Cryptographic proof
    pub signature: String,               // Gate's signature
    pub signer_public_key: String,       // Gate's public key

    // Chain linkage
    pub previous_receipt_hash: Option<String>, // For audit chains
}
```

### Example CommitmentReceipt

```json
{
  "receipt_id": "rcpt-pay-e5f6g7h8",
  "receipt_type": "COMMITMENT",
  "operation": "TRANSFER",
  "from": "buyer-alice",
  "to": "seller-bob",
  "amount": 10000,
  "asset": "IUSD",
  "memo": "Payment for Data Analysis service",
  "permit_id": "permit-7f8a9b",
  "budget_id": "budget-alice-default",
  "intent_id": "intent-c3d4e5",
  "timestamp": "2025-02-08T11:00:00Z",
  "signature": "ed25519:8qRsTuVw...",
  "signer_public_key": "ed25519:4mNpOqRs...",
  "previous_receipt_hash": "sha256:abc123..."
}
```

---

## EscrowReceipt: Multi-Party Trades

Escrow operations produce EscrowReceipts to track the full lifecycle of conditional payments.

### EscrowReceipt Structure

```rust
pub struct EscrowReceipt {
    // Identity
    pub receipt_id: String,
    pub receipt_type: String,             // "ESCROW"

    // Escrow reference
    pub escrow_id: EscrowId,              // Which escrow
    pub state_transition: EscrowTransition, // What happened

    // Parties
    pub buyer: ResonatorId,
    pub seller: ResonatorId,
    pub arbiter: ResonatorId,

    // Financial details
    pub amount: Amount,
    pub asset: AssetId,

    // State change details
    pub from_status: EscrowStatus,
    pub to_status: EscrowStatus,
    pub trigger: String,                  // What triggered this transition

    // Timing
    pub timestamp: DateTime<Utc>,

    // Cryptographic proof
    pub signature: String,
    pub signer_public_key: String,
}

pub enum EscrowTransition {
    Created,          // Escrow was created
    Funded,           // Buyer deposited funds
    DeliveryStarted,  // Seller began delivery
    DeliveryConfirmed,// Buyer confirmed receipt
    Released,         // Funds released to seller
    Disputed,         // Dispute raised
    Resolved,         // Dispute resolved
    Refunded,         // Funds returned to buyer
    Expired,          // Deadline passed
}
```

### Example EscrowReceipt

```json
{
  "receipt_id": "rcpt-escrow-i9j0k1",
  "receipt_type": "ESCROW",
  "escrow_id": "escrow-x1y2z3",
  "state_transition": "Released",
  "buyer": "buyer-alice",
  "seller": "seller-bob",
  "arbiter": "arbiter-charlie",
  "amount": 10000,
  "asset": "IUSD",
  "from_status": "DeliveryConfirmed",
  "to_status": "Released",
  "trigger": "buyer_confirmation",
  "timestamp": "2025-02-08T12:00:00Z",
  "signature": "ed25519:5tUvWxYz...",
  "signer_public_key": "ed25519:6aAbBcCd..."
}
```

---

## Cryptographic Verification

The most important feature of receipts is that anyone can verify them independently.

### How Verification Works

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Receipt Verification Process                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. EXTRACT RECEIPT DATA                                            │
│     ├─ Get all fields except signature                              │
│     └─ Serialize in canonical order (deterministic)                 │
│                                                                      │
│  2. COMPUTE MESSAGE HASH                                            │
│     └─ SHA-256 hash of serialized data                              │
│                                                                      │
│  3. VERIFY ED25519 SIGNATURE                                        │
│     ├─ Get signer_public_key from receipt                           │
│     ├─ Get signature from receipt                                   │
│     └─ Verify: signature(message_hash, public_key)                  │
│                                                                      │
│  4. VALIDATE PUBLIC KEY (optional)                                  │
│     ├─ For IssuerReceipt: Check against known issuer key            │
│     ├─ For CommitmentReceipt: Check against known gate key          │
│     └─ For EscrowReceipt: Check against known escrow service key    │
│                                                                      │
│  Result: VALID (signature matches) or INVALID (signature fails)     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Verification via API

```bash
curl -X POST http://localhost:8080/api/receipts/verify \
  -H "Content-Type: application/json" \
  -d '{
    "receipt_id": "rcpt-pay-e5f6g7h8"
  }'
```

**Response:**
```json
{
  "valid": true,
  "receipt_id": "rcpt-pay-e5f6g7h8",
  "verification": {
    "signature_valid": true,
    "signer_trusted": true,
    "timestamp_valid": true,
    "data_integrity": true
  },
  "verified_at": "2025-02-08T14:00:00Z"
}
```

### Verification in Rust

```rust
use openibank_receipts::{verify_receipt, VerificationResult};
use ed25519_dalek::{PublicKey, Signature, Verifier};

/// Verify any receipt type
fn verify_receipt_signature<T: Receipt>(receipt: &T) -> Result<(), VerificationError> {
    // 1. Serialize the receipt data (excluding signature)
    let message = receipt.canonical_bytes();

    // 2. Parse the public key
    let public_key_bytes = hex::decode(&receipt.signer_public_key()
        .strip_prefix("ed25519:")
        .ok_or(VerificationError::InvalidKeyFormat)?)?;
    let public_key = PublicKey::from_bytes(&public_key_bytes)?;

    // 3. Parse the signature
    let signature_bytes = hex::decode(&receipt.signature()
        .strip_prefix("ed25519:")
        .ok_or(VerificationError::InvalidSignatureFormat)?)?;
    let signature = Signature::from_bytes(&signature_bytes)?;

    // 4. Verify
    public_key.verify(&message, &signature)
        .map_err(|_| VerificationError::SignatureMismatch)
}

// Usage
match verify_receipt_signature(&commitment_receipt) {
    Ok(_) => println!("Receipt signature is VALID"),
    Err(e) => println!("Verification failed: {:?}", e),
}
```

### Complete Verification Example

```rust
use openibank_receipts::CommitmentReceipt;

fn comprehensive_verification(receipt: &CommitmentReceipt) -> VerificationReport {
    let mut report = VerificationReport::new(receipt.receipt_id.clone());

    // 1. Signature verification
    match receipt.verify() {
        Ok(_) => report.signature_valid = true,
        Err(e) => {
            report.signature_valid = false;
            report.errors.push(format!("Signature: {}", e));
        }
    }

    // 2. Timestamp validation (not in future, not too old)
    let now = Utc::now();
    if receipt.timestamp > now {
        report.errors.push("Timestamp is in the future".into());
    } else if receipt.timestamp < now - Duration::days(365) {
        report.warnings.push("Receipt is over 1 year old".into());
    }
    report.timestamp_valid = report.errors.is_empty();

    // 3. Amount validation (positive, reasonable)
    if receipt.amount.cents() == 0 {
        report.errors.push("Amount is zero".into());
    }
    report.amount_valid = receipt.amount.cents() > 0;

    // 4. Party validation (valid ResonatorIds)
    report.parties_valid = !receipt.from.id.is_empty() && !receipt.to.id.is_empty();

    report.overall_valid = report.signature_valid
        && report.timestamp_valid
        && report.amount_valid
        && report.parties_valid;

    report
}

#[derive(Debug)]
struct VerificationReport {
    receipt_id: String,
    signature_valid: bool,
    timestamp_valid: bool,
    amount_valid: bool,
    parties_valid: bool,
    overall_valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}
```

---

## Building Audit Trails

Receipts can be chained together to create complete audit trails of all financial activity.

### Receipt Chain Concept

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Agent Audit Trail                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Receipt 1 (Mint)                                                    │
│  ┌────────────────────────┐                                         │
│  │ rcpt-mint-001          │                                         │
│  │ MINT +$500 to alice    │                                         │
│  │ prev: null             │◄──── Chain starts here                  │
│  │ hash: abc123           │                                         │
│  └──────────┬─────────────┘                                         │
│             │                                                        │
│             ▼                                                        │
│  Receipt 2 (Payment)                                                 │
│  ┌────────────────────────┐                                         │
│  │ rcpt-pay-002           │                                         │
│  │ TRANSFER $100 to bob   │                                         │
│  │ prev: abc123           │◄──── Links to previous                  │
│  │ hash: def456           │                                         │
│  └──────────┬─────────────┘                                         │
│             │                                                        │
│             ▼                                                        │
│  Receipt 3 (Escrow Fund)                                            │
│  ┌────────────────────────┐                                         │
│  │ rcpt-escrow-003        │                                         │
│  │ ESCROW $200 locked     │                                         │
│  │ prev: def456           │◄──── Continues the chain                │
│  │ hash: ghi789           │                                         │
│  └──────────┬─────────────┘                                         │
│             │                                                        │
│             ▼                                                        │
│            ...                                                       │
│                                                                      │
│  Each receipt contains hash of previous receipt                     │
│  Tampering with any receipt breaks the chain                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Building an Audit Trail

```rust
use openibank_receipts::{AuditTrail, Receipt};

/// Build a complete audit trail for an agent
async fn build_audit_trail(
    client: &Client,
    agent_id: &ResonatorId,
) -> Result<AuditTrail, AuditError> {
    // Fetch all receipts involving this agent
    let receipts = client.receipts()
        .list(ReceiptFilter {
            agent_id: Some(agent_id.clone()),
            order_by: OrderBy::TimestampAsc,
            ..Default::default()
        })
        .await?;

    // Build the trail
    let mut trail = AuditTrail::new(agent_id.clone());

    for receipt in receipts {
        // Verify each receipt
        receipt.verify()?;

        // Verify chain linkage
        if let Some(expected_prev) = &receipt.previous_receipt_hash {
            let actual_prev = trail.last_hash();
            if actual_prev.as_ref() != Some(expected_prev) {
                return Err(AuditError::ChainBroken {
                    receipt_id: receipt.receipt_id().to_string(),
                    expected: expected_prev.clone(),
                    actual: actual_prev,
                });
            }
        }

        // Add to trail
        trail.add(receipt);
    }

    Ok(trail)
}

// Usage
let trail = build_audit_trail(&client, &buyer_id).await?;

println!("Audit Trail for {}", buyer_id);
println!("Total receipts: {}", trail.len());
println!("Chain integrity: {}", if trail.is_valid() { "VALID" } else { "BROKEN" });

for entry in trail.entries() {
    println!("  {} | {} | {:+} IUSD",
        entry.timestamp,
        entry.operation,
        entry.balance_change.as_dollars()
    );
}
```

### Querying Audit Trails via API

```bash
# Get complete audit trail for an agent
curl "http://localhost:8080/api/audit/trail?agent_id=buyer-alice"

# Get audit trail for a specific time period
curl "http://localhost:8080/api/audit/trail?agent_id=buyer-alice&from=2025-02-01&to=2025-02-08"

# Verify trail integrity
curl -X POST "http://localhost:8080/api/audit/verify" \
  -H "Content-Type: application/json" \
  -d '{"agent_id": "buyer-alice"}'
```

**Response:**
```json
{
  "agent_id": "buyer-alice",
  "trail_length": 15,
  "first_receipt": "2025-02-01T10:00:00Z",
  "last_receipt": "2025-02-08T14:30:00Z",
  "chain_valid": true,
  "total_minted": 50000,
  "total_spent": 25000,
  "total_received": 5000,
  "current_balance": 30000,
  "balance_matches": true
}
```

---

## Offline Verification

One of the most powerful features of OpeniBank receipts is **offline verification**. You can verify a receipt without any network access.

### Why Offline Verification Matters

- **Network outages**: Verify payments even when disconnected
- **Dispute resolution**: Present proof without live systems
- **Archival**: Store and verify receipts years later
- **Privacy**: Verify without revealing to third parties

### Offline Verification Process

```rust
use openibank_receipts::offline::{OfflineVerifier, TrustStore};

/// Verify a receipt with no network access
fn verify_offline(
    receipt_json: &str,
    trust_store: &TrustStore,
) -> Result<OfflineVerificationResult, VerificationError> {
    // Parse the receipt
    let receipt: CommitmentReceipt = serde_json::from_str(receipt_json)?;

    // Create offline verifier with known trusted keys
    let verifier = OfflineVerifier::new(trust_store);

    // Verify
    verifier.verify(&receipt)
}

// The TrustStore contains public keys you trust
let mut trust_store = TrustStore::new();

// Add known issuer key
trust_store.add_issuer_key(
    "issuer-main",
    "ed25519:9aXb7cYd...",
);

// Add known commitment gate key
trust_store.add_gate_key(
    "gate-primary",
    "ed25519:4mNpOqRs...",
);

// Now you can verify offline
let receipt_json = r#"{
  "receipt_id": "rcpt-pay-e5f6g7h8",
  "signature": "ed25519:8qRsTuVw...",
  "signer_public_key": "ed25519:4mNpOqRs...",
  ...
}"#;

match verify_offline(receipt_json, &trust_store) {
    Ok(result) => {
        println!("Offline verification: PASSED");
        println!("Signer: {}", result.signer_identity);
        println!("Trusted: {}", result.signer_trusted);
    }
    Err(e) => {
        println!("Offline verification: FAILED");
        println!("Reason: {:?}", e);
    }
}
```

### CLI Offline Verification

```bash
# Verify a receipt file offline
openibank receipt verify receipt.json --offline

# Verify with specific trust store
openibank receipt verify receipt.json --trust-store keys.json

# Output detailed verification report
openibank receipt verify receipt.json --offline --verbose
```

**Output:**
```
Receipt Verification Report
═══════════════════════════════════════════════════════════════════

Receipt ID:      rcpt-pay-e5f6g7h8
Receipt Type:    COMMITMENT
Operation:       TRANSFER

Verification Results:
  ✓ Signature valid
  ✓ Signer key recognized (gate-primary)
  ✓ Timestamp within acceptable range
  ✓ Data structure valid

Transaction Details:
  From:    buyer-alice
  To:      seller-bob
  Amount:  $100.00 IUSD
  Memo:    Payment for Data Analysis service

Overall: VALID
```

---

## Complete Working Example

```rust
//! Complete Receipts Tutorial
//!
//! Run with: cargo run --example receipts

use openibank_agents::{BuyerAgent, SellerAgent, AgentBrain};
use openibank_core::{Amount, ResonatorId, PaymentIntent};
use openibank_guard::{CommitmentGate, SpendPermit, BudgetPolicy};
use openibank_issuer::Issuer;
use openibank_ledger::Ledger;
use openibank_receipts::{AuditTrail, OfflineVerifier, TrustStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpeniBank Receipts Tutorial ===\n");

    // Setup
    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());
    let gate = CommitmentGate::new();

    let buyer_id = ResonatorId::from_string("buyer-alice");
    let seller_id = ResonatorId::from_string("seller-bob");

    // ========================================
    // Part 1: Generate Receipts
    // ========================================
    println!("Part 1: Generating Receipts\n");

    // Mint receipt
    println!("Minting $500 to buyer...");
    let mint_receipt = issuer.mint(buyer_id.clone(), Amount::new(50_000)).await?;
    println!("  Generated IssuerReceipt: {}", mint_receipt.receipt_id);

    // Setup for payment
    let budget = BudgetPolicy::default_for(&buyer_id);
    let permit = SpendPermit::new(
        buyer_id.clone(),
        Amount::new(15_000),
        CounterpartyConstraint::Specific(seller_id.clone()),
        "Payment for services",
    ).sign(&buyer_keypair);

    let intent = PaymentIntent {
        intent_id: "intent-001".into(),
        permit_id: permit.permit_id.clone(),
        sender: buyer_id.clone(),
        recipient: seller_id.clone(),
        amount: Amount::new(10_000),
        memo: "Data Analysis - January".into(),
        created_at: Utc::now(),
    };

    // Payment receipt
    println!("Executing payment...");
    let (payment_receipt, _) = gate.create_commitment(
        &intent, &permit, &budget, ConsequenceRef::DirectPayment
    ).await?;
    println!("  Generated CommitmentReceipt: {}", payment_receipt.receipt_id);

    // ========================================
    // Part 2: Verify Receipts
    // ========================================
    println!("\nPart 2: Verifying Receipts\n");

    // Verify mint receipt
    println!("Verifying IssuerReceipt...");
    match mint_receipt.verify() {
        Ok(_) => println!("  Signature: VALID"),
        Err(e) => println!("  Signature: INVALID - {}", e),
    }

    // Verify payment receipt
    println!("Verifying CommitmentReceipt...");
    match payment_receipt.verify() {
        Ok(_) => println!("  Signature: VALID"),
        Err(e) => println!("  Signature: INVALID - {}", e),
    }

    // ========================================
    // Part 3: Inspect Receipt Details
    // ========================================
    println!("\nPart 3: Receipt Details\n");

    println!("IssuerReceipt Details:");
    println!("  Receipt ID:  {}", mint_receipt.receipt_id);
    println!("  Operation:   {:?}", mint_receipt.operation);
    println!("  Amount:      ${:.2}", mint_receipt.amount.as_dollars());
    println!("  Target:      {}", mint_receipt.target);
    println!("  Timestamp:   {}", mint_receipt.issued_at);
    println!("  Signature:   {}...", &mint_receipt.signature[..30]);

    println!("\nCommitmentReceipt Details:");
    println!("  Receipt ID:  {}", payment_receipt.receipt_id);
    println!("  From:        {}", payment_receipt.from);
    println!("  To:          {}", payment_receipt.to);
    println!("  Amount:      ${:.2}", payment_receipt.amount.as_dollars());
    println!("  Permit:      {}", payment_receipt.permit_id);
    println!("  Memo:        {}", payment_receipt.memo);
    println!("  Signature:   {}...", &payment_receipt.signature[..30]);

    // ========================================
    // Part 4: Build Audit Trail
    // ========================================
    println!("\nPart 4: Building Audit Trail\n");

    let mut trail = AuditTrail::new(buyer_id.clone());
    trail.add_issuer_receipt(mint_receipt.clone());
    trail.add_commitment_receipt(payment_receipt.clone());

    println!("Audit Trail for {}:", buyer_id);
    println!("  Total entries: {}", trail.len());
    println!("  Chain valid:   {}", trail.verify_chain()?);
    println!("\n  Timeline:");

    for (i, entry) in trail.entries().iter().enumerate() {
        let change = if entry.is_credit { "+" } else { "-" };
        println!("    {}. {} {} ${:.2} | {}",
            i + 1,
            entry.timestamp.format("%Y-%m-%d %H:%M"),
            change,
            entry.amount.as_dollars(),
            entry.description
        );
    }

    // ========================================
    // Part 5: Offline Verification
    // ========================================
    println!("\nPart 5: Offline Verification\n");

    // Create trust store with known keys
    let mut trust_store = TrustStore::new();
    trust_store.add_issuer_key("issuer-main", &issuer.public_key());
    trust_store.add_gate_key("gate-primary", &gate.public_key());

    // Serialize receipt (as if received from someone)
    let receipt_json = serde_json::to_string_pretty(&payment_receipt)?;
    println!("Receipt JSON (what you'd store/transmit):");
    println!("{}", receipt_json);

    // Verify offline
    println!("\nOffline verification (no network):");
    let verifier = OfflineVerifier::new(&trust_store);
    match verifier.verify_json(&receipt_json) {
        Ok(result) => {
            println!("  Status:     VALID");
            println!("  Signer:     {}", result.signer_identity);
            println!("  Trusted:    {}", result.is_trusted);
        }
        Err(e) => {
            println!("  Status:     INVALID");
            println!("  Reason:     {:?}", e);
        }
    }

    // ========================================
    // Part 6: Receipt Comparison
    // ========================================
    println!("\nPart 6: Receipt Comparison\n");

    println!("Comparing two receipt types:");
    println!("┌────────────────────┬─────────────────────┬─────────────────────┐");
    println!("│ Field              │ IssuerReceipt       │ CommitmentReceipt   │");
    println!("├────────────────────┼─────────────────────┼─────────────────────┤");
    println!("│ Purpose            │ Mint/Burn           │ Transfers           │");
    println!("│ Issuer             │ IUSD Issuer         │ Commitment Gate     │");
    println!("│ Contains Permit    │ No                  │ Yes                 │");
    println!("│ Contains Memo      │ No                  │ Yes                 │");
    println!("│ Chain Linkage      │ Optional            │ Yes                 │");
    println!("└────────────────────┴─────────────────────┴─────────────────────┘");

    println!("\n=== Receipts Tutorial Complete ===");

    Ok(())
}
```

---

## Key Concepts Recap

| Concept | Description |
|---------|-------------|
| **IssuerReceipt** | Proves IUSD was minted or burned |
| **CommitmentReceipt** | Proves a payment passed the Commitment Gate |
| **EscrowReceipt** | Proves escrow state transitions |
| **Ed25519 Signature** | Cryptographic proof of authenticity |
| **Audit Trail** | Chain of receipts for complete history |
| **Offline Verification** | Verify without network access |

---

## Troubleshooting

### "Signature verification failed"

```rust
// Check if public key matches expected
println!("Receipt signer: {}", receipt.signer_public_key);
println!("Expected key: {}", expected_public_key);

// Ensure you're using the correct receipt type
// IssuerReceipts are signed by the Issuer
// CommitmentReceipts are signed by the Gate
```

### "Chain integrity broken"

```rust
// Check for missing receipts
let all_receipts = fetch_all_receipts(&agent_id).await?;
println!("Total receipts found: {}", all_receipts.len());

// Verify each link
for (i, receipt) in all_receipts.iter().enumerate() {
    if let Some(prev) = &receipt.previous_receipt_hash {
        println!("Receipt {} links to: {}", i, prev);
    }
}
```

### "Receipt not found"

```bash
# List all receipts
curl http://localhost:8080/api/receipts

# Check specific agent's receipts
curl "http://localhost:8080/api/receipts?agent_id=buyer-alice"
```

---

## Next Steps

You now understand how receipts provide cryptographic proof for all OpeniBank operations. Continue learning:

1. **[Building with Permits](./04-permits.md)** - Advanced permit hierarchies and policies
2. **[Escrow Workflows](./05-escrow.md)** - Multi-party trades with conditional settlement

---

## Quick Reference

### Receipt Types Summary

| Type | Signed By | Proves |
|------|-----------|--------|
| IssuerReceipt | IUSD Issuer | Mint/burn operations |
| CommitmentReceipt | Commitment Gate | Payment transfers |
| EscrowReceipt | Escrow Service | Escrow state changes |

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/receipts` | GET | List all receipts |
| `/api/receipts/{id}` | GET | Get specific receipt |
| `/api/receipts/verify` | POST | Verify a receipt |
| `/api/audit/trail` | GET | Get agent audit trail |
| `/api/issuer/receipts` | GET | List issuer receipts |

### CLI Commands

| Command | Description |
|---------|-------------|
| `openibank receipt verify <file>` | Verify a receipt file |
| `openibank receipt inspect <id>` | Show receipt details |
| `openibank receipt diff <id1> <id2>` | Compare two receipts |
| `openibank audit trail <agent>` | Show agent audit trail |

---

**Next Tutorial**: [Building with Permits](./04-permits.md) - Learn advanced permit patterns including hierarchies, velocity limits, and real-world authorization strategies.
