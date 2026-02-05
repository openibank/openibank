//! Deterministic policy engine for AgentKernel

use serde::{Deserialize, Serialize};

use crate::propose::ProposalRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelAction {
    ReleaseEscrow { amount: u64, asset: String },
    ReceivePayment { amount: u64, asset: String },
    DeliverService { invoice_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelIntent {
    Proposal(ProposalRequest),
    Action(KernelAction),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    pub reason: Option<String>,
}

pub trait KernelPolicy: Send + Sync {
    fn decide(&self, intent: &KernelIntent) -> PolicyDecision;
}

#[derive(Debug, Default, Clone)]
pub struct DeterministicPolicy;

impl KernelPolicy for DeterministicPolicy {
    fn decide(&self, _intent: &KernelIntent) -> PolicyDecision {
        PolicyDecision {
            allow: true,
            reason: None,
        }
    }
}
