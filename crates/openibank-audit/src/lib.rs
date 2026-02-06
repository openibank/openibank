//! OpeniBank Audit - Immutable audit log
//!
//! All consequential actions produce audit entries. The audit log
//! is append-only and cryptographically verifiable.

use openibank_types::*;
use serde::{Deserialize, Serialize};

/// An audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: AuditEntryId,
    /// Previous entry hash (for chain)
    pub previous_hash: String,
    /// Entry hash
    pub hash: String,
    /// Timestamp
    pub timestamp: TemporalAnchor,
    /// Actor
    pub actor: ResonatorId,
    /// Action type
    pub action: AuditAction,
    /// Related commitment
    pub commitment_id: Option<CommitmentId>,
    /// Related transaction
    pub transaction_id: Option<TransactionId>,
    /// Amount involved
    pub amount: Option<Amount>,
    /// Additional data
    pub data: serde_json::Value,
}

/// Types of auditable actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    /// Wallet created
    WalletCreated { wallet_id: WalletId },
    /// Permit granted
    PermitGranted { permit_id: PermitId },
    /// Permit revoked
    PermitRevoked { permit_id: PermitId },
    /// Commitment created
    CommitmentCreated { commitment_id: CommitmentId },
    /// Transaction initiated
    TransactionInitiated { transaction_id: TransactionId },
    /// Transaction completed
    TransactionCompleted { transaction_id: TransactionId },
    /// Escrow created
    EscrowCreated { escrow_id: EscrowId },
    /// Escrow released
    EscrowReleased { escrow_id: EscrowId },
    /// Escrow refunded
    EscrowRefunded { escrow_id: EscrowId },
    /// Policy updated
    PolicyUpdated { rule_id: String },
    /// Custom action
    Custom { action_type: String },
}

impl AuditEntry {
    /// Compute hash of this entry
    pub fn compute_hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let content = format!(
            "{}:{}:{:?}:{:?}",
            self.previous_hash,
            self.timestamp.timestamp,
            self.actor,
            self.action
        );
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify the entry hash
    pub fn verify(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

/// Audit log trait
#[async_trait::async_trait]
pub trait AuditLog: Send + Sync {
    /// Append an entry
    async fn append(&self, entry: AuditEntry) -> Result<AuditEntryId>;

    /// Get an entry by ID
    async fn get(&self, id: &AuditEntryId) -> Result<AuditEntry>;

    /// Get entries for a commitment
    async fn get_for_commitment(&self, commitment_id: &CommitmentId) -> Result<Vec<AuditEntry>>;

    /// Get entries for a wallet
    async fn get_for_wallet(&self, wallet_id: &WalletId) -> Result<Vec<AuditEntry>>;

    /// Verify the chain
    async fn verify_chain(&self) -> Result<bool>;

    /// Export for compliance
    async fn export(
        &self,
        from: Option<TemporalAnchor>,
        to: Option<TemporalAnchor>,
    ) -> Result<Vec<AuditEntry>>;
}
