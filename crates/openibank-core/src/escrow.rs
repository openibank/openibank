//! OpeniBank Escrow Manager - Commitment-gated conditional settlement
//!
//! The EscrowManager handles all escrow operations through the commitment boundary:
//! - Creating escrows from invoices
//! - Funding escrows (CROSSES COMMITMENT BOUNDARY)
//! - Submitting delivery evidence
//! - Releasing escrows (CROSSES COMMITMENT BOUNDARY)
//! - Refunding escrows (CROSSES COMMITMENT BOUNDARY)
//!
//! # Key Principle
//!
//! ALL escrow operations that move money MUST cross the commitment boundary.
//! The EscrowManager enforces this invariant.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::commitment::{CommitmentGate, CommitmentReceipt, ConsequenceRef, EvidenceBundle};
use crate::crypto::Keypair;
use crate::error::{CoreError, Result};
use crate::types::*;

// ============================================================================
// Enhanced Delivery Conditions
// ============================================================================

/// Structured delivery condition types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryConditionType {
    /// Service completion with identifier
    ServiceCompletion { service_id: String },
    /// Data delivery with expected hash
    DataDelivery { data_hash: String },
    /// Oracle attestation required
    OracleAttestation {
        oracle_id: ResonatorId,
        expected_value: String,
    },
    /// Buyer confirmation required
    BuyerConfirmation,
    /// Time-based (auto-release after duration)
    TimeBased { release_after: Duration },
    /// Custom condition
    Custom {
        condition_type: String,
        parameters: serde_json::Value,
    },
}

/// A delivery condition with status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedDeliveryCondition {
    pub condition_type: DeliveryConditionType,
    pub met: bool,
    pub met_at: Option<DateTime<Utc>>,
    pub evidence: Option<String>,
}

impl TrackedDeliveryCondition {
    pub fn new(condition_type: DeliveryConditionType) -> Self {
        Self {
            condition_type,
            met: false,
            met_at: None,
            evidence: None,
        }
    }

    pub fn mark_met(&mut self, evidence: Option<String>) {
        self.met = true;
        self.met_at = Some(Utc::now());
        self.evidence = evidence;
    }
}

// ============================================================================
// Delivery Evidence
// ============================================================================

/// Evidence of service/product delivery
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryEvidence {
    /// Service completion proof
    ServiceCompletion {
        service_id: String,
        completion_proof: String,
        timestamp: DateTime<Utc>,
    },
    /// Data delivery proof
    DataDelivery {
        data_hash: String,
        delivery_url: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Oracle attestation
    OracleAttestation {
        oracle_id: ResonatorId,
        attestation_value: String,
        signature: String,
        timestamp: DateTime<Utc>,
    },
    /// Buyer explicit confirmation
    BuyerConfirmation {
        confirmation_message: String,
        timestamp: DateTime<Utc>,
    },
    /// Custom evidence
    Custom {
        evidence_type: String,
        data: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
}

// ============================================================================
// Enhanced Invoice
// ============================================================================

/// Payment terms for an invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTerms {
    /// Whether escrow is required
    pub escrow_required: bool,
    /// Escrow timeout duration
    pub escrow_timeout: Duration,
    /// Action to take on timeout
    pub timeout_action: TimeoutAction,
    /// Optional arbiter for disputes
    pub arbiter: Option<ResonatorId>,
}

impl Default for PaymentTerms {
    fn default() -> Self {
        Self {
            escrow_required: true,
            escrow_timeout: Duration::hours(24),
            timeout_action: TimeoutAction::RefundBuyer,
            arbiter: None,
        }
    }
}

/// Action to take when escrow times out
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeoutAction {
    RefundBuyer,
    ReleaseSeller,
    Dispute,
}

/// Enhanced invoice with structured delivery conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedInvoice {
    pub id: InvoiceId,
    pub seller: ResonatorId,
    pub buyer: ResonatorId,
    pub asset: AssetId,
    pub amount: Amount,
    pub description: String,
    pub delivery_conditions: Vec<DeliveryConditionType>,
    pub payment_terms: PaymentTerms,
    pub created_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub seller_signature: String,
}

