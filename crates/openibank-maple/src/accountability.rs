//! IBankAccountability - AAS integration for OpeniBank
//!
//! Wraps the Maple Agent Accountability Service (AAS) to provide
//! identity registration, capability grants, trade commitment evaluation,
//! and outcome recording for every OpeniBank financial operation.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use aas_service::{AasService, AasError};
use aas_identity::{RegistrationRequest, RegisteredAgent, AgentType, AgentMetadata};
use aas_capability::{GrantRequest, CapabilityGrant, CapabilityCheckResult};
use aas_types::{AgentId, CommitmentOutcome, LedgerEntry, PolicyDecisionCard};
use aas_ledger::{LedgerQuery, LedgerStatistics};
use rcf_types::{EffectDomain, ScopeConstraint, TemporalValidity, IdentityRef, ResourceLimits};
use rcf_commitment::{CommitmentBuilder, CommitmentId, RcfCommitment};

use crate::bridge::ResonatorAgentRole;

// ============================================================================
// IBankAccountability
// ============================================================================

/// The OpeniBank accountability service
///
/// Wraps Maple's AAS to provide:
/// - Agent identity registration with role-based metadata
/// - Role-based capability grants (trade.buy, trade.sell, etc.)
/// - Trade commitment submission through AAS pipeline
/// - Trade outcome recording for accountability tracking
pub struct IBankAccountability {
    /// The underlying Maple AAS
    aas: AasService,
}

impl IBankAccountability {
    /// Create a new accountability service
    pub fn new() -> Self {
        Self {
            aas: AasService::new(),
        }
    }

    /// Get a reference to the underlying AAS
    pub fn aas(&self) -> &AasService {
        &self.aas
    }

    // ========================================================================
    // Identity Operations
    // ========================================================================

    /// Register an OpeniBank agent's identity with AAS
    ///
    /// Creates a Resonator identity with the agent's name, role, and system metadata.
    pub fn register_agent_identity(
        &self,
        name: &str,
        role: &ResonatorAgentRole,
    ) -> Result<RegisteredAgent, AasError> {
        let request = RegistrationRequest {
            agent_type: AgentType::Resonator,
            metadata: AgentMetadata {
                name: Some(name.to_string()),
                description: Some(format!("OpeniBank {} agent", role.display_name())),
                owner: Some("openibank".to_string()),
                tags: vec![
                    format!("role:{}", role.display_name().to_lowercase()),
                    "openibank".to_string(),
                    "ibank".to_string(),
                ],
                custom: HashMap::from([
                    ("role".to_string(), role.display_name().to_string()),
                    ("system".to_string(), "openibank".to_string()),
                ]),
            },
        };

        self.aas.register_agent(request)
    }

    // ========================================================================
    // Capability Operations
    // ========================================================================

    /// Grant role-based capabilities to an agent
    ///
    /// Each role gets specific capabilities in the Finance domain:
    /// - Buyer: trade.buy, commitment.draft
    /// - Seller: trade.sell, service.publish, invoice.issue
    /// - Arbiter: dispute.resolve, escrow.release
    /// - Issuer: currency.mint, currency.burn
    pub fn grant_role_capabilities(
        &self,
        agent_id: &AgentId,
        role: &ResonatorAgentRole,
    ) -> Result<Vec<CapabilityGrant>, AasError> {
        let system_id = AgentId::new("openibank_system");
        let scopes = role_capability_scopes(role);
        let mut grants = Vec::new();

        for scope in scopes {
            let grant = self.aas.grant_capability(GrantRequest {
                grantee: agent_id.clone(),
                domain: EffectDomain::Finance,
                scope,
                validity: TemporalValidity::unbounded(),
                issuer: system_id.clone(),
                conditions: vec![],
            })?;
            grants.push(grant);
        }

        Ok(grants)
    }

