//! OpeniBank domain types — pure domain layer, zero Maple dependency.
pub mod amount;
pub mod arena;
pub mod card;
pub mod error;

pub use amount::IusdAmount;
pub use arena::{run_arena, render_champion_svg, ArenaResult, BotResult, BotStrategy};
pub use error::{DomainError, PermitError, ReceiptError};

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ulid::Ulid;

// ── Agent ID ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Legacy Amount alias ───────────────────────────────────────────────────────
pub type Amount = i64;

// ── Spend Permit ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpendPermit {
    pub permit_id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub max_amount: IusdAmount,
    pub spent_amount: IusdAmount,
    pub purpose: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub grantor_sig: String,
    pub grantor_pubkey: String,
}

impl SpendPermit {
    pub fn new(from: AgentId, to: AgentId, max_amount: IusdAmount, purpose: impl Into<String>, expires_at: DateTime<Utc>) -> Self {
        Self {
            permit_id: format!("perm_{}", Ulid::new()),
            from, to, max_amount,
            spent_amount: IusdAmount::ZERO,
            purpose: purpose.into(),
            issued_at: Utc::now(), expires_at,
            grantor_sig: String::new(),
            grantor_pubkey: String::new(),
        }
    }

    pub fn new_daily(from: AgentId, to: AgentId, max_amount: IusdAmount, purpose: impl Into<String>) -> Self {
        Self::new(from, to, max_amount, purpose, Utc::now() + chrono::Duration::hours(24))
    }

    pub fn new_legacy(from: AgentId, to: AgentId, max_amount: Amount) -> Self {
        let iusd = IusdAmount::from_micros(max_amount.unsigned_abs() as u128);
        let mut p = Self::new(from, to, iusd, "legacy permit", Utc::now() + chrono::Duration::hours(24));
        p.permit_id = format!("permit-{}", uuid::Uuid::new_v4());
        p
    }

    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(self.permit_id.as_bytes());
        h.update(self.from.0.as_bytes());
        h.update(self.to.0.as_bytes());
        h.update(&self.max_amount.micros().to_le_bytes());
        h.update(&self.expires_at.timestamp().to_le_bytes());
        *h.finalize().as_bytes()
    }

    pub fn sign(mut self, signing_key: &SigningKey) -> Self {
        let hash = self.canonical_hash();
        let sig = signing_key.sign(&hash);
        self.grantor_sig = hex::encode(sig.to_bytes());
        self.grantor_pubkey = hex::encode(signing_key.verifying_key().to_bytes());
        self
    }

    pub fn validate_spend(&self, spender: &AgentId, amount: &IusdAmount) -> Result<(), PermitError> {
        if spender != &self.to {
            return Err(PermitError::WrongGrantee { expected: self.to.0.clone(), actual: spender.0.clone() });
        }
        if Utc::now() > self.expires_at {
            return Err(PermitError::Expired);
        }
        let remaining = self.remaining().unwrap_or(IusdAmount::ZERO);
        if amount > &remaining {
            return Err(PermitError::AmountExceeded { requested: amount.to_display_string(), remaining: remaining.to_display_string() });
        }
        Ok(())
    }

    pub fn remaining(&self) -> Option<IusdAmount> {
        self.max_amount.checked_sub(&self.spent_amount)
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool { now > self.expires_at }
}