impl EnhancedInvoice {
    pub fn new(
        seller: ResonatorId,
        buyer: ResonatorId,
        amount: Amount,
        description: String,
    ) -> Self {
        Self {
            id: InvoiceId::new(),
            seller,
            buyer,
            asset: AssetId::iusd(),
            amount,
            description,
            delivery_conditions: vec![DeliveryConditionType::BuyerConfirmation],
            payment_terms: PaymentTerms::default(),
            created_at: Utc::now(),
            valid_until: Utc::now() + Duration::days(30),
            seller_signature: String::new(),
        }
    }

    pub fn with_conditions(mut self, conditions: Vec<DeliveryConditionType>) -> Self {
        self.delivery_conditions = conditions;
        self
    }

    pub fn with_payment_terms(mut self, terms: PaymentTerms) -> Self {
        self.payment_terms = terms;
        self
    }

    pub fn is_valid(&self) -> bool {
        Utc::now() < self.valid_until
    }
}

// ============================================================================
// Enhanced Escrow State Machine
// ============================================================================

/// Escrow state with rich transition tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnhancedEscrowState {
    /// Escrow created but not yet funded
    Created,
    /// Funds are locked in escrow
    Funded { funded_at: DateTime<Utc> },
    /// Delivery has been submitted
    DeliverySubmitted {
        funded_at: DateTime<Utc>,
        submitted_at: DateTime<Utc>,
        evidence: DeliveryEvidence,
    },
    /// All conditions met, ready for release
    ConditionsMet {
        funded_at: DateTime<Utc>,
        conditions_met_at: DateTime<Utc>,
    },
    /// Released to seller
    Released {
        to: ResonatorId,
        released_at: DateTime<Utc>,
        receipt_id: String,
    },
    /// Refunded to buyer
    Refunded {
        to: ResonatorId,
        refunded_at: DateTime<Utc>,
        reason: String,
        receipt_id: String,
    },
    /// In dispute
    Disputed {
        disputed_at: DateTime<Utc>,
        dispute_reason: String,
        arbiter: ResonatorId,
    },
    /// Expired
    Expired { expired_at: DateTime<Utc> },
}

/// Enhanced escrow with full state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedEscrow {
    pub id: EscrowId,
    pub invoice_id: InvoiceId,
    pub buyer: ResonatorId,
    pub seller: ResonatorId,
    pub locked_amount: Amount,
    pub asset: AssetId,
    pub state: EnhancedEscrowState,
    pub conditions: Vec<TrackedDeliveryCondition>,
    pub arbiter: Option<ResonatorId>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Commitment receipt for funding (if funded)
    pub funding_receipt: Option<CommitmentReceipt>,
    /// Commitment receipt for release/refund (if settled)
    pub settlement_receipt: Option<CommitmentReceipt>,
}

impl EnhancedEscrow {
    /// Check if all conditions are met
    pub fn all_conditions_met(&self) -> bool {
        self.conditions.iter().all(|c| c.met)
    }

    /// Check if escrow has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if escrow can be released
    pub fn can_release(&self) -> bool {
        matches!(
            self.state,
            EnhancedEscrowState::Funded { .. }
                | EnhancedEscrowState::DeliverySubmitted { .. }
                | EnhancedEscrowState::ConditionsMet { .. }
        ) && self.all_conditions_met()
    }

    /// Check if escrow can be refunded
    pub fn can_refund(&self) -> bool {
        matches!(
            self.state,
            EnhancedEscrowState::Funded { .. }
                | EnhancedEscrowState::DeliverySubmitted { .. }
                | EnhancedEscrowState::Expired { .. }
        )
    }
}

// ============================================================================
// Economic Intent (for Commitment Boundary)
// ============================================================================

