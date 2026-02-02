//! Canonical types for OpeniBank
//!
//! These types form the foundation of all OpeniBank operations.
//! They are designed to be machine-verifiable and audit-friendly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Identity Types
// ============================================================================

/// Unique identifier for a Resonator (the only economic actor in OpeniBank)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorId(pub String);

impl ResonatorId {
    pub fn new() -> Self {
        Self(format!("res_{}", Uuid::new_v4()))
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for ResonatorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ResonatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an asset
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl AssetId {
    pub fn new() -> Self {
        Self(format!("asset_{}", Uuid::new_v4()))
    }

    /// The canonical IUSD stablecoin asset ID
    pub fn iusd() -> Self {
        Self("IUSD".to_string())
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Asset Types
// ============================================================================

/// Classification of assets in OpeniBank
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetClass {
    /// Native blockchain coins (ETH, BTC, etc.)
    CryptoCoin,
    /// Stablecoins (IUSD, USDC, etc.)
    Stablecoin,
    /// Non-fungible tokens
    NFT,
    /// API credits, compute credits
    Credit,
    /// Subscriptions, licenses, SaaS seats
    Entitlement,
    /// Real-world asset claims
    RwaClaim,
    /// Capability proofs, compliance badges
    Attestation,
}

/// How the asset is custodied
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustodyMode {
    /// On a blockchain
    OnChain,
    /// Off-chain with issuer attestation
    OffChainAttested,
    /// Only usable in escrow
    EscrowOnly,
}

/// Whether and how the asset can be transferred
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transferability {
    /// Can be freely transferred
    Transferable,
    /// Cannot be transferred (bound to identity)
    NonTransferable,
    /// Transfer subject to conditions
    Conditional { conditions: Vec<TransferCondition> },
}

/// Conditions that must be met for a conditional transfer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferCondition {
    pub condition_type: String,
    pub parameters: serde_json::Value,
}

/// How to verify the asset's authenticity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// Verify via blockchain proof
    ChainProof { chain_id: String, contract: String },
    /// Verify via issuer signature
    IssuerSignature,
    /// Verify via oracle reference
    OracleReference { oracle_id: String },
}

/// Reference to the asset issuer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuerRef {
    pub issuer_type: IssuerType,
    pub identifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssuerType {
    /// Local mock issuer (for development)
    LocalMock,
    /// On-chain contract
    OnChain,
    /// Centralized issuer with attestation
    Centralized,
}

/// Hint about the asset's value (non-authoritative)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValuationHint {
    pub amount: f64,
    pub currency: String,
    pub as_of: DateTime<Utc>,
}

/// Policy tag for an asset (jurisdiction, risk tier, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyTag {
    pub tag_type: String,
    pub value: String,
}

/// Unified asset representation for OpeniBank
///
/// All assets—crypto and non-crypto—are represented uniformly.
/// Non-crypto assets are claims with verification rules, not balances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetObject {
    pub asset_id: AssetId,
    pub asset_class: AssetClass,
    pub issuer: IssuerRef,
    pub custody_mode: CustodyMode,
    pub transferability: Transferability,
    pub verification: VerificationMethod,
    pub valuation_hint: Option<ValuationHint>,
    pub policy_tags: Vec<PolicyTag>,
}

impl AssetObject {
    /// Create the canonical IUSD stablecoin asset
    pub fn iusd() -> Self {
        Self {
            asset_id: AssetId::iusd(),
            asset_class: AssetClass::Stablecoin,
            issuer: IssuerRef {
                issuer_type: IssuerType::LocalMock,
                identifier: "openibank-issuer".to_string(),
            },
            custody_mode: CustodyMode::OffChainAttested,
            transferability: Transferability::Transferable,
            verification: VerificationMethod::IssuerSignature,
            valuation_hint: Some(ValuationHint {
                amount: 1.0,
                currency: "USD".to_string(),
                as_of: Utc::now(),
            }),
            policy_tags: vec![],
        }
    }
}

// ============================================================================
// Amount Types
// ============================================================================

/// Represents an amount of an asset (in smallest units, e.g., cents for USD)
///
/// This is the simple legacy type using u64. For high-precision amounts,
/// use `AssetAmount` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Amount(pub u64);

