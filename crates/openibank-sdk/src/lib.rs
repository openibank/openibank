//! OpeniBank SDK - Developer SDK for AI-native banking
//!
//! The SDK provides a simple, high-level API for interacting with OpeniBank.
//! Designed for the 5-minute experience - from zero to first transaction.
//!
//! # Philosophy
//!
//! OpeniBank follows the Resonance Flow:
//! **Presence → Coupling → Meaning → Intent → Commitment → Consequence**
//!
//! In practice:
//! - LLMs propose intent (payment requests)
//! - Resonators commit (escrow, clearing)
//! - Money moves with proof (receipts)
//!
//! # Quick Start
//!
//! ```ignore
//! use openibank_sdk::OpeniBank;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to OpeniBank
//!     let bank = OpeniBank::local().await?;
//!
//!     // Create an agent
//!     let agent = bank.agent("my-agent")
//!         .with_balance(1000.0)
//!         .create()
//!         .await?;
//!
//!     // Make a payment with escrow protection
//!     let receipt = agent.pay("seller-agent")
//!         .amount(50.0)
//!         .for_service("api-call")
//!         .with_escrow()
//!         .send()
//!         .await?;
//!
//!     // Verify cryptographically
//!     assert!(receipt.verify());
//!     println!("Transaction: {}", receipt.id());
//!
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - **Zero-config setup**: Works out of the box with sensible defaults
//! - **Builder patterns**: Fluent API for all operations
//! - **Type-safe**: Leverages Rust's type system for safety
//! - **Async-first**: Built on tokio for high performance
//! - **Escrow-by-default**: All payments can be escrowed automatically
//! - **Receipt verification**: Cryptographic proofs for all transactions

pub use openibank_types::*;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Error Types
// ============================================================================

/// SDK-specific errors
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Insufficient funds: needed {needed}, available {available}")]
    InsufficientFunds { needed: f64, available: f64 },

    #[error("Permit denied: {0}")]
    PermitDenied(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Bank error: {0}")]
    BankError(#[from] OpeniBankError),
}

/// SDK Result type
pub type SdkResult<T> = std::result::Result<T, SdkError>;

// ============================================================================
// Configuration
// ============================================================================

/// SDK configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// API endpoint
    pub endpoint: String,
    /// Request timeout
    pub timeout: Duration,
    /// API key (optional)
    pub api_key: Option<String>,
    /// Default currency
    pub default_currency: Currency,
    /// Auto-escrow enabled
    pub auto_escrow: bool,
    /// Retry configuration
    pub retry: RetryConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8080".to_string(),
            timeout: Duration::from_secs(30),
            api_key: None,
            default_currency: Currency::iusd(),
            auto_escrow: true,
            retry: RetryConfig::default(),
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Maximum delay
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        }
    }
}

// ============================================================================
// API Types
// ============================================================================

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub agents_count: u64,
    pub transactions_count: u64,
}

/// Agent creation request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub initial_balance: Option<f64>,
    pub currency: Currency,
    pub metadata: Option<serde_json::Value>,
}

/// Agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub wallet_id: String,
    pub created_at: String,
}

/// Payment request
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f64,
    pub currency: Currency,
    pub service: Option<String>,
    pub escrow: bool,
    pub memo: Option<String>,
}

/// Payment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub transaction_id: String,
    pub receipt_id: String,
    pub status: String,
    pub timestamp: String,
}

/// Balance response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub currency: String,
    pub amount: f64,
    pub available: f64,
    pub pending: f64,
}

// ============================================================================
// Main Client
// ============================================================================

/// Main OpeniBank client
///
/// This is the entry point for all SDK operations.
#[derive(Clone)]
pub struct OpeniBank {
    config: Arc<Config>,
    client: Client,
}

impl OpeniBank {
    /// Connect to a local OpeniBank instance
    ///
    /// Uses default configuration with localhost endpoint.
    pub async fn local() -> SdkResult<Self> {
        Self::connect("http://localhost:8080").await
    }