/// Economic intent types that must cross the commitment boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EconomicIntent {
    /// Transfer funds between Resonators
    Transfer {
        from: ResonatorId,
        to: ResonatorId,
        asset: AssetId,
        amount: Amount,
    },
    /// Lock funds in escrow
    EscrowLock {
        escrow_id: EscrowId,
        funder: ResonatorId,
        amount: Amount,
    },
    /// Release escrow to seller
    EscrowRelease {
        escrow_id: EscrowId,
        to: ResonatorId,
        amount: Amount,
    },
    /// Refund escrow to buyer
    EscrowRefund {
        escrow_id: EscrowId,
        to: ResonatorId,
        amount: Amount,
        reason: String,
    },
    /// Mint new assets (issuer only)
    Mint {
        to: ResonatorId,
        amount: Amount,
        reserve_attestation_hash: String,
    },
    /// Burn assets (issuer only)
    Burn {
        from: ResonatorId,
        amount: Amount,
    },
}

impl EconomicIntent {
    /// Get the spending amount for this intent
    pub fn spending_amount(&self) -> Option<Amount> {
        match self {
            Self::Transfer { amount, .. } => Some(*amount),
            Self::EscrowLock { amount, .. } => Some(*amount),
            Self::EscrowRelease { amount, .. } => Some(*amount),
            Self::EscrowRefund { amount, .. } => Some(*amount),
            Self::Mint { amount, .. } => Some(*amount),
            Self::Burn { amount, .. } => Some(*amount),
        }
    }

    /// Get a description of the intent
    pub fn description(&self) -> String {
        match self {
            Self::Transfer { from, to, amount, .. } => {
                format!("Transfer {} from {} to {}", amount, from, to)
            }
            Self::EscrowLock { escrow_id, amount, .. } => {
                format!("Lock {} in escrow {}", amount, escrow_id.0)
            }
            Self::EscrowRelease { escrow_id, to, amount } => {
                format!("Release {} from escrow {} to {}", amount, escrow_id.0, to)
            }
            Self::EscrowRefund { escrow_id, to, amount, reason } => {
                format!(
                    "Refund {} from escrow {} to {}: {}",
                    amount, escrow_id.0, to, reason
                )
            }
            Self::Mint { to, amount, .. } => {
                format!("Mint {} to {}", amount, to)
            }
            Self::Burn { from, amount } => {
                format!("Burn {} from {}", amount, from)
            }
        }
    }
}

/// Economic commitment - the result of crossing the commitment boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicCommitment {
    /// The base commitment receipt
    pub receipt: CommitmentReceipt,
    /// The intent that was committed
    pub intent: EconomicIntent,
    /// Evidence bundle
    pub evidence: EvidenceBundle,
    /// Permit ID used (if any)
    pub permit_id: Option<PermitId>,
    /// Budget ID used (if any)
    pub budget_id: Option<BudgetId>,
}

// ============================================================================
// Escrow Manager
// ============================================================================

/// The EscrowManager handles all escrow operations through the commitment boundary
///
/// CRITICAL: All money-moving operations MUST go through this manager to ensure
/// the commitment boundary is properly crossed.
pub struct EscrowManager {
    escrows: HashMap<EscrowId, EnhancedEscrow>,
    keypair: Keypair,
    commitment_gate: CommitmentGate,
}

impl EscrowManager {
    /// Create a new EscrowManager
    pub fn new() -> Self {
        let keypair = Keypair::generate();
        let commitment_gate = CommitmentGate::new(keypair.clone());
        Self {
            escrows: HashMap::new(),
            keypair,
            commitment_gate,
        }
    }

    /// Create an EscrowManager with a specific keypair
    pub fn with_keypair(keypair: Keypair) -> Self {
        let commitment_gate = CommitmentGate::new(keypair.clone());
        Self {
            escrows: HashMap::new(),
            keypair,
            commitment_gate,
        }
    }

