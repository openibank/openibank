use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use openibank_domain::card::{write_cards, CardFormat};
use openibank_domain::load_receipt;
use openibank_maple::MapleWorldlineRuntime;
use openibank_tui::run_demo_tui;

#[derive(Parser)]
#[command(name = "openibank")]
#[command(author = "OpenIBank Contributors")]
#[command(version)]
#[command(about = "Maple-powered OpenIBank CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch Maple-driven local simulation TUI.
    Demo {
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long)]
        export: Option<PathBuf>,
    },
    /// Receipt operations.
    Receipt {
        #[command(subcommand)]
        action: ReceiptCommands,
    },
    /// Wallet operations (EVM vault, balances, WalletConnect QR).
    Wallet {
        #[command(subcommand)]
        action: WalletCommands,
    },
    /// WorldLine operations.
    Worldline {
        #[command(subcommand)]
        action: WorldlineCommands,
    },
    /// Export a persisted run bundle.
    Export {
        #[arg(long)]
        run: String,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum ReceiptCommands {
    /// Verify signature + worldline pointer proof.
    Verify {
        #[arg(long)]
        file: PathBuf,
    },
    /// Generate ASCII/SVG card for a receipt.
    Card {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "both")]
        format: CardFormatArg,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CardFormatArg {
    Ascii,
    Svg,
    Html,
    Both,
    All,
}

impl From<CardFormatArg> for CardFormat {
    fn from(value: CardFormatArg) -> Self {
        match value {
            CardFormatArg::Ascii => CardFormat::Ascii,
            CardFormatArg::Svg => CardFormat::Svg,
            CardFormatArg::Html => CardFormat::Html,
            CardFormatArg::Both => CardFormat::Both,
            CardFormatArg::All => CardFormat::All,
        }
    }
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Show EVM address and ed25519 pubkey for an agent.
    Identity {
        /// Agent name (e.g. buyer-01)
        #[arg(long, default_value = "demo-agent")]
        agent: String,
    },
    /// Show simulated on-chain balances for an agent.
    Balances {
        #[arg(long, default_value = "demo-agent")]
        agent: String,
    },
    /// Generate WalletConnect v2 QR code HTML for an agent.
    Qr {
        #[arg(long, default_value = "demo-agent")]
        agent: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum WorldlineCommands {
    /// Describe the WorldLine model.
    Info,
    /// Verify a WorldLine slice file against a receipt.
    Verify {
        #[arg(long)]
        receipt: PathBuf,
        #[arg(long)]
        slice: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Demo { seed, export } => {
            let result = run_demo_tui(seed, export).await?;
            println!("run completed: {}", result.run_id);
        }
        Commands::Receipt { action } => match action {
            ReceiptCommands::Verify { file } => {
                let receipt = load_receipt(&file)?;
                receipt.verify()?;
                verify_worldline_pointer(&file, &receipt)?;
                println!("VERIFIED");
            }
            ReceiptCommands::Card { file, out, format } => {
                let receipt = load_receipt(&file)?;
                let files = write_cards(&receipt, &out, format.into())?;
                for path in files {
                    println!("{}", path.display());
                }
            }
        },
        Commands::Wallet { action } => match action {
            WalletCommands::Identity { agent } => {
                let vault = openibank_wallet::Vault::for_agent(&agent);
                println!("Agent:        {}", agent);
                println!("EVM address:  {}", vault.evm_address());
                println!("Ed25519 pub:  {}", vault.ed25519_pubkey_hex());
                println!("WC URI:       {}", vault.walletconnect_uri());
            }
            WalletCommands::Balances { agent } => {
                let vault = openibank_wallet::Vault::for_agent(&agent);
                let balances = openibank_wallet::simulate_onchain_balances(&vault);
                println!("On-chain balances for {} ({}):", agent, vault.evm_address());
                for b in balances {
                    println!("  {:10} on {:15}: {}", b.symbol, b.chain, b.to_display());
                }
            }
            WalletCommands::Qr { agent, out } => {
                let vault = openibank_wallet::Vault::for_agent(&agent);
                let html = vault.identity_card_html();
                let out_path = out.unwrap_or_else(|| PathBuf::from(format!("{}_wallet.html", agent)));
                std::fs::write(&out_path, &html)?;
                println!("WalletConnect card written to: {}", out_path.display());
            }
        },
        Commands::Worldline { action } => match action {
            WorldlineCommands::Info => {
                println!("OpeniBank WorldLine — Maple-powered append-only event ledger");
                println!();
                println!("Architecture:");
                println!("  Intent → Commitment → Consequence → Receipt");
                println!("  Each event is blake3 hash-chained to the previous.");
                println!("  Conservation proof: sum of net positions = 0.");
                println!();
                println!("Backend modes:");
                println!("  OPENIBANK_MODE=local-sim    → LocalSimWorldLine (embedded)");
                println!("  OPENIBANK_MODE=maple-native → Maple WorldLine (production)");
            }
            WorldlineCommands::Verify { receipt, slice } => {
                let rcpt = openibank_domain::load_receipt(&receipt)?;
                verify_worldline_pointer_explicit(&slice, &rcpt)?;
                println!("WorldLine proof VERIFIED for receipt {}", rcpt.tx_id);
            }
        },
        Commands::Export { run, out } => {
            let path = MapleWorldlineRuntime::export_persisted(&run, &out)?;
            println!("{}", path.display());
        }
    }

    Ok(())
}

fn verify_worldline_pointer(
    receipt_path: &Path,
    receipt: &openibank_domain::Receipt,
) -> anyhow::Result<()> {
    let slice_path = locate_worldline_slice(receipt_path).ok_or_else(|| {
        anyhow::anyhow!(
            "worldline_slice.json not found near receipt, cannot validate WorldLine pointer"
        )
    })?;

    let data = std::fs::read(&slice_path)?;
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&data)?;
    let matched = entries.iter().any(|entry| {
        let event_id = entry
            .get("event_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let hash = entry
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let worldline = entry
            .get("worldline_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        event_id == receipt.worldline_event_id
            && hash == receipt.worldline_event_hash
            && worldline == receipt.worldline_id
    });

    if matched {
        Ok(())
    } else {
        anyhow::bail!("receipt proof not found in worldline slice")
    }
}

fn locate_worldline_slice(receipt_path: &Path) -> Option<PathBuf> {
    let parent = receipt_path.parent()?;
    let direct = parent.join("worldline_slice.json");
    if direct.exists() {
        return Some(direct);
    }
    let upper = parent.parent()?.join("worldline_slice.json");
    if upper.exists() {
        return Some(upper);
    }
    None
}

fn verify_worldline_pointer_explicit(
    slice_path: &Path,
    receipt: &openibank_domain::Receipt,
) -> anyhow::Result<()> {
    let data = std::fs::read(slice_path)?;
    let entries: Vec<serde_json::Value> = serde_json::from_slice(&data)?;
    let matched = entries.iter().any(|entry| {
        let event_id = entry.get("event_id").and_then(|v| v.as_str()).unwrap_or_default();
        let hash = entry.get("hash").and_then(|v| v.as_str()).unwrap_or_default();
        let worldline = entry.get("worldline_id").and_then(|v| v.as_str()).unwrap_or_default();
        event_id == receipt.worldline_event_id
            && hash == receipt.worldline_event_hash
            && worldline == receipt.worldline_id
    });
    if matched {
        Ok(())
    } else {
        anyhow::bail!("receipt proof not found in worldline slice {}", slice_path.display())
    }
}
