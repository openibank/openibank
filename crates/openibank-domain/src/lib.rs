pub mod card;

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("invalid signature bytes")]
    InvalidSignatureBytes,
    #[error("invalid public key bytes")]
    InvalidPublicKeyBytes,
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Amount = i64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpendPermit {
    pub permit_id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub max_amount: Amount,
    pub issued_at: DateTime<Utc>,
}

impl SpendPermit {
    pub fn new(from: AgentId, to: AgentId, max_amount: Amount) -> Self {
        Self {
            permit_id: format!("permit-{}", Uuid::new_v4()),
            from,
            to,
            max_amount,
            issued_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EscrowStatus {
    Locked,
    Settled,
    Canceled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Escrow {
    pub escrow_id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub amount: Amount,
    pub status: EscrowStatus,
    pub created_at: DateTime<Utc>,
}

impl Escrow {
    pub fn new(from: AgentId, to: AgentId, amount: Amount) -> Self {
        Self {
            escrow_id: format!("escrow-{}", Uuid::new_v4()),
            from,
            to,
            amount,
            status: EscrowStatus::Locked,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commitment {
    pub commitment_id: String,
    pub policy_id: String,
    pub contract_id: String,
}

impl Commitment {
    pub fn new(policy_id: impl Into<String>, contract_id: impl Into<String>) -> Self {
        Self {
            commitment_id: format!("commitment-{}", Uuid::new_v4()),
            policy_id: policy_id.into(),
            contract_id: contract_id.into(),
        }
    }
}

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptPayload {
    pub tx_id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub amount: Amount,
    pub permit_id: String,
    pub commitment_id: String,
    pub worldline_id: String,
    pub worldline_event_id: String,
    pub worldline_event_hash: String,
    pub timestamp: DateTime<Utc>,
    pub tagline: String,
}

impl Receipt {
    pub fn new_unsigned(
        from: AgentId,
        to: AgentId,
        amount: Amount,
        permit_id: impl Into<String>,
        commitment_id: impl Into<String>,
        worldline_id: impl Into<String>,
        worldline_event_id: impl Into<String>,
        worldline_event_hash: impl Into<String>,
        tagline: impl Into<String>,
    ) -> Self {
        Self {
            tx_id: format!("tx-{}", Uuid::new_v4()),
            from,
            to,
            amount,
            permit_id: permit_id.into(),
            commitment_id: commitment_id.into(),
            worldline_id: worldline_id.into(),
            worldline_event_id: worldline_event_id.into(),
            worldline_event_hash: worldline_event_hash.into(),
            signer_public_key: String::new(),
            receipt_sig: String::new(),
            timestamp: Utc::now(),
            tagline: tagline.into(),
        }
    }

    pub fn payload(&self) -> ReceiptPayload {
        ReceiptPayload {
            tx_id: self.tx_id.clone(),
            from: self.from.clone(),
            to: self.to.clone(),
            amount: self.amount,
            permit_id: self.permit_id.clone(),
            commitment_id: self.commitment_id.clone(),
            worldline_id: self.worldline_id.clone(),
            worldline_event_id: self.worldline_event_id.clone(),
            worldline_event_hash: self.worldline_event_hash.clone(),
            timestamp: self.timestamp,
            tagline: self.tagline.clone(),
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
        let signature = signing_key.sign(&payload);
        let verifying_key = signing_key.verifying_key();
        self.signer_public_key = hex::encode(verifying_key.to_bytes());
        self.receipt_sig = hex::encode(signature.to_bytes());
        Ok(self)
    }

    pub fn verify(&self) -> Result<(), DomainError> {
        let payload = self.canonical_bytes()?;
        let pk_bytes_vec = hex::decode(&self.signer_public_key)?;
        let sig_bytes_vec = hex::decode(&self.receipt_sig)?;

        let pk_bytes: [u8; 32] = pk_bytes_vec
            .try_into()
            .map_err(|_| DomainError::InvalidPublicKeyBytes)?;
        let sig_bytes: [u8; 64] = sig_bytes_vec
            .try_into()
            .map_err(|_| DomainError::InvalidSignatureBytes)?;

        let key =
            VerifyingKey::from_bytes(&pk_bytes).map_err(|_| DomainError::InvalidPublicKeyBytes)?;
        let signature = Signature::from_bytes(&sig_bytes);

        key.verify(&payload, &signature)
            .map_err(|_| DomainError::SignatureVerificationFailed)?;
        Ok(())
    }

    pub fn worldline_pointer(&self) -> String {
        format!("{}#{}", self.worldline_id, self.worldline_event_id)
    }
}

pub fn save_receipt(path: &std::path::Path, receipt: &Receipt) -> Result<(), DomainError> {
    let data = serde_json::to_vec_pretty(receipt)?;
    std::fs::write(path, data)?;
    Ok(())
}

pub fn load_receipt(path: &std::path::Path) -> Result<Receipt, DomainError> {
    let data = std::fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn demo_receipt() -> Receipt {
        Receipt::new_unsigned(
            AgentId::new("buyer-01"),
            AgentId::new("seller-01"),
            1_500,
            "permit-1",
            "commitment-1",
            "wl-demo",
            "evt-1",
            "abcd1234",
            "AI agents need banks too.",
        )
    }

    #[test]
    fn receipt_sign_verify_roundtrip() {
        let key = demo_signing_key();
        let receipt = demo_receipt().sign(&key).expect("sign should work");
        assert!(receipt.verify().is_ok(), "verify should pass");
    }

    #[test]
    fn receipt_verify_fails_after_tamper() {
        let key = demo_signing_key();
        let mut receipt = demo_receipt().sign(&key).expect("sign should work");
        receipt.amount += 1;
        assert!(receipt.verify().is_err(), "verify should fail on tamper");
    }
}
