//! OpeniBank CLI - Command-line interface for AI Agent Banking
//!
//! A powerful CLI for interacting with OpeniBank services:
//! - Demo: Run viral demos showcasing agent commerce
//! - Wallet: Manage agent wallets
//! - Issuer: Interact with the IUSD issuer
//! - Agent: Run AI agents with LLM support
//!
//! # Quick Start
//!
//! ```bash
//! # Run the viral demo
//! openibank demo
//!
//! # Start the issuer service
//! openibank issuer start
//!
//! # Create a new wallet
//! openibank wallet create --name my-agent
//!
//! # Run an agent with Ollama
//! openibank agent run --llm ollama --model llama3
//! ```

use clap::{Parser, Subcommand};
use colored::*;

mod commands;
mod display;

use commands::{demo, issuer, wallet, agent, receipt};

/// OpeniBank CLI - Programmable Wallets + Receipts for AI Agents
#[derive(Parser)]
#[command(name = "openibank")]
#[command(author = "OpeniBank Contributors")]
#[command(version)]
#[command(about = "AI-agent-only banking with programmable wallets and verifiable receipts", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run viral demos showcasing OpeniBank capabilities
    Demo {
        #[command(subcommand)]
        demo_type: DemoCommands,
    },

    /// Manage agent wallets
    Wallet {
        #[command(subcommand)]
        action: WalletCommands,
    },

    /// Interact with the IUSD issuer service
    Issuer {
        #[command(subcommand)]
        action: IssuerCommands,
    },

    /// Run AI agents with LLM support
    Agent {
        #[command(subcommand)]
        action: AgentCommands,
    },

    /// Verify and inspect receipts
    Receipt {
        #[command(subcommand)]
        action: ReceiptCommands,
    },

    /// Show system information and status
    Status,
}

#[derive(Subcommand)]
enum DemoCommands {
    /// Run the complete asset cycle demo (most viral!)
    #[command(alias = "viral")]
    Full {
        /// Use LLM for agent reasoning
        #[arg(long)]
        llm: Option<String>,

        /// LLM model to use
        #[arg(long, default_value = "llama3")]
        model: String,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run the fail-closed safety demo
    Safety,

    /// Run interactive trading between agents
    Interactive {
        /// Number of trades to simulate
        #[arg(short, long, default_value = "5")]
        trades: u32,
    },
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Create a new agent wallet
    Create {
        /// Wallet name
        #[arg(short, long)]
        name: String,

        /// Initial funding amount (in cents)
        #[arg(short, long, default_value = "100000")]
        funding: u64,

        /// Budget limit (in cents)
        #[arg(short, long, default_value = "50000")]
        budget: u64,
    },

    /// Show wallet balance and info
    Info {
        /// Wallet name or ID
        name: String,
    },

    /// List all wallets
    List,

    /// Transfer between wallets
    Transfer {
        /// Source wallet
        #[arg(long)]
        from: String,

        /// Destination wallet
        #[arg(long)]
        to: String,

        /// Amount (in cents)
        #[arg(long)]
        amount: u64,

        /// Purpose description
        #[arg(long, default_value = "CLI transfer")]
        purpose: String,
    },
}

#[derive(Subcommand)]
enum IssuerCommands {
    /// Start the issuer HTTP service
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Reserve cap (in cents)
        #[arg(long, default_value = "100000000")]
        reserve_cap: u64,
    },

    /// Initialize the issuer (if using remote service)
    Init {
        /// Issuer service URL
        #[arg(long, default_value = "http://localhost:3000")]
        url: String,

        /// Reserve cap (in cents)
        #[arg(long, default_value = "100000000")]
        reserve_cap: u64,
    },

    /// Mint IUSD to an account
    Mint {
        /// Target account
        #[arg(long)]
        to: String,

        /// Amount (in cents)
        #[arg(long)]
        amount: u64,

        /// Reason for minting
        #[arg(long, default_value = "CLI mint")]
        reason: String,

        /// Issuer service URL
        #[arg(long, default_value = "http://localhost:3000")]
        url: String,
    },

    /// Get current supply info
    Supply {
        /// Issuer service URL
        #[arg(long, default_value = "http://localhost:3000")]
        url: String,
    },

    /// Get recent receipts
    Receipts {
        /// Number of receipts to show
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Issuer service URL
        #[arg(long, default_value = "http://localhost:3000")]
        url: String,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Run a buyer agent
    Buyer {
        /// Agent name
        #[arg(short, long, default_value = "buyer")]
        name: String,

        /// LLM provider (ollama, openai, anthropic)
        #[arg(long)]
        llm: Option<String>,

        /// LLM model
        #[arg(long, default_value = "llama3")]
        model: String,

        /// Initial funding
        #[arg(long, default_value = "100000")]
        funding: u64,
    },

    /// Run a seller agent
    Seller {
        /// Agent name
        #[arg(short, long, default_value = "seller")]
        name: String,

        /// Service name to offer
        #[arg(long, default_value = "AI Data Feed")]
        service: String,

        /// Service price (in cents)
        #[arg(long, default_value = "20000")]
        price: u64,
    },

    /// Run the full agent marketplace
    Marketplace {
        /// Number of buyers
        #[arg(long, default_value = "3")]
        buyers: u32,

        /// Number of sellers
        #[arg(long, default_value = "2")]
        sellers: u32,

        /// LLM provider
        #[arg(long)]
        llm: Option<String>,
    },
}

#[derive(Subcommand)]
enum ReceiptCommands {
    /// Verify a receipt
    Verify {
        /// Receipt JSON (file path or inline)
        receipt: String,
    },

