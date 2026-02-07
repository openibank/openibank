//! OpeniBank Escrow - Commitment-gated escrow management
//!
//! Escrow is the default for all value movements in OpeniBank.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use openibank_types::{
    CreateEscrowRequest, Escrow, EscrowActionResult, EscrowDispute, EscrowId, EscrowState,
    ReleaseCondition, ReleaseConditionType, DisputeDecision, DisputeEvidence, DisputeResolution,
};

use openibank_types::{
    AgentId, CommitmentId, OpeniBankError, Result, TemporalAnchor, TransactionId, WalletId,
};

#[derive(Debug, Clone)]
pub struct ConditionResult {
    pub condition_id: usize,
    pub satisfied: bool,
    pub reason: String,
    pub evaluated_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait ConditionOracle: Send + Sync {
    async fn check_oracle_condition(&self, oracle_id: &str, expected_value: &str) -> Result<(bool, String)>;
}

pub struct InMemoryOracle {
    responses: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryOracle {
    pub fn new() -> Self {
        Self { responses: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn set_response(&self, oracle_id: &str, value: String) {
        self.responses.write().await.insert(oracle_id.to_string(), value);
    }
}

impl Default for InMemoryOracle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ConditionOracle for InMemoryOracle {
    async fn check_oracle_condition(&self, oracle_id: &str, expected_value: &str) -> Result<(bool, String)> {
        let responses = self.responses.read().await;
        let actual = responses.get(oracle_id);
        let satisfied = actual.map(|v| v == expected_value).unwrap_or(false);
        Ok((satisfied, if satisfied { "Oracle matched".to_string() } else { "Oracle mismatch".to_string() }))
    }
}

pub struct EscrowManager {
    escrows: Arc<RwLock<HashMap<EscrowId, Escrow>>>,
    oracle: Arc<dyn ConditionOracle>,
    confirmations: Arc<RwLock<HashMap<(EscrowId, AgentId), DateTime<Utc>>>>,
}

impl EscrowManager {
    pub fn new(oracle: Arc<dyn ConditionOracle>) -> Self {
        Self {
            escrows: Arc::new(RwLock::new(HashMap::new())),
            oracle,
            confirmations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_in_memory_oracle() -> Self {
        Self::new(Arc::new(InMemoryOracle::new()))
    }

    pub async fn create(&self, request: CreateEscrowRequest) -> Result<Escrow> {
        let now = TemporalAnchor::now();
        let release_conditions: Vec<ReleaseCondition> = request
            .release_conditions
            .into_iter()
            .map(ReleaseCondition::new)
            .collect();

        let escrow = Escrow {
            id: EscrowId::new(),
            payer: request.payer,
            payee: request.payee,
            amount: request.amount,
            state: EscrowState::Locked,
            release_conditions,
            arbiter: request.arbiter,
            commitment_id: CommitmentId::new(),
            description: request.description,
            created_at: now,
            expires_at: request.expires_at,
            updated_at: now,
        };

        self.escrows.write().await.insert(escrow.id.clone(), escrow.clone());
        Ok(escrow)
    }

    pub async fn get(&self, escrow_id: &EscrowId) -> Result<Escrow> {
        self.escrows.read().await.get(escrow_id).cloned().ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })
    }

    pub async fn record_confirmation(&self, escrow_id: &EscrowId, from: &AgentId) -> Result<EscrowActionResult> {
        let mut escrows = self.escrows.write().await;
        let escrow = escrows.get_mut(escrow_id).ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })?;

        if escrow.state.is_terminal() {
            return Err(OpeniBankError::EscrowAlreadyReleased {
                escrow_id: escrow_id.to_string(),
            });
        }

        for condition in &mut escrow.release_conditions {
            if let ReleaseConditionType::Confirmation { from: expected } = &condition.condition_type {
                if expected == from && !condition.met {
                    condition.mark_met(Some(format!("Confirmed by {}", from)));
                }
            }
        }

        self.confirmations.write().await.insert((escrow_id.clone(), from.clone()), Utc::now());
        escrow.updated_at = TemporalAnchor::now();

        if escrow.all_conditions_met() {
            escrow.state = EscrowState::ReadyToRelease;
        } else {
            escrow.state = EscrowState::ConditionsPending;
        }

        Ok(EscrowActionResult::StateUpdated { new_state: escrow.state })
    }

    pub async fn check_time_conditions(&self, escrow_id: &EscrowId) -> Result<Vec<ConditionResult>> {
        let mut escrows = self.escrows.write().await;
        let escrow = escrows.get_mut(escrow_id).ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })?;

        let now = Utc::now();
        let mut results = Vec::new();

