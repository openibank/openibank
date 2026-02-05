//! Arbiter Agent - Validates delivery, resolves disputes
//!
//! The arbiter agent handles dispute resolution:
//! 1. Receives delivery proofs
//! 2. Evaluates evidence
//! 3. Makes release/refund decisions

use std::sync::Arc;

use openibank_core::{Escrow, EscrowId, ResonatorId};
use openibank_guard::ArbiterDecision;
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::brain::AgentBrain;
use openibank_agent_kernel::{
    AgentKernel, CapabilitySet, Contract, ContractSet, DeterministicPolicy, KernelConfig,
    KernelMode, ProposalRequest,
};
use crate::seller::DeliveryProof;

/// Errors that can occur in arbiter operations
#[derive(Error, Debug)]
pub enum ArbiterError {
    #[error("Case not found: {case_id}")]
    CaseNotFound { case_id: String },

    #[error("Invalid evidence: {reason}")]
    InvalidEvidence { reason: String },

    #[error("Already decided: {case_id}")]
    AlreadyDecided { case_id: String },

    #[error("Kernel error: {0}")]
    KernelError(String),
}

pub type Result<T> = std::result::Result<T, ArbiterError>;

/// A dispute case for arbitration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeCase {
    pub case_id: String,
    pub escrow_id: EscrowId,
    pub payer: ResonatorId,
    pub payee: ResonatorId,
    pub dispute_reason: Option<String>,
    pub delivery_proof: Option<DeliveryProof>,
    pub decision: Option<ArbiterDecision>,
    pub decision_reasoning: Option<String>,
}

/// Decision result from the arbiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub case_id: String,
    pub escrow_id: EscrowId,
    pub decision: ArbiterDecision,
    pub reasoning: String,
}

/// The Arbiter Agent
///
/// Resolves disputes between buyers and sellers:
/// - Reviews delivery proofs
/// - Evaluates dispute claims
/// - Makes fair decisions on escrow release/refund
pub struct ArbiterAgent {
    id: ResonatorId,
    kernel: AgentKernel,
    #[allow(dead_code)]
    ledger: Arc<Ledger>,
    cases: Vec<DisputeCase>,
}

impl ArbiterAgent {
    /// Create a new arbiter agent
    pub fn new(id: ResonatorId, ledger: Arc<Ledger>) -> Self {
        let brain = AgentBrain::deterministic();
        Self::with_brain(id, ledger, brain)
    }

    /// Create with LLM brain
    pub fn with_brain(id: ResonatorId, ledger: Arc<Ledger>, brain: AgentBrain) -> Self {
        let mode = match brain.mode() {
            crate::brain::BrainMode::Deterministic => KernelMode::Deterministic,
            crate::brain::BrainMode::LLM => KernelMode::Llm,
        };
        let capabilities = CapabilitySet::from_attested(["escrow.resolve"]);
        let contracts = ContractSet::new(vec![Contract {
            name: "arbiter_contract".to_string(),
            max_spend: None,
            allowed_assets: vec![],
            require_reversible: false,
            allowed_outcomes: vec!["release".to_string(), "refund".to_string(), "partial".to_string()],
        }]);
        let kernel = AgentKernel::new(KernelConfig {
            agent_id: id.0.clone(),
            role: "arbiter".to_string(),
            mode,
            proposer: Box::new(brain),
            policy: Box::new(DeterministicPolicy::default()),
            capabilities,
            contracts,
            trace_max_entries: Some(1000),
        });
        Self {
            id,
            kernel,
            ledger,
            cases: Vec::new(),
        }
    }

    /// Get the agent's ID
    pub fn id(&self) -> &ResonatorId {
        &self.id
    }

    /// Access kernel trace (for audit/replay)
    pub fn kernel_trace(&self) -> &openibank_agent_kernel::KernelTrace {
        self.kernel.trace()
    }

    /// Set an active commitment context for gating
    pub fn set_active_commitment(&mut self, commitment_id: impl Into<String>, approved: bool) {
        self.kernel.set_active_commitment(commitment_id, approved);
    }

    /// Clear active commitment context
    pub fn clear_active_commitment(&mut self) {
        self.kernel.clear_active_commitment();
    }

    /// Open a new dispute case
    pub fn open_case(
        &mut self,
        escrow: &Escrow,
        dispute_reason: Option<String>,
        delivery_proof: Option<DeliveryProof>,
    ) -> DisputeCase {
        let case = DisputeCase {
            case_id: format!("case_{}", uuid::Uuid::new_v4()),
            escrow_id: escrow.escrow_id.clone(),
            payer: escrow.payer.clone(),
            payee: escrow.payee.clone(),
            dispute_reason,
            delivery_proof,
            decision: None,
            decision_reasoning: None,
        };

        self.cases.push(case.clone());
        case
    }

    /// Submit delivery proof for a case
    pub fn submit_delivery_proof(&mut self, case_id: &str, proof: DeliveryProof) -> Result<()> {
        let case = self
            .cases
            .iter_mut()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| ArbiterError::CaseNotFound {
                case_id: case_id.to_string(),
            })?;

        if case.decision.is_some() {
            return Err(ArbiterError::AlreadyDecided {
                case_id: case_id.to_string(),
            });
        }

