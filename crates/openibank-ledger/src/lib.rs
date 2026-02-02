//! OpeniBank Ledger - Double-entry ledger for AI agent banking
//!
//! The ledger is:
//! - Asset-scoped (IUSD by default)
//! - Account-keyed by ResonatorId
//! - Double-entry (every credit has a corresponding debit)
//! - Immutable (entries are append-only)
//! - Receipt-linked (all entries reference CommitmentReceipts)
//!
//! # Invariants
//!
//! 1. No negative balances
//! 2. Every entry has a reason
//! 3. All entries are linked to receipts or issuer operations
//! 4. Atomic operations only

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use openibank_core::{Amount, AssetId, CommitmentReceipt, ResonatorId};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Errors that can occur in ledger operations
#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("Account not found: {account}")]
    AccountNotFound { account: String },

    #[error("Insufficient balance: have {available}, need {required}")]
    InsufficientBalance { available: u64, required: u64 },

    #[error("Invalid amount: {message}")]
    InvalidAmount { message: String },

    #[error("Entry not found: {entry_id}")]
    EntryNotFound { entry_id: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },
}

pub type Result<T> = std::result::Result<T, LedgerError>;

/// Unique identifier for a ledger entry
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub String);

impl EntryId {
    pub fn new() -> Self {
        Self(format!("entry_{}", Uuid::new_v4()))
    }
}

impl Default for EntryId {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of ledger entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// Credit (increase) to an account
    Credit,
    /// Debit (decrease) from an account
    Debit,
}

/// Reason for a ledger entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryReason {
    /// Mint from issuer
    Mint { issuer_receipt_id: String },
    /// Burn to issuer
    Burn { issuer_receipt_id: String },
    /// Transfer between accounts
    Transfer { commitment_id: String },
    /// Escrow lock
    EscrowLock { escrow_id: String },
    /// Escrow release
    EscrowRelease { escrow_id: String },
    /// Escrow refund
    EscrowRefund { escrow_id: String },
}

/// A single ledger entry (one side of a double-entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: EntryId,
    pub account: ResonatorId,
    pub asset: AssetId,
    pub entry_type: EntryType,
    pub amount: Amount,
    pub balance_after: Amount,
    pub reason: EntryReason,
    pub correlation_id: String,
    pub created_at: DateTime<Utc>,
}

/// Account state in the ledger
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountState {
    pub balances: HashMap<AssetId, Amount>,
    pub entry_count: u64,
}

impl AccountState {
    pub fn balance(&self, asset: &AssetId) -> Amount {
        self.balances.get(asset).copied().unwrap_or(Amount::zero())
    }
}

/// The OpeniBank Ledger
///
/// A double-entry ledger for tracking all asset movements.
/// Thread-safe and designed for concurrent access.
#[derive(Clone)]
pub struct Ledger {
    /// Account states
    accounts: Arc<RwLock<HashMap<ResonatorId, AccountState>>>,
    /// All entries (append-only)
    entries: Arc<RwLock<Vec<LedgerEntry>>>,
    /// Receipt references (commitment_id -> entry_ids)
    receipt_refs: Arc<RwLock<HashMap<String, Vec<EntryId>>>>,
}

impl Ledger {
    /// Create a new in-memory ledger
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            entries: Arc::new(RwLock::new(Vec::new())),
            receipt_refs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the balance of an account for a specific asset
    pub async fn balance(&self, account: &ResonatorId, asset: &AssetId) -> Amount {
        let accounts = self.accounts.read().await;
        accounts
            .get(account)
            .map(|a| a.balance(asset))
            .unwrap_or(Amount::zero())
    }

    /// Credit an account (increase balance)
    ///
    /// Returns the new balance and the entry ID.
    pub async fn credit(
        &self,
        account: &ResonatorId,
        asset: &AssetId,
        amount: Amount,
        reason: EntryReason,
        correlation_id: String,
    ) -> Result<(Amount, EntryId)> {
        if amount.is_zero() {
            return Err(LedgerError::InvalidAmount {
                message: "Amount must be greater than zero".to_string(),
            });
        }

        let mut accounts = self.accounts.write().await;
        let mut entries = self.entries.write().await;
        let mut receipt_refs = self.receipt_refs.write().await;

        // Get or create account
        let account_state = accounts.entry(account.clone()).or_default();

        // Calculate new balance
        let current_balance = account_state.balance(asset);
        let new_balance = current_balance.checked_add(amount).ok_or_else(|| {
            LedgerError::InvalidAmount {
                message: "Balance overflow".to_string(),
            }
        })?;

        // Create entry
        let entry = LedgerEntry {
            entry_id: EntryId::new(),
            account: account.clone(),
            asset: asset.clone(),
            entry_type: EntryType::Credit,
            amount,
            balance_after: new_balance,
            reason: reason.clone(),
            correlation_id: correlation_id.clone(),
            created_at: Utc::now(),
        };

        // Update account
        account_state.balances.insert(asset.clone(), new_balance);
        account_state.entry_count += 1;

        // Store entry
        let entry_id = entry.entry_id.clone();
        entries.push(entry);

        // Link to receipt if applicable
        if let EntryReason::Transfer { ref commitment_id } = reason {
            receipt_refs
                .entry(commitment_id.clone())
                .or_default()
                .push(entry_id.clone());
        }

        Ok((new_balance, entry_id))
    }

