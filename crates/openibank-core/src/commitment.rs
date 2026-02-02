//! Commitment Gate - the heart of OpeniBank
//!
//! Every economic action must:
//! 1. Present Intent
//! 2. Produce Commitment
//! 3. Attach Evidence
//! 4. Pass Policy
//! 5. Emit Receipt
//!
//! No exceptions. This is how we achieve:
//! - Auditability
//! - Replayability
//! - Accountability

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::{hash_object, Keypair};
use crate::error::{CoreError, Result};
use crate::types::*;

// ============================================================================
// Commitment Types
// ============================================================================

/// Unique identifier for a commitment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitmentId(pub String);

impl CommitmentId {
    pub fn new() -> Self {
        Self(format!("commit_{}", Uuid::new_v4()))
    }
}

impl Default for CommitmentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference to the consequence of a commitment (e.g., ledger entry, on-chain tx)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsequenceRef {
    /// Type of consequence (ledger, chain, etc.)
    pub consequence_type: String,
    /// Reference ID
    pub reference_id: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Evidence bundle containing all inputs for a commitment
///
/// This captures everything needed to understand why an action was allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Hash of the payment intent
    pub intent_hash: String,
    /// Hash of the policy snapshot at decision time
    pub policy_snapshot_hash: String,
    /// Hash of the budget state at decision time
    pub budget_snapshot_hash: String,
    /// Hash of the permit used
    pub permit_hash: String,
    /// Additional attestations
    pub attestations: Vec<Attestation>,
    /// Timestamp when evidence was gathered
    pub gathered_at: DateTime<Utc>,
}

/// An attestation from an external source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    pub attestor_id: String,
    pub attestation_type: String,
    pub content_hash: String,
    pub signature: String,
}

/// A commitment receipt - proof that an action was authorized
///
/// Receipts are:
/// - Verifiable
/// - Shareable
/// - Stable (schema won't change incompatibly)
/// - Machine-readable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentReceipt {
    /// Unique receipt ID
    pub commitment_id: CommitmentId,
    /// Who made the commitment
    pub actor: ResonatorId,
    /// Hash of the original intent
    pub intent_hash: String,
    /// Hash of the policy snapshot
    pub policy_snapshot_hash: String,
    /// Hash of all evidence
    pub evidence_hash: String,
    /// Reference to the consequence (settlement, ledger entry, etc.)
    pub consequence_ref: ConsequenceRef,
    /// When the commitment was made
    pub committed_at: DateTime<Utc>,
    /// Signature by the actor's wallet key
    pub signature: String,
    /// Public key that signed this receipt
    pub signer_public_key: String,
}

impl CommitmentReceipt {
    /// Get the canonical bytes for signing/verification
    pub fn signing_bytes(&self) -> Result<Vec<u8>> {
        // Create a signable version without the signature
        let signable = SignableReceipt {
            commitment_id: self.commitment_id.clone(),
            actor: self.actor.clone(),
            intent_hash: self.intent_hash.clone(),
            policy_snapshot_hash: self.policy_snapshot_hash.clone(),
            evidence_hash: self.evidence_hash.clone(),
            consequence_ref: self.consequence_ref.clone(),
            committed_at: self.committed_at,
        };
        Ok(serde_json::to_vec(&signable)?)
    }

    /// Verify the receipt signature
    pub fn verify(&self) -> Result<()> {
        let bytes = self.signing_bytes()?;
        crate::crypto::verify_signature(&self.signer_public_key, &bytes, &self.signature)
    }
}

/// Internal type for creating the signable portion of a receipt
#[derive(Serialize)]
struct SignableReceipt {
    commitment_id: CommitmentId,
    actor: ResonatorId,
    intent_hash: String,
    policy_snapshot_hash: String,
    evidence_hash: String,
    consequence_ref: ConsequenceRef,
    committed_at: DateTime<Utc>,
}

// ============================================================================
// Commitment Gate
// ============================================================================

