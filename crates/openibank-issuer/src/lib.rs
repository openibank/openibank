//! OpeniBank Issuer - Mock IUSD Stablecoin Issuer
//!
//! This crate implements a mocked Issuer Resonator for the IUSD stablecoin.
//! Even mocked, it follows the real rules:
//!
//! 1. Every mint/burn is commitment-gated
//! 2. Every mint/burn produces a CommitmentReceipt + EvidenceBundle hash
//! 3. Issuance is bounded by a ReserveModel
//! 4. Risk Governor can halt minting
//! 5. All receipts are verifiable and stable-schema
//!
//! # Usage
//!
//! The issuer maintains a reserve cap and tracks total supply.
//! Mint operations fail if they would exceed the reserve.
//! All operations produce cryptographically signed receipts.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use openibank_core::{
    crypto::{hash_object, Keypair},
    Amount, AssetId, IssuerOperation, IssuerReceipt, ResonatorId,
};
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Errors that can occur during issuer operations
#[derive(Error, Debug)]
pub enum IssuerError {
    #[error("Reserve exceeded: trying to mint {requested}, but only {available} available")]
    ReserveExceeded { requested: u64, available: u64 },

    #[error("Insufficient supply: trying to burn {requested}, but only {available} circulating")]
    InsufficientSupply { requested: u64, available: u64 },

    #[error("Invalid amount: {message}")]
    InvalidAmount { message: String },

    #[error("Issuer halted: {reason}")]
    IssuerHalted { reason: String },

    #[error("Policy violation: {message}")]
    PolicyViolation { message: String },

    #[error("Ledger error: {0}")]
    LedgerError(#[from] openibank_ledger::LedgerError),

    #[error("Core error: {0}")]
    CoreError(#[from] openibank_core::CoreError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, IssuerError>;

/// Configuration for the issuer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerConfig {
    /// Unique identifier for this issuer
    pub issuer_id: String,
    /// The asset being issued
    pub asset_id: AssetId,
    /// Human-readable name
    pub name: String,
    /// Symbol (e.g., "IUSD")
    pub symbol: String,
    /// Decimals (for display, internally we use smallest units)
    pub decimals: u8,
}

impl Default for IssuerConfig {
    fn default() -> Self {
        Self {
            issuer_id: format!("issuer_{}", Uuid::new_v4()),
            asset_id: AssetId::iusd(),
            name: "OpeniBank USD".to_string(),
            symbol: "IUSD".to_string(),
            decimals: 2,
        }
    }
}

/// The reserve model controls how much can be issued
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveModel {
    /// Maximum total supply that can ever be minted
    pub reserve_cap: Amount,
    /// Last attestation of reserve backing
    pub last_attestation: Option<ReserveAttestation>,
}

impl ReserveModel {
    pub fn new(reserve_cap: Amount) -> Self {
        Self {
            reserve_cap,
            last_attestation: None,
        }
    }

    /// Check if minting amount would exceed reserve
    pub fn can_mint(&self, current_supply: Amount, amount: Amount) -> bool {
        match current_supply.checked_add(amount) {
            Some(new_supply) => new_supply <= self.reserve_cap,
            None => false,
        }
    }
}

/// Attestation of reserve backing (mocked for now)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveAttestation {
    pub attestation_id: String,
    pub reserve_amount: Amount,
    pub attestor: String,
    pub attested_at: DateTime<Utc>,
    pub signature: String,
}

/// Intent to mint new tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintIntent {
    pub intent_id: String,
    pub to: ResonatorId,
    pub amount: Amount,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

impl MintIntent {
    pub fn new(to: ResonatorId, amount: Amount, reason: impl Into<String>) -> Self {
        Self {
            intent_id: format!("mint_{}", Uuid::new_v4()),
            to,
            amount,
            reason: reason.into(),
            created_at: Utc::now(),
        }
    }
}

/// Intent to burn tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnIntent {
    pub intent_id: String,
    pub from: ResonatorId,
    pub amount: Amount,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

impl BurnIntent {
    pub fn new(from: ResonatorId, amount: Amount, reason: impl Into<String>) -> Self {
        Self {
            intent_id: format!("burn_{}", Uuid::new_v4()),
            from,
            amount,
            reason: reason.into(),
            created_at: Utc::now(),
        }
    }
}

/// Policy governing issuance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuancePolicy {
    /// Whether minting is currently enabled
    pub minting_enabled: bool,
    /// Whether burning is currently enabled
    pub burning_enabled: bool,
    /// Maximum single mint amount
    pub max_single_mint: Amount,
    /// Maximum single burn amount
    pub max_single_burn: Amount,
    /// Required attestation age (0 = no requirement)
    pub max_attestation_age_seconds: u64,
}