impl Amount {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Convert to AssetAmount with specified decimals
    pub fn to_asset_amount(self, decimals: u8) -> AssetAmount {
        AssetAmount {
            value: self.0 as u128,
            decimals,
        }
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as dollars with 2 decimal places (assuming cents)
        write!(f, "${:.2}", self.0 as f64 / 100.0)
    }
}

// ============================================================================
// High-Precision Asset Amount
// ============================================================================

/// Fixed-point decimal amount with configurable precision
///
/// Uses u128 for the value and u8 for decimal places, providing:
/// - Support for very large amounts (up to 340 undecillion base units)
/// - Configurable precision (0-18 decimal places typical)
/// - Safe arithmetic with overflow checking
/// - Display formatting respecting decimals
///
/// # Example
///
/// ```ignore
/// // $100.50 with 2 decimals (like USD cents)
/// let usd = AssetAmount::new(10050, 2);
/// assert_eq!(usd.to_string(), "100.50");
///
/// // 1.5 ETH with 18 decimals
/// let eth = AssetAmount::new(1_500_000_000_000_000_000, 18);
/// assert_eq!(eth.to_string(), "1.500000000000000000");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetAmount {
    /// Raw value in smallest units
    pub value: u128,
    /// Number of decimal places
    pub decimals: u8,
}

impl AssetAmount {
    /// Create a new AssetAmount
    pub fn new(value: u128, decimals: u8) -> Self {
        Self { value, decimals }
    }

    /// Create a zero amount with specified decimals
    pub fn zero(decimals: u8) -> Self {
        Self { value: 0, decimals }
    }

    /// Create from a human-readable amount (e.g., "100.50" -> 10050 with 2 decimals)
    pub fn from_human(human_value: f64, decimals: u8) -> Self {
        let multiplier = 10u128.pow(decimals as u32);
        let value = (human_value * multiplier as f64) as u128;
        Self { value, decimals }
    }

    /// Get the human-readable value (e.g., 10050 with 2 decimals -> 100.50)
    pub fn to_human(&self) -> f64 {
        let divisor = 10u128.pow(self.decimals as u32) as f64;
        self.value as f64 / divisor
    }

    /// Check if the amount is zero
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }

    /// Get the multiplier for this decimal precision
    pub fn multiplier(&self) -> u128 {
        10u128.pow(self.decimals as u32)
    }

    /// Scale this amount to a different decimal precision
    pub fn scale_to(&self, target_decimals: u8) -> Option<Self> {
        if target_decimals == self.decimals {
            return Some(*self);
        }

        if target_decimals > self.decimals {
            // Scale up (multiply)
            let diff = target_decimals - self.decimals;
            let multiplier = 10u128.pow(diff as u32);
            self.value.checked_mul(multiplier).map(|v| Self {
                value: v,
                decimals: target_decimals,
            })
        } else {
            // Scale down (divide with rounding)
            let diff = self.decimals - target_decimals;
            let divisor = 10u128.pow(diff as u32);
            Some(Self {
                value: self.value / divisor,
                decimals: target_decimals,
            })
        }
    }

    /// Checked addition (amounts must have same decimals)
    pub fn checked_add(self, other: Self) -> Option<Self> {
        if self.decimals != other.decimals {
            return None; // Decimals must match
        }
        self.value.checked_add(other.value).map(|v| Self {
            value: v,
            decimals: self.decimals,
        })
    }

    /// Checked subtraction (amounts must have same decimals)
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        if self.decimals != other.decimals {
            return None;
        }
        self.value.checked_sub(other.value).map(|v| Self {
            value: v,
            decimals: self.decimals,
        })
    }

    /// Checked multiplication by an integer
    pub fn checked_mul(self, multiplier: u128) -> Option<Self> {
        self.value.checked_mul(multiplier).map(|v| Self {
            value: v,
            decimals: self.decimals,
        })
    }

    /// Checked division by an integer
    pub fn checked_div(self, divisor: u128) -> Option<Self> {
        if divisor == 0 {
            return None;
        }
        Some(Self {
            value: self.value / divisor,
            decimals: self.decimals,
        })
    }

    /// Convert to the legacy Amount type (truncates to u64)
    pub fn to_amount(self) -> Option<Amount> {
        if self.value <= u64::MAX as u128 {
            Some(Amount(self.value as u64))
        } else {
            None
        }
    }

    /// IUSD amount (2 decimal places like USD)
    pub fn iusd(value: u128) -> Self {
        Self { value, decimals: 2 }
    }

    /// IUSD from human-readable value
    pub fn iusd_from_human(dollars: f64) -> Self {
        Self::from_human(dollars, 2)
    }
}

