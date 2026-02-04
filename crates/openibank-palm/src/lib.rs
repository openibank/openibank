//! OpeniBank PALM Integration - Fleet Orchestration for Agent Banking
//!
//! This crate bridges PALM (Persistent Agent Lifecycle Manager) with OpeniBank,
//! enabling fleet-level orchestration of financial agent deployments.
//!
//! # Capabilities
//!
//! - **Agent Fleet Management**: Deploy, scale, and monitor fleets of financial agents
//! - **Health Monitoring**: Multi-dimensional health assessment with IBank-specific thresholds
//! - **Service Discovery**: Find and route to financial agent instances by capability
//! - **Resilience**: Circuit breakers and recovery actions for financial operations

use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

use palm_types::{
    AgentSpec, AgentSpecId, AgentInstance, InstanceId, DeploymentId,
    InstanceStatus, HealthStatus, PlatformProfile,
    AutonomyLevel, RiskTolerance, CapabilityRef, ResourceRequirements,
    ResonatorIdRef, StartupPhase,
};
use palm_types::spec::ResonatorProfileConfig;
use palm_registry::{
    AgentRegistry, InstanceRegistry, DiscoveryService,
    InMemoryAgentRegistry, InMemoryInstanceRegistry, InMemoryDiscoveryService,
    DiscoveryQuery, RoutingStrategy,
};
use palm_health::{
    HealthMonitor, HealthConfig, HealthAssessment,
    resilience::{ResilienceController, NoOpRecoveryExecutor},
};

/// Errors from fleet management operations
#[derive(Debug, Error)]
pub enum FleetError {
    #[error("Registry error: {0}")]
    Registry(#[from] palm_registry::RegistryError),
    #[error("Health error: {0}")]
    Health(#[from] palm_health::HealthError),
    #[error("Agent spec not found: {0}")]
    SpecNotFound(String),
    #[error("Instance not found: {0}")]
    InstanceNotFound(String),
    #[error("Deployment failed: {0}")]
    DeploymentFailed(String),
}

pub type FleetResult<T> = Result<T, FleetError>;

/// Financial agent roles for fleet management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinancialAgentType {
    Buyer,
    Seller,
    Arbiter,
    Issuer,
    Auditor,
    Compliance,
}

impl std::fmt::Display for FinancialAgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buyer => write!(f, "buyer"),
            Self::Seller => write!(f, "seller"),
            Self::Arbiter => write!(f, "arbiter"),
            Self::Issuer => write!(f, "issuer"),
            Self::Auditor => write!(f, "auditor"),
            Self::Compliance => write!(f, "compliance"),
        }
    }
}

/// Configuration for the fleet manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    pub default_replicas: u32,
    pub max_instances: u32,
    pub health_check_interval_secs: u64,
    pub auto_scale: bool,
}

impl Default for FleetConfig {
    fn default() -> Self {
        Self {
            default_replicas: 3,
            max_instances: 100,
            health_check_interval_secs: 30,
            auto_scale: false,
        }
    }
}

/// Summary of fleet status for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetStatusSummary {
    pub total_specs: usize,
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub unhealthy_instances: usize,
    pub specs: Vec<SpecInfo>,
    pub instances: Vec<InstanceInfo>,
    pub timestamp: DateTime<Utc>,
}

/// Info about a registered agent spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecInfo {
    pub spec_id: String,
    pub name: String,
    pub version: String,
    pub agent_type: String,
    pub autonomy_level: String,
    pub instance_count: usize,
}

/// Info about a running instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub instance_id: String,
    pub spec_name: String,
    pub status: String,
    pub health: String,
    pub started_at: DateTime<Utc>,
}

/// IBankFleetManager - Fleet orchestration for financial agents
pub struct IBankFleetManager {
    agent_registry: Arc<InMemoryAgentRegistry>,
    instance_registry: Arc<InMemoryInstanceRegistry>,
    discovery: Arc<InMemoryDiscoveryService>,
    health_monitor: Arc<HealthMonitor>,
    _resilience: Arc<ResilienceController>,
    config: FleetConfig,
    spec_types: Arc<RwLock<HashMap<String, FinancialAgentType>>>,
    instance_specs: Arc<RwLock<HashMap<String, AgentSpecId>>>,
}

