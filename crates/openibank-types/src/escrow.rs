//! Escrow types for OpeniBank
//!
//! Escrow is the default for all value movements - funds never move directly
//! to counterparties. This provides safety, auditability, and dispute resolution.

use crate::{
    AgentId, Amount, CommitmentId, EscrowId, ResonatorId, TemporalAnchor, WalletId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Condition that must be met to release escrow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseCondition {
    /// Type of condition
    pub condition_type: ReleaseConditionType,
    /// Whether this condition has been met
    pub met: bool,
    /// When the condition was met (if applicable)
    pub met_at: Option<TemporalAnchor>,
    /// Evidence for the condition being met
    pub evidence: Option<String>,
}

impl ReleaseCondition {
    /// Create a new condition
    pub fn new(condition_type: ReleaseConditionType) -> Self {
        Self {
            condition_type,
            met: false,
            met_at: None,
            evidence: None,
        }
    }

    /// Mark the condition as met
    pub fn mark_met(&mut self, evidence: Option<String>) {
        self.met = true;
        self.met_at = Some(TemporalAnchor::now());
        self.evidence = evidence;
    }
}

/// Types of release conditions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseConditionType {
    /// Time-based: release after a specific time
    TimeAfter { time: DateTime<Utc> },
    /// Confirmation: requires explicit confirmation from a party
    Confirmation { from: AgentId },
    /// Multi-sig: requires N of M confirmations
    MultiSig { required: u32, signers: Vec<AgentId> },
    /// Delivery proof: requires proof of delivery
    DeliveryProof { expected_hash: String },
    /// Oracle: requires external oracle attestation
    Oracle { oracle_id: String, expected_value: String },
    /// Arbitration: requires arbiter decision
    Arbitration { arbiter: AgentId },
    /// Custom condition
    Custom { name: String, parameters: serde_json::Value },
}

/// State of an escrow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EscrowState {
    /// Funds are locked in escrow
    Locked,
    /// Conditions are being evaluated
    ConditionsPending,
    /// All conditions met, ready for release
    ReadyToRelease,
    /// Funds released to payee
    Released,
    /// Funds refunded to payer
    Refunded,
    /// Partially released
    PartialRelease,
    /// In dispute, awaiting arbitration
    Disputed,
    /// Escrow expired
    Expired,
    /// Escrow cancelled before lock
    Cancelled,
}

impl EscrowState {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Released | Self::Refunded | Self::Expired | Self::Cancelled
        )
    }

    /// Check if funds are still locked
    pub fn is_locked(&self) -> bool {
        matches!(
            self,
            Self::Locked | Self::ConditionsPending | Self::ReadyToRelease | Self::Disputed
        )
    }
}

/// An Escrow in OpeniBank
///
/// Escrows hold funds until release conditions are met. They are the
/// default mechanism for all value movements between parties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escrow {
    /// Unique escrow ID
    pub id: EscrowId,
    /// Wallet that funded the escrow
    pub payer: WalletId,
    /// Wallet that will receive on release
    pub payee: WalletId,
    /// Amount held in escrow
    pub amount: Amount,
    /// Current state
    pub state: EscrowState,
    /// Conditions for release
    pub release_conditions: Vec<ReleaseCondition>,
    /// Optional arbiter for disputes
    pub arbiter: Option<AgentId>,
    /// Commitment that created this escrow
    pub commitment_id: CommitmentId,
    /// Description/purpose
    pub description: String,
    /// When the escrow was created
    pub created_at: TemporalAnchor,
    /// When the escrow expires
    pub expires_at: DateTime<Utc>,
    /// When the escrow was last updated
    pub updated_at: TemporalAnchor,
}

impl Escrow {
    /// Check if all release conditions are met
    pub fn all_conditions_met(&self) -> bool {
        self.release_conditions.iter().all(|c| c.met)
    }

    /// Get the number of met conditions
    pub fn met_conditions_count(&self) -> (usize, usize) {
        let met = self.release_conditions.iter().filter(|c| c.met).count();
        (met, self.release_conditions.len())
    }

    /// Check if the escrow has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the escrow can be released
    pub fn can_release(&self) -> bool {
        !self.state.is_terminal() && self.all_conditions_met() && !self.is_expired()
    }

    /// Check if the escrow can be refunded
    pub fn can_refund(&self) -> bool {
        matches!(self.state, EscrowState::Locked | EscrowState::Expired)
    }
}