    /// Get the public key
    pub fn public_key(&self) -> String {
        self.keypair.public_key_hex()
    }

    /// Create an escrow from an invoice (does NOT cross commitment boundary yet)
    pub fn create_from_invoice(&mut self, invoice: &EnhancedInvoice) -> Result<EscrowId> {
        if !invoice.is_valid() {
            return Err(CoreError::PolicyViolation {
                message: "Invoice has expired".to_string(),
            });
        }

        let escrow_id = EscrowId::new();
        let conditions: Vec<TrackedDeliveryCondition> = invoice
            .delivery_conditions
            .iter()
            .map(|c| TrackedDeliveryCondition::new(c.clone()))
            .collect();

        let escrow = EnhancedEscrow {
            id: escrow_id.clone(),
            invoice_id: invoice.id.clone(),
            buyer: invoice.buyer.clone(),
            seller: invoice.seller.clone(),
            locked_amount: invoice.amount,
            asset: invoice.asset.clone(),
            state: EnhancedEscrowState::Created,
            conditions,
            arbiter: invoice.payment_terms.arbiter.clone(),
            created_at: Utc::now(),
            expires_at: Utc::now() + invoice.payment_terms.escrow_timeout,
            funding_receipt: None,
            settlement_receipt: None,
        };

        self.escrows.insert(escrow_id.clone(), escrow);
        Ok(escrow_id)
    }

    /// Fund an escrow - CROSSES COMMITMENT BOUNDARY
    ///
    /// This method creates a commitment for locking funds in escrow.
    pub fn fund(
        &mut self,
        escrow_id: &EscrowId,
        funder: &ResonatorId,
        permit: &SpendPermit,
        budget: &BudgetPolicy,
    ) -> Result<EconomicCommitment> {
        let escrow = self.escrows.get(escrow_id).ok_or_else(|| CoreError::EscrowNotFound {
            escrow_id: escrow_id.0.clone(),
        })?;

        // Validate state
        if !matches!(escrow.state, EnhancedEscrowState::Created) {
            return Err(CoreError::EscrowConditionsNotMet {
                reason: format!("Escrow is in state {:?}, cannot fund", escrow.state),
            });
        }

        // Validate funder is the buyer
        if funder != &escrow.buyer {
            return Err(CoreError::PolicyViolation {
                message: "Only the buyer can fund escrow".to_string(),
            });
        }

        let amount = escrow.locked_amount;

        // Create economic intent
        let intent = EconomicIntent::EscrowLock {
            escrow_id: escrow_id.clone(),
            funder: funder.clone(),
            amount,
        };

        // ═══════════════════════════════════════════════════════════════
        // CROSSING COMMITMENT BOUNDARY - Accountability begins here
        // ═══════════════════════════════════════════════════════════════

        // Create payment intent for the commitment gate
        let payment_intent = PaymentIntent::new(
            funder.clone(),
            permit.permit_id.clone(),
            escrow.seller.clone(),
            amount,
            escrow.asset.clone(),
            SpendPurpose {
                category: "escrow".to_string(),
                description: format!("Escrow funding for {}", escrow.invoice_id.0),
            },
        );

        let consequence = ConsequenceRef {
            consequence_type: "escrow_lock".to_string(),
            reference_id: escrow_id.0.clone(),
            metadata: serde_json::json!({
                "escrow_id": escrow_id.0,
                "amount": amount.0,
            }),
        };

        let (receipt, evidence) = self
            .commitment_gate
            .create_commitment(&payment_intent, permit, budget, consequence)?;

        // Update escrow state
        let escrow = self.escrows.get_mut(escrow_id).unwrap();
        escrow.state = EnhancedEscrowState::Funded {
            funded_at: Utc::now(),
        };
        escrow.funding_receipt = Some(receipt.clone());

        Ok(EconomicCommitment {
            receipt,
            intent,
            evidence,
            permit_id: Some(permit.permit_id.clone()),
            budget_id: Some(budget.budget_id.clone()),
        })
    }

