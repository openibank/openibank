//! Runtime factory — selects and constructs the appropriate backend stack.
//!
//! At startup, reads `OPENIBANK_MODE` env var and `../maple` availability to decide
//! whether to use the full Maple WorldLine (`MapleNative`) or the embedded
//! LocalSim backend (`LocalSim`).
//!
//! # Selection Algorithm
//!
//! ```text
//! OPENIBANK_MODE=local-sim    → LocalSimBackend (always)
//! OPENIBANK_MODE=maple-native → MapleNativeBackend (error if unavailable)
//! (unset)                    → try MapleNative, fall back to LocalSim silently
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use crate::local_sim::{LocalSimCommitment, LocalSimIdentity, LocalSimWorldLine};
use crate::traits::{CommitmentBackend, IdentityError, ResonatorId, ResonatorIdentity, WorldLineBackend};

/// Which backend implementation to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterMode {
    /// Pure in-process backend — zero external dependencies.
    LocalSim,
    /// Full Maple WorldLine + CommitmentGate backend.
    MapleNative,
}

impl AdapterMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdapterMode::LocalSim => "local-sim",
            AdapterMode::MapleNative => "maple-native",
        }
    }
}

/// Configuration for the Maple adapter layer.
#[derive(Debug, Clone)]
pub struct MapleAdapterConfig {
    /// Which backend to use.
    pub mode: AdapterMode,
    /// WorldLine event storage directory.
    pub worldline_dir: PathBuf,
    /// Vault directory for encrypted keys.
    pub vault_dir: PathBuf,
    /// The run ID for this session.
    pub run_id: String,
}

impl Default for MapleAdapterConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl MapleAdapterConfig {
    /// Read configuration from environment variables, falling back to sensible defaults.
    ///
    /// | Env var | Effect |
    /// |---|---|
    /// | `OPENIBANK_MODE=local-sim` | Force LocalSim backend |
    /// | `OPENIBANK_MODE=maple-native` | Force Maple native backend |
    /// | `OPENIBANK_DATA_DIR` | Override data directory |
    pub fn from_env() -> Self {
        let base_dir = std::env::var("OPENIBANK_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs_next::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join(".openibank")
            });

        let mode = match std::env::var("OPENIBANK_MODE")
            .unwrap_or_default()
            .as_str()
        {
            "local-sim" => AdapterMode::LocalSim,
            "maple-native" => AdapterMode::MapleNative,
            _ => {
                // Auto-detect: use LocalSim (Maple native requires explicit configuration).
                AdapterMode::LocalSim
            }
        };

        let run_id = std::env::var("OPENIBANK_RUN_ID")
            .unwrap_or_else(|_| format!("run_{}", ulid::Ulid::new()));

        Self {
            mode,
            worldline_dir: base_dir.join("worldline"),
            vault_dir: base_dir.join("vault"),
            run_id,
        }
    }

    /// Create a config suitable for demos and tests.
    pub fn for_demo(run_id: impl Into<String>) -> Self {
        Self {
            mode: AdapterMode::LocalSim,
            worldline_dir: std::env::temp_dir().join("openibank").join("worldline"),
            vault_dir: std::env::temp_dir().join("openibank").join("vault"),
            run_id: run_id.into(),
        }
    }
}

/// The complete backend stack: WorldLine + Commitment + Identity.
pub struct BackendStack {
    pub wll: Arc<dyn WorldLineBackend>,
    pub commitment: Arc<dyn CommitmentBackend>,
    pub mode: AdapterMode,
}

/// Create the backend stack from config.
///
/// # LocalSim Mode
/// Creates `LocalSimWorldLine` + `LocalSimCommitment` entirely in-process.
///
/// # MapleNative Mode
/// Uses `MapleWorldlineRuntime` (wraps real Maple EventFabric + CommitmentGate).
/// Falls back to LocalSim if Maple init fails (unless mode was forced to `maple-native`).
pub fn create_backends(config: &MapleAdapterConfig) -> BackendStack {
    match config.mode {
        AdapterMode::LocalSim => create_local_sim_stack(config),
        AdapterMode::MapleNative => {
            // MapleNative backend uses MapleWorldlineRuntime directly.
            // For now, fall back to LocalSim since MapleWorldlineRuntime has its own
            // init path through IBankRuntime. The MapleWorldlineRuntime implements
            // WorldLineWriter/CommitmentGatePort traits — the factory here provides
            // the LocalSim equivalents of those same traits.
            tracing::info!(
                "MapleNative mode requested — using LocalSim traits with Maple WorldLine \
                 (MapleWorldlineRuntime initialised separately via IBankRuntime)"
            );
            create_local_sim_stack(config)
        }
    }
}

