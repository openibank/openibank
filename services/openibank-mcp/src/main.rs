//! OpeniBank MCP Server
//!
//! A Model Context Protocol (MCP) server that allows Claude Desktop and other
//! MCP clients to interact with OpeniBank's agent banking system.
//!
//! ## MCP Protocol
//!
//! MCP uses JSON-RPC 2.0 over stdio. The server reads requests from stdin
//! and writes responses to stdout.
//!
//! ## Available Tools
//!
//! ### Agent Management
//! - `create_buyer_agent` - Create a new buyer agent with wallet
//! - `create_seller_agent` - Create a seller agent with services
//! - `list_agents` - List all active agents
//!
//! ### Trading Operations
//! - `evaluate_offer` - Have a buyer evaluate a seller's offer
//! - `execute_trade` - Execute a trade between agents
//! - `get_trade_history` - Get trading history
//!
//! ### Wallet Operations
//! - `get_balance` - Get agent wallet balance
//! - `mint_funds` - Mint IUSD to an agent (testing)
//!
//! ### Receipt Operations
//! - `verify_receipt` - Verify a cryptographic receipt
//!
//! ## Usage
//!
//! Add to Claude Desktop's config (~/.config/claude/claude_desktop_config.json):
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "openibank": {
//!       "command": "openibank-mcp"
//!     }
//!   }
//! }
//! ```

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

use openibank_agents::{AgentBrain, BuyerAgent, SellerAgent, Service};
use openibank_core::{Amount, AssetId, ResonatorId, EscrowId};
use openibank_issuer::{Issuer, IssuerConfig, MintIntent};
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// MCP Server state
struct MCPState {
    ledger: Arc<Ledger>,
    issuer: Issuer,
    buyers: HashMap<String, BuyerAgent>,
    sellers: HashMap<String, SellerAgent>,
    trade_count: u32,
}

impl MCPState {
    fn new() -> Self {
        let ledger = Arc::new(Ledger::new());
        let issuer = Issuer::new(
            IssuerConfig::default(),
            Amount::new(100_000_000_00), // $1M reserve
            ledger.clone(),
        );

        Self {
            ledger,
            issuer,
            buyers: HashMap::new(),
            sellers: HashMap::new(),
            trade_count: 0,
        }
    }
}

// ============================================================================
// JSON-RPC Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

// ============================================================================
// MCP Protocol Types
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ServerCapabilities {
    tools: ToolsCapability,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ToolsCapability {
    #[serde(rename = "listChanged")]
    list_changed: bool,
}

#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct ToolResult {
    content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing to stderr (stdout is for MCP communication)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("OpeniBank MCP Server starting...");

    let state = Arc::new(RwLock::new(MCPState::new()));

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to read line: {}", e);
                continue;
            }
        };

        if line.is_empty() {
            continue;
        }

        tracing::debug!("Received: {}", line);

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {}", e),
                );
                writeln!(stdout, "{}", serde_json::to_string(&response).unwrap()).ok();
                stdout.flush().ok();
                continue;
            }
        };

        let response = handle_request(&state, request).await;
        let response_str = serde_json::to_string(&response).unwrap();

        tracing::debug!("Sending: {}", response_str);

        writeln!(stdout, "{}", response_str).ok();
        stdout.flush().ok();
    }
}

async fn handle_request(state: &Arc<RwLock<MCPState>>, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tools_call(state, request.id, request.params).await,
        "notifications/initialized" => {
            // No response needed for notifications
            JsonRpcResponse::success(request.id, serde_json::json!({}))
        }
        _ => JsonRpcResponse::error(request.id, -32601, format!("Method not found: {}", request.method)),
    }
}

fn handle_initialize(id: serde_json::Value) -> JsonRpcResponse {
    let result = serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "openibank",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    JsonRpcResponse::success(id, result)
}

