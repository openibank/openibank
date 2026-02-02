//! OpeniBank Receipts - Trust Artifact Toolkit
//!
//! Receipts are the social objects of OpeniBank:
//! - Shareable
//! - Stable (schema won't change incompatibly)
//! - Machine-verifiable
//!
//! This crate provides tools for:
//! - Receipt schema validation
//! - Signature verification
//! - Receipt inspection and diffing
//! - Trust chain verification

use openibank_core::{CommitmentReceipt, IssuerReceipt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during receipt operations
#[derive(Error, Debug)]
pub enum ReceiptError {
    #[error("Invalid receipt format: {message}")]
    InvalidFormat { message: String },

    #[error("Signature verification failed")]
    SignatureInvalid,

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Receipt expired")]
    Expired,

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("File error: {message}")]
    FileError { message: String },

    #[error("JSON error: {message}")]
    JsonError { message: String },
}

pub type Result<T> = std::result::Result<T, ReceiptError>;

/// Type of receipt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "receipt_type")]
pub enum Receipt {
    /// A commitment receipt (for payments/transfers)
    Commitment(CommitmentReceipt),
    /// An issuer receipt (for mint/burn)
    Issuer(IssuerReceipt),
}

impl Receipt {
    /// Verify the receipt's signature
    pub fn verify(&self) -> Result<()> {
        match self {
            Receipt::Commitment(r) => r.verify().map_err(|_| ReceiptError::SignatureInvalid),
            Receipt::Issuer(r) => r.verify().map_err(|_| ReceiptError::SignatureInvalid),
        }
    }

    /// Get the receipt ID
    pub fn id(&self) -> &str {
        match self {
            Receipt::Commitment(r) => &r.commitment_id.0,
            Receipt::Issuer(r) => &r.receipt_id,
        }
    }

    /// Get the signer's public key
    pub fn signer_public_key(&self) -> &str {
        match self {
            Receipt::Commitment(r) => &r.signer_public_key,
            Receipt::Issuer(r) => &r.signer_public_key,
        }
    }
}

/// Result of receipt verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub receipt_id: String,
    pub receipt_type: String,
    pub signer: String,
    pub errors: Vec<String>,
}

/// Verify a receipt from JSON
pub fn verify_receipt_json(json: &str) -> VerificationResult {
    let receipt: std::result::Result<Receipt, _> = serde_json::from_str(json);

    match receipt {
        Ok(r) => {
            let mut errors = vec![];

            // Verify signature
            if let Err(e) = r.verify() {
                errors.push(format!("Signature verification failed: {}", e));
            }

            VerificationResult {
                valid: errors.is_empty(),
                receipt_id: r.id().to_string(),
                receipt_type: match &r {
                    Receipt::Commitment(_) => "commitment".to_string(),
                    Receipt::Issuer(_) => "issuer".to_string(),
                },
                signer: r.signer_public_key().to_string(),
                errors,
            }
        }
        Err(e) => VerificationResult {
            valid: false,
            receipt_id: "unknown".to_string(),
            receipt_type: "unknown".to_string(),
            signer: "unknown".to_string(),
            errors: vec![format!("Failed to parse receipt: {}", e)],
        },
    }
}

/// Load and verify a receipt from a file
pub fn verify_receipt_file(path: &str) -> Result<VerificationResult> {
    let content = std::fs::read_to_string(path).map_err(|e| ReceiptError::FileError {
        message: e.to_string(),
    })?;

    Ok(verify_receipt_json(&content))
}

/// Inspection details for a receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptInspection {
    pub receipt_id: String,
    pub receipt_type: String,
    pub signer_public_key: String,
    pub signature_valid: bool,
    pub details: serde_json::Value,
}

/// Inspect a receipt's contents
pub fn inspect_receipt_json(json: &str) -> Result<ReceiptInspection> {
    let receipt: Receipt = serde_json::from_str(json).map_err(|e| ReceiptError::JsonError {
        message: e.to_string(),
    })?;

    let signature_valid = receipt.verify().is_ok();

    let details = serde_json::to_value(&receipt).map_err(|e| ReceiptError::JsonError {
        message: e.to_string(),
    })?;

    Ok(ReceiptInspection {
        receipt_id: receipt.id().to_string(),
        receipt_type: match &receipt {
            Receipt::Commitment(_) => "commitment".to_string(),
            Receipt::Issuer(_) => "issuer".to_string(),
        },
        signer_public_key: receipt.signer_public_key().to_string(),
        signature_valid,
        details,
    })
}

/// Inspect a receipt from a file
pub fn inspect_receipt_file(path: &str) -> Result<ReceiptInspection> {
    let content = std::fs::read_to_string(path).map_err(|e| ReceiptError::FileError {
        message: e.to_string(),
    })?;

    inspect_receipt_json(&content)
}

/// Compare two receipts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptDiff {
    pub receipt_a_id: String,
    pub receipt_b_id: String,
    pub differences: Vec<FieldDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDiff {
    pub field: String,
    pub value_a: serde_json::Value,
    pub value_b: serde_json::Value,
}