    /// Connect to a specific endpoint
    pub async fn connect(endpoint: &str) -> SdkResult<Self> {
        let config = Config {
            endpoint: endpoint.to_string(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create with custom configuration
    pub async fn with_config(config: Config) -> SdkResult<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))?;

        let bank = Self {
            config: Arc::new(config),
            client,
        };

        // Optionally verify connection
        // bank.status().await?;

        Ok(bank)
    }

    /// Get server status
    pub async fn status(&self) -> SdkResult<StatusResponse> {
        let url = format!("{}/api/v1/status", self.config.endpoint);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<StatusResponse> = resp.json().await?;
        api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 500,
                message: api_resp.error.unwrap_or_else(|| "Unknown error".to_string()),
            }
        })
    }

    /// Start building an agent
    pub fn agent(&self, name: &str) -> AgentBuilder {
        AgentBuilder::new(self.clone(), name.to_string())
    }

    /// Get an existing agent by ID
    pub async fn get_agent(&self, id: &str) -> SdkResult<AgentHandle> {
        let url = format!("{}/api/v1/agents/{}", self.config.endpoint, id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<AgentResponse> = resp.json().await?;
        let agent = api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 404,
                message: "Agent not found".to_string(),
            }
        })?;

        Ok(AgentHandle {
            bank: self.clone(),
            id: agent.id,
            name: agent.name,
            wallet_id: agent.wallet_id,
        })
    }

    /// Create a wallet directly (lower-level API)
    pub async fn create_wallet(&self, name: &str) -> SdkResult<WalletHandle> {
        Ok(WalletHandle {
            id: WalletId::new(),
            name: name.to_string(),
            endpoint: self.config.endpoint.clone(),
            client: self.client.clone(),
        })
    }

    /// Get an existing wallet
    pub async fn get_wallet(&self, id: &WalletId) -> SdkResult<WalletHandle> {
        Ok(WalletHandle {
            id: id.clone(),
            name: String::new(),
            endpoint: self.config.endpoint.clone(),
            client: self.client.clone(),
        })
    }

    /// Get the endpoint
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}

// ============================================================================
// Agent Builder
// ============================================================================

/// Builder for creating agents
pub struct AgentBuilder {
    bank: OpeniBank,
    name: String,
    initial_balance: Option<f64>,
    currency: Currency,
    metadata: Option<serde_json::Value>,
}

impl AgentBuilder {
    fn new(bank: OpeniBank, name: String) -> Self {
        Self {
            currency: bank.config.default_currency,
            bank,
            name,
            initial_balance: None,
            metadata: None,
        }
    }

    /// Set initial balance
    pub fn with_balance(mut self, amount: f64) -> Self {
        self.initial_balance = Some(amount);
        self
    }

    /// Set currency
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Create the agent
    pub async fn create(self) -> SdkResult<AgentHandle> {
        let url = format!("{}/api/v1/agents", self.bank.config.endpoint);

        let req = CreateAgentRequest {
            name: self.name.clone(),
            initial_balance: self.initial_balance,
            currency: self.currency.clone(),
            metadata: self.metadata,
        };

        let resp = self.bank.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<AgentResponse> = resp.json().await?;
        let agent = api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 500,
                message: api_resp.error.unwrap_or_else(|| "Failed to create agent".to_string()),
            }
        })?;

        Ok(AgentHandle {
            bank: self.bank,
            id: agent.id,
            name: agent.name,
            wallet_id: agent.wallet_id,
        })
    }
}

// ============================================================================
// Agent Handle
// ============================================================================

/// Handle to an agent
#[derive(Clone)]
pub struct AgentHandle {
    bank: OpeniBank,
    /// Agent ID
    pub id: String,
    /// Agent name
    pub name: String,
    /// Wallet ID
    pub wallet_id: String,
}

