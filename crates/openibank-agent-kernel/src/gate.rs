//! Commitment + capability + contract gates for AgentKernel

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentContext {
    pub commitment_id: String,
    pub approved: bool,
}

#[derive(Debug, Default, Clone)]
pub struct CommitmentGate {
    active: Option<CommitmentContext>,
}

impl CommitmentGate {
    pub fn new() -> Self {
        Self { active: None }
    }

    pub fn set_active(&mut self, ctx: CommitmentContext) {
        self.active = Some(ctx);
    }

    pub fn clear(&mut self) {
        self.active = None;
    }

    pub fn active(&self) -> Option<&CommitmentContext> {
        self.active.as_ref()
    }

    pub fn require_approved(&self) -> Result<(), GateError> {
        let ctx = self.active.as_ref().ok_or(GateError::CommitmentMissing)?;
        if !ctx.approved {
            return Err(GateError::CommitmentNotApproved {
                commitment_id: ctx.commitment_id.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Unattested,
    Attested,
}

#[derive(Debug, Default, Clone)]
pub struct CapabilitySet {
    caps: HashMap<String, CapabilityStatus>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            caps: HashMap::new(),
        }
    }

    pub fn from_attested(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut set = Self::new();
        for name in names {
            set.caps.insert(name.into(), CapabilityStatus::Attested);
        }
        set
    }

    pub fn attest(&mut self, name: impl Into<String>) {
        self.caps.insert(name.into(), CapabilityStatus::Attested);
    }

    pub fn revoke(&mut self, name: &str) {
        self.caps.insert(name.to_string(), CapabilityStatus::Unattested);
    }

    pub fn is_attested(&self, name: &str) -> bool {
        matches!(self.caps.get(name), Some(CapabilityStatus::Attested))
    }

    pub fn require(&self, name: &str) -> Result<(), GateError> {
        if self.is_attested(name) {
            Ok(())
        } else {
            Err(GateError::CapabilityNotAttested {
                capability: name.to_string(),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub name: String,
    pub max_spend: Option<u64>,
    pub allowed_assets: Vec<String>,
    pub require_reversible: bool,
    pub allowed_outcomes: Vec<String>,
}

impl Contract {
    pub fn allows_asset(&self, asset: &str) -> bool {
        self.allowed_assets.is_empty() || self.allowed_assets.iter().any(|a| a == asset)
    }

    pub fn allows_outcome(&self, outcome: &str) -> bool {
        self.allowed_outcomes.is_empty() || self.allowed_outcomes.iter().any(|o| o == outcome)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ContractSet {
    contracts: Vec<Contract>,
}

impl ContractSet {
    pub fn new(contracts: Vec<Contract>) -> Self {
        Self { contracts }
    }

    pub fn contracts(&self) -> &[Contract] {
        &self.contracts
    }

    pub fn contracts_mut(&mut self) -> &mut Vec<Contract> {
        &mut self.contracts
    }

    pub fn enforce_payment(&self, amount: u64, asset: &str, reversible: bool) -> Result<(), GateError> {
        for contract in &self.contracts {
            if let Some(max) = contract.max_spend {
                if amount > max {
                    return Err(GateError::ContractViolation {
                        contract: contract.name.clone(),
                        reason: format!("amount {} exceeds max_spend {}", amount, max),
                    });
                }
            }

            if !contract.allows_asset(asset) {
                return Err(GateError::ContractViolation {
                    contract: contract.name.clone(),
                    reason: format!("asset {} not allowed", asset),
                });
            }

            if contract.require_reversible && !reversible {
                return Err(GateError::ContractViolation {
                    contract: contract.name.clone(),
                    reason: "reversibility required".to_string(),
                });
            }
        }
        Ok(())
    }

    pub fn enforce_outcome(&self, outcome: &str) -> Result<(), GateError> {
        for contract in &self.contracts {
            if !contract.allows_outcome(outcome) {
                return Err(GateError::ContractViolation {
                    contract: contract.name.clone(),
                    reason: format!("outcome '{}' not allowed", outcome),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateDecision {
    Allowed,
    Blocked { reason: String },
}

#[derive(Error, Debug, Clone)]
pub enum GateError {
    #[error("Missing commitment context")]
    CommitmentMissing,
    #[error("Commitment {commitment_id} not approved")]
    CommitmentNotApproved { commitment_id: String },
    #[error("Capability not attested: {capability}")]
    CapabilityNotAttested { capability: String },
    #[error("Contract violation ({contract}): {reason}")]
    ContractViolation { contract: String, reason: String },
}
