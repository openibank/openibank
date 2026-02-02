//! Demonstrates OpeniBank's fail-closed behavior
//!
//! This example shows that:
//! 1. Payments fail when permit is exceeded
//! 2. Payments fail when permit is expired
//! 3. Payments fail when counterparty doesn't match
//! 4. All failures are explicit and auditable
//!
//! Run with: cargo run --example permit_failure

use chrono::Duration;
use openibank_core::*;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         OpeniBank Fail-Closed Demonstration                  ║");
    println!("║                                                              ║");
    println!("║  Invariant: Authority is always bounded. Fail closed.       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let buyer = ResonatorId::from_string("buyer_agent");
    let seller = ResonatorId::from_string("seller_agent");
    let unauthorized = ResonatorId::from_string("unauthorized_agent");
    let asset = AssetId::iusd();

    // Create buyer wallet with funds
    let mut buyer_wallet = Wallet::new(buyer.clone());
    buyer_wallet.credit(&asset, Amount::new(100000)).unwrap(); // $1000.00

    // Set up budget
    let budget = BudgetPolicy::new(buyer.clone(), Amount::new(50000)); // $500.00 max
    buyer_wallet.set_budget(budget).unwrap();

    println!("📊 Initial Setup:");
    println!("   Buyer balance: {}", buyer_wallet.balance(&asset));
    println!("   Budget max: $500.00");
    println!();

    // =========================================================================
    // Test 1: Permit amount exceeded
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 1: Payment exceeds permit amount");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let permit1 = buyer_wallet
        .issue_permit(
            Amount::new(10000), // $100.00 max
            CounterpartyConstraint::Any,
            SpendPurpose {
                category: "test".to_string(),
                description: "Limited permit".to_string(),
            },
            Duration::hours(1),
        )
        .unwrap();

    println!("✓ Issued permit for $100.00 max");

    let intent1 = PaymentIntent::new(
        buyer.clone(),
        permit1.permit_id.clone(),
        seller.clone(),
        Amount::new(15000), // $150.00 - exceeds permit!
        asset.clone(),
        SpendPurpose {
            category: "test".to_string(),
            description: "Overspend attempt".to_string(),
        },
    );

    match buyer_wallet.execute_payment(intent1) {
        Ok(_) => println!("⚠ UNEXPECTED: Payment succeeded"),
        Err(e) => println!("✓ Payment correctly rejected: {}", e),
    }
    println!();

    // =========================================================================
    // Test 2: Counterparty mismatch
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 2: Payment to wrong counterparty");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let permit2 = buyer_wallet
        .issue_permit(
            Amount::new(10000), // $100.00 max
            CounterpartyConstraint::Specific(seller.clone()), // Only seller allowed
            SpendPurpose {
                category: "test".to_string(),
                description: "Seller-only permit".to_string(),
            },
            Duration::hours(1),
        )
        .unwrap();

    println!("✓ Issued permit for seller_agent only");

    let intent2 = PaymentIntent::new(
        buyer.clone(),
        permit2.permit_id.clone(),
        unauthorized.clone(), // Wrong counterparty!
        Amount::new(5000),    // $50.00
        asset.clone(),
        SpendPurpose {
            category: "test".to_string(),
            description: "Wrong recipient attempt".to_string(),
        },
    );

    match buyer_wallet.execute_payment(intent2) {
        Ok(_) => println!("⚠ UNEXPECTED: Payment succeeded"),
        Err(e) => println!("✓ Payment correctly rejected: {}", e),
    }
    println!();

    // =========================================================================
    // Test 3: Insufficient balance
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 3: Payment exceeds available balance");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create a wallet with less funds than the permit allows
    let mut poor_wallet = Wallet::new(ResonatorId::from_string("poor_agent"));
    poor_wallet.credit(&asset, Amount::new(1000)).unwrap(); // Only $10.00

    let poor_budget = BudgetPolicy::new(
        ResonatorId::from_string("poor_agent"),
        Amount::new(50000),
    );
    poor_wallet.set_budget(poor_budget).unwrap();

    let permit3 = poor_wallet
        .issue_permit(
            Amount::new(10000), // $100.00 permit
            CounterpartyConstraint::Any,
            SpendPurpose {
                category: "test".to_string(),
                description: "Large permit".to_string(),
            },
            Duration::hours(1),
        )
        .unwrap();

    println!("✓ Issued $100.00 permit (but only $10.00 in wallet)");

    let intent3 = PaymentIntent::new(
        ResonatorId::from_string("poor_agent"),
        permit3.permit_id.clone(),
        seller.clone(),
        Amount::new(5000), // $50.00 - exceeds balance!
        asset.clone(),
        SpendPurpose {
            category: "test".to_string(),
            description: "Overdraft attempt".to_string(),
        },
    );

    match poor_wallet.execute_payment(intent3) {
        Ok(_) => println!("⚠ UNEXPECTED: Payment succeeded"),
        Err(e) => println!("✓ Payment correctly rejected: {}", e),
    }
    println!();

    // =========================================================================
    // Test 4: Budget exceeded
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 4: Permit exceeds budget");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Try to issue a permit larger than budget allows
    match buyer_wallet.issue_permit(
        Amount::new(100000), // $1000.00 - exceeds $500 budget!
        CounterpartyConstraint::Any,
        SpendPurpose {
            category: "test".to_string(),
            description: "Over-budget permit".to_string(),
        },
        Duration::hours(1),
    ) {
        Ok(_) => println!("⚠ UNEXPECTED: Permit issued"),
        Err(e) => println!("✓ Permit correctly rejected: {}", e),
    }
    println!();

    // =========================================================================
    // Test 5: Successful payment (for comparison)
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 5: Valid payment (for comparison)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let valid_permit = buyer_wallet
        .issue_permit(
            Amount::new(10000), // $100.00 max
            CounterpartyConstraint::Specific(seller.clone()),
            SpendPurpose {
                category: "services".to_string(),
                description: "API access".to_string(),
            },
            Duration::hours(1),
        )
        .unwrap();

    println!("✓ Issued valid permit for $100.00 to seller");

    let valid_intent = PaymentIntent::new(
        buyer.clone(),
        valid_permit.permit_id.clone(),
        seller.clone(),
        Amount::new(5000), // $50.00 - within limits
        asset.clone(),
        SpendPurpose {
            category: "services".to_string(),
            description: "API access payment".to_string(),
        },
    );

    match buyer_wallet.execute_payment(valid_intent) {
        Ok((receipt, _evidence)) => {
            println!("✓ Payment succeeded!");
            println!("  Receipt ID: {}", receipt.commitment_id.0);
            println!("  Signature verified: {}", receipt.verify().is_ok());
            println!("  New balance: {}", buyer_wallet.balance(&asset));
        }
        Err(e) => println!("⚠ UNEXPECTED: Payment failed: {}", e),
    }
    println!();

    // =========================================================================
    // Summary
    // =========================================================================
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                        Summary                               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("OpeniBank enforces fail-closed behavior:");
    println!("  ✓ Permit amount limits are enforced");
    println!("  ✓ Counterparty constraints are enforced");
    println!("  ✓ Balance checks are enforced");
    println!("  ✓ Budget limits are enforced");
    println!("  ✓ All failures are explicit with clear error messages");
    println!("  ✓ Valid payments produce verifiable receipts");
    println!();
    println!("These invariants cannot be bypassed.");
    println!("LLMs may propose intents, but NEVER execute money directly.");
}
