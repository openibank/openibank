use async_trait::async_trait;

use openibank_agent_kernel::{
    AgentKernel, CapabilitySet, Contract, ContractSet, DeterministicPolicy, KernelConfig,
    KernelMode, KernelProposal, KernelProposer, ProposalRequest, ProposeError,
};
use openibank_guard::{ProposedArbiterDecision, ProposedInvoice, ProposedPaymentIntent, ArbiterDecision};

struct TestProposer;

#[async_trait]
impl KernelProposer for TestProposer {
    async fn propose(&self, request: ProposalRequest) -> Result<KernelProposal, ProposeError> {
        match request {
            ProposalRequest::Payment { seller_id, service_description, price, .. } => {
                Ok(KernelProposal::Payment(ProposedPaymentIntent {
                    target: seller_id,
                    amount: price,
                    asset: "IUSD".to_string(),
                    purpose: service_description,
                    category: "services".to_string(),
                }))
            }
            ProposalRequest::Invoice { buyer_id, service_name, price } => {
                Ok(KernelProposal::Invoice(ProposedInvoice {
                    buyer: buyer_id,
                    amount: price,
                    asset: "IUSD".to_string(),
                    description: format!("Invoice for: {}", service_name),
                    delivery_conditions: vec!["Service delivered".to_string()],
                }))
            }
            ProposalRequest::Arbitration { escrow_id, .. } => {
                Ok(KernelProposal::Arbitration(ProposedArbiterDecision {
                    escrow_id,
                    decision: ArbiterDecision::Release,
                    reasoning: "Deterministic decision".to_string(),
                }))
            }
        }
    }
}

fn base_contracts() -> ContractSet {
    ContractSet::new(vec![Contract {
        name: "default".to_string(),
        max_spend: Some(10_000),
        allowed_assets: vec!["IUSD".to_string()],
        require_reversible: true,
        allowed_outcomes: vec!["release".to_string(), "refund".to_string(), "partial".to_string()],
    }])
}

fn build_kernel(capabilities: CapabilitySet) -> AgentKernel {
    AgentKernel::new(KernelConfig {
        agent_id: "agent-1".to_string(),
        role: "buyer".to_string(),
        mode: KernelMode::Deterministic,
        proposer: Box::new(TestProposer),
        policy: Box::new(DeterministicPolicy::default()),
        capabilities,
        contracts: base_contracts(),
        trace_max_entries: Some(256),
    })
}

#[tokio::test]
async fn test_commitment_boundary_enforced() {
    let mut kernel = build_kernel(CapabilitySet::from_attested(["payment.initiate"]));

    let result = kernel
        .propose_payment(ProposalRequest::Payment {
            seller_id: "seller-1".to_string(),
            service_description: "API access".to_string(),
            price: 500,
            available_budget: 1000,
        })
        .await;

    assert!(result.is_err(), "Expected commitment gate to block payment");
}

#[tokio::test]
async fn test_capability_gating() {
    let mut kernel = build_kernel(CapabilitySet::new());
    kernel.set_active_commitment("commit-1", true);

    let result = kernel
        .propose_payment(ProposalRequest::Payment {
            seller_id: "seller-1".to_string(),
            service_description: "API access".to_string(),
            price: 500,
            available_budget: 1000,
        })
        .await;

    assert!(result.is_err(), "Expected capability gate to block payment");

    kernel.capabilities_mut().attest("payment.initiate");
    let result = kernel
        .propose_payment(ProposalRequest::Payment {
            seller_id: "seller-1".to_string(),
            service_description: "API access".to_string(),
            price: 500,
            available_budget: 1000,
        })
        .await;

    assert!(result.is_ok(), "Expected payment after attestation");
}

#[tokio::test]
async fn test_deterministic_replay() {
    let mut kernel_a = build_kernel(CapabilitySet::from_attested(["payment.initiate"]));
    kernel_a.set_active_commitment("commit-1", true);

    let _ = kernel_a
        .propose_payment(ProposalRequest::Payment {
            seller_id: "seller-1".to_string(),
            service_description: "API access".to_string(),
            price: 500,
            available_budget: 1000,
        })
        .await;

    let mut kernel_b = build_kernel(CapabilitySet::from_attested(["payment.initiate"]));
    kernel_b.set_active_commitment("commit-1", true);

    let _ = kernel_b
        .propose_payment(ProposalRequest::Payment {
            seller_id: "seller-1".to_string(),
            service_description: "API access".to_string(),
            price: 500,
            available_budget: 1000,
        })
        .await;

    assert!(kernel_a.trace().is_replayable_with(kernel_b.trace()));
}