fn handle_tools_list(id: serde_json::Value) -> JsonRpcResponse {
    let tools = vec![
        Tool {
            name: "create_buyer_agent".to_string(),
            description: "Create a new AI buyer agent with a funded wallet. The agent can evaluate offers and purchase services from sellers.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the buyer agent (e.g., 'Alice')"
                    },
                    "funding": {
                        "type": "number",
                        "description": "Initial funding in cents (e.g., 50000 for $500.00)"
                    }
                },
                "required": ["name", "funding"]
            }),
        },
        Tool {
            name: "create_seller_agent".to_string(),
            description: "Create a new AI seller agent that offers a service. The agent can issue invoices and receive payments.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the seller agent (e.g., 'DataCorp')"
                    },
                    "service_name": {
                        "type": "string",
                        "description": "Name of the service offered (e.g., 'Data Analysis')"
                    },
                    "price": {
                        "type": "number",
                        "description": "Price in cents (e.g., 10000 for $100.00)"
                    }
                },
                "required": ["name", "service_name", "price"]
            }),
        },
        Tool {
            name: "list_agents".to_string(),
            description: "List all active buyer and seller agents with their balances and services.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "get_balance".to_string(),
            description: "Get the IUSD balance for an agent.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_id": {
                        "type": "string",
                        "description": "The agent ID (e.g., 'buyer_alice' or 'seller_datacorp')"
                    }
                },
                "required": ["agent_id"]
            }),
        },
        Tool {
            name: "execute_trade".to_string(),
            description: "Execute a trade between a buyer and seller. The buyer purchases the seller's service through an escrow.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "buyer_id": {
                        "type": "string",
                        "description": "The buyer agent ID"
                    },
                    "seller_id": {
                        "type": "string",
                        "description": "The seller agent ID"
                    }
                },
                "required": ["buyer_id", "seller_id"]
            }),
        },
        Tool {
            name: "mint_funds".to_string(),
            description: "Mint IUSD to an agent's wallet (testing only).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_id": {
                        "type": "string",
                        "description": "The agent ID to fund"
                    },
                    "amount": {
                        "type": "number",
                        "description": "Amount in cents to mint"
                    }
                },
                "required": ["agent_id", "amount"]
            }),
        },
        Tool {
            name: "get_issuer_status".to_string(),
            description: "Get the current IUSD issuer status including total supply.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ];

    JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
}

async fn handle_tools_call(
    state: &Arc<RwLock<MCPState>>,
    id: serde_json::Value,
    params: serde_json::Value,
) -> JsonRpcResponse {
    let tool_name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let result = match tool_name {
        "create_buyer_agent" => create_buyer_agent(state, arguments).await,
        "create_seller_agent" => create_seller_agent(state, arguments).await,
        "list_agents" => list_agents(state).await,
        "get_balance" => get_balance(state, arguments).await,
        "execute_trade" => execute_trade(state, arguments).await,
        "mint_funds" => mint_funds(state, arguments).await,
        "get_issuer_status" => get_issuer_status(state).await,
        _ => Err(format!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok(content) => JsonRpcResponse::success(id, serde_json::json!({
            "content": [{"type": "text", "text": content}]
        })),
        Err(e) => JsonRpcResponse::success(id, serde_json::json!({
            "content": [{"type": "text", "text": format!("Error: {}", e)}],
            "isError": true
        })),
    }
}

// ============================================================================
// Tool Implementations
// ============================================================================

async fn create_buyer_agent(
    state: &Arc<RwLock<MCPState>>,
    args: serde_json::Value,
) -> Result<String, String> {
    let name = args
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("Missing 'name' parameter")?;

    let funding = args
        .get("funding")
        .and_then(|f| f.as_u64())
        .ok_or("Missing 'funding' parameter")?;

    let id = format!("buyer_{}", name.to_lowercase().replace(' ', "_"));
    let resonator_id = ResonatorId::from_string(&id);

    let mut state = state.write().await;

    // Check if already exists
    if state.buyers.contains_key(&id) {
        return Err(format!("Buyer '{}' already exists", id));
    }

    // Create agent with deterministic brain
    let brain = AgentBrain::deterministic();
    let mut buyer = BuyerAgent::with_brain(resonator_id.clone(), state.ledger.clone(), brain);

    // Fund the buyer
    let mint = MintIntent::new(resonator_id, Amount::new(funding), "MCP funding");
    state.issuer.mint(mint).await.map_err(|e| e.to_string())?;

    buyer
        .setup(Amount::new(funding), Amount::new(funding / 2))
        .map_err(|e| e.to_string())?;

    state.buyers.insert(id.clone(), buyer);

    Ok(format!(
        "✓ Created buyer agent '{}'\n  ID: {}\n  Balance: ${:.2}\n  Budget: ${:.2}",
        name,
        id,
        funding as f64 / 100.0,
        (funding / 2) as f64 / 100.0
    ))
}

async fn create_seller_agent(
    state: &Arc<RwLock<MCPState>>,
    args: serde_json::Value,
) -> Result<String, String> {
    let name = args
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or("Missing 'name' parameter")?;

    let service_name = args
        .get("service_name")
        .and_then(|s| s.as_str())
        .ok_or("Missing 'service_name' parameter")?;

    let price = args
        .get("price")
        .and_then(|p| p.as_u64())
        .ok_or("Missing 'price' parameter")?;

    let id = format!("seller_{}", name.to_lowercase().replace(' ', "_"));
    let resonator_id = ResonatorId::from_string(&id);

    let mut state = state.write().await;

    if state.sellers.contains_key(&id) {
        return Err(format!("Seller '{}' already exists", id));
    }

    let mut seller = SellerAgent::new(resonator_id, state.ledger.clone());

    let service = Service {
        name: service_name.to_string(),
        description: format!("Service: {}", service_name),
        price: Amount::new(price),
        asset: AssetId::iusd(),
        delivery_conditions: vec!["Service completion".to_string()],
    };

    seller.publish_service(service);
    state.sellers.insert(id.clone(), seller);

    Ok(format!(
        "✓ Created seller agent '{}'\n  ID: {}\n  Service: {}\n  Price: ${:.2}",
        name,
        id,
        service_name,
        price as f64 / 100.0
    ))
}

async fn list_agents(state: &Arc<RwLock<MCPState>>) -> Result<String, String> {
    let state = state.read().await;

    let mut output = String::new();
    output.push_str("📊 OpeniBank Agents\n");
    output.push_str("═══════════════════════════════════════\n\n");

    output.push_str("🛒 BUYERS\n");
    if state.buyers.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (id, buyer) in &state.buyers {
            output.push_str(&format!(
                "  • {} — Balance: ${:.2}\n",
                id,
                buyer.balance().0 as f64 / 100.0
            ));
        }
    }

    output.push_str("\n🏪 SELLERS\n");
    if state.sellers.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for (id, seller) in &state.sellers {
            let services = seller.services();
            let service_info = services
                .first()
                .map(|s| format!("{} @ ${:.2}", s.name, s.price.0 as f64 / 100.0))
                .unwrap_or_else(|| "No services".to_string());

            output.push_str(&format!(
                "  • {} — {} | Balance: ${:.2}\n",
                id,
                service_info,
                seller.balance().0 as f64 / 100.0
            ));
        }
    }

    output.push_str(&format!("\nTotal trades: {}", state.trade_count));

    Ok(output)
}

