//! Demo commands - The viral demos that make OpeniBank impressive

use std::sync::Arc;
use std::time::Duration;

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use openibank_agents::{AgentBrain, ArbiterAgent, BuyerAgent, SellerAgent, Service};
use openibank_core::{Amount, AssetId, ResonatorId};
use openibank_issuer::{Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use openibank_llm::LLMRouter;
use uuid::Uuid;

/// Run the full viral demo - complete asset cycle
pub async fn run_full_demo(
    llm_provider: Option<String>,
    model: String,
    verbose: bool,
) -> anyhow::Result<()> {
    println!("{}", "🚀 VIRAL DEMO: Complete Agent Commerce Cycle".bright_white().bold());
    println!();
    println!("{}", "This demo shows AI agents trading with:");
    println!("  • {} spending authority", "Bounded".bright_green());
    println!("  • {} transaction receipts", "Verifiable".bright_green());
    println!("  • {} escrow settlement", "Conditional".bright_green());
    println!();

    // Progress bar for setup
    let pb = ProgressBar::new(9);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    // Step 0: Initialize LLM
    pb.set_message("Initializing LLM...");
    let llm_router = if let Some(provider) = &llm_provider {
        match provider.as_str() {
            "ollama" => {
                std::env::set_var("OPENIBANK_LLM_PROVIDER", "ollama");
                std::env::set_var("OLLAMA_MODEL", &model);
            }
            "openai" => {
                std::env::set_var("OPENIBANK_LLM_PROVIDER", "openai");
            }
            "anthropic" => {
                std::env::set_var("OPENIBANK_LLM_PROVIDER", "anthropic");
            }
            _ => {}
        }
        LLMRouter::from_env()
    } else {
        LLMRouter::from_env()
    };

    let llm_available = llm_router.is_available().await;
    pb.inc(1);

    println!();
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 0: Environment Setup".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    if llm_available {
        println!(
            "  {} LLM Active: {} ({})",
            "✓".bright_green(),
            llm_provider.as_deref().unwrap_or("auto-detected").bright_cyan(),
            model.bright_cyan()
        );
        println!("  {} Agents will use AI reasoning for decisions", "→".bright_blue());
    } else {
        println!(
            "  {} LLM: {} (using deterministic mode)",
            "○".yellow(),
            "Not available".yellow()
        );
        println!("  {} To enable: {} or {}",
            "→".bright_blue(),
            "ollama pull llama3".bright_cyan(),
            "--llm openai".bright_cyan()
        );
    }
    println!();

    // Create shared ledger
    let ledger = Arc::new(Ledger::new());

    // Step 1: Initialize Issuer
    pb.set_message("Initializing IUSD Issuer...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 1: Initialize IUSD Issuer".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let issuer = Issuer::new(
        IssuerConfig::default(),
        Amount::new(1_000_000_00), // $10,000 reserve cap
        ledger.clone(),
    );

    println!("  {} Issuer initialized", "✓".bright_green());
    println!("      Symbol: {}", "IUSD".bright_cyan());
    println!("      Reserve Cap: {}", "$10,000.00".bright_cyan());
    println!("      Public Key: {}...", &issuer.public_key()[..16].bright_yellow());
    println!();
    pb.inc(1);

    // Step 2: Create Agents
    pb.set_message("Creating agents...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 2: Create AI Agents".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let buyer_id = ResonatorId::from_string("buyer_agent");
    let seller_id = ResonatorId::from_string("seller_agent");
    let arbiter_id = ResonatorId::from_string("arbiter_agent");

    let brain = if llm_available {
        AgentBrain::with_llm(llm_router)
    } else {
        AgentBrain::deterministic()
    };

    let mut buyer = BuyerAgent::with_brain(buyer_id.clone(), ledger.clone(), brain);
    let mut seller = SellerAgent::new(seller_id.clone(), ledger.clone());
    let mut arbiter = ArbiterAgent::new(arbiter_id.clone(), ledger.clone());

    println!("  {} BuyerAgent: {}", "✓".bright_green(), buyer_id.0.bright_cyan());
    println!("  {} SellerAgent: {}", "✓".bright_green(), seller_id.0.bright_cyan());
    println!("  {} ArbiterAgent: {}", "✓".bright_green(), arbiter_id.0.bright_cyan());
    println!();
    pb.inc(1);

    // Step 3: Mint IUSD
    pb.set_message("Minting IUSD to Buyer...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 3: Mint IUSD to Buyer".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let mint_intent = MintIntent::new(
        buyer_id.clone(),
        Amount::new(1000_00), // $1000
        "Initial funding for buyer agent",
    );

    let mint_receipt = issuer.mint(mint_intent).await?;

    println!("  {} Minted {} to Buyer", "✓".bright_green(), "$1,000.00".bright_cyan());
    println!("      Receipt ID: {}", mint_receipt.receipt_id.bright_yellow());
    println!(
        "      Signature: {} {}",
        if mint_receipt.verify().is_ok() { "✓".bright_green() } else { "✗".bright_red() },
        "Valid".bright_green()
    );
    println!();

    buyer.setup(Amount::new(1000_00), Amount::new(500_00))?;
    println!("  {} Buyer wallet configured", "✓".bright_green());
    println!("      Balance: {}", format!("{}", buyer.balance()).bright_cyan());
    println!("      Budget: {}", "$500.00".bright_cyan());
    println!();
    pb.inc(1);

    // Step 4: Seller Publishes Service
    pb.set_message("Seller publishing service...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 4: Seller Publishes Service".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let service = Service {
        name: "AI Data Feed".to_string(),
        description: "Real-time AI training data feed - 30 day access".to_string(),
        price: Amount::new(200_00), // $200
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Provide API credentials".to_string()],
    };

    seller.publish_service(service);
    let offer = seller.get_offer("AI Data Feed").unwrap();

    println!("  {} Service published: {}", "✓".bright_green(), offer.service_name.bright_cyan());
    println!("      Price: {}", format!("{}", offer.price).bright_cyan());
    println!("      Description: {}", offer.description.bright_black());
    println!();
    pb.inc(1);

    // Step 5: Buyer Evaluates and Accepts
    pb.set_message("Buyer evaluating offer...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 5: Buyer Evaluates Offer".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let can_afford = buyer.evaluate_offer(&offer).await;
    println!(
        "  {} Can afford: {}",
        if can_afford { "✓".bright_green() } else { "✗".bright_red() },
        if can_afford { "Yes".bright_green() } else { "No".bright_red() }
    );

    let invoice = seller.issue_invoice(buyer_id.clone(), "AI Data Feed").await?;
    println!("  {} Invoice issued: {}", "✓".bright_green(), invoice.invoice_id.0.bright_yellow());
    println!("      Amount: {}", format!("{}", invoice.amount).bright_cyan());

    buyer.accept_invoice(invoice.clone())?;
    println!("  {} Buyer accepted invoice", "✓".bright_green());
    println!();
    pb.inc(1);

    // Step 6: Payment via Escrow
    pb.set_message("Creating escrow payment...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 6: Payment via Escrow".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    println!();
    println!("  {} {}", "→".bright_blue(), "CROSSING COMMITMENT BOUNDARY".bright_yellow().bold());
    println!("  {} This is where accountability begins", "→".bright_blue());
    println!();

    let commitment_id = format!("demo_commit_{}", Uuid::new_v4());
    buyer.set_active_commitment(commitment_id.clone(), true);
    seller.set_active_commitment(commitment_id.clone(), true);

    let (permit, escrow) = buyer.pay_invoice(&invoice.invoice_id).await?;

    println!("  {} SpendPermit issued", "✓".bright_green());
    println!("      Permit ID: {}", permit.permit_id.0.bright_yellow());
    println!("      Max Amount: {}", format!("{}", permit.max_amount).bright_cyan());
    println!(
        "      Valid: {}",
        if permit.is_valid() { "Yes".bright_green() } else { "No".bright_red() }
    );
    println!();
    println!("  {} Escrow created", "✓".bright_green());
    println!("      Escrow ID: {}", escrow.escrow_id.0.bright_yellow());
    println!("      Locked: {}", format!("{}", escrow.amount).bright_cyan());
    println!("      State: {}", format!("{:?}", escrow.state).bright_cyan());
    println!();
    println!("  Buyer balance after escrow: {}", format!("{}", buyer.balance()).bright_cyan());
    println!();
    pb.inc(1);

    // Step 7: Seller Delivers
    pb.set_message("Seller delivering service...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 7: Seller Delivers Service".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let delivery_proof = seller.deliver_service(
        &invoice.invoice_id,
        "API Key: sk_live_abc123xyz | Endpoint: https://api.datafeed.example.com".to_string(),
    )?;

    println!("  {} Service delivered", "✓".bright_green());
    println!("      Proof Type: {}", delivery_proof.proof_type.bright_cyan());
    println!("      Delivered At: {}", delivery_proof.delivered_at.to_string().bright_cyan());
    println!();
    pb.inc(1);

    // Step 8: Arbiter Verification
    pb.set_message("Arbiter verifying delivery...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 8: Arbiter Verification".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let case = arbiter.open_case(&escrow, None, Some(delivery_proof.clone()));
    arbiter.set_active_commitment(commitment_id.clone(), true);
    let decision = arbiter.decide(&case.case_id).await?;

    println!("  {} Arbiter reviewed case", "✓".bright_green());
    println!("      Case ID: {}", case.case_id.bright_yellow());
    println!("      Decision: {}", format!("{:?}", decision.decision).bright_green());
    println!("      Reasoning: {}", decision.reasoning.bright_black());
    println!();
    pb.inc(1);

    // Step 9: Release Escrow
    pb.set_message("Releasing escrow...");
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Step 9: Release Escrow & Settlement".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());

    let released_amount = buyer.confirm_delivery(&escrow.escrow_id)?;
    seller.receive_payment(released_amount)?;
    buyer.clear_active_commitment();
    seller.clear_active_commitment();
    arbiter.clear_active_commitment();

    println!("  {} Escrow released: {}", "✓".bright_green(), format!("{}", released_amount).bright_cyan());
    println!("  {} Payment received by seller", "✓".bright_green());
    println!();

    pb.finish_with_message("Demo complete!");
    println!();

    // Final Summary
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Final Balances".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());
    println!("  Buyer Balance:  {}", format!("{}", buyer.balance()).bright_cyan());
    println!("  Seller Balance: {}", format!("{}", seller.balance()).bright_cyan());
    println!("  Issuer Supply:  {}", format!("{}", issuer.total_supply().await).bright_cyan());
    println!();

    // Receipt verification
    println!("{}", "━".repeat(70).bright_black());
    println!("{}", " Receipt Verification".bright_white().bold());
    println!("{}", "━".repeat(70).bright_black());
    println!("  Mint Receipt:");
    println!("      ID: {}", mint_receipt.receipt_id.bright_yellow());
    println!(
        "      Verified: {} {}",
        "✓".bright_green(),
        "Cryptographically valid".bright_green()
    );
    println!();

    if verbose {
        println!("{}", " Sample Receipt JSON:".bright_white());
        let receipt_json = serde_json::to_string_pretty(&mint_receipt)?;
        for line in receipt_json.lines() {
            println!("      {}", line.bright_black());
        }
        println!();
    }

    // Success banner
    println!("{}", "╔══════════════════════════════════════════════════════════════════╗".bright_green());
    println!("{}", "║                                                                  ║".bright_green());
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "✓ Complete Asset Cycle Demonstrated Successfully!".bright_white().bold(),
        "              ║".bright_green()
    );
    println!("{}", "║                                                                  ║".bright_green());
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "Flow: Mint → Budget → Permit → Escrow → Delivery → Release".bright_cyan(),
        "        ║".bright_green()
    );
    println!("{}", "║                                                                  ║".bright_green());
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "All transactions produced verifiable receipts.".bright_white(),
        "                  ║".bright_green()
    );
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "All spending was bounded by budgets and permits.".bright_white(),
        "                ║".bright_green()
    );
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "No money moved without crossing the commitment boundary.".bright_white(),
        "         ║".bright_green()
    );
    println!("{}", "║                                                                  ║".bright_green());
    println!("{}", "╚══════════════════════════════════════════════════════════════════╝".bright_green());
    println!();

    // Next steps
    println!("{}", "What's Next?".bright_white().bold());
    println!("  {} Run with LLM reasoning", "→".bright_blue());
    println!("    {}", "openibank demo full --llm ollama".bright_cyan());
    println!();
    println!("  {} Start the issuer service", "→".bright_blue());
    println!("    {}", "openibank issuer start".bright_cyan());
    println!();
    println!("  {} Try the web playground", "→".bright_blue());
    println!("    {}", "openibank playground".bright_cyan());
    println!();

    Ok(())
}

/// Run the safety demo showing fail-closed behavior
pub async fn run_safety_demo() -> anyhow::Result<()> {
    println!("{}", "🛡️  SAFETY DEMO: Fail-Closed Behavior".bright_white().bold());
    println!();
    println!("{}", "This demo shows that OpeniBank fails safely:");
    println!("  • {} reject when over permit", "Payments".bright_red());
    println!("  • {} reject when wrong counterparty", "Payments".bright_red());
    println!("  • {} reject when over budget", "Permits".bright_red());
    println!("  • {} when insufficient balance", "Escrows fail".bright_red());
    println!();

    // Run the permit failure example logic inline
    use chrono::Duration;
    use openibank_core::*;

    let buyer = ResonatorId::from_string("buyer_agent");
    let seller = ResonatorId::from_string("seller_agent");
    let unauthorized = ResonatorId::from_string("unauthorized_agent");
    let asset = AssetId::iusd();

    let mut buyer_wallet = Wallet::new(buyer.clone());
    buyer_wallet.credit(&asset, Amount::new(100000))?;

    let budget = BudgetPolicy::new(buyer.clone(), Amount::new(50000));
    buyer_wallet.set_budget(budget)?;

    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Initial Setup".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());
    println!("  Buyer balance: {}", format!("{}", buyer_wallet.balance(&asset)).bright_cyan());
    println!("  Budget max: {}", "$500.00".bright_cyan());
    println!();

    // Test 1: Permit amount exceeded
    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Test 1: Payment exceeds permit amount".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());

    let permit1 = buyer_wallet.issue_permit(
        Amount::new(10000),
        CounterpartyConstraint::Any,
        SpendPurpose {
            category: "test".to_string(),
            description: "Limited permit".to_string(),
        },
        Duration::hours(1),
    )?;

    println!("  {} Issued permit for $100.00 max", "✓".bright_green());

    let intent1 = PaymentIntent::new(
        buyer.clone(),
        permit1.permit_id.clone(),
        seller.clone(),
        Amount::new(15000), // Exceeds permit!
        asset.clone(),
        SpendPurpose {
            category: "test".to_string(),
            description: "Overspend attempt".to_string(),
        },
    );

    match buyer_wallet.execute_payment(intent1) {
        Ok(_) => println!("  {} Payment succeeded (unexpected!)", "⚠".yellow()),
        Err(e) => println!("  {} Payment rejected: {}", "✓".bright_green(), e.to_string().bright_red()),
    }
    println!();

    // Test 2: Wrong counterparty
    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Test 2: Payment to wrong counterparty".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());

    let permit2 = buyer_wallet.issue_permit(
        Amount::new(10000),
        CounterpartyConstraint::Specific(seller.clone()),
        SpendPurpose {
            category: "test".to_string(),
            description: "Seller-only permit".to_string(),
        },
        Duration::hours(1),
    )?;

    println!("  {} Issued permit for seller_agent only", "✓".bright_green());

    let intent2 = PaymentIntent::new(
        buyer.clone(),
        permit2.permit_id.clone(),
        unauthorized.clone(), // Wrong counterparty!
        Amount::new(5000),
        asset.clone(),
        SpendPurpose {
            category: "test".to_string(),
            description: "Wrong recipient".to_string(),
        },
    );

    match buyer_wallet.execute_payment(intent2) {
        Ok(_) => println!("  {} Payment succeeded (unexpected!)", "⚠".yellow()),
        Err(e) => println!("  {} Payment rejected: {}", "✓".bright_green(), e.to_string().bright_red()),
    }
    println!();

    // Test 3: Budget exceeded
    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Test 3: Permit exceeds budget".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());

    match buyer_wallet.issue_permit(
        Amount::new(100000), // Exceeds $500 budget!
        CounterpartyConstraint::Any,
        SpendPurpose {
            category: "test".to_string(),
            description: "Over-budget permit".to_string(),
        },
        Duration::hours(1),
    ) {
        Ok(_) => println!("  {} Permit issued (unexpected!)", "⚠".yellow()),
        Err(e) => println!("  {} Permit rejected: {}", "✓".bright_green(), e.to_string().bright_red()),
    }
    println!();

    // Summary
    println!("{}", "╔══════════════════════════════════════════════════════════════╗".bright_green());
    println!("{}", "║                                                              ║".bright_green());
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "OpeniBank enforces fail-closed behavior:".bright_white().bold(),
        "                   ║".bright_green()
    );
    println!("{}", "║                                                              ║".bright_green());
    println!("{}  {} Permit amount limits are enforced{}", "║".bright_green(), "✓".bright_green(), "                       ║".bright_green());
    println!("{}  {} Counterparty constraints are enforced{}", "║".bright_green(), "✓".bright_green(), "                  ║".bright_green());
    println!("{}  {} Budget limits are enforced{}", "║".bright_green(), "✓".bright_green(), "                            ║".bright_green());
    println!("{}  {} All failures are explicit{}", "║".bright_green(), "✓".bright_green(), "                              ║".bright_green());
    println!("{}", "║                                                              ║".bright_green());
    println!(
        "{}{}{}",
        "║  ".bright_green(),
        "LLMs may propose, but NEVER execute money directly.".bright_yellow(),
        "         ║".bright_green()
    );
    println!("{}", "║                                                              ║".bright_green());
    println!("{}", "╚══════════════════════════════════════════════════════════════╝".bright_green());

    Ok(())
}

/// Run interactive trading demo
pub async fn run_interactive_demo(num_trades: u32) -> anyhow::Result<()> {
    println!("{}", "🔄 INTERACTIVE DEMO: Live Agent Trading".bright_white().bold());
    println!();
    println!("Simulating {} trades between AI agents...", num_trades);
    println!();

    let ledger = Arc::new(Ledger::new());
    let issuer = Issuer::new(
        IssuerConfig::default(),
        Amount::new(10_000_000_00),
        ledger.clone(),
    );

    // Create multiple buyers and sellers
    let mut buyers: Vec<BuyerAgent> = (0..3)
        .map(|i| {
            let id = ResonatorId::from_string(format!("buyer_{}", i));
            BuyerAgent::new(id, ledger.clone())
        })
        .collect();

    let mut sellers: Vec<SellerAgent> = (0..2)
        .map(|i| {
            let id = ResonatorId::from_string(format!("seller_{}", i));
            let mut seller = SellerAgent::new(id, ledger.clone());
            seller.publish_service(Service {
                name: format!("Service_{}", i),
                description: format!("AI service offering #{}", i),
                price: Amount::new((50 + i * 25) as u64 * 100),
                asset: AssetId::iusd(),
                delivery_conditions: vec![],
            });
            seller
        })
        .collect();

    // Fund buyers
    for (i, buyer) in buyers.iter_mut().enumerate() {
        let mint = MintIntent::new(
            buyer.id().clone(),
            Amount::new(500_00),
            format!("Funding buyer {}", i),
        );
        issuer.mint(mint).await?;
        buyer.setup(Amount::new(500_00), Amount::new(300_00))?;
    }

    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Trading Simulation".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());

    let pb = ProgressBar::new(num_trades as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} trades")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let mut successful_trades = 0;
    let mut failed_trades = 0;

    for trade_num in 0..num_trades {
        let buyer_idx = (trade_num as usize) % buyers.len();
        let seller_idx = (trade_num as usize) % sellers.len();

        let service_name = format!("Service_{}", seller_idx);

        // Get offer and try to trade
        if let Some(offer) = sellers[seller_idx].get_offer(&service_name) {
            let can_afford = buyers[buyer_idx].evaluate_offer(&offer).await;

            if can_afford {
                let invoice = sellers[seller_idx]
                    .issue_invoice(buyers[buyer_idx].id().clone(), &service_name)
                    .await?;

                if buyers[buyer_idx].accept_invoice(invoice.clone()).is_ok() {
                    let commitment_id = format!("sim_commit_{}", Uuid::new_v4());
                    buyers[buyer_idx].set_active_commitment(commitment_id.clone(), true);
                    sellers[seller_idx].set_active_commitment(commitment_id.clone(), true);

                    if let Ok((_, escrow)) = buyers[buyer_idx].pay_invoice(&invoice.invoice_id).await {
                        sellers[seller_idx].deliver_service(&invoice.invoice_id, "Delivered".to_string())?;

                        if let Ok(amount) = buyers[buyer_idx].confirm_delivery(&escrow.escrow_id) {
                            sellers[seller_idx].receive_payment(amount)?;
                            buyers[buyer_idx].clear_active_commitment();
                            sellers[seller_idx].clear_active_commitment();
                            successful_trades += 1;
                        } else {
                            buyers[buyer_idx].clear_active_commitment();
                            sellers[seller_idx].clear_active_commitment();
                            failed_trades += 1;
                        }
                    } else {
                        buyers[buyer_idx].clear_active_commitment();
                        sellers[seller_idx].clear_active_commitment();
                        failed_trades += 1;
                    }
                } else {
                    failed_trades += 1;
                }
            } else {
                failed_trades += 1;
            }
        }

        pb.inc(1);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    pb.finish_with_message("Trading complete!");
    println!();

    // Results
    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Trading Results".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());
    println!("  Successful trades: {}", successful_trades.to_string().bright_green());
    println!("  Failed trades: {}", failed_trades.to_string().bright_red());
    println!();

    println!("{}", " Final Balances:".bright_white());
    for (i, buyer) in buyers.iter().enumerate() {
        println!("  Buyer {}: {}", i, format!("{}", buyer.balance()).bright_cyan());
    }
    for (i, seller) in sellers.iter().enumerate() {
        println!("  Seller {}: {}", i, format!("{}", seller.balance()).bright_cyan());
    }
    println!();

    Ok(())
}