impl Default for AssetAmount {
    fn default() -> Self {
        Self::zero(2) // Default to 2 decimals like USD
    }
}

impl std::fmt::Display for AssetAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.decimals == 0 {
            write!(f, "{}", self.value)
        } else {
            let divisor = 10u128.pow(self.decimals as u32);
            let whole = self.value / divisor;
            let frac = self.value % divisor;
            write!(f, "{}.{:0>width$}", whole, frac, width = self.decimals as usize)
        }
    }
}

impl PartialOrd for AssetAmount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.decimals != other.decimals {
            // Scale to compare
            if let (Some(a), Some(b)) = (self.scale_to(18), other.scale_to(18)) {
                return a.value.partial_cmp(&b.value);
            }
            return None;
        }
        self.value.partial_cmp(&other.value)
    }
}

impl Ord for AssetAmount {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

// ============================================================================
// Budget Types
// ============================================================================

/// Unique identifier for a budget
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BudgetId(pub String);

impl BudgetId {
    pub fn new() -> Self {
        Self(format!("budget_{}", Uuid::new_v4()))
    }
}

impl Default for BudgetId {
    fn default() -> Self {
        Self::new()
    }
}

/// Spending rate limit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendRate {
    /// Maximum amount per time window
    pub max_amount: Amount,
    /// Time window in seconds
    pub window_seconds: u64,
}

/// Rules for counterparty interactions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterpartyPolicy {
    /// Explicit allowlist (if non-empty, only these are allowed)
    pub allowlist: Vec<ResonatorId>,
    /// Explicit denylist (always denied)
    pub denylist: Vec<ResonatorId>,
}

impl CounterpartyPolicy {
    pub fn allow_all() -> Self {
        Self {
            allowlist: vec![],
            denylist: vec![],
        }
    }

    pub fn is_allowed(&self, counterparty: &ResonatorId) -> bool {
        // Check denylist first
        if self.denylist.contains(counterparty) {
            return false;
        }
        // If allowlist is non-empty, must be in it
        if !self.allowlist.is_empty() && !self.allowlist.contains(counterparty) {
            return false;
        }
        true
    }
}

/// Rules for specific spend categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryRule {
    pub category: String,
    pub max_per_transaction: Amount,
    pub max_total: Amount,
}

/// What to do when budget limits are approached or exceeded
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationPolicy {
    /// Automatically deny
    AutoDeny,
    /// Hold for review (queue the intent)
    Hold,
    /// Allow but flag for audit
    AllowAndFlag,
}

/// Policy governing spending authority for an agent
///
/// Budgets are first-class objects, not settings.
/// They define bounded authority for economic actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetPolicy {
    pub budget_id: BudgetId,
    pub owner: ResonatorId,
    pub asset_filters: Vec<AssetClass>,
    pub rate_limit: SpendRate,
    pub max_total: Amount,
    pub spent_total: Amount,
    pub counterparty_rules: CounterpartyPolicy,
    pub category_rules: Vec<CategoryRule>,
    pub escalation: EscalationPolicy,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl BudgetPolicy {
    /// Create a new budget policy with sensible defaults
    pub fn new(owner: ResonatorId, max_total: Amount) -> Self {
        Self {
            budget_id: BudgetId::new(),
            owner,
            asset_filters: vec![AssetClass::Stablecoin],
            rate_limit: SpendRate {
                max_amount: Amount::new(10000), // $100/window
                window_seconds: 3600,           // 1 hour
            },
            max_total,
            spent_total: Amount::zero(),
            counterparty_rules: CounterpartyPolicy::allow_all(),
            category_rules: vec![],
            escalation: EscalationPolicy::AutoDeny,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    /// Check if an amount can be spent under this budget
    pub fn can_spend(&self, amount: Amount, counterparty: &ResonatorId) -> bool {
        // Check expiration
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }

        // Check total limit
        if let Some(new_total) = self.spent_total.checked_add(amount) {
            if new_total > self.max_total {
                return false;
            }
        } else {
            return false;
        }

        // Check counterparty
        if !self.counterparty_rules.is_allowed(counterparty) {
            return false;
        }

        true
    }

    /// Record a spend against this budget
    pub fn record_spend(&mut self, amount: Amount) -> crate::Result<()> {
        self.spent_total = self.spent_total.checked_add(amount).ok_or_else(|| {
            crate::CoreError::BudgetExceeded {
                message: "Overflow in budget tracking".to_string(),
            }
        })?;
        Ok(())
    }
}

// ============================================================================
// Permit Types
// ============================================================================

/// Unique identifier for a spend permit
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermitId(pub String);

impl PermitId {
    pub fn new() -> Self {
        Self(format!("permit_{}", Uuid::new_v4()))
    }
}

impl Default for PermitId {
    fn default() -> Self {
        Self::new()
    }
}

/// Constraint on who can receive funds under a permit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CounterpartyConstraint {
    /// Any counterparty (subject to budget rules)
    Any,
    /// Specific counterparty only
    Specific(ResonatorId),
    /// One of these counterparties
    OneOf(Vec<ResonatorId>),
}

