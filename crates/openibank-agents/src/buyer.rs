//! Buyer Agent - Has budget, issues permits, pays via escrow
//!
//! The buyer agent demonstrates the core buyer flow:
//! 1. Has a wallet with funds and a budget policy
//! 2. Issues SpendPermits for specific purchases
//! 3. Accepts invoices from sellers
//! 4. Creates escrow for conditional payments
//! 5. Releases escrow when delivery is confirmed

use std::sync::Arc;

use chrono::{Duration, Utc};
use openibank_core::{
    Amount, AssetId, BudgetPolicy, CounterpartyConstraint, Escrow, EscrowId, EscrowIntent,
    Invoice, InvoiceId, ReleaseCondition, ResonatorId, SpendPermit, SpendPurpose,
    Wallet,
};
use openibank_ledger::Ledger;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::brain::{AgentBrain, PaymentContext};

/// Errors that can occur in buyer operations
#[derive(Error, Debug)]
pub enum BuyerError {
    #[error("Wallet error: {0}")]
    WalletError(#[from] openibank_core::CoreError),

    #[error("Insufficient funds: have {available}, need {required}")]
    InsufficientFunds { available: u64, required: u64 },

    #[error("Invoice not found: {invoice_id}")]
    InvoiceNotFound { invoice_id: String },

    #[error("Escrow not found: {escrow_id}")]
    EscrowNotFound { escrow_id: String },

    #[error("Invalid invoice: {reason}")]
    InvalidInvoice { reason: String },
}

pub type Result<T> = std::result::Result<T, BuyerError>;

/// Service offer from a seller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceOffer {
    pub seller_id: ResonatorId,
    pub service_name: String,
    pub description: String,
    pub price: Amount,
    pub asset: AssetId,
}

/// The Buyer Agent
///
/// Demonstrates the buyer side of agent commerce:
/// - Manages a wallet with funds
/// - Has a budget policy for bounded spending
/// - Issues permits and pays via escrow
pub struct BuyerAgent {
    id: ResonatorId,
    wallet: Wallet,
    brain: AgentBrain,
    #[allow(dead_code)] // Reserved for future ledger integration
    ledger: Arc<Ledger>,
    pending_invoices: Vec<Invoice>,
}

impl BuyerAgent {
    /// Create a new buyer agent
    pub fn new(id: ResonatorId, ledger: Arc<Ledger>) -> Self {
        let wallet = Wallet::new(id.clone());
        Self {
            id,
            wallet,
            brain: AgentBrain::deterministic(),
            ledger,
            pending_invoices: Vec::new(),
        }
    }