    /// Submit delivery evidence
    pub fn submit_delivery(
        &mut self,
        escrow_id: &EscrowId,
        submitter: &ResonatorId,
        evidence: DeliveryEvidence,
    ) -> Result<()> {
        let escrow = self.escrows.get_mut(escrow_id).ok_or_else(|| CoreError::EscrowNotFound {
            escrow_id: escrow_id.0.clone(),
        })?;

        // Validate submitter is the seller
        if submitter != &escrow.seller {
            return Err(CoreError::PolicyViolation {
                message: "Only the seller can submit delivery evidence".to_string(),
            });
        }

        // Validate state
        let funded_at = match &escrow.state {
            EnhancedEscrowState::Funded { funded_at } => *funded_at,
            _ => {
                return Err(CoreError::EscrowConditionsNotMet {
                    reason: format!("Escrow is in state {:?}, cannot submit delivery", escrow.state),
                })
            }
        };

        // Update state
        escrow.state = EnhancedEscrowState::DeliverySubmitted {
            funded_at,
            submitted_at: Utc::now(),
            evidence,
        };

        Ok(())
    }

    /// Mark a condition as met
    pub fn mark_condition_met(
        &mut self,
        escrow_id: &EscrowId,
        condition_index: usize,
        evidence: Option<String>,
    ) -> Result<()> {
        let escrow = self.escrows.get_mut(escrow_id).ok_or_else(|| CoreError::EscrowNotFound {
            escrow_id: escrow_id.0.clone(),
        })?;

        if condition_index >= escrow.conditions.len() {
            return Err(CoreError::EscrowConditionsNotMet {
                reason: "Condition index out of bounds".to_string(),
            });
        }

        escrow.conditions[condition_index].mark_met(evidence);

        // Check if all conditions are now met
        if escrow.all_conditions_met() {
            let (funded_at, _submitted_at) = match &escrow.state {
                EnhancedEscrowState::Funded { funded_at } => (*funded_at, None),
                EnhancedEscrowState::DeliverySubmitted { funded_at, submitted_at, .. } => {
                    (*funded_at, Some(*submitted_at))
                }
                _ => return Ok(()), // State doesn't need updating
            };

            escrow.state = EnhancedEscrowState::ConditionsMet {
                funded_at,
                conditions_met_at: Utc::now(),
            };
        }

        Ok(())
    }

    /// Release escrow to seller - CROSSES COMMITMENT BOUNDARY
    pub fn release(
        &mut self,
        escrow_id: &EscrowId,
        releaser: &ResonatorId,
    ) -> Result<EconomicCommitment> {
        let escrow = self.escrows.get(escrow_id).ok_or_else(|| CoreError::EscrowNotFound {
            escrow_id: escrow_id.0.clone(),
        })?;

        // Validate releaser is the buyer
        if releaser != &escrow.buyer {
            return Err(CoreError::PolicyViolation {
                message: "Only the buyer can release escrow".to_string(),
            });
        }

        // Validate state
        if !escrow.can_release() {
            return Err(CoreError::EscrowConditionsNotMet {
                reason: format!(
                    "Escrow cannot be released: state={:?}, conditions_met={}",
                    escrow.state,
                    escrow.all_conditions_met()
                ),
            });
        }

        let amount = escrow.locked_amount;
        let to = escrow.seller.clone();

        // Create economic intent
        let intent = EconomicIntent::EscrowRelease {
            escrow_id: escrow_id.clone(),
            to: to.clone(),
            amount,
        };

        // ═══════════════════════════════════════════════════════════════
        // CROSSING COMMITMENT BOUNDARY - Accountability begins here
        // ═══════════════════════════════════════════════════════════════

        let evidence = EvidenceBundle {
            intent_hash: crate::crypto::hash_object(&intent)?,
            policy_snapshot_hash: "escrow_release_policy".to_string(),
            budget_snapshot_hash: "n/a".to_string(),
            permit_hash: "n/a".to_string(),
            attestations: vec![],
            gathered_at: Utc::now(),
        };

        let consequence = ConsequenceRef {
            consequence_type: "escrow_release".to_string(),
            reference_id: format!("release_{}", Uuid::new_v4()),
            metadata: serde_json::json!({
                "escrow_id": escrow_id.0,
                "to": to.0,
                "amount": amount.0,
            }),
        };

        // Sign the release
        let receipt = self.create_escrow_receipt(
            releaser,
            &intent,
            &evidence,
            consequence,
        )?;

        // Update escrow state
        let escrow = self.escrows.get_mut(escrow_id).unwrap();
        escrow.state = EnhancedEscrowState::Released {
            to: to.clone(),
            released_at: Utc::now(),
            receipt_id: receipt.commitment_id.0.clone(),
        };
        escrow.settlement_receipt = Some(receipt.clone());

        Ok(EconomicCommitment {
            receipt,
            intent,
            evidence,
            permit_id: None,
            budget_id: None,
        })
    }