impl CounterpartyConstraint {
    pub fn matches(&self, counterparty: &ResonatorId) -> bool {
        match self {
            Self::Any => true,
            Self::Specific(id) => id == counterparty,
            Self::OneOf(ids) => ids.contains(counterparty),
        }
    }
}

/// Purpose of the spend (for categorization and policy)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendPurpose {
    pub category: String,
    pub description: String,
}

/// A SpendPermit is the agent-native "currency of authority"
///
/// Permits are:
/// - Signed
/// - Expiring
/// - Bounded
/// - Purpose-scoped
/// - Verifiable by third parties
///
/// Agents trade permits, not raw money.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendPermit {
    pub permit_id: PermitId,
    pub issuer: ResonatorId,
    pub bound_budget: BudgetId,
    pub asset_class: AssetClass,
    pub max_amount: Amount,
    pub remaining: Amount,
    pub counterparty: CounterpartyConstraint,
    pub purpose: SpendPurpose,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub signature: String,
}

impl SpendPermit {
    /// Check if the permit is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at && !self.remaining.is_zero()
    }

    /// Check if the permit can cover a payment
    pub fn can_cover(
        &self,
        amount: Amount,
        counterparty: &ResonatorId,
        asset_class: &AssetClass,
    ) -> crate::Result<()> {
        if Utc::now() >= self.expires_at {
            return Err(crate::CoreError::PermitExpired {
                expired_at: self.expires_at.to_rfc3339(),
            });
        }

        if amount > self.remaining {
            return Err(crate::CoreError::PermitExceeded {
                requested: amount.0,
                remaining: self.remaining.0,
            });
        }

        if !self.counterparty.matches(counterparty) {
            return Err(crate::CoreError::PermitCounterpartyMismatch {
                counterparty: counterparty.to_string(),
            });
        }

        if &self.asset_class != asset_class {
            return Err(crate::CoreError::PermitAssetMismatch {
                asset_class: format!("{:?}", asset_class),
            });
        }

        Ok(())
    }

    /// Consume some amount from the permit
    pub fn consume(&mut self, amount: Amount) -> crate::Result<()> {
        self.remaining = self.remaining.checked_sub(amount).ok_or_else(|| {
            crate::CoreError::PermitExceeded {
                requested: amount.0,
                remaining: self.remaining.0,
            }
        })?;
        Ok(())
    }
}

// ============================================================================
// Payment Intent Types
// ============================================================================

/// Unique identifier for a payment intent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentId(pub String);