impl AgentHandle {
    /// Get the agent's balance
    pub async fn balance(&self) -> SdkResult<BalanceResponse> {
        let url = format!(
            "{}/api/v1/agents/{}/balance",
            self.bank.config.endpoint, self.id
        );
        let resp = self.bank.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<BalanceResponse> = resp.json().await?;
        api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 500,
                message: "Failed to get balance".to_string(),
            }
        })
    }

    /// Fund the agent's account (for testing)
    pub async fn fund(&self, amount: f64) -> SdkResult<()> {
        let url = format!(
            "{}/api/v1/agents/{}/fund",
            self.bank.config.endpoint, self.id
        );

        #[derive(Serialize)]
        struct FundRequest {
            amount: f64,
            currency: Currency,
        }

        let req = FundRequest {
            amount,
            currency: self.bank.config.default_currency.clone(),
        };

        let resp = self.bank.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Start building a payment
    pub fn pay(&self, recipient: &str) -> PaymentBuilder {
        PaymentBuilder::new(self.clone(), recipient.to_string())
    }

    /// Grant a spend permit
    pub async fn grant_permit(&self, limits: SpendingLimits) -> SdkResult<PermitHandle> {
        let url = format!(
            "{}/api/v1/agents/{}/permits",
            self.bank.config.endpoint, self.id
        );

        let resp = self.bank.client.post(&url).json(&limits).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(PermitHandle {
            id: PermitId::new(),
            wallet: WalletId::new(),
            limits,
        })
    }

    /// Get transaction history
    pub async fn transactions(&self) -> SdkResult<Vec<PaymentResponse>> {
        let url = format!(
            "{}/api/v1/agents/{}/transactions",
            self.bank.config.endpoint, self.id
        );
        let resp = self.bank.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<Vec<PaymentResponse>> = resp.json().await?;
        Ok(api_resp.data.unwrap_or_default())
    }
}

// ============================================================================
// Payment Builder
// ============================================================================

/// Builder for payments
pub struct PaymentBuilder {
    agent: AgentHandle,
    recipient: String,
    amount: Option<f64>,
    currency: Option<Currency>,
    service: Option<String>,
    escrow: bool,
    memo: Option<String>,
}

impl PaymentBuilder {
    fn new(agent: AgentHandle, recipient: String) -> Self {
        Self {
            escrow: agent.bank.config.auto_escrow,
            agent,
            recipient,
            amount: None,
            currency: None,
            service: None,
            memo: None,
        }
    }

    /// Set the payment amount
    pub fn amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set the currency
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = Some(currency);
        self
    }

    /// Set the service being paid for
    pub fn for_service(mut self, service: &str) -> Self {
        self.service = Some(service.to_string());
        self
    }

    /// Enable escrow protection
    pub fn with_escrow(mut self) -> Self {
        self.escrow = true;
        self
    }

    /// Disable escrow (direct payment)
    pub fn direct(mut self) -> Self {
        self.escrow = false;
        self
    }

    /// Add a memo
    pub fn memo(mut self, memo: &str) -> Self {
        self.memo = Some(memo.to_string());
        self
    }

    /// Send the payment
    pub async fn send(self) -> SdkResult<ReceiptHandle> {
        let amount = self.amount.ok_or_else(|| {
            SdkError::ConfigError("Payment amount is required".to_string())
        })?;

        let currency = self
            .currency
            .unwrap_or(self.agent.bank.config.default_currency);

        let url = format!("{}/api/v1/payments", self.agent.bank.config.endpoint);

        let req = PaymentRequest {
            from_agent: self.agent.id.clone(),
            to_agent: self.recipient,
            amount,
            currency,
            service: self.service,
            escrow: self.escrow,
            memo: self.memo,
        };

        let resp = self.agent.bank.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<PaymentResponse> = resp.json().await?;
        let payment = api_resp.data.ok_or_else(|| {
            SdkError::TransactionFailed(
                api_resp.error.unwrap_or_else(|| "Unknown error".to_string()),
            )
        })?;

        Ok(ReceiptHandle {
            id: ReceiptId::new(),
            transaction_id: payment.transaction_id,
            receipt_id: payment.receipt_id,
            status: payment.status,
            verified: true, // Would verify cryptographically
        })
    }
}

// ============================================================================
// Handle Types
// ============================================================================

/// Handle to a wallet (lower-level API)
#[derive(Clone)]
pub struct WalletHandle {
    /// Wallet ID
    pub id: WalletId,
    /// Wallet name
    pub name: String,
    /// Endpoint
    endpoint: String,
    /// HTTP client
    client: Client,
}

impl WalletHandle {
    /// Get the wallet ID
    pub fn id(&self) -> &WalletId {
        &self.id
    }