        for (idx, condition) in escrow.release_conditions.iter_mut().enumerate() {
            if let ReleaseConditionType::TimeAfter { time } = &condition.condition_type {
                let time_val = *time;
                let satisfied = now >= time_val;
                if satisfied && !condition.met {
                    condition.mark_met(Some(format!("Time reached: {}", now)));
                }
                results.push(ConditionResult {
                    condition_id: idx,
                    satisfied,
                    reason: if satisfied { format!("Time {} passed", time_val) } else { format!("Before {}", time_val) },
                    evaluated_at: now,
                });
            }
        }

        escrow.updated_at = TemporalAnchor::now();
        if escrow.all_conditions_met() {
            escrow.state = EscrowState::ReadyToRelease;
        }

        Ok(results)
    }

    pub async fn release(&self, escrow_id: &EscrowId) -> Result<EscrowActionResult> {
        let mut escrows = self.escrows.write().await;
        let escrow = escrows.get_mut(escrow_id).ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })?;

        if escrow.state.is_terminal() {
            return Err(OpeniBankError::EscrowAlreadyReleased {
                escrow_id: escrow_id.to_string(),
            });
        }

        if !escrow.all_conditions_met() {
            let (met, total) = escrow.met_conditions_count();
            return Err(OpeniBankError::EscrowConditionsNotMet {
                escrow_id: escrow_id.to_string(),
                remaining: total - met,
                total,
            });
        }

        if escrow.is_expired() {
            return Err(OpeniBankError::EscrowAlreadyReleased {
                escrow_id: escrow_id.to_string(),
            });
        }

        let amount = escrow.amount;
        let payee = escrow.payee.clone();
        escrow.state = EscrowState::Released;
        escrow.updated_at = TemporalAnchor::now();

        Ok(EscrowActionResult::Released {
            to: payee,
            amount,
            transaction_id: TransactionId::new(),
        })
    }

    pub async fn refund(&self, escrow_id: &EscrowId) -> Result<EscrowActionResult> {
        let mut escrows = self.escrows.write().await;
        let escrow = escrows.get_mut(escrow_id).ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })?;

        if escrow.state.is_terminal() {
            return Err(OpeniBankError::EscrowAlreadyReleased {
                escrow_id: escrow_id.to_string(),
            });
        }

        if !escrow.can_refund() {
            return Err(OpeniBankError::EscrowAlreadyReleased {
                escrow_id: escrow_id.to_string(),
            });
        }

        let amount = escrow.amount;
        let payer = escrow.payer.clone();
        escrow.state = EscrowState::Refunded;
        escrow.updated_at = TemporalAnchor::now();

        Ok(EscrowActionResult::Refunded {
            to: payer,
            amount,
            transaction_id: TransactionId::new(),
        })
    }

    pub async fn open_dispute(&self, escrow_id: &EscrowId, _opened_by: AgentId, _reason: String) -> Result<EscrowActionResult> {
        let mut escrows = self.escrows.write().await;
        let escrow = escrows.get_mut(escrow_id).ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })?;

        if escrow.state.is_terminal() {
            return Err(OpeniBankError::EscrowAlreadyReleased {
                escrow_id: escrow_id.to_string(),
            });
        }

        let arbiter = escrow.arbiter.clone().ok_or_else(|| OpeniBankError::EscrowNotFound {
            escrow_id: escrow_id.to_string(),
        })?;

        escrow.state = EscrowState::Disputed;
        escrow.updated_at = TemporalAnchor::now();

        Ok(EscrowActionResult::DisputeOpened { arbiter })
    }

    pub async fn list_by_state(&self, state: EscrowState) -> Vec<Escrow> {
        self.escrows.read().await.values().filter(|e| e.state == state).cloned().collect()
    }

    pub async fn list_by_payer(&self, payer: &WalletId) -> Vec<Escrow> {
        self.escrows.read().await.values().filter(|e| &e.payer == payer).cloned().collect()
    }

    pub async fn list_by_payee(&self, payee: &WalletId) -> Vec<Escrow> {
        self.escrows.read().await.values().filter(|e| &e.payee == payee).cloned().collect()
    }

    pub async fn get_expired_escrows(&self) -> Vec<Escrow> {
        let now = Utc::now();
        self.escrows.read().await.values().filter(|e| !e.state.is_terminal() && e.expires_at <= now).cloned().collect()
    }

    pub async fn process_expired(&self) -> Result<Vec<EscrowId>> {
        let expired = self.get_expired_escrows().await;
        let mut processed = Vec::new();

        for escrow in expired {
            let mut escrows = self.escrows.write().await;
            if let Some(e) = escrows.get_mut(&escrow.id) {
                if e.state == EscrowState::Disputed {
                    continue;
                }
                e.state = EscrowState::Expired;
                e.updated_at = TemporalAnchor::now();
                processed.push(escrow.id.clone());
            }
        }

        Ok(processed)
    }
}