/// The Commitment Gate validates and creates commitments
///
/// No economic action can bypass this gate.
#[derive(Clone)]
pub struct CommitmentGate {
    keypair: Keypair,
}

impl CommitmentGate {
    /// Create a new commitment gate with a keypair
    pub fn new(keypair: Keypair) -> Self {
        Self { keypair }
    }

    /// Create a commitment for a payment intent
    ///
    /// This is the core function that ensures all invariants are met.
    pub fn create_commitment(
        &self,
        intent: &PaymentIntent,
        permit: &SpendPermit,
        budget: &BudgetPolicy,
        consequence: ConsequenceRef,
    ) -> Result<(CommitmentReceipt, EvidenceBundle)> {
        // 1. Validate the intent against the permit
        permit.can_cover(
            intent.amount,
            &intent.target,
            &AssetClass::Stablecoin, // TODO: derive from asset
        )?;

        // 2. Validate the permit against the budget
        if permit.bound_budget != budget.budget_id {
            return Err(CoreError::PolicyViolation {
                message: "Permit not bound to this budget".to_string(),
            });
        }

        if !budget.can_spend(intent.amount, &intent.target) {
            return Err(CoreError::BudgetExceeded {
                message: "Budget cannot cover this spend".to_string(),
            });
        }

        // 3. Create evidence bundle
        let intent_hash = hash_object(intent)?;
        let policy_snapshot_hash = hash_object(budget)?;
        let permit_hash = hash_object(permit)?;
        let budget_snapshot_hash = hash_object(budget)?;

        let evidence = EvidenceBundle {
            intent_hash: intent_hash.clone(),
            policy_snapshot_hash: policy_snapshot_hash.clone(),
            budget_snapshot_hash,
            permit_hash,
            attestations: vec![],
            gathered_at: Utc::now(),
        };

        let evidence_hash = hash_object(&evidence)?;

        // 4. Create and sign the receipt
        let commitment_id = CommitmentId::new();
        let committed_at = Utc::now();

        let signable = SignableReceipt {
            commitment_id: commitment_id.clone(),
            actor: intent.actor.clone(),
            intent_hash: intent_hash.clone(),
            policy_snapshot_hash: policy_snapshot_hash.clone(),
            evidence_hash: evidence_hash.clone(),
            consequence_ref: consequence.clone(),
            committed_at,
        };

        let signing_bytes = serde_json::to_vec(&signable)?;
        let signature = self.keypair.sign(&signing_bytes);

        let receipt = CommitmentReceipt {
            commitment_id,
            actor: intent.actor.clone(),
            intent_hash,
            policy_snapshot_hash,
            evidence_hash,
            consequence_ref: consequence,
            committed_at,
            signature,
            signer_public_key: self.keypair.public_key_hex(),
        };

        Ok((receipt, evidence))
    }

    /// Get the public key for this gate
    pub fn public_key(&self) -> String {
        self.keypair.public_key_hex()
    }
}

// ============================================================================
// Issuer Receipt (for mint/burn operations)
// ============================================================================

/// Receipt for an issuer operation (mint/burn)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuerReceipt {
    pub receipt_id: String,
    pub operation: IssuerOperation,
    pub asset: AssetId,
    pub amount: Amount,
    pub target: ResonatorId,
    pub reserve_attestation_hash: String,
    pub policy_snapshot_hash: String,
    pub issued_at: DateTime<Utc>,
    pub signature: String,
    pub signer_public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssuerOperation {
    Mint,
    Burn,
}

impl IssuerReceipt {
    /// Get the canonical bytes for signing/verification
    pub fn signing_bytes(&self) -> Result<Vec<u8>> {
        let signable = SignableIssuerReceipt {
            receipt_id: self.receipt_id.clone(),
            operation: self.operation.clone(),
            asset: self.asset.clone(),
            amount: self.amount,
            target: self.target.clone(),
            reserve_attestation_hash: self.reserve_attestation_hash.clone(),
            policy_snapshot_hash: self.policy_snapshot_hash.clone(),
            issued_at: self.issued_at,
        };
        Ok(serde_json::to_vec(&signable)?)
    }