/// Compare two receipts and return differences
pub fn diff_receipts(json_a: &str, json_b: &str) -> Result<ReceiptDiff> {
    let a: serde_json::Value =
        serde_json::from_str(json_a).map_err(|e| ReceiptError::JsonError {
            message: format!("Failed to parse receipt A: {}", e),
        })?;

    let b: serde_json::Value =
        serde_json::from_str(json_b).map_err(|e| ReceiptError::JsonError {
            message: format!("Failed to parse receipt B: {}", e),
        })?;

    let receipt_a_id = a
        .get("commitment_id")
        .or_else(|| a.get("receipt_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let receipt_b_id = b
        .get("commitment_id")
        .or_else(|| b.get("receipt_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut differences = vec![];

    // Compare top-level fields
    if let (Some(obj_a), Some(obj_b)) = (a.as_object(), b.as_object()) {
        // Get all keys
        let mut all_keys: Vec<&String> = obj_a.keys().chain(obj_b.keys()).collect();
        all_keys.sort();
        all_keys.dedup();

        for key in all_keys {
            let val_a = obj_a.get(key).cloned().unwrap_or(serde_json::Value::Null);
            let val_b = obj_b.get(key).cloned().unwrap_or(serde_json::Value::Null);

            if val_a != val_b {
                differences.push(FieldDiff {
                    field: key.clone(),
                    value_a: val_a,
                    value_b: val_b,
                });
            }
        }
    }

    Ok(ReceiptDiff {
        receipt_a_id,
        receipt_b_id,
        differences,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openibank_core::{
        crypto::Keypair, Amount, AssetId, CommitmentId, ConsequenceRef, IssuerOperation,
        ResonatorId,
    };

    fn create_signed_commitment_receipt() -> CommitmentReceipt {
        let keypair = Keypair::generate();

        let mut receipt = CommitmentReceipt {
            commitment_id: CommitmentId::new(),
            actor: ResonatorId::new(),
            intent_hash: "test_intent_hash".to_string(),
            policy_snapshot_hash: "test_policy_hash".to_string(),
            evidence_hash: "test_evidence_hash".to_string(),
            consequence_ref: ConsequenceRef {
                consequence_type: "ledger".to_string(),
                reference_id: "entry_123".to_string(),
                metadata: serde_json::json!({}),
            },
            committed_at: Utc::now(),
            signature: String::new(),
            signer_public_key: keypair.public_key_hex(),
        };

        // Sign the receipt
        let signing_bytes = receipt.signing_bytes().unwrap();
        receipt.signature = keypair.sign(&signing_bytes);

        receipt
    }

    fn create_signed_issuer_receipt() -> IssuerReceipt {
        let keypair = Keypair::generate();

        let mut receipt = IssuerReceipt {
            receipt_id: format!("receipt_{}", uuid::Uuid::new_v4()),
            operation: IssuerOperation::Mint,
            asset: AssetId::iusd(),
            amount: Amount::new(1000),
            target: ResonatorId::new(),
            reserve_attestation_hash: "test_reserve_hash".to_string(),
            policy_snapshot_hash: "test_policy_hash".to_string(),
            issued_at: Utc::now(),
            signature: String::new(),
            signer_public_key: keypair.public_key_hex(),
        };

        // Sign the receipt
        let signing_bytes = receipt.signing_bytes().unwrap();
        receipt.signature = keypair.sign(&signing_bytes);

        receipt
    }

    #[test]
    fn test_verify_commitment_receipt() {
        let receipt = create_signed_commitment_receipt();
        let wrapped = Receipt::Commitment(receipt);

        assert!(wrapped.verify().is_ok());
    }

    #[test]
    fn test_verify_issuer_receipt() {
        let receipt = create_signed_issuer_receipt();
        let wrapped = Receipt::Issuer(receipt);

        assert!(wrapped.verify().is_ok());
    }

    #[test]
    fn test_verify_tampered_receipt() {
        let mut receipt = create_signed_commitment_receipt();
        receipt.intent_hash = "tampered".to_string(); // Tamper with data

        let wrapped = Receipt::Commitment(receipt);
        assert!(wrapped.verify().is_err());
    }

    #[test]
    fn test_verify_receipt_json() {
        let receipt = create_signed_commitment_receipt();
        let wrapped = Receipt::Commitment(receipt);
        let json = serde_json::to_string(&wrapped).unwrap();

        let result = verify_receipt_json(&json);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_inspect_receipt() {
        let receipt = create_signed_commitment_receipt();
        let wrapped = Receipt::Commitment(receipt.clone());
        let json = serde_json::to_string(&wrapped).unwrap();

        let inspection = inspect_receipt_json(&json).unwrap();
        assert_eq!(inspection.receipt_id, receipt.commitment_id.0);
        assert_eq!(inspection.receipt_type, "commitment");
        assert!(inspection.signature_valid);
    }

    #[test]
    fn test_diff_receipts() {
        let receipt1 = create_signed_commitment_receipt();
        let mut receipt2 = create_signed_commitment_receipt();
        receipt2.intent_hash = "different_hash".to_string();

        let wrapped1 = Receipt::Commitment(receipt1);
        let wrapped2 = Receipt::Commitment(receipt2);

        let json1 = serde_json::to_string(&wrapped1).unwrap();
        let json2 = serde_json::to_string(&wrapped2).unwrap();

        let diff = diff_receipts(&json1, &json2).unwrap();
        assert!(!diff.differences.is_empty());
    }
}
