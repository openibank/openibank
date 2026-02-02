//! OpeniBank Profile - AI-agent-only economic actor configuration
//!
//! The OpeniBank profile extends the concept of MAPLE's ibank_profile() to define
//! AI-agent-only economic actors with:
//! - No human involvement assumed (HumanInvolvementConfig::NotApplicable)
//! - Strict audit requirements
//! - Tightly scoped consequences
//! - Safety and correctness as dominant concerns
//!
//! OpeniBank extends this with specific budget, permit, escrow, and issuer configurations.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{Amount, AssetClass, ResonatorId};

// ============================================================================
// Human Involvement Configuration
// ============================================================================

/// Configuration for human involvement in the Resonator's operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanInvolvementConfig {
    /// Human oversight required for all actions
    Required,
    /// Human oversight for specific action types
    Selective { action_types: Vec<String> },
    /// No human involvement expected (AI-agent-only)
    NotApplicable,
}

impl Default for HumanInvolvementConfig {
    fn default() -> Self {
        Self::NotApplicable // OpeniBank default: AI-agent-only
    }
}

// ============================================================================
// Safety Configuration
// ============================================================================

/// Safety level for the Resonator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyLevel {
    /// Maximum safety - fail closed on any ambiguity
    Maximum,
    /// High safety - conservative but allows some flexibility
    High,
    /// Standard safety - balanced approach
    Standard,
}

impl Default for SafetyLevel {
    fn default() -> Self {
        Self::Maximum // OpeniBank default: maximum safety
    }
}

/// Configuration for resonance boundaries (safety constraints)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceBoundaryConfig {
    /// Safety level
    pub safety_level: SafetyLevel,
    /// Whether to fail closed on policy violations
    pub fail_closed: bool,
    /// Maximum consequence scope (bounds potential impact)
    pub max_consequence_scope: ConsequenceScope,
    /// Coercion detection enabled
    pub coercion_detection: bool,
}

impl Default for ResonanceBoundaryConfig {
    fn default() -> Self {
        Self {
            safety_level: SafetyLevel::Maximum,
            fail_closed: true,
            max_consequence_scope: ConsequenceScope::Limited,
            coercion_detection: true,
        }
    }
}

/// Scope of potential consequences
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsequenceScope {
    /// Minimal scope - single operation only
    Minimal,
    /// Limited scope - bounded by budget/permit
    Limited,
    /// Extended scope - may affect multiple resources
    Extended,
}

// ============================================================================
// Budget Configuration
// ============================================================================

/// Budget-related configuration for the profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfiguration {
    /// Maximum amount per single transaction
    pub max_per_transaction: Amount,
    /// Maximum amount per time window
    pub max_per_window: Amount,
    /// Duration of the rate-limiting window
    pub window_duration: Duration,
    /// Allowed asset classes
    pub allowed_asset_classes: Vec<AssetClass>,
    /// Optional counterparty whitelist (empty = allow all)
    pub counterparty_whitelist: Vec<ResonatorId>,
    /// Whether auto-renewal of budgets is allowed
    pub auto_renewal_enabled: bool,
}

impl Default for BudgetConfiguration {
    fn default() -> Self {
        Self {
            max_per_transaction: Amount::new(10000), // $100
            max_per_window: Amount::new(100000),     // $1000
            window_duration: Duration::from_secs(3600), // 1 hour
            allowed_asset_classes: vec![AssetClass::Stablecoin],
            counterparty_whitelist: vec![], // Allow all
            auto_renewal_enabled: false,
        }
    }
}

// ============================================================================
// Permit Configuration
// ============================================================================

/// Permit-related configuration for the profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermitConfiguration {
    /// Default permit duration
    pub default_duration: Duration,
    /// Maximum permit duration
    pub max_duration: Duration,
    /// Whether permits can be extended
    pub extension_allowed: bool,
    /// Maximum number of active permits
    pub max_active_permits: usize,
    /// Whether permits must be counterparty-specific
    pub require_counterparty_specific: bool,
}

impl Default for PermitConfiguration {
    fn default() -> Self {
        Self {
            default_duration: Duration::from_secs(600), // 10 minutes
            max_duration: Duration::from_secs(86400),   // 24 hours
            extension_allowed: false,
            max_active_permits: 10,
            require_counterparty_specific: true,
        }
    }
}

// ============================================================================
// Escrow Configuration
// ============================================================================

/// Escrow-related configuration for the profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowConfiguration {
    /// Default escrow timeout
    pub default_timeout: Duration,
    /// Maximum escrow duration
    pub max_duration: Duration,
    /// Whether arbitration is required
    pub require_arbiter: bool,
    /// Default timeout action
    pub timeout_action: EscrowTimeoutAction,
    /// Maximum number of delivery conditions
    pub max_delivery_conditions: usize,
}