    /// Verify the receipt signature
    pub fn verify(&self) -> Result<()> {
        let bytes = self.signing_bytes()?;
        crate::crypto::verify_signature(&self.signer_public_key, &bytes, &self.signature)
    }
}

#[derive(Serialize)]
struct SignableIssuerReceipt {
    receipt_id: String,
    operation: IssuerOperation,
    asset: AssetId,
    amount: Amount,
    target: ResonatorId,
    reserve_attestation_hash: String,
    policy_snapshot_hash: String,
    issued_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_intent() -> PaymentIntent {
        PaymentIntent::new(
            ResonatorId::from_string("buyer"),
            PermitId::new(),
            ResonatorId::from_string("seller"),
            Amount::new(100),
            AssetId::iusd(),
            SpendPurpose {
                category: "test".to_string(),
                description: "Test payment".to_string(),
            },
        )
    }

    fn create_test_permit(intent: &PaymentIntent, budget_id: &BudgetId) -> SpendPermit {
        SpendPermit {
            permit_id: intent.permit.clone(),
            issuer: intent.actor.clone(),
            bound_budget: budget_id.clone(),
            asset_class: AssetClass::Stablecoin,
            max_amount: Amount::new(1000),
            remaining: Amount::new(1000),
            counterparty: CounterpartyConstraint::Any,
            purpose: SpendPurpose {
                category: "test".to_string(),
                description: "Test permit".to_string(),
            },
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            signature: "test".to_string(),
        }
    }

    #[test]
    fn test_commitment_creation() {
        let keypair = Keypair::generate();
        let gate = CommitmentGate::new(keypair);

        let intent = create_test_intent();
        let budget = BudgetPolicy::new(intent.actor.clone(), Amount::new(10000));
        let permit = create_test_permit(&intent, &budget.budget_id);

        let consequence = ConsequenceRef {
            consequence_type: "ledger".to_string(),
            reference_id: "entry_123".to_string(),
            metadata: serde_json::json!({}),
        };

        let (receipt, evidence) = gate
            .create_commitment(&intent, &permit, &budget, consequence)
            .unwrap();

        // Verify the receipt
        assert!(receipt.verify().is_ok());

        // Check evidence was properly captured
        assert!(!evidence.intent_hash.is_empty());
        assert!(!evidence.policy_snapshot_hash.is_empty());
    }

    #[test]
    fn test_commitment_fails_on_permit_exceeded() {
        let keypair = Keypair::generate();
        let gate = CommitmentGate::new(keypair);

        let mut intent = create_test_intent();
        intent.amount = Amount::new(2000); // More than permit allows

        let budget = BudgetPolicy::new(intent.actor.clone(), Amount::new(10000));
        let permit = create_test_permit(&intent, &budget.budget_id);

        let consequence = ConsequenceRef {
            consequence_type: "ledger".to_string(),
            reference_id: "entry_123".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = gate.create_commitment(&intent, &permit, &budget, consequence);
        assert!(matches!(result, Err(CoreError::PermitExceeded { .. })));
    }

    #[test]
    fn test_receipt_verification() {
        let keypair = Keypair::generate();
        let gate = CommitmentGate::new(keypair);

        let intent = create_test_intent();
        let budget = BudgetPolicy::new(intent.actor.clone(), Amount::new(10000));
        let permit = create_test_permit(&intent, &budget.budget_id);

        let consequence = ConsequenceRef {
            consequence_type: "ledger".to_string(),
            reference_id: "entry_123".to_string(),
            metadata: serde_json::json!({}),
        };

        let (receipt, _) = gate
            .create_commitment(&intent, &permit, &budget, consequence)
            .unwrap();

        // Valid signature should verify
        assert!(receipt.verify().is_ok());

        // Tampered receipt should fail
        let mut tampered = receipt.clone();
        tampered.intent_hash = "tampered".to_string();
        assert!(tampered.verify().is_err());
    }
}