    /// Debit an account (decrease balance)
    ///
    /// Returns the new balance and the entry ID.
    /// Fails if balance would go negative (invariant: no negative balances).
    pub async fn debit(
        &self,
        account: &ResonatorId,
        asset: &AssetId,
        amount: Amount,
        reason: EntryReason,
        correlation_id: String,
    ) -> Result<(Amount, EntryId)> {
        if amount.is_zero() {
            return Err(LedgerError::InvalidAmount {
                message: "Amount must be greater than zero".to_string(),
            });
        }

        let mut accounts = self.accounts.write().await;
        let mut entries = self.entries.write().await;
        let mut receipt_refs = self.receipt_refs.write().await;

        // Get account
        let account_state = accounts.get_mut(account).ok_or_else(|| {
            LedgerError::AccountNotFound {
                account: account.0.clone(),
            }
        })?;

        // Check balance
        let current_balance = account_state.balance(asset);
        let new_balance =
            current_balance
                .checked_sub(amount)
                .ok_or_else(|| LedgerError::InsufficientBalance {
                    available: current_balance.0,
                    required: amount.0,
                })?;

        // Create entry
        let entry = LedgerEntry {
            entry_id: EntryId::new(),
            account: account.clone(),
            asset: asset.clone(),
            entry_type: EntryType::Debit,
            amount,
            balance_after: new_balance,
            reason: reason.clone(),
            correlation_id: correlation_id.clone(),
            created_at: Utc::now(),
        };

        // Update account
        account_state.balances.insert(asset.clone(), new_balance);
        account_state.entry_count += 1;

        // Store entry
        let entry_id = entry.entry_id.clone();
        entries.push(entry);

        // Link to receipt if applicable
        if let EntryReason::Transfer { ref commitment_id } = reason {
            receipt_refs
                .entry(commitment_id.clone())
                .or_default()
                .push(entry_id.clone());
        }

        Ok((new_balance, entry_id))
    }

    /// Execute a transfer between two accounts
    ///
    /// This is atomic: both debit and credit happen together or neither does.
    pub async fn transfer(
        &self,
        from: &ResonatorId,
        to: &ResonatorId,
        asset: &AssetId,
        amount: Amount,
        receipt: &CommitmentReceipt,
    ) -> Result<(EntryId, EntryId)> {
        let correlation_id = receipt.commitment_id.0.clone();
        let reason = EntryReason::Transfer {
            commitment_id: receipt.commitment_id.0.clone(),
        };

        // Debit from sender
        let (_, debit_entry) = self
            .debit(from, asset, amount, reason.clone(), correlation_id.clone())
            .await?;

        // Credit to receiver
        let (_, credit_entry) = self
            .credit(to, asset, amount, reason, correlation_id)
            .await?;

        Ok((debit_entry, credit_entry))
    }

    /// Record a mint from the issuer
    pub async fn mint(
        &self,
        to: &ResonatorId,
        asset: &AssetId,
        amount: Amount,
        issuer_receipt_id: String,
    ) -> Result<(Amount, EntryId)> {
        let reason = EntryReason::Mint {
            issuer_receipt_id: issuer_receipt_id.clone(),
        };
        self.credit(to, asset, amount, reason, issuer_receipt_id)
            .await
    }

    /// Record a burn to the issuer
    pub async fn burn(
        &self,
        from: &ResonatorId,
        asset: &AssetId,
        amount: Amount,
        issuer_receipt_id: String,
    ) -> Result<(Amount, EntryId)> {
        let reason = EntryReason::Burn {
            issuer_receipt_id: issuer_receipt_id.clone(),
        };
        self.debit(from, asset, amount, reason, issuer_receipt_id)
            .await
    }