// ── Escrow ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscrowStatus { Locked, Settled, Canceled }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscrowCondition {
    BuyerConfirmation,
    TimeLock { release_after: DateTime<Utc> },
    DataDelivery { data_hash: [u8; 32] },
    ServiceCompletion { service_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Escrow {
    pub escrow_id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub amount: IusdAmount,
    pub condition: EscrowCondition,
    pub status: EscrowStatus,
    pub created_at: DateTime<Utc>,
    pub funded_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Escrow {
    pub fn new(from: AgentId, to: AgentId, amount: IusdAmount, condition: EscrowCondition) -> Self {
        Self {
            escrow_id: format!("escrow_{}", Ulid::new()),
            from, to, amount, condition,
            status: EscrowStatus::Locked,
            created_at: Utc::now(), funded_at: None, resolved_at: None,
        }
    }
    pub fn new_legacy(from: AgentId, to: AgentId, amount: Amount) -> Self {
        Self::new(from, to, IusdAmount::from_micros(amount.unsigned_abs() as u128), EscrowCondition::BuyerConfirmation)
    }
}

// ── Legacy Commitment ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commitment {
    pub commitment_id: String,
    pub policy_id: String,
    pub contract_id: String,
}

impl Commitment {
    pub fn new(policy_id: impl Into<String>, contract_id: impl Into<String>) -> Self {
        Self {
            commitment_id: format!("commitment-{}", uuid::Uuid::new_v4()),
            policy_id: policy_id.into(),
            contract_id: contract_id.into(),
        }
    }
}

// ── Receipt ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReceiptActionType {
    Transfer, Mint, Burn, EscrowFund, EscrowRelease, EscrowRefund, SettlementBatch, ArenaWin,
}

fn default_action_type() -> ReceiptActionType { ReceiptActionType::Transfer }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub tx_id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub amount: Amount,
    pub permit_id: String,
    pub commitment_id: String,
    pub worldline_id: String,
    pub worldline_event_id: String,
    pub worldline_event_hash: String,
    pub signer_public_key: String,
    pub receipt_sig: String,
    pub timestamp: DateTime<Utc>,
    pub tagline: String,
    #[serde(default = "default_action_type")]
    pub action_type: ReceiptActionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iusd_amount: Option<IusdAmount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReceiptPayload {
    tx_id: String, from: AgentId, to: AgentId, amount: Amount,
    permit_id: String, commitment_id: String,
    worldline_id: String, worldline_event_id: String, worldline_event_hash: String,
    timestamp: DateTime<Utc>, tagline: String, action_type: ReceiptActionType,
}

impl Receipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new_unsigned(
        from: AgentId, to: AgentId, amount: Amount,
        permit_id: impl Into<String>, commitment_id: impl Into<String>,
        worldline_id: impl Into<String>, worldline_event_id: impl Into<String>,
        worldline_event_hash: impl Into<String>, tagline: impl Into<String>,
    ) -> Self {
        Self {
            tx_id: format!("rcpt_{}", Ulid::new()),
            from, to, amount,
            permit_id: permit_id.into(), commitment_id: commitment_id.into(),
            worldline_id: worldline_id.into(), worldline_event_id: worldline_event_id.into(),
            worldline_event_hash: worldline_event_hash.into(),
            signer_public_key: String::new(), receipt_sig: String::new(),
            timestamp: Utc::now(), tagline: tagline.into(),
            action_type: ReceiptActionType::Transfer,
            chain_tx_hash: None, chain_name: None, iusd_amount: None,
        }
    }

    fn payload(&self) -> ReceiptPayload {
        ReceiptPayload {
            tx_id: self.tx_id.clone(), from: self.from.clone(), to: self.to.clone(),
            amount: self.amount, permit_id: self.permit_id.clone(),
            commitment_id: self.commitment_id.clone(), worldline_id: self.worldline_id.clone(),
            worldline_event_id: self.worldline_event_id.clone(),
            worldline_event_hash: self.worldline_event_hash.clone(),
            timestamp: self.timestamp, tagline: self.tagline.clone(),
            action_type: self.action_type.clone(),
        }
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, DomainError> {
        Ok(serde_json::to_vec(&self.payload())?)
    }

    pub fn canonical_hash_hex(&self) -> Result<String, DomainError> {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_bytes()?);
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn sign(mut self, signing_key: &SigningKey) -> Result<Self, DomainError> {
        let payload = self.canonical_bytes()?;
        let sig = signing_key.sign(&payload);
        self.signer_public_key = hex::encode(signing_key.verifying_key().to_bytes());
        self.receipt_sig = hex::encode(sig.to_bytes());
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), DomainError> {
        let payload = self.canonical_bytes()?;
        let pk_bytes: [u8; 32] = hex::decode(&self.signer_public_key)?
            .try_into().map_err(|_| DomainError::InvalidPublicKeyBytes)?;
        let sig_bytes: [u8; 64] = hex::decode(&self.receipt_sig)?
            .try_into().map_err(|_| DomainError::InvalidSignatureBytes)?;
        let key = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| DomainError::InvalidPublicKeyBytes)?;
        let sig = Signature::from_bytes(&sig_bytes);
        key.verify(&payload, &sig).map_err(|_| DomainError::SignatureVerificationFailed)?;
        Ok(())
    }

    pub fn worldline_pointer(&self) -> String {
        format!("{}#{}", self.worldline_id, self.worldline_event_id)
    }

    pub fn verify_url(&self) -> String {
        format!("https://openibank.com/verify/{}", self.tx_id)
    }

    pub fn iusd_amount(&self) -> IusdAmount {
        self.iusd_amount.unwrap_or_else(|| IusdAmount::from_micros(self.amount.unsigned_abs() as u128))
    }
}

