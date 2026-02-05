//! OpeniBank UAL Integration - Universal Agent Language for Banking
//!
//! This crate extends UAL (Universal Agent Language) with OpeniBank-specific
//! banking commands and provides a SQL-like interface for financial agent operations.
//!
//! # Banking Commands
//!
//! OpeniBank adds financial domain commands on top of standard UAL:
//!
//! ```text
//! -- Standard UAL commitment (compiles to RCF)
//! COMMIT BY buyer-001
//!     DOMAIN Finance
//!     OUTCOME "Purchase data analysis service for $100"
//!     SCOPE GLOBAL
//!     TAG trade TAG payment
//!     REVERSIBLE;
//!
//! -- Standard UAL operations (compile to PALM ops)
//! CREATE SPEC "buyer-agent" VERSION "1.0.0";
//! CREATE DEPLOYMENT "buyer-agent" REPLICAS 3;
//! SCALE DEPLOYMENT "deploy-001" REPLICAS 5;
//! HEALTH CHECK "instance-001";
//!
//! -- OpeniBank extensions (parsed separately)
//! MINT 10000 IUSD TO "issuer-reserve";
//! TRANSFER 5000 IUSD FROM "buyer-001" TO "seller-001";
//! BALANCE "buyer-001";
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export UAL types for consumers
pub use ual_types::{UalStatement, CommitStatement, OperationStatement, ReversibilitySpec};
pub use ual_parser::{parse as ual_parse, UalParseError};
pub use ual_compiler::{compile as ual_compile, UalCompiled, UalCompileError};
use openibank_agent_kernel::AgentKernel;

/// Errors from banking UAL operations
#[derive(Debug, Error)]
pub enum BankingUalError {
    #[error("UAL parse error: {0}")]
    Parse(#[from] UalParseError),
    #[error("UAL compile error: {0}")]
    Compile(#[from] UalCompileError),
    #[error("Banking command error: {0}")]
    BankingCommand(String),
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    #[error("Unknown agent: {0}")]
    UnknownAgent(String),
}

/// OpeniBank-specific banking commands (beyond standard UAL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BankingCommand {
    /// Mint currency to an account
    Mint {
        amount: u64,
        currency: String,
        to_account: String,
    },
    /// Burn currency from an account
    Burn {
        amount: u64,
        currency: String,
        from_account: String,
    },
    /// Transfer between accounts
    Transfer {
        amount: u64,
        currency: String,
        from_account: String,
        to_account: String,
    },
    /// Check balance
    Balance {
        account: String,
    },
    /// Create a new agent
    CreateAgent {
        name: String,
        role: String,
        budget: Option<u64>,
    },
    /// List agents
    ListAgents,
    /// Show system status
    Status,
    /// Start a trade between buyer and seller
    Trade {
        buyer: String,
        seller: String,
        service: String,
        amount: u64,
    },
    /// Show transaction history
    History {
        account: Option<String>,
        limit: usize,
    },
    /// Verify a receipt
    VerifyReceipt {
        receipt_id: String,
    },
    /// Deploy agent fleet
    DeployFleet {
        agent_type: String,
        count: u32,
    },
    /// Show fleet status
    FleetStatus,
}

/// Result of executing a banking command or UAL statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Human-readable summary
    pub summary: String,
    /// Structured output data
    pub data: Option<serde_json::Value>,
    /// Any receipts generated
    pub receipts: Vec<String>,
    /// Compiled artifacts (if UAL)
    pub artifacts: Vec<String>,
}

impl ExecutionResult {
    pub fn ok(summary: impl Into<String>) -> Self {
        Self {
            success: true,
            summary: summary.into(),
            data: None,
            receipts: Vec::new(),
            artifacts: Vec::new(),
        }
    }