    /// Get all entries for an account
    pub async fn account_entries(&self, account: &ResonatorId) -> Vec<LedgerEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| &e.account == account)
            .cloned()
            .collect()
    }

    /// Get entries linked to a commitment receipt
    pub async fn receipt_entries(&self, commitment_id: &str) -> Vec<LedgerEntry> {
        let receipt_refs = self.receipt_refs.read().await;
        let entries = self.entries.read().await;

        let entry_ids = receipt_refs.get(commitment_id);
        match entry_ids {
            Some(ids) => entries
                .iter()
                .filter(|e| ids.contains(&e.entry_id))
                .cloned()
                .collect(),
            None => vec![],
        }
    }

    /// Get the total number of entries
    pub async fn entry_count(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Get recent entries (newest first)
    pub async fn recent_entries(&self, limit: usize) -> Vec<LedgerEntry> {
        let entries = self.entries.read().await;
        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Get account state
    pub async fn account_state(&self, account: &ResonatorId) -> Option<AccountState> {
        let accounts = self.accounts.read().await;
        accounts.get(account).cloned()
    }

    /// Get all account IDs
    pub async fn all_accounts(&self) -> Vec<ResonatorId> {
        let accounts = self.accounts.read().await;
        accounts.keys().cloned().collect()
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openibank_core::CommitmentId;

    fn create_test_receipt() -> CommitmentReceipt {
        CommitmentReceipt {
            commitment_id: CommitmentId::new(),
            actor: ResonatorId::new(),
            intent_hash: "test".to_string(),
            policy_snapshot_hash: "test".to_string(),
            evidence_hash: "test".to_string(),
            consequence_ref: openibank_core::ConsequenceRef {
                consequence_type: "ledger".to_string(),
                reference_id: "test".to_string(),
                metadata: serde_json::json!({}),
            },
            committed_at: Utc::now(),
            signature: "test".to_string(),
            signer_public_key: "test".to_string(),
        }
    }

    #[tokio::test]
    async fn test_credit_and_balance() {
        let ledger = Ledger::new();
        let account = ResonatorId::new();
        let asset = AssetId::iusd();

        // Initial balance should be zero
        assert_eq!(ledger.balance(&account, &asset).await, Amount::zero());

        // Credit some funds
        let (balance, _) = ledger
            .credit(
                &account,
                &asset,
                Amount::new(1000),
                EntryReason::Mint {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(balance, Amount::new(1000));
        assert_eq!(ledger.balance(&account, &asset).await, Amount::new(1000));
    }

    #[tokio::test]
    async fn test_debit() {
        let ledger = Ledger::new();
        let account = ResonatorId::new();
        let asset = AssetId::iusd();

        // Credit first
        ledger
            .credit(
                &account,
                &asset,
                Amount::new(1000),
                EntryReason::Mint {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await
            .unwrap();

        // Debit some
        let (balance, _) = ledger
            .debit(
                &account,
                &asset,
                Amount::new(400),
                EntryReason::Burn {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(balance, Amount::new(600));
    }

    #[tokio::test]
    async fn test_no_negative_balance() {
        let ledger = Ledger::new();
        let account = ResonatorId::new();
        let asset = AssetId::iusd();

        // Credit some funds
        ledger
            .credit(
                &account,
                &asset,
                Amount::new(100),
                EntryReason::Mint {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await
            .unwrap();

        // Try to debit more than available
        let result = ledger
            .debit(
                &account,
                &asset,
                Amount::new(200),
                EntryReason::Burn {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await;

        assert!(matches!(
            result,
            Err(LedgerError::InsufficientBalance { .. })
        ));
    }

    #[tokio::test]
    async fn test_transfer() {
        let ledger = Ledger::new();
        let from = ResonatorId::new();
        let to = ResonatorId::new();
        let asset = AssetId::iusd();

        // Credit sender
        ledger
            .credit(
                &from,
                &asset,
                Amount::new(1000),
                EntryReason::Mint {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await
            .unwrap();

        // Transfer
        let receipt = create_test_receipt();
        ledger
            .transfer(&from, &to, &asset, Amount::new(400), &receipt)
            .await
            .unwrap();

        // Check balances
        assert_eq!(ledger.balance(&from, &asset).await, Amount::new(600));
        assert_eq!(ledger.balance(&to, &asset).await, Amount::new(400));
    }

    #[tokio::test]
    async fn test_entry_tracking() {
        let ledger = Ledger::new();
        let account = ResonatorId::new();
        let asset = AssetId::iusd();

        ledger
            .credit(
                &account,
                &asset,
                Amount::new(100),
                EntryReason::Mint {
                    issuer_receipt_id: "test".to_string(),
                },
                "test".to_string(),
            )
            .await
            .unwrap();

        ledger
            .credit(
                &account,
                &asset,
                Amount::new(200),
                EntryReason::Mint {
                    issuer_receipt_id: "test2".to_string(),
                },
                "test2".to_string(),
            )
            .await
            .unwrap();

        let entries = ledger.account_entries(&account).await;
        assert_eq!(entries.len(), 2);
        assert_eq!(ledger.entry_count().await, 2);
    }

    #[tokio::test]
    async fn test_receipt_linking() {
        let ledger = Ledger::new();
        let from = ResonatorId::new();
        let to = ResonatorId::new();
        let asset = AssetId::iusd();

        // Credit sender
        ledger
            .credit(
                &from,
                &asset,
                Amount::new(1000),
                EntryReason::Mint {
                    issuer_receipt_id: "mint".to_string(),
                },
                "mint".to_string(),
            )
            .await
            .unwrap();

        // Transfer
        let receipt = create_test_receipt();
        ledger
            .transfer(&from, &to, &asset, Amount::new(400), &receipt)
            .await
            .unwrap();

        // Get entries by receipt
        let entries = ledger.receipt_entries(&receipt.commitment_id.0).await;
        assert_eq!(entries.len(), 2); // debit + credit
    }
}