        case.delivery_proof = Some(proof);
        Ok(())
    }

    /// Submit a dispute reason for a case
    pub fn submit_dispute(&mut self, case_id: &str, reason: String) -> Result<()> {
        let case = self
            .cases
            .iter_mut()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| ArbiterError::CaseNotFound {
                case_id: case_id.to_string(),
            })?;

        if case.decision.is_some() {
            return Err(ArbiterError::AlreadyDecided {
                case_id: case_id.to_string(),
            });
        }

        case.dispute_reason = Some(reason);
        Ok(())
    }

    /// Make a decision on a case
    pub async fn decide(&mut self, case_id: &str) -> Result<DecisionResult> {
        let case = self
            .cases
            .iter()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| ArbiterError::CaseNotFound {
                case_id: case_id.to_string(),
            })?
            .clone();

        if case.decision.is_some() {
            return Err(ArbiterError::AlreadyDecided {
                case_id: case_id.to_string(),
            });
        }

        // Use brain to propose decision
        let proposal = self
            .kernel
            .propose_arbitration(ProposalRequest::Arbitration {
                escrow_id: case.escrow_id.0.clone(),
                delivery_proof: case.delivery_proof.as_ref().map(|p| p.proof_data.clone()),
                dispute_reason: case.dispute_reason.clone(),
            })
            .await
            .map_err(|e| ArbiterError::KernelError(e.to_string()))?;

        // Update case with decision
        let case = self
            .cases
            .iter_mut()
            .find(|c| c.case_id == case_id)
            .unwrap();

        case.decision = Some(proposal.decision.clone());
        case.decision_reasoning = Some(proposal.reasoning.clone());

        Ok(DecisionResult {
            case_id: case_id.to_string(),
            escrow_id: case.escrow_id.clone(),
            decision: proposal.decision,
            reasoning: proposal.reasoning,
        })
    }

    /// Get a case by ID
    pub fn get_case(&self, case_id: &str) -> Option<&DisputeCase> {
        self.cases.iter().find(|c| c.case_id == case_id)
    }

    /// Get all cases
    pub fn cases(&self) -> &[DisputeCase] {
        &self.cases
    }

    /// Get pending cases (no decision yet)
    pub fn pending_cases(&self) -> Vec<&DisputeCase> {
        self.cases.iter().filter(|c| c.decision.is_none()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openibank_core::{Amount, AssetId, EscrowState, InvoiceId, ReleaseCondition};

    fn create_test_escrow() -> Escrow {
        Escrow {
            escrow_id: EscrowId::new(),
            invoice_id: InvoiceId::new(),
            payer: ResonatorId::new(),
            payee: ResonatorId::new(),
            amount: Amount::new(5000),
            asset: AssetId::iusd(),
            state: EscrowState::Locked,
            release_conditions: vec![ReleaseCondition {
                condition_type: "delivery".to_string(),
                parameters: serde_json::json!({}),
                met: false,
            }],
            arbiter: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(7),
        }
    }

    #[tokio::test]
    async fn test_arbiter_open_case() {
        let ledger = Arc::new(Ledger::new());
        let mut arbiter = ArbiterAgent::new(ResonatorId::new(), ledger);

        let escrow = create_test_escrow();
        let case = arbiter.open_case(&escrow, None, None);

        assert!(case.decision.is_none());
        assert_eq!(arbiter.cases().len(), 1);
    }

    #[tokio::test]
    async fn test_arbiter_decide_with_proof() {
        let ledger = Arc::new(Ledger::new());
        let mut arbiter = ArbiterAgent::new(ResonatorId::new(), ledger);

        let escrow = create_test_escrow();
        let proof = DeliveryProof {
            invoice_id: escrow.invoice_id.clone(),
            proof_type: "delivery".to_string(),
            proof_data: "Service delivered successfully".to_string(),
            delivered_at: Utc::now(),
        };

        let case = arbiter.open_case(&escrow, None, Some(proof));
        arbiter.set_active_commitment("test_commitment", true);
        let result = arbiter.decide(&case.case_id).await.unwrap();

        // With delivery proof and no dispute, should release
        assert_eq!(result.decision, ArbiterDecision::Release);
    }

    #[tokio::test]
    async fn test_arbiter_decide_with_dispute() {
        let ledger = Arc::new(Ledger::new());
        let mut arbiter = ArbiterAgent::new(ResonatorId::new(), ledger);

        let escrow = create_test_escrow();
        let case = arbiter.open_case(
            &escrow,
            Some("Service not delivered as promised".to_string()),
            None,
        );

        arbiter.set_active_commitment("test_commitment", true);
        let result = arbiter.decide(&case.case_id).await.unwrap();

        // With dispute and no proof, should refund
        assert_eq!(result.decision, ArbiterDecision::Refund);
    }

    #[tokio::test]
    async fn test_arbiter_cannot_decide_twice() {
        let ledger = Arc::new(Ledger::new());
        let mut arbiter = ArbiterAgent::new(ResonatorId::new(), ledger);

        let escrow = create_test_escrow();
        let case = arbiter.open_case(&escrow, None, None);

        // First decision
        arbiter.set_active_commitment("test_commitment", true);
        arbiter.decide(&case.case_id).await.unwrap();

        // Second decision should fail
        let result = arbiter.decide(&case.case_id).await;
        assert!(matches!(result, Err(ArbiterError::AlreadyDecided { .. })));
    }
}
