//! TradeCommitmentManager - Commitment lifecycle for trades
//!
//! Every OpeniBank trade becomes a Maple RcfCommitment that goes through
//! the full accountability pipeline:
//!
//! ```text
//! Create Commitment → Submit to AAS → Policy Evaluation → Adjudication
//!     → Record in Ledger → Execute Trade → Record Outcome
//! ```

use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};

use aas_types::PolicyDecisionCard;
use rcf_types::{IdentityRef, EffectDomain, ScopeConstraint, ResourceLimits};
use rcf_commitment::{
    CommitmentBuilder, CommitmentId, RcfCommitment,
    IntendedOutcome, Target, TargetType,
    Reversibility, EvidenceRequirements, AuditLevel,
};

use crate::accountability::IBankAccountability;

// ============================================================================
// Trade Commitment Tracking
// ============================================================================

/// Status of a trade commitment through its lifecycle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeCommitmentStatus {
    /// Commitment created, not yet submitted
    Created,
    /// Submitted to AAS, awaiting decision
    Submitted,
    /// Approved by AAS
    Approved,
    /// Rejected by AAS
    Rejected,
    /// Trade execution started
    Executing,
    /// Trade completed successfully
    Completed,
    /// Trade failed
    Failed,
}

/// Tracking record for a trade commitment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeCommitmentRecord {
    pub commitment_id: String,
    pub buyer_name: String,
    pub seller_name: String,
    pub service_name: String,
    pub amount: u64,
    pub status: TradeCommitmentStatus,
    pub decision: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// TradeCommitmentManager
// ============================================================================

/// Manages the commitment lifecycle for trades
///
/// Creates RcfCommitments for each trade and tracks them through
/// the AAS pipeline from submission to outcome recording.
pub struct TradeCommitmentManager {
    /// Track all trade commitments
    commitments: RwLock<HashMap<String, TradeCommitmentRecord>>,
}

impl TradeCommitmentManager {
    /// Create a new commitment manager
    pub fn new() -> Self {
        Self {
            commitments: RwLock::new(HashMap::new()),
        }
    }

