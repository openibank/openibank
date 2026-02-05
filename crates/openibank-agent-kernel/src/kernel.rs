//! AgentKernel core runtime

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::gate::{CapabilitySet, CommitmentContext, CommitmentGate, ContractSet, GateError};
use crate::policy::{KernelAction, KernelIntent, KernelPolicy};
use crate::propose::{KernelProposal, KernelProposer, ProposalRequest, ProposeError};
use crate::trace::{KernelStage, KernelTrace};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KernelMode {
    Deterministic,
    Llm,
}

pub struct KernelConfig {
    pub agent_id: String,
    pub role: String,
    pub mode: KernelMode,
    pub proposer: Box<dyn KernelProposer>,
    pub policy: Box<dyn KernelPolicy>,
    pub capabilities: CapabilitySet,
    pub contracts: ContractSet,
    pub trace_max_entries: Option<usize>,
}

pub struct AgentKernel {
    agent_id: String,
    role: String,
    mode: KernelMode,
    proposer: Box<dyn KernelProposer>,
    policy: Box<dyn KernelPolicy>,
    capabilities: CapabilitySet,
    contracts: ContractSet,
    gate: CommitmentGate,
    trace: KernelTrace,
}

impl AgentKernel {
    pub fn new(config: KernelConfig) -> Self {
        let trace = KernelTrace::new(&config.agent_id, &config.role, config.trace_max_entries);
        Self {
            agent_id: config.agent_id,
            role: config.role,
            mode: config.mode,
            proposer: config.proposer,
            policy: config.policy,
            capabilities: config.capabilities,
            contracts: config.contracts,
            gate: CommitmentGate::new(),
            trace,
        }
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn mode(&self) -> KernelMode {
        self.mode
    }

    pub fn trace(&self) -> &KernelTrace {
        &self.trace
    }

    pub fn trace_mut(&mut self) -> &mut KernelTrace {
        &mut self.trace
    }

    pub fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    pub fn capabilities_mut(&mut self) -> &mut CapabilitySet {
        &mut self.capabilities
    }

    pub fn contracts(&self) -> &ContractSet {
        &self.contracts
    }

    pub fn contracts_mut(&mut self) -> &mut ContractSet {
        &mut self.contracts
    }

    pub fn set_active_commitment(&mut self, commitment_id: impl Into<String>, approved: bool) {
        self.gate.set_active(CommitmentContext {
            commitment_id: commitment_id.into(),
            approved,
        });
        self.trace.record(
            KernelStage::Gate,
            "commitment context set",
            Some(serde_json::json!({
                "approved": approved,
                "commitment_id": self.gate.active().map(|c| c.commitment_id.clone()),
            })),
        );
    }

    pub fn clear_active_commitment(&mut self) {
        self.gate.clear();
        self.trace.record(KernelStage::Gate, "commitment context cleared", None);
    }

    pub async fn propose_payment(
        &mut self,
        request: ProposalRequest,
    ) -> Result<openibank_guard::ProposedPaymentIntent, KernelError> {
        let intent = KernelIntent::Proposal(request.clone());
        let decision = self.policy.decide(&intent);
        self.trace.record(
            KernelStage::Policy,
            "policy evaluated",
            Some(serde_json::json!({"allow": decision.allow, "reason": decision.reason})),
        );
        if !decision.allow {
            return Err(KernelError::PolicyDenied {
                reason: decision.reason.unwrap_or_else(|| "denied".to_string()),
            });
        }

        let proposal = self.proposer.propose(request).await?;
        self.trace.record(
            KernelStage::Propose,
            "proposal generated",
            Some(serde_json::to_value(&proposal).unwrap_or_default()),
        );

        let payment = match proposal {
            KernelProposal::Payment(p) => p,
            _ => return Err(KernelError::ProposalMismatch),
        };

        self.gate.require_approved()?;
        self.capabilities.require("payment.initiate")?;
        self.contracts.enforce_payment(payment.amount, &payment.asset, true)?;

        self.trace.record(
            KernelStage::Gate,
            "payment gated",
            Some(serde_json::json!({
                "amount": payment.amount,
                "asset": payment.asset,
            })),
        );

        Ok(payment)
    }

    pub async fn propose_invoice(
        &mut self,
        request: ProposalRequest,
    ) -> Result<openibank_guard::ProposedInvoice, KernelError> {
        let intent = KernelIntent::Proposal(request.clone());
        let decision = self.policy.decide(&intent);
        self.trace.record(
            KernelStage::Policy,
            "policy evaluated",
            Some(serde_json::json!({"allow": decision.allow, "reason": decision.reason})),
        );
        if !decision.allow {
            return Err(KernelError::PolicyDenied {
                reason: decision.reason.unwrap_or_else(|| "denied".to_string()),
            });
        }

        let proposal = self.proposer.propose(request).await?;
        self.trace.record(
            KernelStage::Propose,
            "proposal generated",
            Some(serde_json::to_value(&proposal).unwrap_or_default()),
        );

        let invoice = match proposal {
            KernelProposal::Invoice(p) => p,
            _ => return Err(KernelError::ProposalMismatch),
        };

        self.capabilities.require("invoice.issue")?;
        self.contracts.enforce_payment(invoice.amount, &invoice.asset, true)?;

        self.trace.record(
            KernelStage::Gate,
            "invoice gated",
            Some(serde_json::json!({
                "amount": invoice.amount,
                "asset": invoice.asset,
            })),
        );

        Ok(invoice)
    }

    pub async fn propose_arbitration(
        &mut self,
        request: ProposalRequest,
    ) -> Result<openibank_guard::ProposedArbiterDecision, KernelError> {
        let intent = KernelIntent::Proposal(request.clone());
        let decision = self.policy.decide(&intent);
        self.trace.record(
            KernelStage::Policy,
            "policy evaluated",
            Some(serde_json::json!({"allow": decision.allow, "reason": decision.reason})),
        );
        if !decision.allow {
            return Err(KernelError::PolicyDenied {
                reason: decision.reason.unwrap_or_else(|| "denied".to_string()),
            });
        }

        let proposal = self.proposer.propose(request).await?;
        self.trace.record(
            KernelStage::Propose,
            "proposal generated",
            Some(serde_json::to_value(&proposal).unwrap_or_default()),
        );

        let decision = match proposal {
            KernelProposal::Arbitration(p) => p,
            _ => return Err(KernelError::ProposalMismatch),
        };

        self.gate.require_approved()?;
        self.capabilities.require("escrow.resolve")?;
        let outcome = match &decision.decision {
            openibank_guard::ArbiterDecision::Release => "release",
            openibank_guard::ArbiterDecision::Refund => "refund",
            openibank_guard::ArbiterDecision::Partial { .. } => "partial",
        };
        self.contracts.enforce_outcome(outcome)?;

        self.trace.record(
            KernelStage::Gate,
            "arbitration gated",
            Some(serde_json::json!({
                "escrow_id": decision.escrow_id,
                "decision": decision.decision,
            })),
        );

        Ok(decision)
    }

    pub fn authorize_action(&mut self, action: KernelAction) -> Result<(), KernelError> {
        let intent = KernelIntent::Action(action.clone());
        let decision = self.policy.decide(&intent);
        self.trace.record(
            KernelStage::Policy,
            "policy evaluated",
            Some(serde_json::json!({"allow": decision.allow, "reason": decision.reason})),
        );
        if !decision.allow {
            return Err(KernelError::PolicyDenied {
                reason: decision.reason.unwrap_or_else(|| "denied".to_string()),
            });
        }

        match action {
            KernelAction::ReleaseEscrow { amount, asset } => {
                self.gate.require_approved()?;
                self.capabilities.require("escrow.release")?;
                self.contracts.enforce_payment(amount, &asset, true)?;
            }
            KernelAction::ReceivePayment { amount, asset } => {
                self.gate.require_approved()?;
                self.capabilities.require("payment.receive")?;
                self.contracts.enforce_payment(amount, &asset, true)?;
            }
            KernelAction::DeliverService { .. } => {
                self.capabilities.require("service.deliver")?;
            }
        }

        self.trace.record(
            KernelStage::Gate,
            "action gated",
            Some(serde_json::json!({"action": serde_json::to_value(&intent).ok()})),
        );

        Ok(())
    }

    /// Consume compiled UAL artifacts (RCF commitments / PALM ops) without executing raw text
    pub fn consume_ual_artifacts<T: Serialize>(&mut self, artifacts: &[T]) -> Result<(), KernelError> {
        let mut serialized = Vec::with_capacity(artifacts.len());
        for artifact in artifacts {
            let value = serde_json::to_value(artifact)
                .map_err(|e| KernelError::Serialization(e.to_string()))?;
            serialized.push(value);
        }

        self.trace.record(
            KernelStage::Decision,
            "UAL artifacts consumed",
            Some(serde_json::json!({
                "count": serialized.len(),
                "artifacts": serialized,
            })),
        );

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum KernelError {
    #[error("Policy denied: {reason}")]
    PolicyDenied { reason: String },
    #[error("Proposal failed: {0}")]
    Proposal(#[from] ProposeError),
    #[error("Gate failed: {0}")]
    Gate(#[from] GateError),
    #[error("Unexpected proposal type")]
    ProposalMismatch,
    #[error("Serialization error: {0}")]
    Serialization(String),
}
