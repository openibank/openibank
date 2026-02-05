//! Seller Agent - Publishes services, issues invoices, delivers
//!
//! The seller agent demonstrates the seller flow:
//! 1. Publishes service offers
//! 2. Issues invoices to buyers
//! 3. Delivers services
//! 4. Receives payment from escrow

use std::sync::Arc;

use chrono::{Duration, Utc};
use openibank_core::{
    Amount, AssetId, DeliveryCondition, Invoice, InvoiceId, ResonatorId, Wallet,
};
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::brain::AgentBrain;
use openibank_agent_kernel::{
    AgentKernel, CapabilitySet, Contract, ContractSet, DeterministicPolicy, KernelAction,
    KernelConfig, KernelMode, ProposalRequest,
};
use crate::buyer::ServiceOffer;

/// Errors that can occur in seller operations
#[derive(Error, Debug)]
pub enum SellerError {
    #[error("Wallet error: {0}")]
    WalletError(#[from] openibank_core::CoreError),

    #[error("Service not found: {service_name}")]
    ServiceNotFound { service_name: String },

    #[error("Invoice not found: {invoice_id}")]
    InvoiceNotFound { invoice_id: String },

    #[error("Kernel error: {0}")]
    KernelError(String),
}

pub type Result<T> = std::result::Result<T, SellerError>;

/// A service that the seller offers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub description: String,
    pub price: Amount,
    pub asset: AssetId,
    pub delivery_conditions: Vec<String>,
}

/// Delivery proof for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryProof {
    pub invoice_id: InvoiceId,
    pub proof_type: String,
    pub proof_data: String,
    pub delivered_at: chrono::DateTime<Utc>,
}

/// The Seller Agent
///
/// Demonstrates the seller side of agent commerce:
/// - Publishes service offerings
/// - Issues invoices to buyers
/// - Delivers services and provides proof
pub struct SellerAgent {
    id: ResonatorId,
    wallet: Wallet,
    kernel: AgentKernel,
    #[allow(dead_code)] // Reserved for future ledger integration
    ledger: Arc<Ledger>,
    services: Vec<Service>,
    issued_invoices: Vec<Invoice>,
    deliveries: Vec<DeliveryProof>,
}

impl SellerAgent {
    /// Create a new seller agent
    pub fn new(id: ResonatorId, ledger: Arc<Ledger>) -> Self {
        let brain = AgentBrain::deterministic();
        Self::with_brain(id, ledger, brain)
    }

    /// Create with LLM brain
    pub fn with_brain(id: ResonatorId, ledger: Arc<Ledger>, brain: AgentBrain) -> Self {
        let wallet = Wallet::new(id.clone());
        let mode = match brain.mode() {
            crate::brain::BrainMode::Deterministic => KernelMode::Deterministic,
            crate::brain::BrainMode::LLM => KernelMode::Llm,
        };
        let capabilities = CapabilitySet::from_attested([
            "invoice.issue",
            "service.deliver",
            "payment.receive",
        ]);
        let contracts = ContractSet::new(vec![Contract {
            name: "seller_contract".to_string(),
            max_spend: None,
            allowed_assets: vec!["IUSD".to_string()],
            require_reversible: false,
            allowed_outcomes: vec![],
        }]);
        let kernel = AgentKernel::new(KernelConfig {
            agent_id: id.0.clone(),
            role: "seller".to_string(),
            mode,
            proposer: Box::new(brain),
            policy: Box::new(DeterministicPolicy::default()),
            capabilities,
            contracts,
            trace_max_entries: Some(1000),
        });
        Self {
            id,
            wallet,
            kernel,
            ledger,
            services: Vec::new(),
            issued_invoices: Vec::new(),
            deliveries: Vec::new(),
        }
    }

    /// Get the agent's ID
    pub fn id(&self) -> &ResonatorId {
        &self.id
    }

    /// Get the wallet
    pub fn wallet(&self) -> &Wallet {
        &self.wallet
    }

