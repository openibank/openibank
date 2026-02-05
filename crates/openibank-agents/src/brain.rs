//! Agent Brain - LLM integration with deterministic fallback
//!
//! The AgentBrain abstraction allows agents to optionally use LLMs
//! while maintaining deterministic behavior as the default.

use openibank_guard::{Guard, ProposedArbiterDecision, ProposedInvoice, ProposedPaymentIntent};
use openibank_agent_kernel::{KernelProposal, KernelProposer, ProposalRequest, ProposeError};
use async_trait::async_trait;
use openibank_llm::{
    CompletionRequest, LLMRouter, Message, ProviderKind, Result as LLMResult,
};
use serde::{Deserialize, Serialize};

/// Brain mode - determines how the agent makes decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainMode {
    /// Always use deterministic logic
    Deterministic,
    /// Use LLM if available, fall back to deterministic
    LLM,
}

impl Default for BrainMode {
    fn default() -> Self {
        Self::Deterministic
    }
}

/// The agent's brain - handles decision making
pub struct AgentBrain {
    llm: Option<LLMRouter>,
    guard: Guard,
    mode: BrainMode,
}

#[async_trait]
impl KernelProposer for AgentBrain {
    async fn propose(&self, request: ProposalRequest) -> Result<KernelProposal, ProposeError> {
        match request {
            ProposalRequest::Payment {
                seller_id,
                service_description,
                price,
                available_budget,
            } => {
                let context = PaymentContext {
                    seller_id,
                    service_description,
                    price,
                    available_budget,
                };
                let proposal = self.propose_payment(&context).await;
                Ok(KernelProposal::Payment(proposal))
            }
            ProposalRequest::Invoice {
                buyer_id,
                service_name,
                price,
            } => {
                let context = InvoiceContext {
                    buyer_id,
                    service_name,
                    price,
                };
                let proposal = self.propose_invoice(&context).await;
                Ok(KernelProposal::Invoice(proposal))
            }
            ProposalRequest::Arbitration {
                escrow_id,
                delivery_proof,
                dispute_reason,
            } => {
                let context = ArbiterContext {
                    escrow_id,
                    delivery_proof,
                    dispute_reason,
                };
                let proposal = self.propose_arbiter_decision(&context).await;
                Ok(KernelProposal::Arbitration(proposal))
            }
        }
    }
}

impl AgentBrain {
    /// Create a deterministic brain (no LLM)
    pub fn deterministic() -> Self {
        Self {
            llm: None,
            guard: Guard::new(),
            mode: BrainMode::Deterministic,
        }
    }

    /// Create a brain with LLM support
    pub fn with_llm(llm: LLMRouter) -> Self {
        Self {
            llm: Some(llm),
            guard: Guard::new(),
            mode: BrainMode::LLM,
        }
    }

    /// Create from environment
    pub fn from_env() -> Self {
        let llm = LLMRouter::from_env();
        Self {
            llm: Some(llm),
            guard: Guard::new(),
            mode: BrainMode::LLM,
        }
    }

    /// Get the current mode
    pub fn mode(&self) -> BrainMode {
        self.mode
    }

    /// Get which provider is being used
    pub fn provider_kind(&self) -> ProviderKind {
        self.llm
            .as_ref()
            .map(|l| l.kind())
            .unwrap_or(ProviderKind::Deterministic)
    }

    /// Check if LLM is available
    pub async fn is_llm_available(&self) -> bool {
        match &self.llm {
            Some(llm) => llm.is_available().await,
            None => false,
        }
    }

    /// Propose a payment intent (for buyers)
    pub async fn propose_payment(
        &self,
        context: &PaymentContext,
    ) -> ProposedPaymentIntent {
        if self.mode == BrainMode::LLM {
            if let Some(llm) = &self.llm {
                if let Ok(proposal) = self.llm_propose_payment(llm, context).await {
                    return proposal;
                }
            }
        }

        // Deterministic fallback
        self.deterministic_payment(context)
    }

