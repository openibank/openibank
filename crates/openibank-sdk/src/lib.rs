//! OpeniBank SDK - Developer SDK for AI-native banking
//!
//! The SDK provides a simple, high-level API for interacting with OpeniBank.
//! Designed for the 5-minute experience.
//!
//! # Quick Start
//!
//! ```ignore
//! use openibank_sdk::OpeniBank;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to local OpeniBank
//!     let bank = OpeniBank::local().await?;
//!
//!     // Create a wallet
//!     let wallet = bank.create_wallet("my-agent").await?;
//!
//!     // Grant a spend permit
//!     let permit = wallet.grant_permit(SpendingLimits::daily(1000.0)).await?;
//!
//!     // Send payment
//!     let receipt = wallet.send(permit, "recipient-agent", 50.0, "IUSD").await?;
//!
//!     println!("Receipt: {}", receipt.verify());
//!     Ok(())
//! }
//! ```

pub use openibank_types::*;

/// Main OpeniBank client
#[derive(Clone)]
pub struct OpeniBank {
    endpoint: String,
}

impl OpeniBank {
    /// Connect to a local OpeniBank instance
    pub async fn local() -> Result<Self> {
        Self::connect("http://localhost:8080").await
    }

    /// Connect to a specific endpoint
    pub async fn connect(endpoint: &str) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.to_string(),
        })
    }

    /// Create a new wallet
    pub async fn create_wallet(&self, name: &str) -> Result<WalletHandle> {
        Ok(WalletHandle {
            id: WalletId::new(),
            name: name.to_string(),
            endpoint: self.endpoint.clone(),
        })
    }

    /// Get an existing wallet
    pub async fn get_wallet(&self, id: &WalletId) -> Result<WalletHandle> {
        Ok(WalletHandle {
            id: id.clone(),
            name: String::new(),
            endpoint: self.endpoint.clone(),
        })
    }
}

/// Handle to a wallet
#[derive(Clone)]
pub struct WalletHandle {
    /// Wallet ID
    pub id: WalletId,
    /// Wallet name
    pub name: String,
    /// Endpoint
    endpoint: String,
}

impl WalletHandle {
    /// Get the wallet ID
    pub fn id(&self) -> &WalletId {
        &self.id
    }

    /// Get balance
    pub async fn balance(&self, currency: Currency) -> Result<Amount> {
        // Would call API
        Ok(Amount::zero(currency))
    }

    /// Grant a spend permit
    pub async fn grant_permit(&self, limits: SpendingLimits) -> Result<PermitHandle> {
        Ok(PermitHandle {
            id: PermitId::new(),
            wallet: self.id.clone(),
            limits,
        })
    }

    /// Send a payment
    pub async fn send(
        &self,
        permit: PermitHandle,
        recipient: &str,
        amount: f64,
        currency: &str,
    ) -> Result<ReceiptHandle> {
        // Would call API
        Ok(ReceiptHandle {
            id: ReceiptId::new(),
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

/// Handle to a receipt
#[derive(Clone)]
pub struct ReceiptHandle {
    /// Receipt ID
    pub id: ReceiptId,
    /// Whether verified
    pub verified: bool,
}

impl ReceiptHandle {
    /// Verify the receipt
    pub fn verify(&self) -> bool {
        self.verified
    }
}