    /// Get mutable wallet
    pub fn wallet_mut(&mut self) -> &mut Wallet {
        &mut self.wallet
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

    /// Get current balance
    pub fn balance(&self) -> Amount {
        self.wallet.balance(&AssetId::iusd())
    }

    /// Publish a new service
    pub fn publish_service(&mut self, service: Service) {
        self.services.push(service);
    }

    /// Get available services
    pub fn services(&self) -> &[Service] {
        &self.services
    }

    /// Get a service offer for a specific service
    pub fn get_offer(&self, service_name: &str) -> Option<ServiceOffer> {
        self.services
            .iter()
            .find(|s| s.name == service_name)
            .map(|s| ServiceOffer {
                seller_id: self.id.clone(),
                service_name: s.name.clone(),
                description: s.description.clone(),
                price: s.price,
                asset: s.asset.clone(),
            })
    }

    /// Issue an invoice to a buyer
    pub async fn issue_invoice(
        &mut self,
        buyer_id: ResonatorId,
        service_name: &str,
    ) -> Result<Invoice> {
        // Find the service
        let service = self
            .services
            .iter()
            .find(|s| s.name == service_name)
            .ok_or_else(|| SellerError::ServiceNotFound {
                service_name: service_name.to_string(),
            })?
            .clone();

        // Use brain to propose invoice details
        let proposal = self
            .kernel
            .propose_invoice(ProposalRequest::Invoice {
                buyer_id: buyer_id.0.clone(),
                service_name: service.name.clone(),
                price: service.price.0,
            })
            .await
            .map_err(|e| SellerError::KernelError(e.to_string()))?;

        // Create the invoice
        let invoice = Invoice {
            invoice_id: InvoiceId::new(),
            seller: self.id.clone(),
            buyer: buyer_id,
            asset: service.asset,
            amount: Amount::new(proposal.amount),
            description: proposal.description,
            delivery_conditions: proposal
                .delivery_conditions
                .into_iter()
                .map(|c| DeliveryCondition {
                    condition_type: "requirement".to_string(),
                    parameters: serde_json::json!({"description": c}),
                })
                .collect(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
        };

        self.issued_invoices.push(invoice.clone());

        Ok(invoice)
    }

    /// Deliver a service and create proof
    pub fn deliver_service(&mut self, invoice_id: &InvoiceId, proof_data: String) -> Result<DeliveryProof> {
        self.kernel
            .authorize_action(KernelAction::DeliverService {
                invoice_id: invoice_id.0.clone(),
            })
            .map_err(|e| SellerError::KernelError(e.to_string()))?;

        // Verify invoice exists
        let _invoice = self
            .issued_invoices
            .iter()
            .find(|i| &i.invoice_id == invoice_id)
            .ok_or_else(|| SellerError::InvoiceNotFound {
                invoice_id: invoice_id.0.clone(),
            })?;

        let proof = DeliveryProof {
            invoice_id: invoice_id.clone(),
            proof_type: "service_delivered".to_string(),
            proof_data,
            delivered_at: Utc::now(),
        };

        self.deliveries.push(proof.clone());

        Ok(proof)
    }

    /// Receive payment (called when escrow is released)
    pub fn receive_payment(&mut self, amount: Amount) -> Result<()> {
        self.kernel
            .authorize_action(KernelAction::ReceivePayment {
                amount: amount.0,
                asset: AssetId::iusd().0,
            })
            .map_err(|e| SellerError::KernelError(e.to_string()))?;
        self.wallet.credit(&AssetId::iusd(), amount)?;
        Ok(())
    }

    /// Get issued invoices
    pub fn issued_invoices(&self) -> &[Invoice] {
        &self.issued_invoices
    }

    /// Get delivery proofs
    pub fn deliveries(&self) -> &[DeliveryProof] {
        &self.deliveries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_seller_publish_service() {
        let ledger = Arc::new(Ledger::new());
        let mut seller = SellerAgent::new(ResonatorId::new(), ledger);

        let service = Service {
            name: "API Access".to_string(),
            description: "Full API access for 30 days".to_string(),
            price: Amount::new(5000),
            asset: AssetId::iusd(),
            delivery_conditions: vec!["Provide API key".to_string()],
        };

        seller.publish_service(service);

        assert_eq!(seller.services().len(), 1);
        assert!(seller.get_offer("API Access").is_some());
    }

    #[tokio::test]
    async fn test_seller_issue_invoice() {
        let ledger = Arc::new(Ledger::new());
        let mut seller = SellerAgent::new(ResonatorId::new(), ledger);

        let service = Service {
            name: "Data Feed".to_string(),
            description: "Real-time data feed".to_string(),
            price: Amount::new(10000),
            asset: AssetId::iusd(),
            delivery_conditions: vec![],
        };

        seller.publish_service(service);

        let buyer_id = ResonatorId::new();
        let invoice = seller.issue_invoice(buyer_id.clone(), "Data Feed").await.unwrap();

        assert_eq!(invoice.buyer, buyer_id);
        assert_eq!(invoice.amount, Amount::new(10000));
    }

    #[tokio::test]
    async fn test_seller_deliver_service() {
        let ledger = Arc::new(Ledger::new());
        let mut seller = SellerAgent::new(ResonatorId::new(), ledger);

        let service = Service {
            name: "Report".to_string(),
            description: "Custom report".to_string(),
            price: Amount::new(2000),
            asset: AssetId::iusd(),
            delivery_conditions: vec![],
        };

        seller.publish_service(service);

        let buyer_id = ResonatorId::new();
        let invoice = seller.issue_invoice(buyer_id, "Report").await.unwrap();

        let proof = seller
            .deliver_service(&invoice.invoice_id, "Report completed: https://example.com/report".to_string())
            .unwrap();

        assert_eq!(proof.invoice_id, invoice.invoice_id);
    }

    #[tokio::test]
    async fn test_seller_receive_payment() {
        let ledger = Arc::new(Ledger::new());
        let mut seller = SellerAgent::new(ResonatorId::new(), ledger);

        assert_eq!(seller.balance(), Amount::zero());

        seller.set_active_commitment("test_commitment", true);
        seller.receive_payment(Amount::new(5000)).unwrap();

        assert_eq!(seller.balance(), Amount::new(5000));
    }
}