    /// Propose an invoice (for sellers)
    pub async fn propose_invoice(
        &self,
        context: &InvoiceContext,
    ) -> ProposedInvoice {
        if self.mode == BrainMode::LLM {
            if let Some(llm) = &self.llm {
                if let Ok(proposal) = self.llm_propose_invoice(llm, context).await {
                    return proposal;
                }
            }
        }

        // Deterministic fallback
        self.deterministic_invoice(context)
    }

    /// Propose an arbiter decision
    pub async fn propose_arbiter_decision(
        &self,
        context: &ArbiterContext,
    ) -> ProposedArbiterDecision {
        if self.mode == BrainMode::LLM {
            if let Some(llm) = &self.llm {
                if let Ok(proposal) = self.llm_propose_decision(llm, context).await {
                    return proposal;
                }
            }
        }

        // Deterministic fallback
        self.deterministic_decision(context)
    }

    // LLM proposal methods

    async fn llm_propose_payment(
        &self,
        llm: &LLMRouter,
        context: &PaymentContext,
    ) -> LLMResult<ProposedPaymentIntent> {
        let system = r#"You are a buyer agent. Output valid JSON only.

Schema:
{
  "target": "resonator_id",
  "amount": 1000,
  "asset": "IUSD",
  "purpose": "description",
  "category": "category"
}

Rules:
- amount must be <= available_budget
- target must be the seller_id
- be concise in purpose"#;

        let user = format!(
            "Seller: {}\nService: {}\nPrice: {}\nAvailable budget: {}\n\nCreate a payment intent.",
            context.seller_id, context.service_description, context.price, context.available_budget
        );

        let request = CompletionRequest::new(vec![Message::user(user)])
            .with_system(system)
            .with_json_mode()
            .with_max_tokens(256);

        let response = llm.complete(request).await?;

        // Parse and validate
        let proposal = self
            .guard
            .parse_payment_intent(&response.content)
            .map_err(|e| openibank_llm::LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        Ok(proposal)
    }

    async fn llm_propose_invoice(
        &self,
        llm: &LLMRouter,
        context: &InvoiceContext,
    ) -> LLMResult<ProposedInvoice> {
        let system = r#"You are a seller agent. Output valid JSON only.

Schema:
{
  "buyer": "resonator_id",
  "amount": 1000,
  "asset": "IUSD",
  "description": "service description",
  "delivery_conditions": ["condition1", "condition2"]
}

Rules:
- amount must equal the service price
- be clear and professional in description"#;

        let user = format!(
            "Buyer: {}\nService: {}\nPrice: {}\n\nCreate an invoice.",
            context.buyer_id, context.service_name, context.price
        );

        let request = CompletionRequest::new(vec![Message::user(user)])
            .with_system(system)
            .with_json_mode()
            .with_max_tokens(256);

        let response = llm.complete(request).await?;

        let proposal = self
            .guard
            .parse_invoice(&response.content)
            .map_err(|e| openibank_llm::LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        Ok(proposal)
    }

    async fn llm_propose_decision(
        &self,
        llm: &LLMRouter,
        context: &ArbiterContext,
    ) -> LLMResult<ProposedArbiterDecision> {
        let system = r#"You are an arbiter agent. Output valid JSON only.

Schema:
{
  "escrow_id": "escrow_xxx",
  "decision": "release" | "refund" | {"partial": {"release_percent": 50}},
  "reasoning": "explanation"
}

Rules:
- If delivery_proof is valid, release funds
- If delivery_proof is invalid, refund
- Be fair and objective"#;

        let user = format!(
            "Escrow: {}\nDelivery proof: {}\nDispute reason: {}\n\nMake a decision.",
            context.escrow_id,
            context.delivery_proof.as_deref().unwrap_or("None"),
            context.dispute_reason.as_deref().unwrap_or("None")
        );

        let request = CompletionRequest::new(vec![Message::user(user)])
            .with_system(system)
            .with_json_mode()
            .with_max_tokens(256);

        let response = llm.complete(request).await?;

        let proposal = self
            .guard
            .parse_arbiter_decision(&response.content)
            .map_err(|e| openibank_llm::LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        Ok(proposal)
    }

    // Deterministic fallback methods

    fn deterministic_payment(&self, context: &PaymentContext) -> ProposedPaymentIntent {
        ProposedPaymentIntent {
            target: context.seller_id.clone(),
            amount: context.price.min(context.available_budget),
            asset: "IUSD".to_string(),
            purpose: format!("Payment for: {}", context.service_description),
            category: "services".to_string(),
        }
    }

    fn deterministic_invoice(&self, context: &InvoiceContext) -> ProposedInvoice {
        ProposedInvoice {
            buyer: context.buyer_id.clone(),
            amount: context.price,
            asset: "IUSD".to_string(),
            description: format!("Invoice for: {}", context.service_name),
            delivery_conditions: vec!["Service delivered as specified".to_string()],
        }
    }

    fn deterministic_decision(&self, context: &ArbiterContext) -> ProposedArbiterDecision {
        use openibank_guard::ArbiterDecision;

        // Simple deterministic logic: if there's delivery proof, release
        let decision = if context.delivery_proof.is_some() {
            ArbiterDecision::Release
        } else if context.dispute_reason.is_some() {
            ArbiterDecision::Refund
        } else {
            // Default to release if no dispute
            ArbiterDecision::Release
        };

        ProposedArbiterDecision {
            escrow_id: context.escrow_id.clone(),
            decision,
            reasoning: "Deterministic decision based on available evidence".to_string(),
        }
    }
}

/// Context for payment proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentContext {
    pub seller_id: String,
    pub service_description: String,
    pub price: u64,
    pub available_budget: u64,
}

/// Context for invoice proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceContext {
    pub buyer_id: String,
    pub service_name: String,
    pub price: u64,
}

/// Context for arbiter decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbiterContext {
    pub escrow_id: String,
    pub delivery_proof: Option<String>,
    pub dispute_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deterministic_payment() {
        let brain = AgentBrain::deterministic();

        let context = PaymentContext {
            seller_id: "seller_123".to_string(),
            service_description: "API access".to_string(),
            price: 1000,
            available_budget: 5000,
        };

        let proposal = brain.propose_payment(&context).await;

        assert_eq!(proposal.target, "seller_123");
        assert_eq!(proposal.amount, 1000);
    }