    /// Create with LLM brain
    pub fn with_brain(id: ResonatorId, ledger: Arc<Ledger>, brain: AgentBrain) -> Self {
        let wallet = Wallet::new(id.clone());
        Self {
            id,
            wallet,
            brain,
            ledger,
            pending_invoices: Vec::new(),
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

    /// Set up the buyer with initial funds and budget
    pub fn setup(&mut self, initial_funds: Amount, budget_max: Amount) -> Result<()> {
        // Credit initial funds
        self.wallet.credit(&AssetId::iusd(), initial_funds)?;

        // Set up budget policy
        let budget = BudgetPolicy::new(self.id.clone(), budget_max);
        self.wallet.set_budget(budget)?;

        Ok(())
    }

    /// Get current balance
    pub fn balance(&self) -> Amount {
        self.wallet.balance(&AssetId::iusd())
    }

    /// Get remaining budget
    pub fn remaining_budget(&self) -> Amount {
        self.wallet
            .budget()
            .map(|b| b.max_total.checked_sub(b.spent_total).unwrap_or(Amount::zero()))
            .unwrap_or(Amount::zero())
    }

    /// Evaluate a service offer and decide whether to buy
    pub async fn evaluate_offer(&self, offer: &ServiceOffer) -> bool {
        // Check if we can afford it
        let balance = self.balance();
        let remaining = self.remaining_budget();

        balance >= offer.price && remaining >= offer.price
    }

    /// Accept an invoice from a seller
    pub fn accept_invoice(&mut self, invoice: Invoice) -> Result<()> {
        // Validate invoice
        if invoice.buyer != self.id {
            return Err(BuyerError::InvalidInvoice {
                reason: "Invoice is not addressed to this buyer".to_string(),
            });
        }

        if invoice.amount > self.balance() {
            return Err(BuyerError::InsufficientFunds {
                available: self.balance().0,
                required: invoice.amount.0,
            });
        }

        self.pending_invoices.push(invoice);
        Ok(())
    }

    /// Create a payment for an invoice via escrow
    pub async fn pay_invoice(&mut self, invoice_id: &InvoiceId) -> Result<(SpendPermit, Escrow)> {
        // Find the invoice
        let invoice_idx = self
            .pending_invoices
            .iter()
            .position(|i| &i.invoice_id == invoice_id)
            .ok_or_else(|| BuyerError::InvoiceNotFound {
                invoice_id: invoice_id.0.clone(),
            })?;

        let invoice = self.pending_invoices.remove(invoice_idx);

        // Use brain to propose payment details
        let context = PaymentContext {
            seller_id: invoice.seller.0.clone(),
            service_description: invoice.description.clone(),
            price: invoice.amount.0,
            available_budget: self.remaining_budget().0,
        };

        let proposal = self.brain.propose_payment(&context).await;

        // Issue a permit for this payment
        let permit = self.wallet.issue_permit(
            Amount::new(proposal.amount),
            CounterpartyConstraint::Specific(invoice.seller.clone()),
            SpendPurpose {
                category: proposal.category,
                description: proposal.purpose,
            },
            Duration::hours(24),
        )?;

        // Create escrow
        let escrow_intent = EscrowIntent {
            escrow_id: EscrowId::new(),
            invoice: invoice.invoice_id.clone(),
            payer: self.id.clone(),
            payee: invoice.seller.clone(),
            locked_amount: invoice.amount,
            asset: invoice.asset.clone(),
            release_conditions: vec![ReleaseCondition {
                condition_type: "delivery_verified".to_string(),
                parameters: serde_json::json!({}),
                met: false,
            }],
            arbiter: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(7),
        };

        let escrow = self.wallet.create_escrow(escrow_intent)?;

        Ok((permit, escrow))
    }

    /// Confirm delivery and release escrow
    pub fn confirm_delivery(&mut self, escrow_id: &EscrowId) -> Result<Amount> {
        // Mark condition as met
        self.wallet.update_escrow_conditions(escrow_id, 0, true)?;

        // Release the escrow
        let amount = self.wallet.release_escrow(escrow_id)?;

        Ok(amount)
    }

    /// Get pending invoices
    pub fn pending_invoices(&self) -> &[Invoice] {
        &self.pending_invoices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buyer_setup() {
        let ledger = Arc::new(Ledger::new());
        let mut buyer = BuyerAgent::new(ResonatorId::new(), ledger);

        buyer
            .setup(Amount::new(100000), Amount::new(50000))
            .unwrap();

        assert_eq!(buyer.balance(), Amount::new(100000));
        assert_eq!(buyer.remaining_budget(), Amount::new(50000));
    }

    #[tokio::test]
    async fn test_buyer_evaluate_offer() {
        let ledger = Arc::new(Ledger::new());
        let mut buyer = BuyerAgent::new(ResonatorId::new(), ledger);

        buyer
            .setup(Amount::new(100000), Amount::new(50000))
            .unwrap();

        let affordable_offer = ServiceOffer {
            seller_id: ResonatorId::new(),
            service_name: "API".to_string(),
            description: "API access".to_string(),
            price: Amount::new(10000),
            asset: AssetId::iusd(),
        };

        let expensive_offer = ServiceOffer {
            seller_id: ResonatorId::new(),
            service_name: "Premium".to_string(),
            description: "Expensive service".to_string(),
            price: Amount::new(200000),
            asset: AssetId::iusd(),
        };

        assert!(buyer.evaluate_offer(&affordable_offer).await);
        assert!(!buyer.evaluate_offer(&expensive_offer).await);
    }

    #[tokio::test]
    async fn test_buyer_accept_invoice() {
        let ledger = Arc::new(Ledger::new());
        let buyer_id = ResonatorId::new();
        let mut buyer = BuyerAgent::new(buyer_id.clone(), ledger);

        buyer
            .setup(Amount::new(100000), Amount::new(50000))
            .unwrap();

        let invoice = Invoice {
            invoice_id: InvoiceId::new(),
            seller: ResonatorId::new(),
            buyer: buyer_id,
            asset: AssetId::iusd(),
            amount: Amount::new(5000),
            description: "Test service".to_string(),
            delivery_conditions: vec![],
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(7),
        };

        buyer.accept_invoice(invoice).unwrap();
        assert_eq!(buyer.pending_invoices().len(), 1);
    }

    #[tokio::test]
    async fn test_buyer_pay_invoice() {
        let ledger = Arc::new(Ledger::new());
        let buyer_id = ResonatorId::new();
        let seller_id = ResonatorId::new();
        let mut buyer = BuyerAgent::new(buyer_id.clone(), ledger);

        buyer
            .setup(Amount::new(100000), Amount::new(50000))
            .unwrap();

        let invoice = Invoice {
            invoice_id: InvoiceId::new(),
            seller: seller_id,
            buyer: buyer_id,
            asset: AssetId::iusd(),
            amount: Amount::new(5000),
            description: "Test service".to_string(),
            delivery_conditions: vec![],
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(7),
        };

        let invoice_id = invoice.invoice_id.clone();
        buyer.accept_invoice(invoice).unwrap();

        let (permit, escrow) = buyer.pay_invoice(&invoice_id).await.unwrap();

        assert!(permit.is_valid());
        assert_eq!(escrow.amount, Amount::new(5000));
    }
}