impl Default for EscrowConfiguration {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(3600),     // 1 hour
            max_duration: Duration::from_secs(86400 * 30),  // 30 days
            require_arbiter: false,
            timeout_action: EscrowTimeoutAction::RefundBuyer,
            max_delivery_conditions: 10,
        }
    }
}

/// Action to take when escrow times out
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowTimeoutAction {
    /// Refund funds to the buyer
    RefundBuyer,
    /// Release funds to the seller
    ReleaseSeller,
    /// Hold for manual resolution
    Hold,
}

// ============================================================================
// Issuer Configuration
// ============================================================================

/// Issuer-related configuration (for Resonators that can issue assets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerConfiguration {
    /// Whether this Resonator can act as an issuer
    pub is_issuer: bool,
    /// Maximum mint amount per transaction
    pub max_mint_per_transaction: Amount,
    /// Maximum mint rate (per window)
    pub max_mint_rate: Amount,
    /// Rate window duration
    pub rate_window: Duration,
    /// Minimum reserve ratio (1.0 = 100% backed)
    pub min_reserve_ratio: f64,
}

impl Default for IssuerConfiguration {
    fn default() -> Self {
        Self {
            is_issuer: false,
            max_mint_per_transaction: Amount::new(1_000_000_00), // $10,000
            max_mint_rate: Amount::new(10_000_000_00),           // $100,000
            rate_window: Duration::from_secs(3600),
            min_reserve_ratio: 1.0, // 100% backed
        }
    }
}

// ============================================================================
// Audit Configuration
// ============================================================================

/// Audit and logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfiguration {
    /// Whether all actions are logged
    pub log_all_actions: bool,
    /// Whether receipts are required for all operations
    pub require_receipts: bool,
    /// Receipt retention period
    pub retention_period: Duration,
    /// Whether to hash sensitive data in logs
    pub hash_sensitive_data: bool,
}

impl Default for AuditConfiguration {
    fn default() -> Self {
        Self {
            log_all_actions: true,
            require_receipts: true,
            retention_period: Duration::from_secs(86400 * 365), // 1 year
            hash_sensitive_data: true,
        }
    }
}

// ============================================================================
// OpeniBank Profile
// ============================================================================

/// OpeniBank-specific profile configuration
///
/// This is the canonical configuration for an AI-agent-only economic actor
/// in the OpeniBank system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeniBankProfileConfig {
    /// Human involvement configuration
    pub human_involvement: HumanInvolvementConfig,
    /// Safety boundary configuration
    pub safety: ResonanceBoundaryConfig,
    /// Budget configuration
    pub budget: BudgetConfiguration,
    /// Permit configuration
    pub permit: PermitConfiguration,
    /// Escrow configuration
    pub escrow: EscrowConfiguration,
    /// Issuer configuration (optional)
    pub issuer: Option<IssuerConfiguration>,
    /// Audit configuration
    pub audit: AuditConfiguration,
}

impl Default for OpeniBankProfileConfig {
    fn default() -> Self {
        Self {
            human_involvement: HumanInvolvementConfig::NotApplicable,
            safety: ResonanceBoundaryConfig::default(),
            budget: BudgetConfiguration::default(),
            permit: PermitConfiguration::default(),
            escrow: EscrowConfiguration::default(),
            issuer: None,
            audit: AuditConfiguration::default(),
        }
    }
}

/// The OpeniBank Profile
///
/// Wraps configuration and provides validation/enforcement methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeniBankProfile {
    /// Profile ID
    pub profile_id: String,
    /// Owner Resonator
    pub owner: ResonatorId,
    /// Configuration
    pub config: OpeniBankProfileConfig,
    /// When the profile was created
    pub created_at: DateTime<Utc>,
    /// When the profile was last updated
    pub updated_at: DateTime<Utc>,
}