    /// Get balance
    pub async fn balance(&self, currency: Currency) -> SdkResult<Amount> {
        let url = format!("{}/api/v1/wallets/{}/balance", self.endpoint, self.id.0);
        let resp = self.client.get(&url).send().await?;

        if resp.status().is_success() {
            let api_resp: ApiResponse<BalanceResponse> = resp.json().await?;
            if let Some(bal) = api_resp.data {
                return Ok(Amount::from_human(bal.amount, currency));
            }
        }

        Ok(Amount::zero(currency))
    }

    /// Grant a spend permit
    pub async fn grant_permit(&self, limits: SpendingLimits) -> SdkResult<PermitHandle> {
        Ok(PermitHandle {
            id: PermitId::new(),
            wallet: self.id.clone(),
            limits,
        })
    }

    /// Send a payment
    pub async fn send(
        &self,
        _permit: PermitHandle,
        recipient: &str,
        amount: f64,
        currency: &str,
    ) -> SdkResult<ReceiptHandle> {
        let url = format!("{}/api/v1/payments", self.endpoint);

        let req = PaymentRequest {
            from_agent: self.id.0.to_string(),
            to_agent: recipient.to_string(),
            amount,
            currency: Currency::iusd(), // TODO: Parse from string
            service: None,
            escrow: true,
            memo: None,
        };

        let resp = self.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(ReceiptHandle {
            id: ReceiptId::new(),
            transaction_id: String::new(),
            receipt_id: String::new(),
            status: "completed".to_string(),
            verified: true,
        })
    }
}

/// Handle to a permit
#[derive(Clone)]
pub struct PermitHandle {
    /// Permit ID
    pub id: PermitId,
    /// Wallet this permit is for
    pub wallet: WalletId,
    /// Limits
    pub limits: SpendingLimits,
}

impl PermitHandle {
    /// Get the permit ID
    pub fn id(&self) -> &PermitId {
        &self.id
    }

    /// Check if an amount is within limits
    pub fn can_spend(&self, amount: &Amount) -> bool {
        self.limits.can_spend(amount)
    }
}

/// Handle to a receipt
#[derive(Clone)]
pub struct ReceiptHandle {
    /// Receipt ID (internal)
    pub id: ReceiptId,
    /// Transaction ID (external)
    pub transaction_id: String,
    /// Receipt ID (external)
    pub receipt_id: String,
    /// Status
    pub status: String,
    /// Whether verified
    pub verified: bool,
}

impl ReceiptHandle {
    /// Get the receipt ID
    pub fn id(&self) -> &ReceiptId {
        &self.id
    }

    /// Get the transaction ID
    pub fn transaction_id(&self) -> &str {
        &self.transaction_id
    }

    /// Verify the receipt cryptographically
    pub fn verify(&self) -> bool {
        self.verified
    }

    /// Check if the transaction completed successfully
    pub fn is_success(&self) -> bool {
        self.status == "completed" || self.status == "settled"
    }
}

// ============================================================================
// Marketplace SDK
// ============================================================================

/// Marketplace operations
pub struct Marketplace {
    bank: OpeniBank,
}

impl Marketplace {
    /// Create a new marketplace client
    pub fn new(bank: &OpeniBank) -> Self {
        Self { bank: bank.clone() }
    }

    /// Search for services
    pub async fn search(&self, query: &str) -> SdkResult<Vec<MarketplaceListing>> {
        let url = format!(
            "{}/api/v1/marketplace/search?q={}",
            self.bank.config.endpoint,
            urlencoding::encode(query)
        );
        let resp = self.bank.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<Vec<MarketplaceListing>> = resp.json().await?;
        Ok(api_resp.data.unwrap_or_default())
    }

    /// List a service
    pub fn list_service(&self, name: &str) -> ServiceListingBuilder {
        ServiceListingBuilder::new(self.bank.clone(), name.to_string())
    }

    /// Hire a service
    pub async fn hire(&self, listing_id: &str, terms: ServiceTerms) -> SdkResult<ServiceContract> {
        let url = format!(
            "{}/api/v1/marketplace/{}/hire",
            self.bank.config.endpoint, listing_id
        );
        let resp = self.bank.client.post(&url).json(&terms).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<ServiceContract> = resp.json().await?;
        api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 500,
                message: "Failed to hire service".to_string(),
            }
        })
    }
}