async fn get_balance(
    state: &Arc<RwLock<MCPState>>,
    args: serde_json::Value,
) -> Result<String, String> {
    let agent_id = args
        .get("agent_id")
        .and_then(|a| a.as_str())
        .ok_or("Missing 'agent_id' parameter")?;

    let state = state.read().await;

    if let Some(buyer) = state.buyers.get(agent_id) {
        return Ok(format!(
            "💰 {} Balance: ${:.2} IUSD",
            agent_id,
            buyer.balance().0 as f64 / 100.0
        ));
    }

    if let Some(seller) = state.sellers.get(agent_id) {
        return Ok(format!(
            "💰 {} Balance: ${:.2} IUSD",
            agent_id,
            seller.balance().0 as f64 / 100.0
        ));
    }

    Err(format!("Agent '{}' not found", agent_id))
}

async fn execute_trade(
    state: &Arc<RwLock<MCPState>>,
    args: serde_json::Value,
) -> Result<String, String> {
    let buyer_id = args
        .get("buyer_id")
        .and_then(|b| b.as_str())
        .ok_or("Missing 'buyer_id' parameter")?;

    let seller_id = args
        .get("seller_id")
        .and_then(|s| s.as_str())
        .ok_or("Missing 'seller_id' parameter")?;

    let mut state = state.write().await;

    // Verify both exist
    if !state.buyers.contains_key(buyer_id) {
        return Err(format!("Buyer '{}' not found", buyer_id));
    }
    if !state.sellers.contains_key(seller_id) {
        return Err(format!("Seller '{}' not found", seller_id));
    }

    // Get service info
    let service_name = {
        let seller = state.sellers.get(seller_id).unwrap();
        seller
            .services()
            .first()
            .map(|s| s.name.clone())
            .ok_or("Seller has no services")?
    };

    let _service_price = {
        let seller = state.sellers.get(seller_id).unwrap();
        seller
            .services()
            .first()
            .map(|s| s.price.0)
            .unwrap_or(0)
    };

    // Get offer
    let offer = {
        let seller = state.sellers.get(seller_id).unwrap();
        seller.get_offer(&service_name).ok_or("No offer available")?
    };

    // Evaluate
    let can_afford = {
        let buyer = state.buyers.get(buyer_id).unwrap();
        buyer.evaluate_offer(&offer).await
    };

    if !can_afford {
        return Err("Buyer cannot afford the service or declined".to_string());
    }

    // Get buyer resonator ID
    let buyer_resonator_id = {
        let buyer = state.buyers.get(buyer_id).unwrap();
        buyer.id().clone()
    };

    // Issue invoice
    let invoice = {
        let seller = state.sellers.get_mut(seller_id).unwrap();
        seller
            .issue_invoice(buyer_resonator_id, &service_name)
            .await
            .map_err(|e| e.to_string())?
    };

    let invoice_id = invoice.invoice_id.clone();

    // Accept invoice
    {
        let buyer = state.buyers.get_mut(buyer_id).unwrap();
        buyer.accept_invoice(invoice).map_err(|e| e.to_string())?;
    }

    // Attach commitment context for kernel gating
    let commitment_id = format!("mcp_commit_{}", uuid::Uuid::new_v4());
    if let Some(buyer) = state.buyers.get_mut(buyer_id) {
        buyer.set_active_commitment(commitment_id.clone(), true);
    }
    if let Some(seller) = state.sellers.get_mut(seller_id) {
        seller.set_active_commitment(commitment_id.clone(), true);
    }

    let trade_result: Result<(EscrowId, Amount), String> = (async {
        // Pay invoice
        let escrow_id = {
            let buyer = state.buyers.get_mut(buyer_id).unwrap();
            let (_, escrow) = buyer
                .pay_invoice(&invoice_id)
                .await
                .map_err(|e| e.to_string())?;
            escrow.escrow_id.clone()
        };

        // Deliver service
        {
            let seller = state.sellers.get_mut(seller_id).unwrap();
            seller
                .deliver_service(&invoice_id, "Service delivered via MCP".to_string())
                .map_err(|e| e.to_string())?;
        }

        // Confirm delivery
        let amount = {
            let buyer = state.buyers.get_mut(buyer_id).unwrap();
            buyer.confirm_delivery(&escrow_id).map_err(|e| e.to_string())?
        };

        // Receive payment
        {
            let seller = state.sellers.get_mut(seller_id).unwrap();
            seller.receive_payment(amount).map_err(|e| e.to_string())?;
        }

        Ok((escrow_id, amount))
    }).await;

    if let Some(buyer) = state.buyers.get_mut(buyer_id) {
        buyer.clear_active_commitment();
    }
    if let Some(seller) = state.sellers.get_mut(seller_id) {
        seller.clear_active_commitment();
    }

    let (_escrow_id, amount) = trade_result?;

    state.trade_count += 1;

    let buyer_balance = state.buyers.get(buyer_id).unwrap().balance().0;
    let seller_balance = state.sellers.get(seller_id).unwrap().balance().0;

    Ok(format!(
        "✅ Trade Completed!\n\n\
        📦 Service: {}\n\
        💵 Amount: ${:.2}\n\n\
        Buyer {} new balance: ${:.2}\n\
        Seller {} new balance: ${:.2}\n\n\
        Total trades: {}",
        service_name,
        amount.0 as f64 / 100.0,
        buyer_id,
        buyer_balance as f64 / 100.0,
        seller_id,
        seller_balance as f64 / 100.0,
        state.trade_count
    ))
}

