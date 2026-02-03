//! Agent commands - Run AI agents with LLM support

use std::sync::Arc;
use std::time::Duration;

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use openibank_agents::{AgentBrain, BuyerAgent, SellerAgent, Service};
use openibank_core::{Amount, AssetId, ResonatorId};
use openibank_issuer::{Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use openibank_llm::LLMRouter;

/// Run a buyer agent
pub async fn run_buyer(
    name: &str,
    llm_provider: Option<String>,
    model: &str,
    funding: u64,
) -> anyhow::Result<()> {
    println!("{}", format!("Starting Buyer Agent: {}", name).bright_white().bold());
    println!();

    // Set up LLM if provided
    if let Some(provider) = &llm_provider {
        std::env::set_var("OPENIBANK_LLM_PROVIDER", provider);
        if provider == "ollama" {
            std::env::set_var("OLLAMA_MODEL", model);
        }
    }

    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;

    println!("  LLM: {}", if llm_available {
        format!("{} ({})", llm_provider.as_deref().unwrap_or("auto"), model).bright_green()
    } else {
        "Deterministic mode".yellow()
    });
    println!();

    let ledger = Arc::new(Ledger::new());
    let issuer = Issuer::new(
        IssuerConfig::default(),
        Amount::new(10_000_000_00),
        ledger.clone(),
    );

    let buyer_id = ResonatorId::from_string(format!("buyer_{}", name));

    let brain = if llm_available {
        AgentBrain::with_llm(llm_router)
    } else {
        AgentBrain::deterministic()
    };

    let mut buyer = BuyerAgent::with_brain(buyer_id.clone(), ledger.clone(), brain);

    // Fund the buyer
    let mint = MintIntent::new(buyer_id.clone(), Amount::new(funding), "Initial funding");
    issuer.mint(mint).await?;
    buyer.setup(Amount::new(funding), Amount::new(funding / 2))?;

    println!("  {} Buyer agent ready", "✓".bright_green());
    println!("      ID: {}", buyer_id.0.bright_yellow());
    println!("      Balance: {}", format!("${:.2}", funding as f64 / 100.0).bright_cyan());
    println!("      Budget: {}", format!("${:.2}", (funding / 2) as f64 / 100.0).bright_cyan());
    println!();

    println!("{}", "Agent is running. In a full implementation, this would:".bright_black());
    println!("{}", "  • Listen for service offers".bright_black());
    println!("{}", "  • Evaluate offers using LLM reasoning".bright_black());
    println!("{}", "  • Issue permits and create escrows".bright_black());
    println!("{}", "  • Confirm deliveries and release payments".bright_black());
    println!();

    println!("{}", "Press Ctrl+C to stop the agent.".bright_black());

    // Keep running (in a real implementation, this would be an event loop)
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

/// Run a seller agent
pub async fn run_seller(name: &str, service_name: &str, price: u64) -> anyhow::Result<()> {
    println!("{}", format!("Starting Seller Agent: {}", name).bright_white().bold());
    println!();

    let ledger = Arc::new(Ledger::new());
    let seller_id = ResonatorId::from_string(format!("seller_{}", name));

    let mut seller = SellerAgent::new(seller_id.clone(), ledger);

    // Publish the service
    let service = Service {
        name: service_name.to_string(),
        description: format!("AI service: {}", service_name),
        price: Amount::new(price),
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Service completion verification".to_string()],
    };

    seller.publish_service(service);

    println!("  {} Seller agent ready", "✓".bright_green());
    println!("      ID: {}", seller_id.0.bright_yellow());
    println!();
    println!("  {} Published Service:", "📦".bright_blue());
    println!("      Name: {}", service_name.bright_cyan());
    println!("      Price: {}", format!("${:.2}", price as f64 / 100.0).bright_cyan());
    println!();

    println!("{}", "Agent is running. In a full implementation, this would:".bright_black());
    println!("{}", "  • Advertise services to buyers".bright_black());
    println!("{}", "  • Issue invoices on request".bright_black());
    println!("{}", "  • Deliver services and provide proof".bright_black());
    println!("{}", "  • Receive payments from escrow".bright_black());
    println!();

    println!("{}", "Press Ctrl+C to stop the agent.".bright_black());

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

/// Run the full agent marketplace
pub async fn run_marketplace(
    num_buyers: u32,
    num_sellers: u32,
    llm_provider: Option<String>,
) -> anyhow::Result<()> {
    println!("{}", "🏪 Starting Agent Marketplace".bright_white().bold());
    println!();
    println!("  Buyers: {}", num_buyers.to_string().bright_cyan());
    println!("  Sellers: {}", num_sellers.to_string().bright_cyan());
    println!();

    // Set up LLM if provided
    if let Some(provider) = &llm_provider {
        std::env::set_var("OPENIBANK_LLM_PROVIDER", provider);
    }

    let llm_router = LLMRouter::from_env();
    let llm_available = llm_router.is_available().await;

    println!("  LLM: {}", if llm_available {
        llm_provider.as_deref().unwrap_or("auto").bright_green()
    } else {
        "Deterministic mode".yellow()
    });
    println!();

    let ledger = Arc::new(Ledger::new());
    let issuer = Issuer::new(
        IssuerConfig::default(),
        Amount::new(100_000_000_00), // $1M reserve
        ledger.clone(),
    );

    // Create sellers
    let mut sellers: Vec<SellerAgent> = Vec::new();
    println!("{}", "Creating sellers...".bright_white());

    for i in 0..num_sellers {
        let seller_id = ResonatorId::from_string(format!("seller_{}", i));
        let mut seller = SellerAgent::new(seller_id.clone(), ledger.clone());

        let services = vec![
            ("Data Analysis", 150_00u64, "AI-powered data analysis service"),
            ("Code Review", 100_00u64, "Automated code review and suggestions"),
            ("Translation", 50_00u64, "Multi-language translation service"),
            ("Image Processing", 200_00u64, "AI image enhancement and processing"),
        ];

        let (name, price, desc) = &services[i as usize % services.len()];
        seller.publish_service(Service {
            name: name.to_string(),
            description: desc.to_string(),
            price: Amount::new(*price),
            asset: AssetId::iusd(),
            delivery_conditions: vec![],
        });

        println!("  {} Seller {}: {} @ ${:.2}",
            "✓".bright_green(),
            i,
            name.bright_cyan(),
            *price as f64 / 100.0
        );

        sellers.push(seller);
    }
    println!();

    // Create buyers
    let mut buyers: Vec<BuyerAgent> = Vec::new();
    println!("{}", "Creating buyers...".bright_white());

    for i in 0..num_buyers {
        let buyer_id = ResonatorId::from_string(format!("buyer_{}", i));
        let brain = if llm_available {
            AgentBrain::with_llm(LLMRouter::from_env())
        } else {
            AgentBrain::deterministic()
        };

        let mut buyer = BuyerAgent::with_brain(buyer_id.clone(), ledger.clone(), brain);

        // Fund buyer
        let funding = 500_00u64; // $500
        let mint = MintIntent::new(buyer_id.clone(), Amount::new(funding), "Marketplace funding");
        issuer.mint(mint).await?;
        buyer.setup(Amount::new(funding), Amount::new(funding / 2))?;

        println!("  {} Buyer {}: ${:.2} balance",
            "✓".bright_green(),
            i,
            funding as f64 / 100.0
        );

        buyers.push(buyer);
    }
    println!();

    // Run marketplace simulation
    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Marketplace Trading Session".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());
    println!();

    let num_rounds = 10;
    let pb = ProgressBar::new(num_rounds);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] Round {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let mut total_trades = 0;
    let mut total_volume = 0u64;

    for round in 0..num_rounds {
        // Each round, random buyer tries to buy from random seller
        let buyer_idx = (round as usize) % buyers.len();
        let seller_idx = (round as usize) % sellers.len();

        // Get service info first (clone to avoid borrow issues)
        let service_name = {
            let seller = &sellers[seller_idx];
            seller.services().first().map(|s| s.name.clone())
        };

        if let Some(svc_name) = service_name {
            let offer = {
                let seller = &sellers[seller_idx];
                seller.get_offer(&svc_name)
            };

            if let Some(offer) = offer {
                let can_afford = buyers[buyer_idx].evaluate_offer(&offer).await;

                if can_afford {
                    let buyer_id = buyers[buyer_idx].id().clone();

                    // Issue invoice
                    if let Ok(invoice) = sellers[seller_idx].issue_invoice(buyer_id, &svc_name).await {
                        let invoice_id = invoice.invoice_id.clone();

                        if buyers[buyer_idx].accept_invoice(invoice).is_ok() {
                            if let Ok((_, escrow)) = buyers[buyer_idx].pay_invoice(&invoice_id).await {
                                let escrow_id = escrow.escrow_id.clone();

                                if sellers[seller_idx].deliver_service(&invoice_id, "Delivered".to_string()).is_ok() {
                                    if let Ok(amount) = buyers[buyer_idx].confirm_delivery(&escrow_id) {
                                        sellers[seller_idx].receive_payment(amount)?;
                                        total_trades += 1;
                                        total_volume += amount.0;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        pb.inc(1);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    pb.finish_with_message("Session complete");
    println!();

    // Results
    println!("{}", "━".repeat(60).bright_black());
    println!("{}", " Marketplace Results".bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());
    println!();
    println!("  Total Trades: {}", total_trades.to_string().bright_green());
    println!("  Total Volume: {}", format!("${:.2}", total_volume as f64 / 100.0).bright_cyan());
    println!();

    println!("{}", " Final Balances:".bright_white());
    println!();
    println!("  {}", "Buyers:".bright_white());
    for (i, buyer) in buyers.iter().enumerate() {
        println!("    Buyer {}: {}", i, format!("{}", buyer.balance()).bright_cyan());
    }
    println!();
    println!("  {}", "Sellers:".bright_white());
    for (i, seller) in sellers.iter().enumerate() {
        println!("    Seller {}: {}", i, format!("{}", seller.balance()).bright_cyan());
    }
    println!();

    Ok(())
}