    /// Create a trade commitment (without submitting)
    ///
    /// Builds an RcfCommitment with:
    /// - Finance domain
    /// - Buyer as principal
    /// - Seller as target
    /// - Resource limits based on trade amount
    /// - Standard audit level
    /// - Escrow reversibility
    pub fn create_trade_commitment(
        &self,
        buyer_identity: &IdentityRef,
        buyer_name: &str,
        seller_name: &str,
        amount: u64,
        service_name: &str,
    ) -> Result<RcfCommitment, String> {
        let commitment = CommitmentBuilder::new(
            buyer_identity.clone(),
            EffectDomain::Finance,
        )
        .with_outcome(
            IntendedOutcome::new(format!(
                "Purchase '{}' from {} for ${:.2}",
                service_name, seller_name, amount as f64 / 100.0
            ))
            .with_criteria("Payment transferred via escrow")
            .with_criteria("Service delivered and verified")
        )
        .with_scope(ScopeConstraint {
            targets: vec![seller_name.to_string()],
            operations: vec!["trade.buy".to_string()],
            limits: Some(ResourceLimits {
                max_value: Some(amount),
                max_operations: Some(1),
                max_duration_secs: Some(3600),
                max_data_bytes: None,
                max_concurrent: Some(1),
            }),
        })
        .with_target(Target {
            target_type: TargetType::Identity,
            identifier: seller_name.to_string(),
        })
        .with_limits(ResourceLimits {
            max_value: Some(amount),
            max_operations: Some(1),
            max_duration_secs: Some(3600),
            max_data_bytes: None,
            max_concurrent: Some(1),
        })
        .with_reversibility(Reversibility::PartiallyReversible(
            "Escrow refund before delivery".to_string(),
        ))
        .with_evidence(EvidenceRequirements {
            audit_level: AuditLevel::Standard,
        })
        .with_policy_tag("ibank_trade")
        .with_policy_tag("escrow_required")
        .build()
        .map_err(|e| format!("Failed to build commitment: {:?}", e))?;

        // Track it
        let record = TradeCommitmentRecord {
            commitment_id: commitment.commitment_id.0.clone(),
            buyer_name: buyer_name.to_string(),
            seller_name: seller_name.to_string(),
            service_name: service_name.to_string(),
            amount,
            status: TradeCommitmentStatus::Created,
            decision: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if let Ok(mut commitments) = self.commitments.write() {
            commitments.insert(commitment.commitment_id.0.clone(), record);
        }

        Ok(commitment)
    }

    /// Submit a commitment to the AAS and update tracking
    pub async fn submit_and_track(
        &self,
        accountability: &IBankAccountability,
        commitment: RcfCommitment,
    ) -> Result<PolicyDecisionCard, String> {
        let commitment_id = commitment.commitment_id.0.clone();

        // Update status to Submitted
        self.update_status(&commitment_id, TradeCommitmentStatus::Submitted);

        // Submit to AAS
        let decision = accountability
            .aas()
            .submit_commitment(commitment)
            .await
            .map_err(|e| format!("AAS submission failed: {}", e))?;

        // Update based on decision
        if decision.decision.allows_execution() {
            self.update_status(&commitment_id, TradeCommitmentStatus::Approved);
            self.update_decision(&commitment_id, "Approved");
        } else {
            self.update_status(&commitment_id, TradeCommitmentStatus::Rejected);
            self.update_decision(&commitment_id, &format!("Rejected: {:?}", decision.rationale));
        }

        Ok(decision)
    }

    /// Record that trade execution has started
    pub async fn record_execution_started(
        &self,
        accountability: &IBankAccountability,
        commitment_id: &str,
    ) -> Result<(), String> {
        let cid = CommitmentId::new(commitment_id);
        accountability
            .record_trade_started(&cid)
            .await
            .map_err(|e| format!("Failed to record execution start: {}", e))?;

        self.update_status(commitment_id, TradeCommitmentStatus::Executing);
        Ok(())
    }

    /// Record trade outcome (success or failure)
    pub async fn record_outcome(
        &self,
        accountability: &IBankAccountability,
        commitment_id: &str,
        success: bool,
        details: &str,
    ) -> Result<(), String> {
        let cid = CommitmentId::new(commitment_id);
        accountability
            .record_trade_outcome(&cid, success, details)
            .await
            .map_err(|e| format!("Failed to record outcome: {}", e))?;

        let status = if success {
            TradeCommitmentStatus::Completed
        } else {
            TradeCommitmentStatus::Failed
        };
        self.update_status(commitment_id, status);
        self.update_decision(commitment_id, details);

        Ok(())
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get all commitment records
    pub fn all_commitments(&self) -> Vec<TradeCommitmentRecord> {
        self.commitments
            .read()
            .map(|c| c.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get recent commitments (latest N)
    pub fn recent_commitments(&self, n: usize) -> Vec<TradeCommitmentRecord> {
        let mut all = self.all_commitments();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.truncate(n);
        all
    }

    /// Get commitment by ID
    pub fn get_commitment(&self, commitment_id: &str) -> Option<TradeCommitmentRecord> {
        self.commitments
            .read()
            .ok()
            .and_then(|c| c.get(commitment_id).cloned())
    }

    /// Count commitments by status
    pub fn count_by_status(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        if let Ok(commitments) = self.commitments.read() {
            for record in commitments.values() {
                let status_str = format!("{:?}", record.status);
                *counts.entry(status_str).or_insert(0) += 1;
            }
        }
        counts
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    fn update_status(&self, commitment_id: &str, status: TradeCommitmentStatus) {
        if let Ok(mut commitments) = self.commitments.write() {
            if let Some(record) = commitments.get_mut(commitment_id) {
                record.status = status;
                record.updated_at = chrono::Utc::now();
            }
        }
    }

    fn update_decision(&self, commitment_id: &str, decision: &str) {
        if let Ok(mut commitments) = self.commitments.write() {
            if let Some(record) = commitments.get_mut(commitment_id) {
                record.decision = Some(decision.to_string());
                record.updated_at = chrono::Utc::now();
            }
        }
    }
}

impl Default for TradeCommitmentManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Dashboard Types
// ============================================================================

/// Commitment summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentsSummary {
    pub total: usize,
    pub by_status: HashMap<String, usize>,
    pub recent: Vec<TradeCommitmentRecord>,
}

impl TradeCommitmentManager {
    /// Get commitment summary for dashboard
    pub fn dashboard_summary(&self) -> CommitmentsSummary {
        CommitmentsSummary {
            total: self.all_commitments().len(),
            by_status: self.count_by_status(),
            recent: self.recent_commitments(20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_manager_new() {
        let manager = TradeCommitmentManager::new();
        assert!(manager.all_commitments().is_empty());
    }

    #[test]
    fn test_create_commitment() {
        let manager = TradeCommitmentManager::new();
        let identity = IdentityRef::new("test_buyer");

        let commitment = manager
            .create_trade_commitment(
                &identity,
                "Alice",
                "DataCorp",
                10000,
                "Data Analysis",
            )
            .expect("Failed to create commitment");

        assert!(!commitment.commitment_id.0.is_empty());

        let all = manager.all_commitments();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].buyer_name, "Alice");
        assert_eq!(all[0].seller_name, "DataCorp");
        assert_eq!(all[0].amount, 10000);
        assert_eq!(all[0].status, TradeCommitmentStatus::Created);
    }

    #[test]
    fn test_dashboard_summary() {
        let manager = TradeCommitmentManager::new();
        let summary = manager.dashboard_summary();
        assert_eq!(summary.total, 0);
    }
}