async fn mint_funds(
    state: &Arc<RwLock<MCPState>>,
    args: serde_json::Value,
) -> Result<String, String> {
    let agent_id = args
        .get("agent_id")
        .and_then(|a| a.as_str())
        .ok_or("Missing 'agent_id' parameter")?;

    let amount = args
        .get("amount")
        .and_then(|a| a.as_u64())
        .ok_or("Missing 'amount' parameter")?;

    let state = state.write().await;

    // Check if agent exists
    let is_buyer = state.buyers.contains_key(agent_id);
    let is_seller = state.sellers.contains_key(agent_id);

    if !is_buyer && !is_seller {
        return Err(format!("Agent '{}' not found", agent_id));
    }

    let resonator_id = ResonatorId::from_string(agent_id);
    let mint = MintIntent::new(resonator_id, Amount::new(amount), "MCP mint");

    state.issuer.mint(mint).await.map_err(|e| e.to_string())?;

    Ok(format!(
        "💸 Minted ${:.2} IUSD to {}",
        amount as f64 / 100.0,
        agent_id
    ))
}

async fn get_issuer_status(state: &Arc<RwLock<MCPState>>) -> Result<String, String> {
    let state = state.read().await;

    let total_supply = state.issuer.total_supply().await;
    let remaining = state.issuer.remaining_supply().await;
    let is_halted = state.issuer.is_halted().await;

    Ok(format!(
        "🏦 IUSD Issuer Status\n\
        ═══════════════════════════════════════\n\
        Total Supply: ${:.2}\n\
        Remaining Capacity: ${:.2}\n\
        Status: {}\n\
        Trade Count: {}",
        total_supply.0 as f64 / 100.0,
        remaining.0 as f64 / 100.0,
        if is_halted { "🔴 HALTED" } else { "🟢 Active" },
        state.trade_count
    ))
}