impl IntentId {
    pub fn new() -> Self {
        Self(format!("intent_{}", Uuid::new_v4()))
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

/// A proposed payment that has not yet been authorized
///
/// This is the "meaning" phase of the commitment boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub intent_id: IntentId,
    pub actor: ResonatorId,
    pub permit: PermitId,
    pub target: ResonatorId,
    pub amount: Amount,
    pub asset: AssetId,
    pub purpose: SpendPurpose,
    pub created_at: DateTime<Utc>,
}

impl PaymentIntent {
    pub fn new(
        actor: ResonatorId,
        permit: PermitId,
        target: ResonatorId,
        amount: Amount,
        asset: AssetId,
        purpose: SpendPurpose,
    ) -> Self {
        Self {
            intent_id: IntentId::new(),
            actor,
            permit,
            target,
            amount,
            asset,
            purpose,
            created_at: Utc::now(),
        }
    }
}

// ============================================================================
// Escrow Types
// ============================================================================

/// Unique identifier for an escrow
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EscrowId(pub String);

impl EscrowId {
    pub fn new() -> Self {
        Self(format!("escrow_{}", Uuid::new_v4()))
    }
}

impl Default for EscrowId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an invoice
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvoiceId(pub String);

impl InvoiceId {
    pub fn new() -> Self {
        Self(format!("invoice_{}", Uuid::new_v4()))
    }
}

impl Default for InvoiceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Condition that must be met for delivery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryCondition {
    pub condition_type: String,
    pub parameters: serde_json::Value,
}

/// Condition that must be met to release escrow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseCondition {
    pub condition_type: String,
    pub parameters: serde_json::Value,
    pub met: bool,
}

/// An invoice from a seller to a buyer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: InvoiceId,
    pub seller: ResonatorId,
    pub buyer: ResonatorId,
    pub asset: AssetId,
    pub amount: Amount,
    pub description: String,
    pub delivery_conditions: Vec<DeliveryCondition>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Intent to create an escrow for a payment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowIntent {
    pub escrow_id: EscrowId,
    pub invoice: InvoiceId,
    pub payer: ResonatorId,
    pub payee: ResonatorId,
    pub locked_amount: Amount,
    pub asset: AssetId,
    pub release_conditions: Vec<ReleaseCondition>,
    pub arbiter: Option<ResonatorId>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// State of an escrow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowState {
    /// Funds are locked
    Locked,
    /// Released to payee
    Released,
    /// Refunded to payer
    Refunded,
    /// In dispute, awaiting arbitration
    Disputed,
}

