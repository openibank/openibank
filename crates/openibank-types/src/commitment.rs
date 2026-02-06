//! Commitment types for OpeniBank
//!
//! Commitments are the accountability primitive - they represent explicit
//! promises with audit trails. No consequence without commitment.

use crate::{
    AgentId, Amount, AuditEntryId, CommitmentId, PermitId, ResonatorId,
    TemporalAnchor, TransactionId, WalletId,
};
use serde::{Deserialize, Serialize};

/// The action being committed to
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommittedAction {
    /// Transfer funds
    Transfer {
        from: WalletId,
        to: WalletId,
        amount: Amount,
    },
    /// Create escrow
    CreateEscrow {
        payer: WalletId,
        payee: WalletId,
        amount: Amount,
    },
    /// Release escrow
    ReleaseEscrow {
        escrow_id: crate::EscrowId,
        to: WalletId,
    },
    /// Refund escrow
    RefundEscrow {
        escrow_id: crate::EscrowId,
        to: WalletId,
    },
    /// Issue currency (IUSD)
    IssueCurrency {
        currency: crate::Currency,
        amount: Amount,
        to: WalletId,
    },
    /// Burn currency
    BurnCurrency {
        currency: crate::Currency,
        amount: Amount,
        from: WalletId,
    },
    /// Grant permit
    GrantPermit {
        wallet: WalletId,
        to: AgentId,
        max_amount: Amount,
    },
    /// Revoke permit
    RevokePermit {
        permit_id: PermitId,
    },
    /// Execute trade
    ExecuteTrade {
        sell: Amount,
        buy: Amount,
        from: WalletId,
    },
    /// Custom action
    Custom {
        action_type: String,
        parameters: serde_json::Value,
    },
}

impl CommittedAction {
    /// Get the effect domain of this action
    pub fn effect_domain(&self) -> EffectDomain {
        match self {
            Self::Transfer { .. } => EffectDomain::ValueMovement,
            Self::CreateEscrow { .. } | Self::ReleaseEscrow { .. } | Self::RefundEscrow { .. } => {
                EffectDomain::Escrow
            }
            Self::IssueCurrency { .. } | Self::BurnCurrency { .. } => EffectDomain::Issuance,
            Self::GrantPermit { .. } | Self::RevokePermit { .. } => EffectDomain::Authorization,
            Self::ExecuteTrade { .. } => EffectDomain::Trading,
            Self::Custom { .. } => EffectDomain::Custom,
        }
    }

    /// Get the primary amount involved in this action (if any)
    pub fn primary_amount(&self) -> Option<&Amount> {
        match self {
            Self::Transfer { amount, .. }
            | Self::CreateEscrow { amount, .. }
            | Self::IssueCurrency { amount, .. }
            | Self::BurnCurrency { amount, .. }
            | Self::GrantPermit { max_amount: amount, .. } => Some(amount),
            Self::ExecuteTrade { sell, .. } => Some(sell),
            _ => None,
        }
    }
}

/// Domain of effect for a commitment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectDomain {
    /// Value movement (transfers)
    ValueMovement,
    /// Escrow operations
    Escrow,
    /// Currency issuance/burn
    Issuance,
    /// Authorization (permits)
    Authorization,
    /// Trading
    Trading,
    /// Custom domain
    Custom,
}

/// Result of policy check before commitment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyCheckResult {
    /// Whether the policy check passed
    pub passed: bool,
    /// Policies that were checked
    pub policies_checked: Vec<String>,
    /// Any warnings (non-blocking)
    pub warnings: Vec<String>,
    /// Reason for failure (if any)
    pub failure_reason: Option<String>,
    /// Hash of the policy state at check time
    pub policy_state_hash: String,
}

impl PolicyCheckResult {
    /// Create a passing result
    pub fn pass(policies: Vec<String>) -> Self {
        Self {
            passed: true,
            policies_checked: policies,
            warnings: vec![],
            failure_reason: None,
            policy_state_hash: String::new(),
        }
    }

    /// Create a failing result
    pub fn fail(reason: String, policies: Vec<String>) -> Self {
        Self {
            passed: false,
            policies_checked: policies,
            warnings: vec![],
            failure_reason: Some(reason),
            policy_state_hash: String::new(),
        }
    }
}

