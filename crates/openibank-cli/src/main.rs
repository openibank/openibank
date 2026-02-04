//! OpeniBank CLI - Command-line interface for AI Agent Banking
//!
//! When the Playground is running (default: http://localhost:8080), the CLI
//! routes commands through its API so state is unified. Otherwise, falls back
//! to local in-memory mode.
//!
//! # Quick Start
//!
//! ```bash
//! # Start Playground first (in one terminal)
//! cargo run -p openibank-playground
//!
//! # Then use CLI (auto-detects Playground)
//! openibank agent buyer -n Alice --funding 50000
//! openibank agent seller -n DataCorp --service "Data Analysis" --price 10000
//! openibank agent marketplace --buyers 3 --sellers 2
//! openibank status
//! ```

use clap::{Parser, Subcommand};
use colored::*;

mod client;
mod commands;
mod display;

use client::PlaygroundClient;
use commands::{demo, issuer, wallet, agent, receipt};

const DEFAULT_PLAYGROUND_URL: &str = "http://localhost:8080";

/// OpeniBank CLI - Programmable Wallets + Receipts for AI Agents
#[derive(Parser)]
#[command(name = "openibank")]
#[command(author = "OpeniBank Contributors")]
#[command(version)]
#[command(about = "AI-agent-only banking with programmable wallets and verifiable receipts", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Playground server URL (auto-detected if running on default port)
    #[arg(long, global = true, default_value = DEFAULT_PLAYGROUND_URL)]
    server: String,

    /// Force local mode (skip Playground detection)
    #[arg(long, global = true)]
    local: bool,

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
        #[arg(short, long)]
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

        /// Number of trading rounds
        #[arg(long, default_value = "10")]
        rounds: u32,

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

    // Try to connect to Playground (unless --local is set)
    let playground = if cli.local {
        None
    } else {
        let client = PlaygroundClient::new(&cli.server);
        if client.is_available().await {
            println!("  {} Connected to Playground at {}", "●".bright_green(), cli.server.bright_cyan());
            println!();
            Some(client)
        } else {
            if cli.server != DEFAULT_PLAYGROUND_URL {
                println!("  {} Could not connect to Playground at {}", "○".yellow(), cli.server);
            }
            println!("  {} Running in local mode", "ℹ".bright_cyan());
            println!();
            None
        }
    };

    match cli.command {
        // ===== Demo commands always run locally =====
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

        // ===== Wallet commands =====
        Commands::Wallet { action } => {
            if let Some(ref pg) = playground {
                // Route wallet list through playground
                match action {
                    WalletCommands::List => {
                        client::display_agents(pg).await?;
                    }
                    WalletCommands::Info { name } => {
                        let id = format!("res_{}", name.to_lowercase().replace(' ', "_"));
                        let info = pg.get_agent(&id).await?;
                        println!("{}", serde_json::to_string_pretty(&info)?);
                    }
                    _ => {
                        // Create/Transfer still go local
                        match action {
                            WalletCommands::Create { name, funding, budget } => {
                                wallet::create_wallet(&name, funding, budget).await?;
                            }
                            WalletCommands::Transfer { from, to, amount, purpose } => {
                                wallet::transfer(&from, &to, amount, &purpose).await?;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            } else {
                match action {
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
                }
            }
        },

        // ===== Issuer commands =====
        Commands::Issuer { action } => {
            if let Some(ref pg) = playground {
                match action {
                    IssuerCommands::Supply { .. } => {
                        let supply = pg.get_supply().await?;
                        let total = supply.get("total_display").and_then(|v| v.as_str()).unwrap_or("$0.00");
                        let remaining = supply.get("remaining_display").and_then(|v| v.as_str()).unwrap_or("$0.00");
                        let receipts = supply.get("receipt_count").and_then(|v| v.as_u64()).unwrap_or(0);

                        println!("{}", "IUSD Supply (from Playground)".bright_white().bold());
                        println!("{}", "─".repeat(40));
                        println!("  Total Supply:     {}", total.bright_cyan());
                        println!("  Remaining:        {}", remaining.bright_green());
                        println!("  Issuer Receipts:  {}", receipts);
                    }
                    IssuerCommands::Receipts { .. } => {
                        let data = pg.get_issuer_receipts().await?;
                        let receipts = data.get("receipts").and_then(|v| v.as_array());
                        println!("{}", "Issuer Receipts (from Playground)".bright_white().bold());
                        println!("{}", "─".repeat(60));
                        if let Some(recs) = receipts {
                            for r in recs {
                                let op = r.get("operation").and_then(|v| v.as_str()).unwrap_or("?");
                                let target = r.get("target").and_then(|v| v.as_str()).unwrap_or("?");
                                let amount = r.get("amount_display").and_then(|v| v.as_str()).unwrap_or("$0");
                                let time = r.get("issued_at").and_then(|v| v.as_str()).unwrap_or("?");
                                let op_color = if op == "Mint" { op.bright_green() } else { op.bright_yellow() };
                                println!("  {} {:8}  {:>10}  {}  {}", "●".bright_cyan(), op_color, amount.bright_white(), target, time.bright_black());
                            }
                        } else {
                            println!("  {}", "No receipts yet".yellow());
                        }
                    }
                    _ => {
                        // Start/Init/Mint fall through to local
                        match action {
                            IssuerCommands::Start { port, reserve_cap } => {
                                issuer::start_issuer(port, reserve_cap).await?;
                            }
                            IssuerCommands::Init { url, reserve_cap } => {
                                issuer::init_issuer(&url, reserve_cap).await?;
                            }
                            IssuerCommands::Mint { to, amount, reason, url } => {
                                issuer::mint(&url, &to, amount, &reason).await?;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            } else {
                match action {
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
                }
            }
        },

        // ===== Agent commands — prefer Playground =====
        Commands::Agent { action } => {
            if let Some(ref pg) = playground {
                match action {
                    AgentCommands::Buyer { name, funding, .. } => {
                        println!("{}", "Creating buyer agent via Playground...".bright_white());
                        let result = pg.create_buyer(&name, funding).await?;
                        let agent = result.get("agent");
                        if let Some(a) = agent {
                            let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            let bal = a.get("balance").and_then(|v| v.as_u64()).unwrap_or(0);
                            let has_res = a.get("has_resonator").and_then(|v| v.as_bool()).unwrap_or(false);
                            let resonator_str = if has_res { " (Maple Resonator)" } else { "" };
                            display::success(&format!(
                                "Buyer '{}' created with ${:.2}{}",
                                name, bal as f64 / 100.0, resonator_str
                            ));
                            display::info(&format!("ID: {}", id));
                            display::info("Agent is now visible in the web dashboard");
                        }
                    }
                    AgentCommands::Seller { name, service, price } => {
                        println!("{}", "Creating seller agent via Playground...".bright_white());
                        let result = pg.create_seller(&name, &service, price).await?;
                        let agent = result.get("agent");
                        if let Some(a) = agent {
                            let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            let has_res = a.get("has_resonator").and_then(|v| v.as_bool()).unwrap_or(false);
                            let resonator_str = if has_res { " (Maple Resonator)" } else { "" };
                            display::success(&format!(
                                "Seller '{}' created: {} @ ${:.2}{}",
                                name, service, price as f64 / 100.0, resonator_str
                            ));
                            display::info(&format!("ID: {}", id));
                            display::info("Agent is now visible in the web dashboard");
                        }
                    }
                    AgentCommands::Marketplace { buyers, sellers, rounds, .. } => {
                        println!("{}", "Running marketplace simulation via Playground...".bright_white());
                        let result = pg.simulate(buyers, sellers, rounds, 500).await?;
                        let msg = result.get("message").and_then(|v| v.as_str()).unwrap_or("Started");
                        display::success(msg);
                        display::info("Watch the simulation at http://localhost:8080");
                    }
                }
            } else {
                // Fallback to local mode
                match action {
                    AgentCommands::Buyer { name, llm, model, funding } => {
                        agent::run_buyer(&name, llm, &model, funding).await?;
                    }
                    AgentCommands::Seller { name, service, price } => {
                        agent::run_seller(&name, &service, price).await?;
                    }
                    AgentCommands::Marketplace { buyers, sellers, llm, .. } => {
                        agent::run_marketplace(buyers, sellers, llm).await?;
                    }
                }
            }
        },

        // ===== Receipt commands always run locally =====
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

        // ===== Status — prefer Playground =====
        Commands::Status => {
            if let Some(ref pg) = playground {
                client::display_playground_status(pg).await?;
            } else {
                show_local_status().await?;
            }
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
        " - Programmable Wallets + Receipts for AI Agents       ║".bright_cyan()
    );
    println!("{}", "║  Powered by Maple AI Framework (Resonance Architecture)         ║".bright_cyan());
    println!("{}", "║                                                                  ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
}

async fn show_local_status() -> anyhow::Result<()> {
    println!("{}", "System Status (Local Mode)".bright_white().bold());
    println!("{}", "─".repeat(50));

    // Check Playground
    print!("  Playground (localhost:8080): ");
    match reqwest::get("http://localhost:8080/api/status").await {
        Ok(resp) if resp.status().is_success() => {
            println!("{}", "● Online (use without --local to connect)".bright_green());
        }
        _ => {
            println!("{}", "○ Not running".bright_red());
            println!("  {}", "Start with: cargo run -p openibank-playground".bright_black());
        }
    }

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
    println!("  {} - Start Playground + Dashboard", "cargo run -p openibank-playground".bright_cyan());
    println!("  {} - Create buyer (connected to Playground)", "openibank agent buyer -n Alice".bright_cyan());
    println!("  {} - Run marketplace", "openibank agent marketplace --buyers 3 --sellers 2".bright_cyan());
    println!("  {} - Run the viral demo (local)", "openibank demo full --llm ollama".bright_cyan());

    Ok(())
}
