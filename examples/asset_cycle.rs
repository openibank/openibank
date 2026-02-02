//! OpeniBank Viral Demo - Complete Asset Cycle
//!
//! This example demonstrates the full AI-agent asset lifecycle:
//!
//! Mint → Budget → Permit → Escrow → Settlement → Receipt → Verification
//!
//! Run with:
//!   cargo run --example asset_cycle
//!
//! Or with LLM support:
//!   OPENIBANK_LLM_PROVIDER=ollama cargo run --example asset_cycle

use std::sync::Arc;

use openibank_agents::{AgentBrain, ArbiterAgent, BuyerAgent, SellerAgent, Service};
use openibank_core::{Amount, AssetId, ResonatorId};
use openibank_issuer::{Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use openibank_llm::LLMRouter;

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║     ██████╗ ██████╗ ███████╗███╗   ██╗██╗██████╗  █████╗ ███╗   ██╗██╗  ██╗║");
    println!("║    ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██║██╔══██╗██╔══██╗████╗  ██║██║ ██╔╝║");
    println!("║    ██║   ██║██████╔╝█████╗  ██╔██╗ ██║██║██████╔╝███████║██╔██╗ ██║█████╔╝ ║");
    println!("║    ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██║██╔══██╗██╔══██║██║╚██╗██║██╔═██╗ ║");
    println!("║    ╚██████╔╝██║     ███████╗██║ ╚████║██║██████╔╝██║  ██║██║ ╚████║██║  ██╗║");
    println!("║     ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝╚═╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝║");
    println!("║                                                                      ║");
    println!("║          Programmable Wallets + Receipts for AI Agents               ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    // =========================================================================
    // Step 0: Initialize LLM (optional)
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 0: Initialize Environment");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;

    println!("  LLM Provider: {}", llm_router.kind());
    println!("  LLM Available: {}", if llm_available { "✓ Yes" } else { "✗ No (using deterministic)" });
    println!();

    // Create shared ledger
    let ledger = Arc::new(Ledger::new());

    // =========================================================================
    // Step 1: Initialize Issuer
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 1: Initialize IUSD Issuer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let issuer = Issuer::new(
        IssuerConfig::default(),
        Amount::new(1_000_000_00), // $10,000 reserve cap
        ledger.clone(),
    );

    println!("  ✓ Issuer initialized");
    println!("    Symbol: IUSD");
    println!("    Reserve Cap: $10,000.00");
    println!("    Public Key: {}...", &issuer.public_key()[..16]);
    println!();

    // =========================================================================
    // Step 2: Create Agents
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 2: Create Agents");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let buyer_id = ResonatorId::from_string("buyer_agent");
    let seller_id = ResonatorId::from_string("seller_agent");
    let arbiter_id = ResonatorId::from_string("arbiter_agent");

    // Create brain based on LLM availability
    let brain = if llm_available {
        AgentBrain::with_llm(llm_router)
    } else {
        AgentBrain::deterministic()
    };

    let mut buyer = BuyerAgent::with_brain(buyer_id.clone(), ledger.clone(), brain);
    let mut seller = SellerAgent::new(seller_id.clone(), ledger.clone());
    let mut arbiter = ArbiterAgent::new(arbiter_id.clone(), ledger.clone());

    println!("  ✓ BuyerAgent created: {}", buyer_id);
    println!("  ✓ SellerAgent created: {}", seller_id);
    println!("  ✓ ArbiterAgent created: {}", arbiter_id);
    println!();

    // =========================================================================
    // Step 3: Mint IUSD to Buyer
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 3: Mint IUSD to Buyer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mint_intent = MintIntent::new(
        buyer_id.clone(),
        Amount::new(1000_00), // $1000
        "Initial funding for buyer agent",
    );

    let mint_receipt = issuer.mint(mint_intent).await.expect("Mint failed");

    println!("  ✓ Minted $1,000.00 IUSD to Buyer");
    println!("    Receipt ID: {}", mint_receipt.receipt_id);
    println!("    Signature Valid: {}", mint_receipt.verify().is_ok());
    println!();

    // Setup buyer wallet with the minted funds
    buyer.setup(Amount::new(1000_00), Amount::new(500_00)).expect("Setup failed");

    println!("  ✓ Buyer wallet configured");
    println!("    Balance: {}", buyer.balance());
    println!("    Budget: $500.00");
    println!();

    // =========================================================================
    // Step 4: Seller Publishes Service
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 4: Seller Publishes Service");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let service = Service {
        name: "AI Data Feed".to_string(),
        description: "Real-time AI training data feed - 30 day access".to_string(),
        price: Amount::new(200_00), // $200
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Provide API credentials".to_string()],
    };

    seller.publish_service(service);

    let offer = seller.get_offer("AI Data Feed").unwrap();
    println!("  ✓ Service published: {}", offer.service_name);
    println!("    Price: {}", offer.price);
    println!("    Description: {}", offer.description);
    println!();

    // =========================================================================
    // Step 5: Buyer Evaluates and Accepts
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 5: Buyer Evaluates Offer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let can_afford = buyer.evaluate_offer(&offer).await;
    println!("  Can afford: {}", if can_afford { "✓ Yes" } else { "✗ No" });

    // Seller issues invoice
    let invoice = seller.issue_invoice(buyer_id.clone(), "AI Data Feed").await.unwrap();
    println!("  ✓ Invoice issued: {}", invoice.invoice_id.0);
    println!("    Amount: {}", invoice.amount);
    println!();

    // Buyer accepts invoice
    buyer.accept_invoice(invoice.clone()).unwrap();
    println!("  ✓ Buyer accepted invoice");
    println!();

    // =========================================================================
    // Step 6: Payment via Escrow
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 6: Payment via Escrow");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let (permit, escrow) = buyer.pay_invoice(&invoice.invoice_id).await.unwrap();

    println!("  ✓ SpendPermit issued");
    println!("    Permit ID: {}", permit.permit_id.0);
    println!("    Max Amount: {}", permit.max_amount);
    println!("    Valid: {}", permit.is_valid());
    println!();
    println!("  ✓ Escrow created");
    println!("    Escrow ID: {}", escrow.escrow_id.0);
    println!("    Locked Amount: {}", escrow.amount);
    println!("    State: {:?}", escrow.state);
    println!();
    println!("  Buyer balance after escrow: {}", buyer.balance());
    println!();

    // =========================================================================
    // Step 7: Seller Delivers Service
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 7: Seller Delivers Service");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let delivery_proof = seller
        .deliver_service(
            &invoice.invoice_id,
            "API Key: sk_live_abc123xyz | Endpoint: https://api.datafeed.example.com".to_string(),
        )
        .unwrap();

    println!("  ✓ Service delivered");
    println!("    Proof Type: {}", delivery_proof.proof_type);
    println!("    Delivered At: {}", delivery_proof.delivered_at);
    println!();

    // =========================================================================
    // Step 8: Arbiter Verifies (Optional)
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 8: Arbiter Verification");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let case = arbiter.open_case(&escrow, None, Some(delivery_proof.clone()));
    let decision = arbiter.decide(&case.case_id).await.unwrap();

    println!("  ✓ Arbiter reviewed case");
    println!("    Case ID: {}", case.case_id);
    println!("    Decision: {:?}", decision.decision);
    println!("    Reasoning: {}", decision.reasoning);
    println!();

    // =========================================================================
    // Step 9: Release Escrow
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Step 9: Release Escrow & Final Settlement");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Buyer confirms delivery and releases escrow
    let released_amount = buyer.confirm_delivery(&escrow.escrow_id).unwrap();

    println!("  ✓ Escrow released");
    println!("    Released Amount: {}", released_amount);
    println!();

    // Seller receives payment
    seller.receive_payment(released_amount).unwrap();

    println!("  ✓ Payment received by seller");
    println!();

    // =========================================================================
    // Final Summary
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Final Balances");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Buyer Balance:  {}", buyer.balance());
    println!("  Seller Balance: {}", seller.balance());
    println!("  Issuer Supply:  {}", issuer.total_supply().await);
    println!();

    // =========================================================================
    // Receipt Verification
    // =========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Receipt Verification");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("  Mint Receipt:");
    println!("    ID: {}", mint_receipt.receipt_id);
    println!("    Verified: {}", if mint_receipt.verify().is_ok() { "✓" } else { "✗" });
    println!();

    // Print sample receipt JSON
    println!("  Sample Receipt JSON:");
    let receipt_json = serde_json::to_string_pretty(&mint_receipt).unwrap();
    for line in receipt_json.lines().take(15) {
        println!("    {}", line);
    }
    println!("    ...");
    println!();

    // =========================================================================
    // Success!
    // =========================================================================
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║   ✓ Complete Asset Cycle Demonstrated Successfully!                  ║");
    println!("║                                                                      ║");
    println!("║   Flow: Mint → Budget → Permit → Escrow → Delivery → Release         ║");
    println!("║                                                                      ║");
    println!("║   All transactions produced verifiable receipts.                     ║");
    println!("║   All spending was bounded by budgets and permits.                   ║");
    println!("║   No direct settlement without commitment.                           ║");
    println!("║                                                                      ║");
    println!("║   OpeniBank is AI-agent-only by design.                              ║");
    println!("║                                                                      ║");
    println!("║   Learn more: https://www.openibank.com/                             ║");
    println!("║   GitHub: https://github.com/openibank/openibank/                    ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
}
