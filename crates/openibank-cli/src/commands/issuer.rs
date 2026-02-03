//! Issuer commands - Interact with the IUSD issuer service

use colored::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct InitRequest {
    reserve_cap: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitResponse {
    success: bool,
    issuer_id: String,
    public_key: String,
    reserve_cap: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct MintRequest {
    to: String,
    amount: u64,
    reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MintResponse {
    success: bool,
    receipt_id: String,
    amount: u64,
    new_supply: u64,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SupplyResponse {
    total_supply: u64,
    remaining_mintable: u64,
    is_halted: bool,
}

/// Start the issuer HTTP service
pub async fn start_issuer(port: u16, reserve_cap: u64) -> anyhow::Result<()> {
    println!("{}", "Starting IUSD Issuer Service...".bright_white().bold());
    println!();
    println!("  Port: {}", port.to_string().bright_cyan());
    println!("  Reserve Cap: {}", format!("${:.2}", reserve_cap as f64 / 100.0).bright_cyan());
    println!();

    println!("{}", "To use the issuer service:".bright_white());
    println!();
    println!("  {} Initialize:", "1.".bright_yellow());
    println!("     curl -X POST http://localhost:{}/v1/issuer/init \\", port);
    println!("       -H 'Content-Type: application/json' \\");
    println!("       -d '{{\"reserve_cap\": {}}}'", reserve_cap);
    println!();
    println!("  {} Mint IUSD:", "2.".bright_yellow());
    println!("     curl -X POST http://localhost:{}/v1/issuer/mint \\", port);
    println!("       -H 'Content-Type: application/json' \\");
    println!("       -d '{{\"to\": \"agent_1\", \"amount\": 100000}}'");
    println!();
    println!("  {} Check supply:", "3.".bright_yellow());
    println!("     curl http://localhost:{}/v1/issuer/supply", port);
    println!();

    // Set environment and run the issuer resonator
    std::env::set_var("ISSUER_PORT", port.to_string());
    std::env::set_var("ISSUER_RESERVE_CAP", reserve_cap.to_string());

    println!("{}", "━".repeat(60).bright_black());
    println!("{} Issuer service starting on port {}...", "→".bright_blue(), port);
    println!("{}", "━".repeat(60).bright_black());
    println!();

    // Actually start the service by running the issuer resonator main
    // For now, we'll spawn it as a subprocess
    let status = std::process::Command::new("cargo")
        .args(["run", "--package", "openibank-issuer-resonator"])
        .current_dir(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
        .status()?;

    if !status.success() {
        anyhow::bail!("Issuer service exited with error");
    }

    Ok(())
}

/// Initialize the issuer via HTTP
pub async fn init_issuer(url: &str, reserve_cap: u64) -> anyhow::Result<()> {
    println!("{}", "Initializing IUSD Issuer...".bright_white().bold());
    println!();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/issuer/init", url))
        .json(&InitRequest { reserve_cap })
        .send()
        .await?;

    if resp.status().is_success() {
        let init: InitResponse = resp.json().await?;
        println!("  {} Issuer initialized", "✓".bright_green());
        println!("      Issuer ID: {}", init.issuer_id.bright_cyan());
        println!("      Public Key: {}...", &init.public_key[..32].bright_yellow());
        println!("      Reserve Cap: {}", format!("${:.2}", init.reserve_cap as f64 / 100.0).bright_cyan());
    } else {
        let error: serde_json::Value = resp.json().await?;
        println!("  {} Failed to initialize: {}", "✗".bright_red(), error["message"]);
    }

    Ok(())
}

/// Mint IUSD to an account
pub async fn mint(url: &str, to: &str, amount: u64, reason: &str) -> anyhow::Result<()> {
    println!("{}", "Minting IUSD...".bright_white().bold());
    println!();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/issuer/mint", url))
        .json(&MintRequest {
            to: to.to_string(),
            amount,
            reason: Some(reason.to_string()),
        })
        .send()
        .await?;

    if resp.status().is_success() {
        let mint: MintResponse = resp.json().await?;
        println!("  {} Minted {} to {}", "✓".bright_green(), format!("${:.2}", mint.amount as f64 / 100.0).bright_cyan(), to.bright_cyan());
        println!("      Receipt ID: {}", mint.receipt_id.bright_yellow());
        println!("      New Supply: {}", format!("${:.2}", mint.new_supply as f64 / 100.0).bright_cyan());
        println!("      Signature: {}...", &mint.signature[..32].bright_yellow());
    } else {
        let error: serde_json::Value = resp.json().await?;
        println!("  {} Failed to mint: {}", "✗".bright_red(), error["message"]);
    }

    Ok(())
}

/// Show current supply info
pub async fn show_supply(url: &str) -> anyhow::Result<()> {
    println!("{}", "IUSD Supply Info".bright_white().bold());
    println!("{}", "─".repeat(40));

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/v1/issuer/supply", url))
        .send()
        .await?;

    if resp.status().is_success() {
        let supply: SupplyResponse = resp.json().await?;
        println!("  Total Supply:      {}", format!("${:.2}", supply.total_supply as f64 / 100.0).bright_cyan());
        println!("  Remaining Mintable: {}", format!("${:.2}", supply.remaining_mintable as f64 / 100.0).bright_cyan());
        println!(
            "  Status:            {}",
            if supply.is_halted {
                "HALTED".bright_red()
            } else {
                "Active".bright_green()
            }
        );
    } else {
        let error: serde_json::Value = resp.json().await?;
        println!("  {} Error: {}", "✗".bright_red(), error["message"]);
    }

    Ok(())
}

/// Show recent receipts
pub async fn show_receipts(url: &str, limit: usize) -> anyhow::Result<()> {
    println!("{}", "Recent Issuer Receipts".bright_white().bold());
    println!("{}", "─".repeat(60));

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/v1/issuer/receipts?limit={}", url, limit))
        .send()
        .await?;

    if resp.status().is_success() {
        let data: serde_json::Value = resp.json().await?;
        let empty_vec = vec![];
        let receipts = data["receipts"].as_array().unwrap_or(&empty_vec);

        if receipts.is_empty() {
            println!("  No receipts found.");
        } else {
            for receipt in receipts {
                println!();
                println!("  {} {}", "Receipt:".bright_white(), receipt["receipt_id"].as_str().unwrap_or("").bright_yellow());
                println!("      Operation: {}", receipt["operation"].to_string().bright_cyan());
                println!("      Amount: ${:.2}", receipt["amount"].as_u64().unwrap_or(0) as f64 / 100.0);
                println!("      Target: {}", receipt["target"].as_str().unwrap_or(""));
            }
        }
    } else {
        let error: serde_json::Value = resp.json().await?;
        println!("  {} Error: {}", "✗".bright_red(), error["message"]);
    }

    Ok(())
}