impl Default for IssuancePolicy {
    fn default() -> Self {
        Self {
            minting_enabled: true,
            burning_enabled: true,
            max_single_mint: Amount::new(1_000_000_00), // $1M
            max_single_burn: Amount::new(1_000_000_00),
            max_attestation_age_seconds: 0, // No requirement for mock
        }
    }
}

/// Internal state of the issuer
#[derive(Debug, Default)]
struct IssuerState {
    /// Current total supply
    total_supply: Amount,
    /// All issued receipts
    receipts: Vec<IssuerReceipt>,
    /// Whether issuer is halted
    halted: bool,
    /// Halt reason if halted
    halt_reason: Option<String>,
}

/// The OpeniBank IUSD Issuer
///
/// A commitment-gated stablecoin issuer that produces verifiable receipts
/// for every mint/burn operation.
pub struct Issuer {
    config: IssuerConfig,
    reserve: Arc<RwLock<ReserveModel>>,
    policy: Arc<RwLock<IssuancePolicy>>,
    state: Arc<RwLock<IssuerState>>,
    keypair: Keypair,
    ledger: Arc<Ledger>,
}

impl Issuer {
    /// Create a new issuer with the given configuration
    pub fn new(config: IssuerConfig, reserve_cap: Amount, ledger: Arc<Ledger>) -> Self {
        Self {
            config,
            reserve: Arc::new(RwLock::new(ReserveModel::new(reserve_cap))),
            policy: Arc::new(RwLock::new(IssuancePolicy::default())),
            state: Arc::new(RwLock::new(IssuerState::default())),
            keypair: Keypair::generate(),
            ledger,
        }
    }

    /// Create with a specific keypair (for testing or persistence)
    pub fn with_keypair(
        config: IssuerConfig,
        reserve_cap: Amount,
        ledger: Arc<Ledger>,
        keypair: Keypair,
    ) -> Self {
        Self {
            config,
            reserve: Arc::new(RwLock::new(ReserveModel::new(reserve_cap))),
            policy: Arc::new(RwLock::new(IssuancePolicy::default())),
            state: Arc::new(RwLock::new(IssuerState::default())),
            keypair,
            ledger,
        }
    }

    /// Get the issuer's public key
    pub fn public_key(&self) -> String {
        self.keypair.public_key_hex()
    }

    /// Get issuer configuration
    pub fn config(&self) -> &IssuerConfig {
        &self.config
    }

    /// Get current total supply
    pub async fn total_supply(&self) -> Amount {
        self.state.read().await.total_supply
    }

    /// Get remaining mintable amount
    pub async fn remaining_supply(&self) -> Amount {
        let state = self.state.read().await;
        let reserve = self.reserve.read().await;
        reserve
            .reserve_cap
            .checked_sub(state.total_supply)
            .unwrap_or(Amount::zero())
    }