    /// Inspect receipt details
    Inspect {
        /// Receipt JSON (file path or inline)
        receipt: String,
    },

    /// Compare two receipts
    Diff {
        /// First receipt
        receipt1: String,

        /// Second receipt
        receipt2: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Print banner for all commands
    print_banner();

    match cli.command {
        Commands::Demo { demo_type } => match demo_type {
            DemoCommands::Full { llm, model, verbose } => {
                demo::run_full_demo(llm, model, verbose).await?;
            }
            DemoCommands::Safety => {
                demo::run_safety_demo().await?;
            }
            DemoCommands::Interactive { trades } => {
                demo::run_interactive_demo(trades).await?;
            }
        },

        Commands::Wallet { action } => match action {
            WalletCommands::Create { name, funding, budget } => {
                wallet::create_wallet(&name, funding, budget).await?;
            }
            WalletCommands::Info { name } => {
                wallet::show_wallet_info(&name).await?;
            }
            WalletCommands::List => {
                wallet::list_wallets().await?;
            }
            WalletCommands::Transfer { from, to, amount, purpose } => {
                wallet::transfer(&from, &to, amount, &purpose).await?;
            }
        },

        Commands::Issuer { action } => match action {
            IssuerCommands::Start { port, reserve_cap } => {
                issuer::start_issuer(port, reserve_cap).await?;
            }
            IssuerCommands::Init { url, reserve_cap } => {
                issuer::init_issuer(&url, reserve_cap).await?;
            }
            IssuerCommands::Mint { to, amount, reason, url } => {
                issuer::mint(&url, &to, amount, &reason).await?;
            }
            IssuerCommands::Supply { url } => {
                issuer::show_supply(&url).await?;
            }
            IssuerCommands::Receipts { limit, url } => {
                issuer::show_receipts(&url, limit).await?;
            }
        },

        Commands::Agent { action } => match action {
            AgentCommands::Buyer { name, llm, model, funding } => {
                agent::run_buyer(&name, llm, &model, funding).await?;
            }
            AgentCommands::Seller { name, service, price } => {
                agent::run_seller(&name, &service, price).await?;
            }
            AgentCommands::Marketplace { buyers, sellers, llm } => {
                agent::run_marketplace(buyers, sellers, llm).await?;
            }
        },

        Commands::Receipt { action } => match action {
            ReceiptCommands::Verify { receipt } => {
                receipt::verify_receipt(&receipt).await?;
            }
            ReceiptCommands::Inspect { receipt } => {
                receipt::inspect_receipt(&receipt).await?;
            }
            ReceiptCommands::Diff { receipt1, receipt2 } => {
                receipt::diff_receipts(&receipt1, &receipt2).await?;
            }
        },

        Commands::Status => {
            show_status().await?;
        }
    }

    Ok(())
}

fn print_banner() {
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║                                                                  ║".bright_cyan());
    println!("{}{}{}",
        "║  ".bright_cyan(),
        "OpeniBank".bright_white().bold(),
        " - Programmable Wallets + Receipts for AI Agents    ║".bright_cyan()
    );
    println!("{}", "║                                                                  ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
}

async fn show_status() -> anyhow::Result<()> {
    println!("{}", "System Status".bright_white().bold());
    println!("{}", "─".repeat(50));

    // Check issuer service
    print!("  Issuer Service (localhost:3000): ");
    match reqwest::get("http://localhost:3000/health").await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "● Online".bright_green());
        }
        _ => {
            println!("{}", "○ Offline".bright_red());
        }
    }

    // Check LLM providers
    println!();
    println!("{}", "LLM Providers:".bright_white());

    // Check Ollama
    print!("  Ollama (localhost:11434): ");
    match reqwest::get("http://localhost:11434/api/tags").await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "● Available".bright_green());
        }
        _ => {
            println!("{}", "○ Not running".yellow());
        }
    }

    // Check OpenAI
    print!("  OpenAI API: ");
    if std::env::var("OPENAI_API_KEY").is_ok() {
        println!("{}", "● Configured".bright_green());
    } else {
        println!("{}", "○ Not configured".yellow());
    }

    // Check Anthropic
    print!("  Anthropic API: ");
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        println!("{}", "● Configured".bright_green());
    } else {
        println!("{}", "○ Not configured".yellow());
    }

    println!();
    println!("{}", "Quick Start:".bright_white());
    println!("  {} - Run the viral demo", "openibank demo full".bright_cyan());
    println!("  {} - Start issuer service", "openibank issuer start".bright_cyan());
    println!("  {} - Run with Ollama", "openibank demo full --llm ollama".bright_cyan());

    Ok(())
}