    pub fn ok_with_data(summary: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            summary: summary.into(),
            data: Some(data),
            receipts: Vec::new(),
            artifacts: Vec::new(),
        }
    }

    pub fn err(summary: impl Into<String>) -> Self {
        Self {
            success: false,
            summary: summary.into(),
            data: None,
            receipts: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

/// Parse input as either standard UAL or OpeniBank banking commands
///
/// Tries UAL parsing first, then falls back to banking command parsing.
pub fn parse_input(input: &str) -> Result<ParsedInput, BankingUalError> {
    let trimmed = input.trim();

    // Try banking command first (simple keyword match)
    if let Some(cmd) = try_parse_banking_command(trimmed) {
        return Ok(ParsedInput::Banking(cmd));
    }

    // Try standard UAL
    match ual_parse(trimmed) {
        Ok(statements) => {
            if statements.is_empty() {
                Err(BankingUalError::BankingCommand(
                    "Empty input".to_string(),
                ))
            } else {
                Ok(ParsedInput::Ual(statements))
            }
        }
        Err(e) => Err(BankingUalError::Parse(e)),
    }
}

/// Compile parsed UAL statements into artifacts
pub fn compile_statements(statements: &[UalStatement]) -> Result<Vec<UalCompiled>, BankingUalError> {
    Ok(ual_compile(statements)?)
}

/// Compile UAL statements and feed the compiled artifacts into an AgentKernel.
///
/// This ensures the kernel only consumes compiled RCF/PALM artifacts (never raw text).
pub fn compile_statements_with_kernel(
    statements: &[UalStatement],
    kernel: &mut AgentKernel,
) -> Result<Vec<UalCompiled>, BankingUalError> {
    let compiled = compile_statements(statements)?;
    kernel
        .consume_ual_artifacts(&compiled)
        .map_err(|e| BankingUalError::BankingCommand(e.to_string()))?;
    Ok(compiled)
}

/// Parsed input - either standard UAL or banking commands
#[derive(Debug, Clone)]
pub enum ParsedInput {
    /// Standard UAL statements (commitments or PALM operations)
    Ual(Vec<UalStatement>),
    /// OpeniBank banking commands
    Banking(BankingCommand),
}

/// Try to parse a banking command from input
fn try_parse_banking_command(input: &str) -> Option<BankingCommand> {
    let upper = input.to_uppercase();
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    match parts[0].to_uppercase().as_str() {
        "MINT" => parse_mint_command(&parts),
        "BURN" => parse_burn_command(&parts),
        "TRANSFER" => parse_transfer_command(&parts),
        "BALANCE" => parse_balance_command(&parts),
        "AGENTS" | "LIST" if upper.contains("AGENT") => Some(BankingCommand::ListAgents),
        "STATUS" => Some(BankingCommand::Status),
        "HISTORY" => parse_history_command(&parts),
        "VERIFY" => parse_verify_command(&parts),
        "FLEET" if upper.contains("STATUS") => Some(BankingCommand::FleetStatus),
        "DEPLOY" => parse_deploy_command(&parts),
        "TRADE" => parse_trade_command(&parts),
        "AGENT" if parts.len() >= 2 && parts[1].to_uppercase() == "CREATE" => {
            parse_create_agent_command(&parts)
        }
        "CREATE" if parts.len() >= 2 && parts[1].to_uppercase() == "AGENT" => {
            parse_create_agent_command(&parts)
        }
        _ => None,
    }
}

fn parse_mint_command(parts: &[&str]) -> Option<BankingCommand> {
    // MINT <amount> <currency> TO <account>
    if parts.len() < 5 { return None; }
    let amount = parts[1].parse::<u64>().ok()?;
    let currency = parts[2].to_string();
    // Skip "TO"
    let to_account = unquote(parts[4]);
    Some(BankingCommand::Mint { amount, currency, to_account })
}

fn parse_burn_command(parts: &[&str]) -> Option<BankingCommand> {
    // BURN <amount> <currency> FROM <account>
    if parts.len() < 5 { return None; }
    let amount = parts[1].parse::<u64>().ok()?;
    let currency = parts[2].to_string();
    let from_account = unquote(parts[4]);
    Some(BankingCommand::Burn { amount, currency, from_account })
}

fn parse_transfer_command(parts: &[&str]) -> Option<BankingCommand> {
    // TRANSFER <amount> <currency> FROM <from> TO <to>
    if parts.len() < 7 { return None; }
    let amount = parts[1].parse::<u64>().ok()?;
    let currency = parts[2].to_string();
    let from_account = unquote(parts[4]);
    let to_account = unquote(parts[6]);
    Some(BankingCommand::Transfer { amount, currency, from_account, to_account })
}

fn parse_balance_command(parts: &[&str]) -> Option<BankingCommand> {
    // BALANCE <account>
    if parts.len() < 2 { return None; }
    let account = unquote(parts[1]);
    Some(BankingCommand::Balance { account })
}

fn parse_history_command(parts: &[&str]) -> Option<BankingCommand> {
    // HISTORY [account] [LIMIT n]
    let mut account = None;
    let mut limit = 50;
    let mut i = 1;
    while i < parts.len() {
        if parts[i].to_uppercase() == "LIMIT" && i + 1 < parts.len() {
            limit = parts[i + 1].parse().unwrap_or(50);
            i += 2;
        } else {
            account = Some(unquote(parts[i]));
            i += 1;
        }
    }
    Some(BankingCommand::History { account, limit })
}

fn parse_verify_command(parts: &[&str]) -> Option<BankingCommand> {
    // VERIFY <receipt_id>
    if parts.len() < 2 { return None; }
    Some(BankingCommand::VerifyReceipt { receipt_id: unquote(parts[1]) })
}

fn parse_deploy_command(parts: &[&str]) -> Option<BankingCommand> {
    // DEPLOY <agent_type> COUNT <n>
    if parts.len() < 2 { return None; }
    let agent_type = parts[1].to_lowercase();
    let count = if parts.len() >= 4 && parts[2].to_uppercase() == "COUNT" {
        parts[3].parse().unwrap_or(1)
    } else {
        1
    };
    Some(BankingCommand::DeployFleet { agent_type, count })
}

fn parse_trade_command(parts: &[&str]) -> Option<BankingCommand> {
    // TRADE <buyer> <seller> <service> <amount>
    if parts.len() < 5 { return None; }
    let buyer = unquote(parts[1]);
    let seller = unquote(parts[2]);
    let service = unquote(parts[3]);
    let amount = parts[4].parse::<u64>().ok()?;
    Some(BankingCommand::Trade { buyer, seller, service, amount })
}

fn parse_create_agent_command(parts: &[&str]) -> Option<BankingCommand> {
    // CREATE AGENT <name> <role> [BUDGET <amount>]
    // or AGENT CREATE <name> <role> [BUDGET <amount>]
    let start = if parts[0].to_uppercase() == "CREATE" { 2 } else { 2 };
    if parts.len() < start + 2 { return None; }
    let name = unquote(parts[start]);
    let role = parts[start + 1].to_lowercase();
    let budget = if parts.len() >= start + 4 && parts[start + 2].to_uppercase() == "BUDGET" {
        parts[start + 3].parse().ok()
    } else {
        None
    };
    Some(BankingCommand::CreateAgent { name, role, budget })
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').trim_matches('\'').to_string()
}

/// Convenience: parse and compile a UAL string in one step
pub fn parse_and_compile(input: &str) -> Result<Vec<UalCompiled>, BankingUalError> {
    let statements = ual_parse(input)?;
    Ok(ual_compile(&statements)?)
}

/// Generate a UAL COMMIT statement for a trade
pub fn generate_trade_commitment(
    buyer: &str,
    seller: &str,
    service: &str,
    amount: u64,
) -> String {
    format!(
        r#"COMMIT BY "{buyer}"
    DOMAIN Finance
    OUTCOME "Purchase {service} from {seller} for ${amount_display}"
    SCOPE GLOBAL
    TARGET "{seller}"
    TAG trade
    TAG payment
    REVERSIBLE;"#,
        buyer = buyer,
        seller = seller,
        service = service,
        amount_display = amount as f64 / 100.0,
    )
}

/// Generate a UAL COMMIT statement for minting
pub fn generate_mint_commitment(
    issuer: &str,
    amount: u64,
    recipient: &str,
) -> String {
    format!(
        r#"COMMIT BY "{issuer}"
    DOMAIN Finance
    OUTCOME "Mint ${amount_display} IUSD to {recipient}"
    SCOPE GLOBAL
    TARGET "{recipient}"
    TAG mint
    TAG issuance
    IRREVERSIBLE;"#,
        issuer = issuer,
        amount_display = amount as f64 / 100.0,
        recipient = recipient,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_banking_commands() {
        let mint = try_parse_banking_command("MINT 10000 IUSD TO reserve-001");
        assert!(mint.is_some());
        if let Some(BankingCommand::Mint { amount, currency, to_account }) = mint {
            assert_eq!(amount, 10000);
            assert_eq!(currency, "IUSD");
            assert_eq!(to_account, "reserve-001");
        }

        let balance = try_parse_banking_command("BALANCE buyer-001");
        assert!(balance.is_some());

        let status = try_parse_banking_command("STATUS");
        assert!(status.is_some());

        let transfer = try_parse_banking_command("TRANSFER 5000 IUSD FROM buyer-001 TO seller-001");
        assert!(transfer.is_some());
    }

    #[test]
    fn test_parse_ual_commitment() {
        let input = r#"COMMIT BY "agent-001"
            DOMAIN Finance
            OUTCOME "Test payment"
            SCOPE GLOBAL
            TAG payment
            REVERSIBLE;"#;

        let result = parse_input(input);
        assert!(result.is_ok());
        if let Ok(ParsedInput::Ual(stmts)) = result {
            assert_eq!(stmts.len(), 1);
        }
    }

    #[test]
    fn test_generate_trade_commitment() {
        let ual = generate_trade_commitment("buyer-001", "seller-001", "Data Analysis", 10000);
        assert!(ual.contains("COMMIT BY"));
        assert!(ual.contains("Finance"));
        assert!(ual.contains("Data Analysis"));
    }
}