    /// Refund escrow to buyer - CROSSES COMMITMENT BOUNDARY
    pub fn refund(
        &mut self,
        escrow_id: &EscrowId,
        refunder: &ResonatorId,
        reason: String,
    ) -> Result<EconomicCommitment> {
        let escrow = self.escrows.get(escrow_id).ok_or_else(|| CoreError::EscrowNotFound {
            escrow_id: escrow_id.0.clone(),
        })?;

        // Validate refunder is seller or arbiter (or buyer if expired)
        let is_authorized = refunder == &escrow.seller
            || escrow.arbiter.as_ref() == Some(refunder)
            || (refunder == &escrow.buyer && escrow.is_expired());

        if !is_authorized {
            return Err(CoreError::PolicyViolation {
                message: "Not authorized to refund escrow".to_string(),
            });
        }

        // Validate state
        if !escrow.can_refund() {
            return Err(CoreError::EscrowConditionsNotMet {
                reason: format!("Escrow cannot be refunded: state={:?}", escrow.state),
            });
        }

        let amount = escrow.locked_amount;
        let to = escrow.buyer.clone();

        // Create economic intent
        let intent = EconomicIntent::EscrowRefund {
            escrow_id: escrow_id.clone(),
            to: to.clone(),
            amount,
            reason: reason.clone(),
        };

        // ═══════════════════════════════════════════════════════════════
        // CROSSING COMMITMENT BOUNDARY - Accountability begins here
        // ═══════════════════════════════════════════════════════════════

        let evidence = EvidenceBundle {
            intent_hash: crate::crypto::hash_object(&intent)?,
            policy_snapshot_hash: "escrow_refund_policy".to_string(),
            budget_snapshot_hash: "n/a".to_string(),
            permit_hash: "n/a".to_string(),
            attestations: vec![],
            gathered_at: Utc::now(),
        };

        let consequence = ConsequenceRef {
            consequence_type: "escrow_refund".to_string(),
            reference_id: format!("refund_{}", Uuid::new_v4()),
            metadata: serde_json::json!({
                "escrow_id": escrow_id.0,
                "to": to.0,
                "amount": amount.0,
                "reason": reason,
            }),
        };

        let receipt = self.create_escrow_receipt(
            refunder,
            &intent,
            &evidence,
            consequence,
        )?;

        // Update escrow state
        let escrow = self.escrows.get_mut(escrow_id).unwrap();
        escrow.state = EnhancedEscrowState::Refunded {
            to: to.clone(),
            refunded_at: Utc::now(),
            reason,
            receipt_id: receipt.commitment_id.0.clone(),
        };
        escrow.settlement_receipt = Some(receipt.clone());

        Ok(EconomicCommitment {
            receipt,
            intent,
            evidence,
            permit_id: None,
            budget_id: None,
        })
    }

    /// Get an escrow by ID
    pub fn get(&self, escrow_id: &EscrowId) -> Option<&EnhancedEscrow> {
        self.escrows.get(escrow_id)
    }