/// A live escrow holding funds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escrow {
    pub escrow_id: EscrowId,
    pub invoice_id: InvoiceId,
    pub payer: ResonatorId,
    pub payee: ResonatorId,
    pub amount: Amount,
    pub asset: AssetId,
    pub state: EscrowState,
    pub release_conditions: Vec<ReleaseCondition>,
    pub arbiter: Option<ResonatorId>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Escrow {
    /// Check if all release conditions are met
    pub fn conditions_met(&self) -> bool {
        self.release_conditions.iter().all(|c| c.met)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resonator_id_creation() {
        let id = ResonatorId::new();
        assert!(id.0.starts_with("res_"));
    }

    #[test]
    fn test_amount_operations() {
        let a = Amount::new(100);
        let b = Amount::new(50);

        assert_eq!(a.checked_add(b), Some(Amount::new(150)));
        assert_eq!(a.checked_sub(b), Some(Amount::new(50)));
        assert_eq!(b.checked_sub(a), None); // Would underflow
    }

    #[test]
    fn test_counterparty_policy() {
        let allowed = ResonatorId::from_string("allowed");
        let denied = ResonatorId::from_string("denied");
        let other = ResonatorId::from_string("other");

        let policy = CounterpartyPolicy {
            allowlist: vec![allowed.clone()],
            denylist: vec![denied.clone()],
        };

        assert!(policy.is_allowed(&allowed));
        assert!(!policy.is_allowed(&denied));
        assert!(!policy.is_allowed(&other)); // Not in allowlist
    }

    #[test]
    fn test_budget_policy_spend_limits() {
        let owner = ResonatorId::new();
        let counterparty = ResonatorId::new();
        let mut budget = BudgetPolicy::new(owner, Amount::new(10000));

        assert!(budget.can_spend(Amount::new(5000), &counterparty));
        budget.record_spend(Amount::new(5000)).unwrap();

        assert!(budget.can_spend(Amount::new(5000), &counterparty));
        assert!(!budget.can_spend(Amount::new(5001), &counterparty));
    }

    #[test]
    fn test_spend_permit_expiration() {
        let issuer = ResonatorId::new();
        let permit = SpendPermit {
            permit_id: PermitId::new(),
            issuer: issuer.clone(),
            bound_budget: BudgetId::new(),
            asset_class: AssetClass::Stablecoin,
            max_amount: Amount::new(1000),
            remaining: Amount::new(1000),
            counterparty: CounterpartyConstraint::Any,
            purpose: SpendPurpose {
                category: "test".to_string(),
                description: "Test permit".to_string(),
            },
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            signature: "test_sig".to_string(),
        };

        assert!(permit.is_valid());

        let expired_permit = SpendPermit {
            expires_at: Utc::now() - chrono::Duration::hours(1),
            ..permit
        };

        assert!(!expired_permit.is_valid());
    }

    #[test]
    fn test_asset_amount_creation() {
        // $100.50 with 2 decimals
        let amt = AssetAmount::new(10050, 2);
        assert_eq!(amt.value, 10050);
        assert_eq!(amt.decimals, 2);
        assert_eq!(amt.to_string(), "100.50");
    }

    #[test]
    fn test_asset_amount_from_human() {
        let amt = AssetAmount::from_human(100.50, 2);
        assert_eq!(amt.value, 10050);
        assert_eq!(amt.to_human(), 100.50);
    }

    #[test]
    fn test_asset_amount_arithmetic() {
        let a = AssetAmount::new(10000, 2); // $100.00
        let b = AssetAmount::new(5000, 2);  // $50.00

        // Addition
        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum.value, 15000);
        assert_eq!(sum.to_string(), "150.00");

        // Subtraction
        let diff = a.checked_sub(b).unwrap();
        assert_eq!(diff.value, 5000);
        assert_eq!(diff.to_string(), "50.00");

        // Multiplication
        let doubled = a.checked_mul(2).unwrap();
        assert_eq!(doubled.value, 20000);

        // Division
        let halved = a.checked_div(2).unwrap();
        assert_eq!(halved.value, 5000);
    }

    #[test]
    fn test_asset_amount_mismatched_decimals() {
        let a = AssetAmount::new(10000, 2);
        let b = AssetAmount::new(10000, 4);

        // Addition should fail with mismatched decimals
        assert!(a.checked_add(b).is_none());
    }

    #[test]
    fn test_asset_amount_scaling() {
        let amt = AssetAmount::new(10050, 2); // 100.50 with 2 decimals

        // Scale up to 4 decimals
        let scaled_up = amt.scale_to(4).unwrap();
        assert_eq!(scaled_up.value, 1005000);
        assert_eq!(scaled_up.decimals, 4);
        assert_eq!(scaled_up.to_string(), "100.5000");

        // Scale down to 1 decimal (loses precision)
        let scaled_down = amt.scale_to(1).unwrap();
        assert_eq!(scaled_down.value, 1005); // 100.5
        assert_eq!(scaled_down.decimals, 1);
    }

    #[test]
    fn test_asset_amount_iusd() {
        let amt = AssetAmount::iusd(10050); // 100.50 IUSD
        assert_eq!(amt.decimals, 2);
        assert_eq!(amt.to_string(), "100.50");

        let from_human = AssetAmount::iusd_from_human(100.50);
        assert_eq!(from_human.value, 10050);
    }

    #[test]
    fn test_asset_amount_to_legacy() {
        let amt = AssetAmount::new(10050, 2);
        let legacy = amt.to_amount().unwrap();
        assert_eq!(legacy.0, 10050);
    }

    #[test]
    fn test_amount_to_asset_amount() {
        let legacy = Amount::new(10050);
        let asset = legacy.to_asset_amount(2);
        assert_eq!(asset.value, 10050);
        assert_eq!(asset.decimals, 2);
    }

    #[test]
    fn test_asset_amount_comparison() {
        let a = AssetAmount::new(10000, 2);
        let b = AssetAmount::new(5000, 2);
        let c = AssetAmount::new(10000, 2);

        assert!(a > b);
        assert!(b < a);
        assert_eq!(a, c);
    }

    #[test]
    fn test_asset_amount_large_values() {
        // Test with very large values (u128 range)
        let large = AssetAmount::new(u128::MAX / 2, 18);
        assert!(!large.is_zero());

        // Should handle without overflow
        let sum = large.checked_add(AssetAmount::new(1, 18));
        assert!(sum.is_some());
    }
}
