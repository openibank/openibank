//! IBankRuntime - Bootstrap MapleRuntime with iBank configuration
//!
//! This module wraps the Maple runtime initialization specifically for
//! the OpeniBank use case, using `ibank_runtime_config()`.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use maple_runtime::{
    MapleRuntime, ResonatorHandle, ResonatorSpec, ResonatorIdentitySpec,
    config::{ibank_runtime_config, RuntimeConfig},
    ResonatorProfile, PresenceConfig, AttentionBudgetSpec,
    BootstrapError, ShutdownError, RegistrationError,
};
use maple_runtime::runtime_core::CapabilitySpec;

use crate::bridge::ResonatorAgentRole;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration options for the IBankRuntime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IBankRuntimeConfig {
    /// Custom name for this runtime instance
    pub instance_name: String,

    /// Attention budget per resonator (default: 1000)
    pub default_attention_budget: u64,

    /// Enable detailed telemetry
    pub enable_telemetry: bool,

    /// Maximum resonators (agents) allowed
    pub max_resonators: usize,
}

impl Default for IBankRuntimeConfig {
    fn default() -> Self {
        Self {
            instance_name: "openibank".to_string(),
            default_attention_budget: 1000,
            enable_telemetry: true,
            max_resonators: 100,
        }
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur in IBankRuntime operations
#[derive(Debug, Error)]
pub enum IBankRuntimeError {
    /// Failed to bootstrap the Maple runtime
    #[error("Maple runtime bootstrap failed: {0}")]
    BootstrapFailed(#[from] BootstrapError),

    /// Failed to shut down the Maple runtime
    #[error("Maple runtime shutdown failed: {0}")]
    ShutdownFailed(#[from] ShutdownError),

    /// Failed to register a resonator
    #[error("Resonator registration failed: {0}")]
    RegistrationFailed(#[from] RegistrationError),

    /// Runtime is not initialized
    #[error("IBankRuntime is not initialized")]
    NotInitialized,

    /// Maximum resonators reached
    #[error("Maximum resonators ({max}) reached")]
    MaxResonatorsReached { max: usize },
}

// ============================================================================
// IBankRuntime
// ============================================================================

/// The OpeniBank-specific Maple runtime wrapper
///
/// Manages the lifecycle of the Maple runtime configured for autonomous finance,
/// and provides convenient methods for registering OpeniBank agents as Resonators.
///
/// # Example
///
/// ```ignore
/// let config = IBankRuntimeConfig::default();
/// let runtime = IBankRuntime::new(config).await?;
///
/// // Register a buyer agent as a Resonator
/// let handle = runtime.register_agent("Alice", ResonatorAgentRole::Buyer).await?;
///
/// // The handle can be used for presence signaling, coupling, etc.
/// ```
pub struct IBankRuntime {
    /// The underlying Maple runtime
    maple_runtime: MapleRuntime,

    /// Configuration
    config: IBankRuntimeConfig,

    /// Track registered resonator count
    resonator_count: std::sync::atomic::AtomicUsize,
}

impl IBankRuntime {
    /// Create and bootstrap a new IBankRuntime
    ///
    /// Uses `ibank_runtime_config()` from Maple for autonomous finance settings:
    /// - No human profiles (AI-agent-only)
    /// - `audit_all_commitments: true`
    /// - `risk_bounded_consequences: true`
    /// - Max consequence value: $1,000,000
    pub async fn new(config: IBankRuntimeConfig) -> Result<Self, IBankRuntimeError> {
        tracing::info!(
            "Bootstrapping IBankRuntime '{}' with Maple ibank_runtime_config()",
            config.instance_name
        );

        let maple_config = ibank_runtime_config();
        let maple_runtime = MapleRuntime::bootstrap(maple_config).await?;

        tracing::info!("IBankRuntime '{}' bootstrapped successfully", config.instance_name);

        Ok(Self {
            maple_runtime,
            config,
            resonator_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Create with a custom RuntimeConfig (for testing)
    pub async fn with_config(
        ibank_config: IBankRuntimeConfig,
        maple_config: RuntimeConfig,
    ) -> Result<Self, IBankRuntimeError> {
        let maple_runtime = MapleRuntime::bootstrap(maple_config).await?;

        Ok(Self {
            maple_runtime,
            config: ibank_config,
            resonator_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Register an OpeniBank agent as a Maple Resonator
    ///
    /// Creates a fully-populated ResonatorSpec based on the agent's role:
    /// - Identity: agent name + role metadata
    /// - Profile: IBank (autonomous finance)
    /// - Capabilities: role-specific cognitive capabilities
    /// - Attention budget: configurable capacity with safety reserves
    /// - Presence: discoverable and responsive by default
    /// - Coupling affinity: role-specific preferred coupling patterns
    pub async fn register_agent(
        &self,
        name: &str,
        role: ResonatorAgentRole,
    ) -> Result<ResonatorHandle, IBankRuntimeError> {
        let count = self.resonator_count.load(std::sync::atomic::Ordering::Relaxed);
        if count >= self.config.max_resonators {
            return Err(IBankRuntimeError::MaxResonatorsReached {
                max: self.config.max_resonators,
            });
        }

        tracing::info!(
            "Registering {} agent '{}' as Maple Resonator (enriched spec)",
            role.display_name(),
            name
        );

        let mut spec = ResonatorSpec::default();

        // IBank runtime only allows IBank profiles
        spec.profile = ResonatorProfile::IBank;

        // Identity: agent name + role metadata
        spec.identity = ResonatorIdentitySpec {
            name: Some(name.to_string()),
            metadata: HashMap::from([
                ("role".to_string(), role.display_name().to_string()),
                ("system".to_string(), "openibank".to_string()),
                ("instance".to_string(), self.config.instance_name.clone()),
            ]),
        };

        // Capabilities: role-specific cognitive capabilities
        spec.capabilities = role.cognitive_capabilities().iter().map(|c| {
            CapabilitySpec {
                name: format!("{:?}", c),
                version: "1.0".to_string(),
            }
        }).collect();

        // Attention budget: capacity with safety reserves
        spec.attention = AttentionBudgetSpec {
            total_capacity: self.config.default_attention_budget,
            safety_reserve: Some(100),
            exhaustion_threshold: Some(0.9),
        };

        // Presence config: discoverable and responsive
        spec.presence = PresenceConfig {
            initial_discoverability: 0.5,
            initial_responsiveness: 1.0,
            start_silent: false,
            max_signal_frequency_ms: 1000,
        };

        // Coupling affinity: role-specific preferred patterns
        spec.coupling_affinity = role.coupling_affinity();

        let handle = self.maple_runtime.register_resonator(spec).await?;

        self.resonator_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        tracing::info!(
            "Resonator '{}' registered with ID {:?} (capabilities: {}, attention: {})",
            name,
            handle.id,
            role.cognitive_capabilities().len(),
            self.config.default_attention_budget,
        );

        Ok(handle)
    }

    /// Get the underlying Maple runtime reference
    pub fn maple_runtime(&self) -> &MapleRuntime {
        &self.maple_runtime
    }

    /// Get the runtime configuration
    pub fn config(&self) -> &IBankRuntimeConfig {
        &self.config
    }

    /// Get the number of registered resonators
    pub fn resonator_count(&self) -> usize {
        self.resonator_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Check if the runtime is shutting down
    pub async fn is_shutting_down(&self) -> bool {
        self.maple_runtime.is_shutting_down().await
    }

    /// Gracefully shut down the runtime
    pub async fn shutdown(self) -> Result<(), IBankRuntimeError> {
        tracing::info!("Shutting down IBankRuntime '{}'", self.config.instance_name);
        self.maple_runtime.shutdown().await?;
        tracing::info!("IBankRuntime '{}' shut down successfully", self.config.instance_name);
        Ok(())
    }
}

// ============================================================================
// Runtime Status (for dashboard)
// ============================================================================

/// Runtime status information for API/dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    /// Instance name
    pub instance_name: String,
    /// Number of registered resonators
    pub resonator_count: usize,
    /// Maximum allowed resonators
    pub max_resonators: usize,
    /// Whether the runtime is running
    pub is_running: bool,
    /// Runtime profile (always "IBank" for OpeniBank)
    pub profile: String,
    /// Key runtime features
    pub features: Vec<String>,
}

impl IBankRuntime {
    /// Get runtime status for dashboard display
    pub async fn status(&self) -> RuntimeStatus {
        RuntimeStatus {
            instance_name: self.config.instance_name.clone(),
            resonator_count: self.resonator_count(),
            max_resonators: self.config.max_resonators,
            is_running: !self.is_shutting_down().await,
            profile: "IBank (Autonomous Finance)".to_string(),
            features: vec![
                "audit_all_commitments".to_string(),
                "risk_bounded_consequences".to_string(),
                "no_human_profiles".to_string(),
                "fail_closed".to_string(),
                "8_canonical_invariants".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ibank_runtime_new() {
        let config = IBankRuntimeConfig::default();
        let runtime = IBankRuntime::new(config).await;
        assert!(runtime.is_ok());

        let rt = runtime.unwrap();
        assert_eq!(rt.resonator_count(), 0);
        assert_eq!(rt.config().instance_name, "openibank");

        rt.shutdown().await.expect("shutdown failed");
    }

    #[tokio::test]
    async fn test_register_agent() {
        let config = IBankRuntimeConfig::default();
        let runtime = IBankRuntime::new(config).await.unwrap();

        let handle = runtime.register_agent("Alice", ResonatorAgentRole::Buyer).await;
        assert!(handle.is_ok(), "Failed to register Alice: {:?}", handle.err());
        assert_eq!(runtime.resonator_count(), 1);

        let handle2 = runtime.register_agent("DataCorp", ResonatorAgentRole::Seller).await;
        assert!(handle2.is_ok(), "Failed to register DataCorp: {:?}", handle2.err());
        assert_eq!(runtime.resonator_count(), 2);

        runtime.shutdown().await.expect("shutdown failed");
    }

    #[tokio::test]
    async fn test_runtime_status() {
        let config = IBankRuntimeConfig::default();
        let runtime = IBankRuntime::new(config).await.unwrap();

        let status = runtime.status().await;
        assert!(status.is_running);
        assert_eq!(status.profile, "IBank (Autonomous Finance)");
        assert!(status.features.contains(&"audit_all_commitments".to_string()));

        runtime.shutdown().await.expect("shutdown failed");
    }
}