    /// Get all escrows
    pub fn all_escrows(&self) -> Vec<&EnhancedEscrow> {
        self.escrows.values().collect()
    }

    /// Create a signed receipt for escrow operations
    fn create_escrow_receipt(
        &self,
        actor: &ResonatorId,
        _intent: &EconomicIntent,
        evidence: &EvidenceBundle,
        consequence: ConsequenceRef,
    ) -> Result<CommitmentReceipt> {
        use crate::commitment::CommitmentId;

        let commitment_id = CommitmentId::new();
        let committed_at = Utc::now();

        #[derive(serde::Serialize)]
        struct SignableReceipt {
            commitment_id: CommitmentId,
            actor: ResonatorId,
            intent_hash: String,
            policy_snapshot_hash: String,
            evidence_hash: String,
            consequence_ref: ConsequenceRef,
            committed_at: DateTime<Utc>,
        }

        let evidence_hash = crate::crypto::hash_object(evidence)?;

        let signable = SignableReceipt {
            commitment_id: commitment_id.clone(),
            actor: actor.clone(),
            intent_hash: evidence.intent_hash.clone(),
            policy_snapshot_hash: evidence.policy_snapshot_hash.clone(),
            evidence_hash: evidence_hash.clone(),
            consequence_ref: consequence.clone(),
            committed_at,
        };

        let signing_bytes = serde_json::to_vec(&signable)?;
        let signature = self.keypair.sign(&signing_bytes);

        Ok(CommitmentReceipt {
            commitment_id,
            actor: actor.clone(),
            intent_hash: evidence.intent_hash.clone(),
            policy_snapshot_hash: evidence.policy_snapshot_hash.clone(),
            evidence_hash,
            consequence_ref: consequence,
            committed_at,
            signature,
            signer_public_key: self.keypair.public_key_hex(),
        })
    }
}