impl OpeniBankProfile {
    /// Create a new OpeniBank profile with default configuration
    pub fn new(owner: ResonatorId) -> Self {
        let now = Utc::now();
        Self {
            profile_id: format!("profile_{}", uuid::Uuid::new_v4()),
            owner,
            config: OpeniBankProfileConfig::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a profile with custom configuration
    pub fn with_config(owner: ResonatorId, config: OpeniBankProfileConfig) -> Self {
        let now = Utc::now();
        Self {
            profile_id: format!("profile_{}", uuid::Uuid::new_v4()),
            owner,
            config,
            created_at: now,
            updated_at: now,
        }
    }

    /// Validate that an amount is within budget limits
    pub fn validate_amount(&self, amount: Amount) -> crate::Result<()> {
        if amount > self.config.budget.max_per_transaction {
            return Err(crate::CoreError::PolicyViolation {
                message: format!(
                    "Amount {} exceeds max per transaction {}",
                    amount, self.config.budget.max_per_transaction
                ),
            });
        }
        Ok(())
    }

    /// Validate that an asset class is allowed
    pub fn validate_asset_class(&self, asset_class: &AssetClass) -> crate::Result<()> {
        if !self.config.budget.allowed_asset_classes.contains(asset_class) {
            return Err(crate::CoreError::PolicyViolation {
                message: format!("Asset class {:?} not allowed", asset_class),
            });
        }
        Ok(())
    }

    /// Validate that a counterparty is allowed
    pub fn validate_counterparty(&self, counterparty: &ResonatorId) -> crate::Result<()> {
        let whitelist = &self.config.budget.counterparty_whitelist;
        if !whitelist.is_empty() && !whitelist.contains(counterparty) {
            return Err(crate::CoreError::PolicyViolation {
                message: format!("Counterparty {} not in whitelist", counterparty),
            });
        }
        Ok(())
    }

    /// Check if this profile has issuer capabilities
    pub fn is_issuer(&self) -> bool {
        self.config.issuer.as_ref().map(|i| i.is_issuer).unwrap_or(false)
    }
}

/// Create the canonical ibank profile (AI-agent-only banking profile)
///
/// This is the foundation profile for all OpeniBank Resonators.
/// It enforces:
/// - No human involvement
/// - Maximum safety
/// - Fail-closed on violations
/// - Full audit logging
pub fn ibank_profile(owner: ResonatorId) -> OpeniBankProfile {
    let config = OpeniBankProfileConfig {
        human_involvement: HumanInvolvementConfig::NotApplicable,
        safety: ResonanceBoundaryConfig {
            safety_level: SafetyLevel::Maximum,
            fail_closed: true,
            max_consequence_scope: ConsequenceScope::Limited,
            coercion_detection: true,
        },
        budget: BudgetConfiguration::default(),
        permit: PermitConfiguration {
            require_counterparty_specific: true,
            ..Default::default()
        },
        escrow: EscrowConfiguration::default(),
        issuer: None,
        audit: AuditConfiguration {
            log_all_actions: true,
            require_receipts: true,
            ..Default::default()
        },
    };

    OpeniBankProfile::with_config(owner, config)
}

/// Create an issuer profile for IUSD minting/burning
pub fn issuer_profile(owner: ResonatorId) -> OpeniBankProfile {
    let mut profile = ibank_profile(owner);
    profile.config.issuer = Some(IssuerConfiguration {
        is_issuer: true,
        ..Default::default()
    });
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibank_profile_defaults() {
        let owner = ResonatorId::new();
        let profile = ibank_profile(owner.clone());

        assert_eq!(profile.owner, owner);
        assert_eq!(
            profile.config.human_involvement,
            HumanInvolvementConfig::NotApplicable
        );
        assert_eq!(
            profile.config.safety.safety_level,
            SafetyLevel::Maximum
        );
        assert!(profile.config.safety.fail_closed);
    }

    #[test]
    fn test_amount_validation() {
        let profile = ibank_profile(ResonatorId::new());

        // Should pass for amount within limits
        assert!(profile.validate_amount(Amount::new(1000)).is_ok());

        // Should fail for amount exceeding max per transaction
        let large_amount = Amount::new(1_000_000_000);
        assert!(profile.validate_amount(large_amount).is_err());
    }

    #[test]
    fn test_asset_class_validation() {
        let profile = ibank_profile(ResonatorId::new());

        // Stablecoin should be allowed by default
        assert!(profile.validate_asset_class(&AssetClass::Stablecoin).is_ok());

        // NFT should not be allowed by default
        assert!(profile.validate_asset_class(&AssetClass::NFT).is_err());
    }

    #[test]
    fn test_issuer_profile() {
        let profile = issuer_profile(ResonatorId::new());

        assert!(profile.is_issuer());
        assert!(profile.config.issuer.is_some());
    }

    #[test]
    fn test_counterparty_validation() {
        let owner = ResonatorId::new();
        let allowed = ResonatorId::from_string("allowed_party");
        let denied = ResonatorId::from_string("denied_party");

        let mut config = OpeniBankProfileConfig::default();
        config.budget.counterparty_whitelist = vec![allowed.clone()];

        let profile = OpeniBankProfile::with_config(owner, config);

        assert!(profile.validate_counterparty(&allowed).is_ok());
        assert!(profile.validate_counterparty(&denied).is_err());
    }
}