    /// Check if an agent can execute a trade
    pub fn check_trade_capability(
        &self,
        agent_id: &AgentId,
    ) -> Result<CapabilityCheckResult, AasError> {
        let scope = ScopeConstraint {
            targets: vec![],
            operations: vec!["trade".to_string()],
            limits: None,
        };
        self.aas.check_capability(agent_id, &EffectDomain::Finance, &scope)
    }

    // ========================================================================
    // Commitment Operations
    // ========================================================================

    /// Create and submit a trade commitment through the AAS pipeline
    ///
    /// The AAS pipeline:
    /// 1. Verify buyer identity
    /// 2. Check buyer has trade.buy capability
    /// 3. Evaluate against policies (risk, limits, etc.)
    /// 4. Adjudicate (approve/deny)
    /// 5. Record in accountability ledger
    pub fn submit_trade_commitment(
        &self,
        buyer_identity: &IdentityRef,
        seller_name: &str,
        amount: u64,
        service_name: &str,
    ) -> Result<PolicyDecisionCard, AasError> {
        let commitment = build_trade_commitment(
            buyer_identity,
            seller_name,
            amount,
            service_name,
        )?;

        self.aas.submit_commitment(commitment)
    }

    /// Record that trade execution has started
    pub fn record_trade_started(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<(), AasError> {
        self.aas.record_execution_started(commitment_id)
    }

    /// Record the trade outcome (success or failure)
    pub fn record_trade_outcome(
        &self,
        commitment_id: &CommitmentId,
        success: bool,
        details: &str,
    ) -> Result<(), AasError> {
        let outcome = CommitmentOutcome {
            success,
            description: details.to_string(),
            completed_at: chrono::Utc::now(),
        };
        self.aas.record_outcome(commitment_id, outcome)
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get an agent's accountability history
    pub fn agent_history(
        &self,
        agent_id: &AgentId,
    ) -> Result<Vec<LedgerEntry>, AasError> {
        self.aas.get_agent_history(agent_id)
    }

    /// Get a specific commitment entry
    pub fn get_commitment(
        &self,
        commitment_id: &CommitmentId,
    ) -> Result<Option<LedgerEntry>, AasError> {
        self.aas.get_commitment(commitment_id)
    }

    /// Query the accountability ledger
    pub fn query_ledger(
        &self,
        query: LedgerQuery,
    ) -> Result<Vec<LedgerEntry>, AasError> {
        self.aas.query_ledger(query)
    }

    /// Get overall accountability statistics
    pub fn statistics(&self) -> Result<LedgerStatistics, AasError> {
        self.aas.statistics()
    }

    /// Get items pending human review
    pub fn pending_reviews(&self) -> Result<Vec<CommitmentId>, AasError> {
        self.aas.get_pending_reviews()
    }
}

impl Default for IBankAccountability {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get capability scopes for a given role
fn role_capability_scopes(role: &ResonatorAgentRole) -> Vec<ScopeConstraint> {
    match role {
        ResonatorAgentRole::Buyer => vec![
            ScopeConstraint {
                targets: vec!["*".to_string()],
                operations: vec!["trade.buy".to_string(), "commitment.draft".to_string()],
                limits: None,
            },
        ],
        ResonatorAgentRole::Seller => vec![
            ScopeConstraint {
                targets: vec!["*".to_string()],
                operations: vec![
                    "trade.sell".to_string(),
                    "service.publish".to_string(),
                    "invoice.issue".to_string(),
                ],
                limits: None,
            },
        ],
        ResonatorAgentRole::Arbiter => vec![
            ScopeConstraint {
                targets: vec!["*".to_string()],
                operations: vec![
                    "dispute.resolve".to_string(),
                    "escrow.release".to_string(),
                ],
                limits: None,
            },
        ],
        ResonatorAgentRole::Issuer => vec![
            ScopeConstraint {
                targets: vec!["*".to_string()],
                operations: vec![
                    "currency.mint".to_string(),
                    "currency.burn".to_string(),
                ],
                limits: None,
            },
        ],
    }
}

/// Build an RcfCommitment for a trade
fn build_trade_commitment(
    buyer_identity: &IdentityRef,
    seller_name: &str,
    amount: u64,
    service_name: &str,
) -> Result<RcfCommitment, AasError> {
    use rcf_commitment::{
        IntendedOutcome, Target, TargetType,
        Reversibility, EvidenceRequirements, AuditLevel,
    };

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
        .with_criteria("Service delivered and verified by arbiter")
    )
    .with_scope(ScopeConstraint {
        targets: vec![seller_name.to_string()],
        operations: vec!["trade.buy".to_string()],
        limits: Some(ResourceLimits {
            max_value: Some(amount),
            max_operations: Some(1),
            max_duration_secs: Some(3600), // 1 hour max
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
        "Escrow refund available before delivery confirmation".to_string(),
    ))
    .with_evidence(EvidenceRequirements {
        audit_level: AuditLevel::Standard,
    })
    .with_policy_tag("ibank_trade")
    .with_policy_tag("escrow_required")
    .build()
    .map_err(|e| AasError::CapabilityDenied(format!("Failed to build commitment: {:?}", e)))?;

    Ok(commitment)
}

// ============================================================================
// API Response Types
// ============================================================================

/// Accountability info for dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountabilityInfo {
    pub total_commitments: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub pending_reviews: usize,
    pub by_status: HashMap<String, usize>,
}

impl IBankAccountability {
    /// Get accountability info for dashboard
    pub fn dashboard_info(&self) -> AccountabilityInfo {
        let stats = self.statistics().unwrap_or(LedgerStatistics {
            total_commitments: 0,
            by_status: HashMap::new(),
            successful_executions: 0,
            failed_executions: 0,
        });
        let pending = self.pending_reviews().map(|p| p.len()).unwrap_or(0);

        AccountabilityInfo {
            total_commitments: stats.total_commitments,
            successful_executions: stats.successful_executions,
            failed_executions: stats.failed_executions,
            pending_reviews: pending,
            by_status: stats.by_status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_grant() {
        let accountability = IBankAccountability::new();

        let agent = accountability
            .register_agent_identity("Alice", &ResonatorAgentRole::Buyer)
            .expect("Failed to register agent");

        assert_eq!(agent.metadata.name, Some("Alice".to_string()));

        let grants = accountability
            .grant_role_capabilities(&agent.agent_id, &ResonatorAgentRole::Buyer)
            .expect("Failed to grant capabilities");

        assert!(!grants.is_empty());
    }

    #[test]
    fn test_full_trade_commitment_flow() {
        let accountability = IBankAccountability::new();

        // Register buyer
        let buyer = accountability
            .register_agent_identity("Alice", &ResonatorAgentRole::Buyer)
            .expect("register buyer");

        // Grant capabilities
        accountability
            .grant_role_capabilities(&buyer.agent_id, &ResonatorAgentRole::Buyer)
            .expect("grant capabilities");

        // Submit trade commitment
        let decision = accountability
            .submit_trade_commitment(
                &buyer.identity_ref,
                "DataCorp",
                10000, // $100.00
                "Data Analysis",
            )
            .expect("submit commitment");

        // Finance domain defaults to PendingHumanReview in Maple's default policies.
        // This is correct behavior — financial commitments require human approval.
        // In production, the ibank runtime would auto-approve via configured policies.
        let commitment_id = decision.commitment_id.clone();

        // The commitment is now recorded in the ledger regardless of decision
        let entry = accountability
            .get_commitment(&commitment_id)
            .expect("get commitment");
        assert!(entry.is_some(), "Commitment should be in ledger");
    }

    #[test]
    fn test_dashboard_info() {
        let accountability = IBankAccountability::new();
        let info = accountability.dashboard_info();
        assert_eq!(info.total_commitments, 0);
    }
}