impl Default for EscrowManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_invoice(buyer: ResonatorId, seller: ResonatorId) -> EnhancedInvoice {
        EnhancedInvoice::new(
            seller,
            buyer,
            Amount::new(10000),
            "Test service".to_string(),
        )
        .with_conditions(vec![DeliveryConditionType::BuyerConfirmation])
    }

    fn create_test_permit(buyer: &ResonatorId, budget_id: &BudgetId) -> SpendPermit {
        SpendPermit {
            permit_id: PermitId::new(),
            issuer: buyer.clone(),
            bound_budget: budget_id.clone(),
            asset_class: AssetClass::Stablecoin,
            max_amount: Amount::new(100000),
            remaining: Amount::new(100000),
            counterparty: CounterpartyConstraint::Any,
            purpose: SpendPurpose {
                category: "escrow".to_string(),
                description: "Test".to_string(),
            },
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
            signature: "test".to_string(),
        }
    }

    fn create_test_budget(owner: &ResonatorId) -> BudgetPolicy {
        BudgetPolicy::new(owner.clone(), Amount::new(1000000))
    }

    #[test]
    fn test_create_escrow_from_invoice() {
        let mut manager = EscrowManager::new();
        let buyer = ResonatorId::new();
        let seller = ResonatorId::new();
        let invoice = create_test_invoice(buyer.clone(), seller.clone());

        let escrow_id = manager.create_from_invoice(&invoice).unwrap();
        let escrow = manager.get(&escrow_id).unwrap();

        assert_eq!(escrow.buyer, buyer);
        assert_eq!(escrow.seller, seller);
        assert!(matches!(escrow.state, EnhancedEscrowState::Created));
    }

    #[test]
    fn test_fund_escrow() {
        let mut manager = EscrowManager::new();
        let buyer = ResonatorId::new();
        let seller = ResonatorId::new();
        let invoice = create_test_invoice(buyer.clone(), seller.clone());

        let escrow_id = manager.create_from_invoice(&invoice).unwrap();
        let budget = create_test_budget(&buyer);
        let permit = create_test_permit(&buyer, &budget.budget_id);

        let commitment = manager.fund(&escrow_id, &buyer, &permit, &budget).unwrap();

        assert!(commitment.receipt.verify().is_ok());

        let escrow = manager.get(&escrow_id).unwrap();
        assert!(matches!(escrow.state, EnhancedEscrowState::Funded { .. }));
    }

    #[test]
    fn test_release_escrow() {
        let mut manager = EscrowManager::new();
        let buyer = ResonatorId::new();
        let seller = ResonatorId::new();
        let invoice = create_test_invoice(buyer.clone(), seller.clone());

        let escrow_id = manager.create_from_invoice(&invoice).unwrap();
        let budget = create_test_budget(&buyer);
        let permit = create_test_permit(&buyer, &budget.budget_id);

        // Fund
        manager.fund(&escrow_id, &buyer, &permit, &budget).unwrap();

        // Mark condition as met
        manager.mark_condition_met(&escrow_id, 0, Some("Confirmed".to_string())).unwrap();

        // Release
        let commitment = manager.release(&escrow_id, &buyer).unwrap();

        assert!(commitment.receipt.verify().is_ok());

        let escrow = manager.get(&escrow_id).unwrap();
        assert!(matches!(escrow.state, EnhancedEscrowState::Released { .. }));
    }

    #[test]
    fn test_cannot_release_without_conditions_met() {
        let mut manager = EscrowManager::new();
        let buyer = ResonatorId::new();
        let seller = ResonatorId::new();
        let invoice = create_test_invoice(buyer.clone(), seller.clone());

        let escrow_id = manager.create_from_invoice(&invoice).unwrap();
        let budget = create_test_budget(&buyer);
        let permit = create_test_permit(&buyer, &budget.budget_id);

        // Fund
        manager.fund(&escrow_id, &buyer, &permit, &budget).unwrap();

        // Try to release without marking condition met
        let result = manager.release(&escrow_id, &buyer);
        assert!(result.is_err());
    }

    #[test]
    fn test_refund_escrow() {
        let mut manager = EscrowManager::new();
        let buyer = ResonatorId::new();
        let seller = ResonatorId::new();
        let invoice = create_test_invoice(buyer.clone(), seller.clone());

        let escrow_id = manager.create_from_invoice(&invoice).unwrap();
        let budget = create_test_budget(&buyer);
        let permit = create_test_permit(&buyer, &budget.budget_id);

        // Fund
        manager.fund(&escrow_id, &buyer, &permit, &budget).unwrap();

        // Seller refunds
        let commitment = manager
            .refund(&escrow_id, &seller, "Service cancelled".to_string())
            .unwrap();

        assert!(commitment.receipt.verify().is_ok());

        let escrow = manager.get(&escrow_id).unwrap();
        assert!(matches!(escrow.state, EnhancedEscrowState::Refunded { .. }));
    }

    #[test]
    fn test_delivery_submission() {
        let mut manager = EscrowManager::new();
        let buyer = ResonatorId::new();
        let seller = ResonatorId::new();
        let invoice = create_test_invoice(buyer.clone(), seller.clone());

        let escrow_id = manager.create_from_invoice(&invoice).unwrap();
        let budget = create_test_budget(&buyer);
        let permit = create_test_permit(&buyer, &budget.budget_id);

        // Fund
        manager.fund(&escrow_id, &buyer, &permit, &budget).unwrap();

        // Submit delivery
        let evidence = DeliveryEvidence::ServiceCompletion {
            service_id: "test_service".to_string(),
            completion_proof: "Done!".to_string(),
            timestamp: Utc::now(),
        };

        manager.submit_delivery(&escrow_id, &seller, evidence).unwrap();

        let escrow = manager.get(&escrow_id).unwrap();
        assert!(matches!(
            escrow.state,
            EnhancedEscrowState::DeliverySubmitted { .. }
        ));
    }
}