/// Request to create an escrow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateEscrowRequest {
    /// Payer wallet
    pub payer: WalletId,
    /// Payee wallet
    pub payee: WalletId,
    /// Amount to escrow
    pub amount: Amount,
    /// Release conditions
    pub release_conditions: Vec<ReleaseConditionType>,
    /// Optional arbiter
    pub arbiter: Option<AgentId>,
    /// Description
    pub description: String,
    /// Expiration time
    pub expires_at: DateTime<Utc>,
}

/// Result of an escrow action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowActionResult {
    /// Funds released to payee
    Released {
        to: WalletId,
        amount: Amount,
        transaction_id: crate::TransactionId,
    },
    /// Funds refunded to payer
    Refunded {
        to: WalletId,
        amount: Amount,
        transaction_id: crate::TransactionId,
    },
    /// Partial release
    PartialRelease {
        released: Amount,
        remaining: Amount,
    },
    /// State updated (e.g., condition met)
    StateUpdated {
        new_state: EscrowState,
    },
    /// Dispute opened
    DisputeOpened {
        arbiter: AgentId,
    },
}

/// Dispute information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowDispute {
    /// Escrow being disputed
    pub escrow_id: EscrowId,
    /// Who opened the dispute
    pub opened_by: AgentId,
    /// Reason for dispute
    pub reason: String,
    /// Evidence provided
    pub evidence: Vec<DisputeEvidence>,
    /// Arbiter assigned
    pub arbiter: AgentId,
    /// When opened
    pub opened_at: TemporalAnchor,
    /// Resolution (if resolved)
    pub resolution: Option<DisputeResolution>,
}

/// Evidence for a dispute
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeEvidence {
    /// Who submitted this evidence
    pub submitted_by: AgentId,
    /// Type of evidence
    pub evidence_type: String,
    /// Content or reference
    pub content: String,
    /// When submitted
    pub submitted_at: TemporalAnchor,
}

/// Resolution of a dispute
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeResolution {
    /// Decision
    pub decision: DisputeDecision,
    /// Reasoning
    pub reasoning: String,
    /// Decided by
    pub decided_by: AgentId,
    /// When resolved
    pub resolved_at: TemporalAnchor,
}

/// Decision in a dispute
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeDecision {
    /// Release to payee
    ReleaseToPayee,
    /// Refund to payer
    RefundToPayer,
    /// Split between parties
    Split { payer_percent: u8 },
    /// Escalate to higher authority
    Escalate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_release_condition() {
        let mut condition = ReleaseCondition::new(ReleaseConditionType::Confirmation {
            from: AgentId::new(),
        });

        assert!(!condition.met);
        condition.mark_met(Some("User confirmed".to_string()));
        assert!(condition.met);
        assert!(condition.evidence.is_some());
    }

    #[test]
    fn test_escrow_state() {
        assert!(!EscrowState::Locked.is_terminal());
        assert!(EscrowState::Released.is_terminal());
        assert!(EscrowState::Locked.is_locked());
        assert!(!EscrowState::Released.is_locked());
    }

    #[test]
    fn test_escrow_conditions() {
        let escrow = Escrow {
            id: EscrowId::new(),
            payer: WalletId::new(),
            payee: WalletId::new(),
            amount: Amount::iusd(100.0),
            state: EscrowState::Locked,
            release_conditions: vec![
                ReleaseCondition {
                    condition_type: ReleaseConditionType::TimeAfter {
                        time: Utc::now() - Duration::hours(1),
                    },
                    met: true,
                    met_at: Some(TemporalAnchor::now()),
                    evidence: None,
                },
                ReleaseCondition {
                    condition_type: ReleaseConditionType::Confirmation {
                        from: AgentId::new(),
                    },
                    met: false,
                    met_at: None,
                    evidence: None,
                },
            ],
            arbiter: None,
            commitment_id: CommitmentId::new(),
            description: "Test escrow".to_string(),
            created_at: TemporalAnchor::now(),
            expires_at: Utc::now() + Duration::days(7),
            updated_at: TemporalAnchor::now(),
        };

        let (met, total) = escrow.met_conditions_count();
        assert_eq!(met, 1);
        assert_eq!(total, 2);
        assert!(!escrow.all_conditions_met());
        assert!(!escrow.can_release());
    }
}