impl IBankFleetManager {
    /// Create a new fleet manager with IBank-optimized configuration
    pub fn new(config: FleetConfig) -> Self {
        let agent_registry = Arc::new(InMemoryAgentRegistry::new());
        let instance_registry: Arc<InMemoryInstanceRegistry> = Arc::new(InMemoryInstanceRegistry::new());
        let discovery = Arc::new(InMemoryDiscoveryService::new(
            instance_registry.clone() as Arc<dyn InstanceRegistry>,
        ));

        let health_config = HealthConfig::for_platform(PlatformProfile::IBank);
        let resilience_config = health_config.resilience.clone();
        let executor = Arc::new(NoOpRecoveryExecutor);
        let resilience = Arc::new(ResilienceController::new(
            resilience_config,
            PlatformProfile::IBank,
            executor,
        ));
        let health_monitor = Arc::new(HealthMonitor::new(health_config, resilience.clone()));

        info!("IBankFleetManager initialized with IBank health thresholds");

        Self {
            agent_registry,
            instance_registry,
            discovery,
            health_monitor,
            _resilience: resilience,
            config,
            spec_types: Arc::new(RwLock::new(HashMap::new())),
            instance_specs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a financial agent spec (template for deployment)
    pub async fn register_agent_spec(
        &self,
        name: &str,
        version: &str,
        agent_type: FinancialAgentType,
        _description: &str,
    ) -> FleetResult<AgentSpecId> {
        let ver = semver::Version::parse(version)
            .unwrap_or_else(|_| semver::Version::new(0, 1, 0));

        let cap = |n: &str| CapabilityRef {
            name: n.to_string(),
            version: "1.0.0".to_string(),
        };

        let capabilities = match agent_type {
            FinancialAgentType::Buyer => vec![
                cap("payment.initiate"), cap("escrow.deposit"), cap("trade.negotiate"),
            ],
            FinancialAgentType::Seller => vec![
                cap("service.publish"), cap("invoice.issue"), cap("delivery.prove"),
            ],
            FinancialAgentType::Arbiter => vec![
                cap("dispute.evaluate"), cap("escrow.release"), cap("escrow.refund"),
            ],
            FinancialAgentType::Issuer => vec![
                cap("currency.mint"), cap("currency.burn"), cap("reserve.manage"),
            ],
            FinancialAgentType::Auditor => vec![
                cap("ledger.audit"), cap("receipt.verify"), cap("compliance.check"),
            ],
            FinancialAgentType::Compliance => vec![
                cap("policy.enforce"), cap("risk.assess"), cap("aml.screen"),
            ],
        };

        let mut spec = AgentSpec::new(name, ver);
        spec.platform = PlatformProfile::IBank;
        spec.capabilities = capabilities;
        spec.resonator_profile = ResonatorProfileConfig {
            profile_type: "IBank".to_string(),
            risk_tolerance: RiskTolerance::Conservative,
            autonomy_level: AutonomyLevel::FullHumanOversight,
            parameters: {
                let mut p = HashMap::new();
                p.insert("agent.type".to_string(), agent_type.to_string());
                p.insert("domain".to_string(), "finance".to_string());
                p
            },
        };
        spec.metadata.insert("agent.type".to_string(), agent_type.to_string());
        spec.metadata.insert("domain".to_string(), "finance".to_string());

        let spec_id = spec.id.clone();
        self.agent_registry.register(spec).await?;

        let mut types = self.spec_types.write().await;
        types.insert(spec_id.to_string(), agent_type);

        info!(spec_id = %spec_id, name = %name, agent_type = %agent_type,
              "Financial agent spec registered");

        Ok(spec_id)
    }

    /// Deploy instances of a financial agent spec
    pub async fn deploy_instances(
        &self,
        spec_id: &AgentSpecId,
        count: u32,
    ) -> FleetResult<Vec<InstanceId>> {
        let spec = self.agent_registry.get(spec_id).await?
            .ok_or_else(|| FleetError::SpecNotFound(spec_id.to_string()))?;

        let count = count.min(self.config.max_instances);
        let deployment_id = DeploymentId::generate();
        let mut instance_ids = Vec::new();

        for _ in 0..count {
            let instance_id = InstanceId::generate();
            let instance = AgentInstance {
                id: instance_id.clone(),
                deployment_id: deployment_id.clone(),
                resonator_id: ResonatorIdRef::new(format!("res:{}", uuid::Uuid::new_v4())),
                status: InstanceStatus::Starting {
                    phase: StartupPhase::Initializing,
                },
                health: HealthStatus::Unknown,
                placement: Default::default(),
                metrics: Default::default(),
                started_at: Utc::now(),
                last_heartbeat: Utc::now(),
            };

            self.instance_registry.register(instance).await?;

            {
                let mut specs = self.instance_specs.write().await;
                specs.insert(instance_id.to_string(), spec_id.clone());
            }

            if let Err(e) = self.health_monitor.register_instance(instance_id.clone()) {
                warn!(instance_id = %instance_id, error = %e,
                      "Failed to register for health monitoring");
            }

            instance_ids.push(instance_id);
        }

        info!(spec = %spec.name, count = count, deployment = %deployment_id,
              "Deployed financial agent instances");

        Ok(instance_ids)
    }

    /// Get fleet status summary for dashboard
    pub async fn fleet_status(&self) -> FleetResult<FleetStatusSummary> {
        let specs = self.agent_registry.list().await?;
        let instances = self.instance_registry.list_all().await?;
        let types = self.spec_types.read().await;
        let inst_specs = self.instance_specs.read().await;

        let mut spec_infos = Vec::new();
        for spec in &specs {
            let instance_count = instances.iter()
                .filter(|i| {
                    inst_specs.get(&i.id.to_string())
                        .map(|s| s.to_string() == spec.id.to_string())
                        .unwrap_or(false)
                })
                .count();

            spec_infos.push(SpecInfo {
                spec_id: spec.id.to_string(),
                name: spec.name.clone(),
                version: spec.version.to_string(),
                agent_type: types.get(&spec.id.to_string())
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                autonomy_level: format!("{:?}", spec.resonator_profile.autonomy_level),
                instance_count,
            });
        }

        let mut instance_infos = Vec::new();
        let mut healthy_count = 0;
        let mut unhealthy_count = 0;

        for inst in &instances {
            let health_str = match &inst.health {
                HealthStatus::Healthy => { healthy_count += 1; "healthy" },
                HealthStatus::Unhealthy { .. } => { unhealthy_count += 1; "unhealthy" },
                HealthStatus::Unknown => "unknown",
                HealthStatus::Degraded { .. } => { unhealthy_count += 1; "degraded" },
            };

            let spec_name = inst_specs.get(&inst.id.to_string())
                .and_then(|sid| specs.iter().find(|s| s.id.to_string() == sid.to_string()))
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "unknown".to_string());

            instance_infos.push(InstanceInfo {
                instance_id: inst.id.to_string(),
                spec_name,
                status: format!("{:?}", inst.status),
                health: health_str.to_string(),
                started_at: inst.started_at,
            });
        }

        Ok(FleetStatusSummary {
            total_specs: specs.len(),
            total_instances: instances.len(),
            healthy_instances: healthy_count,
            unhealthy_instances: unhealthy_count,
            specs: spec_infos,
            instances: instance_infos,
            timestamp: Utc::now(),
        })
    }

    /// Discover financial agents by capability
    pub async fn discover_agents(&self, capability: &str) -> FleetResult<Vec<InstanceInfo>> {
        let query = DiscoveryQuery {
            limit: Some(50),
            routing_strategy: RoutingStrategy::RoundRobin,
            ..Default::default()
        };

        let results = self.discovery.discover_by_capability(capability, &query).await?;
        let mut infos = Vec::new();

        for result in results {
            infos.push(InstanceInfo {
                instance_id: result.instance_id.to_string(),
                spec_name: "discovered".to_string(),
                status: "available".to_string(),
                health: format!("{:.2}", result.health_score),
                started_at: Utc::now(),
            });
        }

        Ok(infos)
    }

    /// Get health assessment for a specific instance
    pub async fn instance_health(&self, instance_id: &InstanceId) -> FleetResult<HealthAssessment> {
        Ok(self.health_monitor.probe_instance(instance_id).await?)
    }

    /// Terminate an instance
    pub async fn terminate_instance(&self, instance_id: &InstanceId) -> FleetResult<()> {
        self.instance_registry.remove(instance_id).await?;
        let mut specs = self.instance_specs.write().await;
        specs.remove(&instance_id.to_string());
        info!(instance_id = %instance_id, "Financial agent instance terminated");
        Ok(())
    }

    pub fn agent_registry(&self) -> &Arc<InMemoryAgentRegistry> {
        &self.agent_registry
    }

    pub fn instance_registry(&self) -> &Arc<InMemoryInstanceRegistry> {
        &self.instance_registry
    }

    pub fn health_monitor(&self) -> &Arc<HealthMonitor> {
        &self.health_monitor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fleet_manager_creation() {
        let manager = IBankFleetManager::new(FleetConfig::default());
        let status = manager.fleet_status().await.unwrap();
        assert_eq!(status.total_specs, 0);
        assert_eq!(status.total_instances, 0);
    }

    #[tokio::test]
    async fn test_register_and_deploy() {
        let manager = IBankFleetManager::new(FleetConfig::default());

        let spec_id = manager.register_agent_spec(
            "test-buyer",
            "1.0.0",
            FinancialAgentType::Buyer,
            "Test buyer agent",
        ).await.unwrap();

        let instances = manager.deploy_instances(&spec_id, 2).await.unwrap();
        assert_eq!(instances.len(), 2);

        let status = manager.fleet_status().await.unwrap();
        assert_eq!(status.total_specs, 1);
        assert_eq!(status.total_instances, 2);
    }
}
