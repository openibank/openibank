//! OpeniBank Receipts CLI
//!
//! Command-line interface for receipt operations:
//! - Verify: Check receipt signature and format
//! - Inspect: Display receipt details

use clap::{Parser, Subcommand};
use openibank_receipts::{inspect_receipt_file, verify_receipt_file};

#[derive(Parser)]
#[command(name = "openibank-receipts")]
#[command(about = "OpeniBank receipt verification and inspection tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a receipt's signature and format
    Verify {
        /// Path to the receipt JSON file
        file: String,
    },
    /// Inspect a receipt's contents
    Inspect {
        /// Path to the receipt JSON file
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify { file } => {
            match verify_receipt_file(&file) {
                Ok(result) => {
                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║              Receipt Verification Result                 ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    if result.valid {
                        println!("✓ Receipt is VALID");
                    } else {
                        println!("✗ Receipt is INVALID");
                    }

                    println!();
                    println!("Receipt ID:   {}", result.receipt_id);
                    println!("Receipt Type: {}", result.receipt_type);
                    println!("Signer:       {}...", &result.signer[..16.min(result.signer.len())]);

                    if !result.errors.is_empty() {
                        println!();
                        println!("Errors:");
                        for error in &result.errors {
                            println!("  - {}", error);
                        }
                    }

                    std::process::exit(if result.valid { 0 } else { 1 });
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Inspect { file } => {
            match inspect_receipt_file(&file) {
                Ok(inspection) => {
                    println!("╔══════════════════════════════════════════════════════════╗");
                    println!("║                  Receipt Inspection                      ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!();

                    println!("Receipt ID:       {}", inspection.receipt_id);
                    println!("Receipt Type:     {}", inspection.receipt_type);
                    println!("Signature Valid:  {}", if inspection.signature_valid { "✓ Yes" } else { "✗ No" });
                    println!("Signer Key:       {}", inspection.signer_public_key);
                    println!();
                    println!("Details:");
                    println!("{}", serde_json::to_string_pretty(&inspection.details).unwrap());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