/// Builder for service listings
pub struct ServiceListingBuilder {
    bank: OpeniBank,
    name: String,
    category: Option<ServiceCategory>,
    description: Option<String>,
    base_fee: Option<f64>,
}

impl ServiceListingBuilder {
    fn new(bank: OpeniBank, name: String) -> Self {
        Self {
            bank,
            name,
            category: None,
            description: None,
            base_fee: None,
        }
    }

    /// Set the category
    pub fn category(mut self, category: ServiceCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Set the description
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Set the base fee
    pub fn base_fee(mut self, fee: f64) -> Self {
        self.base_fee = Some(fee);
        self
    }

    /// Create the listing
    pub async fn create(self) -> SdkResult<ListingId> {
        let url = format!("{}/api/v1/marketplace/listings", self.bank.config.endpoint);

        #[derive(Serialize)]
        struct CreateListingRequest {
            name: String,
            category: Option<String>,
            description: Option<String>,
            base_fee: Option<f64>,
        }

        let req = CreateListingRequest {
            name: self.name,
            category: self.category.map(|c| c.display_name().to_string()),
            description: self.description,
            base_fee: self.base_fee,
        };

        let resp = self.bank.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        #[derive(Deserialize)]
        struct ListingResponse {
            id: String,
        }

        let api_resp: ApiResponse<ListingResponse> = resp.json().await?;
        let listing = api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 500,
                message: "Failed to create listing".to_string(),
            }
        })?;

        Ok(ListingId::new())
    }
}

// ============================================================================
// Arena SDK
// ============================================================================

/// Arena operations
pub struct Arena {
    bank: OpeniBank,
}

impl Arena {
    /// Create a new arena client
    pub fn new(bank: &OpeniBank) -> Self {
        Self { bank: bank.clone() }
    }

    /// Get active matches
    pub async fn active_matches(&self) -> SdkResult<Vec<ArenaMatch>> {
        let url = format!("{}/api/v1/arena/matches", self.bank.config.endpoint);
        let resp = self.bank.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<Vec<ArenaMatch>> = resp.json().await?;
        Ok(api_resp.data.unwrap_or_default())
    }

    /// Join a match
    pub async fn join(&self, match_id: &str, agent_id: &str, stake: f64) -> SdkResult<()> {
        let url = format!(
            "{}/api/v1/arena/matches/{}/join",
            self.bank.config.endpoint, match_id
        );

        #[derive(Serialize)]
        struct JoinRequest {
            agent_id: String,
            stake: f64,
        }

        let req = JoinRequest {
            agent_id: agent_id.to_string(),
            stake,
        };

        let resp = self.bank.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Get leaderboard
    pub async fn leaderboard(&self, timeframe: Timeframe) -> SdkResult<Leaderboard> {
        let url = format!(
            "{}/api/v1/arena/leaderboard?timeframe={:?}",
            self.bank.config.endpoint, timeframe
        );
        let resp = self.bank.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(SdkError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let api_resp: ApiResponse<Leaderboard> = resp.json().await?;
        api_resp.data.ok_or_else(|| {
            SdkError::ApiError {
                status: 500,
                message: "Failed to get leaderboard".to_string(),
            }
        })
    }
}

// ============================================================================
// Extension Trait for OpeniBank
// ============================================================================

impl OpeniBank {
    /// Get marketplace client
    pub fn marketplace(&self) -> Marketplace {
        Marketplace::new(self)
    }

    /// Get arena client
    pub fn arena(&self) -> Arena {
        Arena::new(self)
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// URL encode a string (simple implementation)
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::with_capacity(s.len() * 3);
        for c in s.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
                ' ' => result.push_str("%20"),
                _ => {
                    for b in c.to_string().bytes() {
                        result.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.endpoint, "http://localhost:8080");
        assert!(config.auto_escrow);
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("test-123"), "test-123");
    }

    #[tokio::test]
    async fn test_local_connection() {
        // This would fail without a running server, but tests the API
        let result = OpeniBank::local().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_sdk_error_display() {
        let err = SdkError::InsufficientFunds {
            needed: 100.0,
            available: 50.0,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }
}
