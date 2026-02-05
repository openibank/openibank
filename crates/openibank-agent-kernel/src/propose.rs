//! Proposal generation API for AgentKernel

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use openibank_guard::{ProposedArbiterDecision, ProposedInvoice, ProposedPaymentIntent};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ProposalRequest {
    Payment {
        seller_id: String,
        service_description: String,
        price: u64,
        available_budget: u64,
    },
    Invoice {
        buyer_id: String,
        service_name: String,
        price: u64,
    },
    Arbitration {
        escrow_id: String,
        delivery_proof: Option<String>,
        dispute_reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum KernelProposal {
    Payment(ProposedPaymentIntent),
    Invoice(ProposedInvoice),
    Arbitration(ProposedArbiterDecision),
}

#[derive(Error, Debug)]
pub enum ProposeError {
    #[error("Proposal generation failed: {0}")]
    Failed(String),
}

#[async_trait]
pub trait KernelProposer: Send + Sync {
    async fn propose(&self, request: ProposalRequest) -> Result<KernelProposal, ProposeError>;
}