// ── Domain Intent ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DomainIntent {
    MintIusd { to: AgentId, amount: IusdAmount },
    TransferIusd { from: AgentId, to: AgentId, amount: IusdAmount, permit_id: String },
    FundEscrow { escrow_id: String, from: AgentId, amount: IusdAmount, permit_id: String },
    ReleaseEscrow { escrow_id: String, to: AgentId },
    RefundEscrow { escrow_id: String, to: AgentId },
    BurnIusd { from: AgentId, amount: IusdAmount },
    SettleBatch { batch_id: String, legs: Vec<SettlementLeg> },
}

impl DomainIntent {
    pub fn canonical_hash(&self) -> [u8; 32] {
        let json = serde_json::to_vec(self).unwrap_or_default();
        let mut h = blake3::Hasher::new();
        h.update(&json);
        *h.finalize().as_bytes()
    }

    pub fn description(&self) -> String {
        match self {
            DomainIntent::MintIusd { to, amount } => format!("mint {} to {}", amount, to),
            DomainIntent::TransferIusd { from, to, amount, .. } => format!("transfer {} from {} to {}", amount, from, to),
            DomainIntent::FundEscrow { from, amount, .. } => format!("fund escrow: {} from {}", amount, from),
            DomainIntent::ReleaseEscrow { to, .. } => format!("release escrow to {}", to),
            DomainIntent::RefundEscrow { to, .. } => format!("refund escrow to {}", to),
            DomainIntent::BurnIusd { from, amount } => format!("burn {} from {}", amount, from),
            DomainIntent::SettleBatch { batch_id, legs } => format!("settle batch {} ({} legs)", batch_id, legs.len()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettlementLeg {
    pub from: AgentId,
    pub to: AgentId,
    pub amount: IusdAmount,
}

// ── Agent Profile ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentRole { Issuer, Buyer, Seller, Auditor, Bot }

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Issuer => write!(f, "ISSUER"),
            AgentRole::Buyer => write!(f, "BUYER"),
            AgentRole::Seller => write!(f, "SELLER"),
            AgentRole::Auditor => write!(f, "AUDITOR"),
            AgentRole::Bot => write!(f, "BOT"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStrategy { Passive, Aggressive, Conservative, Arbitrage, Random }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: AgentId,
    pub display_name: String,
    pub role: AgentRole,
    pub initial_iusd: IusdAmount,
    pub spend_limit: IusdAmount,
    pub strategy: AgentStrategy,
}

impl AgentProfile {
    pub fn default_demo_agents() -> Vec<AgentProfile> {
        vec![
            AgentProfile { id: AgentId::new("issuer-01"), display_name: "Issuer".into(), role: AgentRole::Issuer, initial_iusd: IusdAmount::from_dollars(1_000_000), spend_limit: IusdAmount::from_dollars(1_000_000), strategy: AgentStrategy::Passive },
            AgentProfile { id: AgentId::new("buyer-01"),  display_name: "Buyer".into(),  role: AgentRole::Buyer,  initial_iusd: IusdAmount::ZERO, spend_limit: IusdAmount::from_dollars(10_000), strategy: AgentStrategy::Aggressive },
            AgentProfile { id: AgentId::new("seller-01"), display_name: "Seller".into(), role: AgentRole::Seller, initial_iusd: IusdAmount::ZERO, spend_limit: IusdAmount::from_dollars(10_000), strategy: AgentStrategy::Conservative },
            AgentProfile { id: AgentId::new("auditor-01"),display_name: "Auditor".into(),role: AgentRole::Auditor,initial_iusd: IusdAmount::ZERO, spend_limit: IusdAmount::ZERO, strategy: AgentStrategy::Passive },
        ]
    }
}

// ── IO helpers ────────────────────────────────────────────────────────────────

pub fn save_receipt(path: &std::path::Path, receipt: &Receipt) -> Result<(), DomainError> {
    std::fs::write(path, serde_json::to_vec_pretty(receipt)?)?;
    Ok(())
}

pub fn load_receipt(path: &std::path::Path) -> Result<Receipt, DomainError> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_key() -> SigningKey { SigningKey::from_bytes(&[7u8; 32]) }

    fn demo_receipt() -> Receipt {
        Receipt::new_unsigned(AgentId::new("buyer-01"), AgentId::new("seller-01"),
            1_500_000, "perm_test", "cmmt_test", "wl-demo", "evt-1", "abcd1234",
            "AI agents need banks too.")
    }

    #[test]
    fn receipt_sign_verify_roundtrip() {
        let r = demo_receipt().sign(&demo_key()).unwrap();
        assert!(r.verify().is_ok());
    }

    #[test]
    fn receipt_verify_fails_after_tamper() {
        let mut r = demo_receipt().sign(&demo_key()).unwrap();
        r.amount += 1;
        assert!(r.verify().is_err());
    }

    #[test]
    fn receipt_verify_url() {
        assert!(demo_receipt().verify_url().starts_with("https://openibank.com/verify/rcpt_"));
    }

    #[test]
    fn spend_permit_validation() {
        let buyer = AgentId::new("buyer-01");
        let seller = AgentId::new("seller-01");
        let permit = SpendPermit::new(buyer.clone(), seller.clone(), IusdAmount::from_dollars(100), "test", Utc::now() + chrono::Duration::hours(1));
        assert!(permit.validate_spend(&seller, &IusdAmount::from_dollars(50)).is_ok());
        assert!(permit.validate_spend(&seller, &IusdAmount::from_dollars(200)).is_err());
        assert!(permit.validate_spend(&buyer, &IusdAmount::from_dollars(10)).is_err());
    }

    #[test]
    fn spend_permit_expired() {
        let buyer = AgentId::new("buyer-01");
        let seller = AgentId::new("seller-01");
        let permit = SpendPermit::new(buyer, seller.clone(), IusdAmount::from_dollars(100), "expired", Utc::now() - chrono::Duration::hours(1));
        assert!(permit.validate_spend(&seller, &IusdAmount::from_dollars(1)).is_err());
    }

    #[test]
    fn domain_intent_hash_deterministic() {
        let i = DomainIntent::MintIusd { to: AgentId::new("buyer-01"), amount: IusdAmount::from_dollars(50) };
        assert_eq!(i.canonical_hash(), i.canonical_hash());
    }

    #[test]
    fn agent_profiles_unique_ids() {
        let profiles = AgentProfile::default_demo_agents();
        let ids: std::collections::HashSet<String> = profiles.iter().map(|p| p.id.0.clone()).collect();
        assert_eq!(ids.len(), profiles.len());
    }
}
