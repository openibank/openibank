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
    Both,
}

impl From<CardFormatArg> for CardFormat {
    fn from(value: CardFormatArg) -> Self {
        match value {
            CardFormatArg::Ascii => CardFormat::Ascii,
            CardFormatArg::Svg => CardFormat::Svg,
            CardFormatArg::Both => CardFormat::Both,
        }
    }
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