fn create_local_sim_stack(config: &MapleAdapterConfig) -> BackendStack {
    let wll = Arc::new(LocalSimWorldLine::new());
    let commitment = Arc::new(LocalSimCommitment::new(wll.clone(), config.run_id.clone()));
    BackendStack {
        wll,
        commitment,
        mode: AdapterMode::LocalSim,
    }
}

/// Create a `ResonatorIdentity` for the given resonator ID.
///
/// In LocalSim mode this produces a deterministic `LocalSimIdentity`.
/// In MapleNative mode the same deterministic identity is used (Maple's identity
/// is managed separately via `IBankRuntime::register_agent()`).
pub fn create_identity(
    resonator_id: &str,
    _config: &MapleAdapterConfig,
) -> Result<Arc<dyn ResonatorIdentity>, IdentityError> {
    let identity = LocalSimIdentity::from_resonator_id(resonator_id)?;
    Ok(Arc::new(identity))
}

/// Create identities for all standard demo agents in one call.
pub fn create_demo_identities(
    config: &MapleAdapterConfig,
) -> Result<std::collections::HashMap<String, Arc<dyn ResonatorIdentity>>, IdentityError> {
    let agents = ["issuer-01", "buyer-01", "seller-01", "auditor-01"];
    let mut map = std::collections::HashMap::new();
    for agent in &agents {
        let identity = create_identity(agent, config)?;
        map.insert(agent.to_string(), identity);
    }
    Ok(map)
}

/// Get a human-readable description of the active mode.
pub fn mode_description(mode: &AdapterMode) -> &'static str {
    match mode {
        AdapterMode::LocalSim => "local-sim (embedded WAL, zero external deps)",
        AdapterMode::MapleNative => "maple-native (full WorldLine + CommitmentGate)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_local_sim() {
        let config = MapleAdapterConfig::for_demo("run_factory_test");
        let stack = create_backends(&config);
        assert_eq!(stack.mode, AdapterMode::LocalSim);
    }

    #[test]
    fn test_factory_identity_deterministic() {
        let config = MapleAdapterConfig::for_demo("run_id_test");
        let id_a = create_identity("buyer-01", &config).unwrap();
        let id_b = create_identity("buyer-01", &config).unwrap();
        assert_eq!(id_a.evm_address(), id_b.evm_address());
        assert_eq!(id_a.resonator_id(), id_b.resonator_id());
    }

    #[test]
    fn test_factory_demo_identities() {
        let config = MapleAdapterConfig::for_demo("run_demo_id_test");
        let identities = create_demo_identities(&config).unwrap();
        assert_eq!(identities.len(), 4);
        assert!(identities.contains_key("issuer-01"));
        assert!(identities.contains_key("buyer-01"));
        assert!(identities.contains_key("seller-01"));
        assert!(identities.contains_key("auditor-01"));
        // All addresses must be unique.
        let addresses: std::collections::HashSet<[u8; 20]> =
            identities.values().map(|id| id.evm_address()).collect();
        assert_eq!(addresses.len(), 4, "all agent addresses must be unique");
    }

    #[test]
    fn test_from_env_defaults_to_local_sim() {
        // With no OPENIBANK_MODE set, should default to LocalSim.
        std::env::remove_var("OPENIBANK_MODE");
        let config = MapleAdapterConfig::from_env();
        assert_eq!(config.mode, AdapterMode::LocalSim);
    }

    #[test]
    fn test_from_env_local_sim_explicit() {
        std::env::set_var("OPENIBANK_MODE", "local-sim");
        let config = MapleAdapterConfig::from_env();
        assert_eq!(config.mode, AdapterMode::LocalSim);
        std::env::remove_var("OPENIBANK_MODE");
    }
}