/// A Commitment in OpeniBank
///
/// Commitments are created after intent stabilization and before any
/// consequential action. They provide:
/// - Explicit record of what was promised
/// - Policy check results at commitment time
/// - Audit trail linkage
/// - Cryptographic accountability
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    /// Unique commitment ID
    pub id: CommitmentId,
    /// Resonator that made this commitment
    pub resonator: ResonatorId,
    /// The action being committed to
    pub action: CommittedAction,
    /// Effect domain
    pub effect_domain: EffectDomain,
    /// Permit authorizing this action
    pub permit: PermitId,
    /// Result of policy check
    pub policy_check: PolicyCheckResult,
    /// Linked audit entry
    pub audit_entry: AuditEntryId,
    /// Hash of the intent that led to this commitment
    pub intent_hash: String,
    /// When the commitment was made
    pub created_at: TemporalAnchor,
    /// Cryptographic signature
    pub signature: String,
    /// Status of the commitment
    pub status: CommitmentStatus,
}

/// Status of a commitment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommitmentStatus {
    /// Commitment is pending execution
    Pending,
    /// Commitment is being executed
    Executing,
    /// Commitment was fulfilled
    Fulfilled,
    /// Commitment failed
    Failed,
    /// Commitment was cancelled
    Cancelled,
    /// Commitment expired before execution
    Expired,
}

impl CommitmentStatus {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Fulfilled | Self::Failed | Self::Cancelled | Self::Expired
        )
    }
}

/// Consequence of a fulfilled commitment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consequence {
    /// The commitment that produced this consequence
    pub commitment_id: CommitmentId,
    /// The outcome
    pub outcome: ConsequenceOutcome,
    /// Transactions produced
    pub transactions: Vec<TransactionId>,
    /// Ledger entries produced
    pub ledger_entries: Vec<crate::JournalEntryId>,
    /// When the consequence was produced
    pub produced_at: TemporalAnchor,
}

/// Outcome of a commitment execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsequenceOutcome {
    /// Successfully completed
    Success {
        /// Brief description
        summary: String,
    },
    /// Partially completed
    Partial {
        /// What was completed
        completed: String,
        /// What remains
        remaining: String,
    },
    /// Failed with error
    Failed {
        /// Error message
        error: String,
        /// Error code
        code: String,
    },
}

impl ConsequenceOutcome {
    /// Check if the outcome was successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// Commitment receipt - proof that a commitment was made
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentReceipt {
    /// The commitment ID
    pub commitment_id: CommitmentId,
    /// Hash of the commitment
    pub commitment_hash: String,
    /// Resonator that made the commitment
    pub resonator: ResonatorId,
    /// Action summary
    pub action_summary: String,
    /// Amount involved (if any)
    pub amount: Option<Amount>,
    /// When committed
    pub committed_at: TemporalAnchor,
    /// Signature
    pub signature: String,
}

impl CommitmentReceipt {
    /// Create a receipt from a commitment
    pub fn from_commitment(commitment: &Commitment) -> Self {
        Self {
            commitment_id: commitment.id.clone(),
            commitment_hash: String::new(), // Would be computed
            resonator: commitment.resonator.clone(),
            action_summary: format!("{:?}", commitment.action.effect_domain()),
            amount: commitment.action.primary_amount().cloned(),
            committed_at: commitment.created_at,
            signature: commitment.signature.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_committed_action_domain() {
        let transfer = CommittedAction::Transfer {
            from: WalletId::new(),
            to: WalletId::new(),
            amount: Amount::iusd(100.0),
        };
        assert_eq!(transfer.effect_domain(), EffectDomain::ValueMovement);

        let escrow = CommittedAction::CreateEscrow {
            payer: WalletId::new(),
            payee: WalletId::new(),
            amount: Amount::iusd(100.0),
        };
        assert_eq!(escrow.effect_domain(), EffectDomain::Escrow);
    }

    #[test]
    fn test_policy_check_result() {
        let pass = PolicyCheckResult::pass(vec!["spending_limit".to_string()]);
        assert!(pass.passed);
        assert!(pass.failure_reason.is_none());

        let fail = PolicyCheckResult::fail(
            "Exceeded daily limit".to_string(),
            vec!["daily_limit".to_string()],
        );
        assert!(!fail.passed);
        assert!(fail.failure_reason.is_some());
    }

    #[test]
    fn test_commitment_status() {
        assert!(!CommitmentStatus::Pending.is_terminal());
        assert!(CommitmentStatus::Fulfilled.is_terminal());
        assert!(CommitmentStatus::Failed.is_terminal());
    }

    #[test]
    fn test_consequence_outcome() {
        let success = ConsequenceOutcome::Success {
            summary: "Transfer complete".to_string(),
        };
        assert!(success.is_success());

        let failed = ConsequenceOutcome::Failed {
            error: "Insufficient funds".to_string(),
            code: "INSUFFICIENT_FUNDS".to_string(),
        };
        assert!(!failed.is_success());
    }
}
