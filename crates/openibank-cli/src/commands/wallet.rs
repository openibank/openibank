//! Wallet commands - Manage agent wallets

use colored::*;
use openibank_core::{Amount, AssetId, BudgetPolicy, ResonatorId, Wallet};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

// In-memory wallet storage for demo purposes
// In production, this would be persisted
static WALLETS: OnceLock<RwLock<HashMap<String, WalletState>>> = OnceLock::new();

fn wallets() -> &'static RwLock<HashMap<String, WalletState>> {
    WALLETS.get_or_init(|| RwLock::new(HashMap::new()))
}

struct WalletState {
    wallet: Wallet,
    name: String,
}

/// Create a new agent wallet
pub async fn create_wallet(name: &str, funding: u64, budget: u64) -> anyhow::Result<()> {
    println!("{}", "Creating Agent Wallet...".bright_white().bold());
    println!();

    let id = ResonatorId::from_string(format!("agent_{}", name));
    let mut wallet = Wallet::new(id.clone());

    // Credit initial funds
    wallet.credit(&AssetId::iusd(), Amount::new(funding))?;

    // Set up budget
    let budget_policy = BudgetPolicy::new(id.clone(), Amount::new(budget));
    wallet.set_budget(budget_policy)?;

    // Store wallet
    let mut wallets = wallets().write().await;
    wallets.insert(
        name.to_string(),
        WalletState {
            wallet,
            name: name.to_string(),
        },
    );

    println!("  {} Wallet created: {}", "✓".bright_green(), name.bright_cyan());
    println!("      ID: {}", id.0.bright_yellow());
    println!("      Balance: {}", format!("${:.2}", funding as f64 / 100.0).bright_cyan());
    println!("      Budget: {}", format!("${:.2}", budget as f64 / 100.0).bright_cyan());
    println!();

    println!("{}", "Note: Wallets are stored in memory for this demo.".bright_black());
    println!("{}", "In production, wallets would be persisted.".bright_black());

    Ok(())
}

/// Show wallet info
pub async fn show_wallet_info(name: &str) -> anyhow::Result<()> {
    println!("{}", format!("Wallet: {}", name).bright_white().bold());
    println!("{}", "─".repeat(40));

    let wallets = wallets().read().await;

    if let Some(state) = wallets.get(name) {
        let balance = state.wallet.balance(&AssetId::iusd());
        let budget = state.wallet.budget();

        println!("  Name: {}", state.name.bright_cyan());
        println!("  ID: {}", state.wallet.owner().0.bright_yellow());
        println!("  Balance: {}", format!("{}", balance).bright_cyan());

        if let Some(b) = budget {
            let remaining = b.max_total.checked_sub(b.spent_total).unwrap_or(Amount::zero());
            println!("  Budget Max: {}", format!("{}", b.max_total).bright_cyan());
            println!("  Budget Spent: {}", format!("{}", b.spent_total).bright_cyan());
            println!("  Budget Remaining: {}", format!("{}", remaining).bright_cyan());
        }
    } else {
        println!("  {} Wallet not found: {}", "✗".bright_red(), name);
        println!();
        println!("  Create a wallet with: {}", format!("openibank wallet create --name {}", name).bright_cyan());
    }

    Ok(())
}

/// List all wallets
pub async fn list_wallets() -> anyhow::Result<()> {
    println!("{}", "Agent Wallets".bright_white().bold());
    println!("{}", "─".repeat(60));

    let wallets = wallets().read().await;

    if wallets.is_empty() {
        println!("  No wallets found.");
        println!();
        println!("  Create a wallet with: {}", "openibank wallet create --name my-agent".bright_cyan());
    } else {
        println!();
        println!("  {:<20} {:<15} {:<15}", "NAME", "BALANCE", "BUDGET LEFT");
        println!("  {}", "─".repeat(50));

        for (name, state) in wallets.iter() {
            let balance = state.wallet.balance(&AssetId::iusd());
            let remaining = state.wallet.budget()
                .map(|b| b.max_total.checked_sub(b.spent_total).unwrap_or(Amount::zero()))
                .unwrap_or(Amount::zero());

            println!(
                "  {:<20} {:<15} {:<15}",
                name.bright_cyan(),
                format!("{}", balance),
                format!("{}", remaining)
            );
        }
    }

    Ok(())
}

/// Transfer between wallets
pub async fn transfer(from: &str, to: &str, amount: u64, _purpose: &str) -> anyhow::Result<()> {
    println!("{}", "Executing Transfer...".bright_white().bold());
    println!();

    println!("  From: {}", from.bright_cyan());
    println!("  To: {}", to.bright_cyan());
    println!("  Amount: {}", format!("${:.2}", amount as f64 / 100.0).bright_cyan());
    println!();

    let mut wallets = wallets().write().await;

    // Check both wallets exist
    if !wallets.contains_key(from) {
        anyhow::bail!("Source wallet '{}' not found", from);
    }
    if !wallets.contains_key(to) {
        anyhow::bail!("Destination wallet '{}' not found", to);
    }

    // For demo purposes, use balance compartment directly
    // In production, this would go through the commitment gate

    let from_balance = {
        let from_state = wallets.get(from).unwrap();
        from_state.wallet.balance(&AssetId::iusd())
    };

    if from_balance.0 < amount {
        anyhow::bail!("Insufficient balance: have ${:.2}, need ${:.2}",
            from_balance.0 as f64 / 100.0,
            amount as f64 / 100.0
        );
    }

    // For now, we'll simulate transfer by adjusting balances
    // This is a simplified demo - real transfers go through commitment gate
    println!("  {} Transfer simulated (demo mode)", "→".bright_blue());
    println!("      In production, this would create a commitment receipt.");
    println!();

    // Simulate: show what would happen
    println!("  Simulated new balances:");
    println!("    {}: ${:.2}", from.bright_cyan(), (from_balance.0 - amount) as f64 / 100.0);

    let to_balance = {
        let to_state = wallets.get(to).unwrap();
        to_state.wallet.balance(&AssetId::iusd())
    };
    println!("    {}: ${:.2}", to.bright_cyan(), (to_balance.0 + amount) as f64 / 100.0);

    Ok(())
}