    #[tokio::test]
    async fn test_deterministic_payment_budget_limit() {
        let brain = AgentBrain::deterministic();

        let context = PaymentContext {
            seller_id: "seller_123".to_string(),
            service_description: "Expensive service".to_string(),
            price: 10000,
            available_budget: 5000, // Less than price
        };

        let proposal = brain.propose_payment(&context).await;

        // Should cap at available budget
        assert_eq!(proposal.amount, 5000);
    }

    #[tokio::test]
    async fn test_deterministic_invoice() {
        let brain = AgentBrain::deterministic();

        let context = InvoiceContext {
            buyer_id: "buyer_123".to_string(),
            service_name: "Data Feed".to_string(),
            price: 500,
        };

        let proposal = brain.propose_invoice(&context).await;

        assert_eq!(proposal.buyer, "buyer_123");
        assert_eq!(proposal.amount, 500);
    }

    #[tokio::test]
    async fn test_deterministic_arbiter_with_proof() {
        use openibank_guard::ArbiterDecision;

        let brain = AgentBrain::deterministic();

        let context = ArbiterContext {
            escrow_id: "escrow_123".to_string(),
            delivery_proof: Some("Service delivered successfully".to_string()),
            dispute_reason: None,
        };

        let decision = brain.propose_arbiter_decision(&context).await;

        assert_eq!(decision.decision, ArbiterDecision::Release);
    }

    #[tokio::test]
    async fn test_deterministic_arbiter_with_dispute() {
        use openibank_guard::ArbiterDecision;

        let brain = AgentBrain::deterministic();

        let context = ArbiterContext {
            escrow_id: "escrow_123".to_string(),
            delivery_proof: None,
            dispute_reason: Some("Service not delivered".to_string()),
        };

        let decision = brain.propose_arbiter_decision(&context).await;

        assert_eq!(decision.decision, ArbiterDecision::Refund);
    }
}