    /// Mint new tokens
    ///
    /// This is commitment-gated: it validates policy, creates evidence,
    /// and produces a signed receipt.
    pub async fn mint(&self, intent: MintIntent) -> Result<IssuerReceipt> {
        // Check if halted
        {
            let state = self.state.read().await;
            if state.halted {
                return Err(IssuerError::IssuerHalted {
                    reason: state.halt_reason.clone().unwrap_or_default(),
                });
            }
        }

        // Validate amount
        if intent.amount.is_zero() {
            return Err(IssuerError::InvalidAmount {
                message: "Amount must be greater than zero".to_string(),
            });
        }

        // Check policy
        let policy = self.policy.read().await;
        if !policy.minting_enabled {
            return Err(IssuerError::PolicyViolation {
                message: "Minting is disabled".to_string(),
            });
        }
        if intent.amount > policy.max_single_mint {
            return Err(IssuerError::PolicyViolation {
                message: format!(
                    "Amount {} exceeds max single mint {}",
                    intent.amount, policy.max_single_mint
                ),
            });
        }
        drop(policy);

        // Check reserve
        let reserve = self.reserve.read().await;
        let state = self.state.read().await;
        if !reserve.can_mint(state.total_supply, intent.amount) {
            let available = reserve
                .reserve_cap
                .checked_sub(state.total_supply)
                .unwrap_or(Amount::zero());
            return Err(IssuerError::ReserveExceeded {
                requested: intent.amount.0,
                available: available.0,
            });
        }
        let reserve_attestation_hash = hash_object(&reserve.last_attestation)?;
        let policy_snapshot_hash = {
            let policy = self.policy.read().await;
            hash_object(&*policy)?
        };
        drop(state);
        drop(reserve);

        // Credit to ledger
        let receipt_id = format!("receipt_{}", Uuid::new_v4());
        self.ledger
            .mint(
                &intent.to,
                &self.config.asset_id,
                intent.amount,
                receipt_id.clone(),
            )
            .await?;

        // Update supply
        {
            let mut state = self.state.write().await;
            state.total_supply = state
                .total_supply
                .checked_add(intent.amount)
                .ok_or_else(|| IssuerError::InvalidAmount {
                    message: "Supply overflow".to_string(),
                })?;
        }

        // Create and sign receipt
        let receipt = self.create_receipt(
            receipt_id,
            IssuerOperation::Mint,
            intent.to,
            intent.amount,
            reserve_attestation_hash,
            policy_snapshot_hash,
        )?;

        // Store receipt
        {
            let mut state = self.state.write().await;
            state.receipts.push(receipt.clone());
        }

        Ok(receipt)
    }

    /// Burn tokens
    ///
    /// This is commitment-gated: it validates policy, creates evidence,
    /// and produces a signed receipt.
    pub async fn burn(&self, intent: BurnIntent) -> Result<IssuerReceipt> {
        // Check if halted
        {
            let state = self.state.read().await;
            if state.halted {
                return Err(IssuerError::IssuerHalted {
                    reason: state.halt_reason.clone().unwrap_or_default(),
                });
            }
        }

        // Validate amount
        if intent.amount.is_zero() {
            return Err(IssuerError::InvalidAmount {
                message: "Amount must be greater than zero".to_string(),
            });
        }

        // Check policy
        let policy = self.policy.read().await;
        if !policy.burning_enabled {
            return Err(IssuerError::PolicyViolation {
                message: "Burning is disabled".to_string(),
            });
        }
        if intent.amount > policy.max_single_burn {
            return Err(IssuerError::PolicyViolation {
                message: format!(
                    "Amount {} exceeds max single burn {}",
                    intent.amount, policy.max_single_burn
                ),
            });
        }
        drop(policy);

        // Check supply
        {
            let state = self.state.read().await;
            if intent.amount > state.total_supply {
                return Err(IssuerError::InsufficientSupply {
                    requested: intent.amount.0,
                    available: state.total_supply.0,
                });
            }
        }

        let reserve_attestation_hash = {
            let reserve = self.reserve.read().await;
            hash_object(&reserve.last_attestation)?
        };
        let policy_snapshot_hash = {
            let policy = self.policy.read().await;
            hash_object(&*policy)?
        };

        // Debit from ledger
        let receipt_id = format!("receipt_{}", Uuid::new_v4());
        self.ledger
            .burn(
                &intent.from,
                &self.config.asset_id,
                intent.amount,
                receipt_id.clone(),
            )
            .await?;

        // Update supply
        {
            let mut state = self.state.write().await;
            state.total_supply = state
                .total_supply
                .checked_sub(intent.amount)
                .ok_or_else(|| IssuerError::InvalidAmount {
                    message: "Supply underflow".to_string(),
                })?;
        }

        // Create and sign receipt
        let receipt = self.create_receipt(
            receipt_id,
            IssuerOperation::Burn,
            intent.from,
            intent.amount,
            reserve_attestation_hash,
            policy_snapshot_hash,
        )?;

        // Store receipt
        {
            let mut state = self.state.write().await;
            state.receipts.push(receipt.clone());
        }

        Ok(receipt)
    }

    /// Attest to reserve backing
    pub async fn attest_reserve(&self, reserve_amount: Amount, attestor: String) -> Result<ReserveAttestation> {
        let attestation = ReserveAttestation {
            attestation_id: format!("attest_{}", Uuid::new_v4()),
            reserve_amount,
            attestor,
            attested_at: Utc::now(),
            signature: "mock_signature".to_string(), // In production, this would be a real signature
        };

        let mut reserve = self.reserve.write().await;
        reserve.last_attestation = Some(attestation.clone());

        Ok(attestation)
    }

    /// Halt the issuer (emergency stop)
    pub async fn halt(&self, reason: impl Into<String>) {
        let mut state = self.state.write().await;
        state.halted = true;
        state.halt_reason = Some(reason.into());
    }

    /// Resume the issuer
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        state.halted = false;
        state.halt_reason = None;
    }

    /// Check if issuer is halted
    pub async fn is_halted(&self) -> bool {
        self.state.read().await.halted
    }

    /// Get all receipts
    pub async fn receipts(&self) -> Vec<IssuerReceipt> {
        self.state.read().await.receipts.clone()
    }

    /// Get recent receipts
    pub async fn recent_receipts(&self, limit: usize) -> Vec<IssuerReceipt> {
        let state = self.state.read().await;
        state.receipts.iter().rev().take(limit).cloned().collect()
    }

    /// Update issuance policy
    pub async fn update_policy(&self, policy: IssuancePolicy) {
        let mut p = self.policy.write().await;
        *p = policy;
    }

    /// Get current policy
    pub async fn policy(&self) -> IssuancePolicy {
        self.policy.read().await.clone()
    }

    /// Create a signed receipt
    fn create_receipt(
        &self,
        receipt_id: String,
        operation: IssuerOperation,
        target: ResonatorId,
        amount: Amount,
        reserve_attestation_hash: String,
        policy_snapshot_hash: String,
    ) -> Result<IssuerReceipt> {
        let issued_at = Utc::now();

        // Create signable content
        #[derive(Serialize)]
        struct SignableReceipt {
            receipt_id: String,
            operation: IssuerOperation,
            asset: AssetId,
            amount: Amount,
            target: ResonatorId,
            reserve_attestation_hash: String,
            policy_snapshot_hash: String,
            issued_at: DateTime<Utc>,
        }

        let signable = SignableReceipt {
            receipt_id: receipt_id.clone(),
            operation: operation.clone(),
            asset: self.config.asset_id.clone(),
            amount,
            target: target.clone(),
            reserve_attestation_hash: reserve_attestation_hash.clone(),
            policy_snapshot_hash: policy_snapshot_hash.clone(),
            issued_at,
        };

        let signing_bytes = serde_json::to_vec(&signable)?;
        let signature = self.keypair.sign(&signing_bytes);

        Ok(IssuerReceipt {
            receipt_id,
            operation,
            asset: self.config.asset_id.clone(),
            amount,
            target,
            reserve_attestation_hash,
            policy_snapshot_hash,
            issued_at,
            signature,
            signer_public_key: self.keypair.public_key_hex(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_issuer() -> Issuer {
        let ledger = Arc::new(Ledger::new());
        Issuer::new(
            IssuerConfig::default(),
            Amount::new(1_000_000_00), // $1M reserve cap
            ledger,
        )
    }

    #[tokio::test]
    async fn test_mint() {
        let issuer = create_test_issuer().await;
        let recipient = ResonatorId::new();

        let intent = MintIntent::new(recipient.clone(), Amount::new(10000), "Test mint");

        let receipt = issuer.mint(intent).await.unwrap();

        assert_eq!(receipt.amount, Amount::new(10000));
        assert!(receipt.verify().is_ok());
        assert_eq!(issuer.total_supply().await, Amount::new(10000));
    }

    #[tokio::test]
    async fn test_mint_exceeds_reserve() {
        let ledger = Arc::new(Ledger::new());
        // Create issuer with small reserve cap but large single mint allowance
        let mut policy = IssuancePolicy::default();
        policy.max_single_mint = Amount::new(100_000_000_00); // Allow large single mints

        let issuer = Issuer::new(
            IssuerConfig::default(),
            Amount::new(1_000_00), // Only $10 reserve cap
            ledger,
        );
        issuer.update_policy(policy).await;

        let recipient = ResonatorId::new();

        // Try to mint more than reserve cap (but within single mint limit)
        let intent = MintIntent::new(recipient, Amount::new(2_000_00), "Over-mint"); // $20

        let result = issuer.mint(intent).await;
        assert!(matches!(result, Err(IssuerError::ReserveExceeded { .. })));
    }

    #[tokio::test]
    async fn test_burn() {
        let issuer = create_test_issuer().await;
        let account = ResonatorId::new();

        // First mint
        let mint_intent = MintIntent::new(account.clone(), Amount::new(10000), "Mint for burn");
        issuer.mint(mint_intent).await.unwrap();

        // Then burn
        let burn_intent = BurnIntent::new(account, Amount::new(5000), "Test burn");
        let receipt = issuer.burn(burn_intent).await.unwrap();

        assert_eq!(receipt.amount, Amount::new(5000));
        assert!(receipt.verify().is_ok());
        assert_eq!(issuer.total_supply().await, Amount::new(5000));
    }

    #[tokio::test]
    async fn test_burn_exceeds_supply() {
        let issuer = create_test_issuer().await;
        let account = ResonatorId::new();

        // Mint a small amount
        let mint_intent = MintIntent::new(account.clone(), Amount::new(1000), "Small mint");
        issuer.mint(mint_intent).await.unwrap();

        // Try to burn more
        let burn_intent = BurnIntent::new(account, Amount::new(5000), "Over-burn");
        let result = issuer.burn(burn_intent).await;

        assert!(matches!(result, Err(IssuerError::InsufficientSupply { .. })));
    }

    #[tokio::test]
    async fn test_halt_and_resume() {
        let issuer = create_test_issuer().await;
        let account = ResonatorId::new();

        // Halt the issuer
        issuer.halt("Maintenance").await;
        assert!(issuer.is_halted().await);

        // Try to mint while halted
        let intent = MintIntent::new(account.clone(), Amount::new(1000), "Test");
        let result = issuer.mint(intent).await;
        assert!(matches!(result, Err(IssuerError::IssuerHalted { .. })));

        // Resume
        issuer.resume().await;
        assert!(!issuer.is_halted().await);

        // Now mint should work
        let intent = MintIntent::new(account, Amount::new(1000), "Test");
        let result = issuer.mint(intent).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_receipt_verification() {
        let issuer = create_test_issuer().await;
        let recipient = ResonatorId::new();

        let intent = MintIntent::new(recipient, Amount::new(10000), "Test");
        let receipt = issuer.mint(intent).await.unwrap();

        // Verify the receipt
        assert!(receipt.verify().is_ok());

        // Check receipt is stored
        let receipts = issuer.receipts().await;
        assert_eq!(receipts.len(), 1);
    }

    #[tokio::test]
    async fn test_reserve_attestation() {
        let issuer = create_test_issuer().await;

        let attestation = issuer
            .attest_reserve(Amount::new(1_000_000_00), "Mock Attestor".to_string())
            .await
            .unwrap();

        assert_eq!(attestation.reserve_amount, Amount::new(1_000_000_00));
    }
}
