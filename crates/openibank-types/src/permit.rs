//! Spend permit types for OpeniBank
//!
//! SpendPermits are the agent-native "currency of authority" - they define
//! bounded, expiring, purpose-scoped authorization to spend funds.

use crate::{
    AgentId, Amount, BudgetId, CompartmentId, Currency, OpeniBankError, PermitId,
    ResonatorId, Result, SpendingLimits, TemporalAnchor, WalletId,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Scope of what the permit allows
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermitScope {
    /// Currencies this permit can be used for
    pub currencies: Vec<Currency>,
    /// Recipient policy
    pub recipients: RecipientPolicy,
    /// Allowed payment channels
    pub channels: Vec<crate::PaymentChannel>,
    /// Specific compartments this permit can spend from
    pub compartments: Vec<CompartmentId>,
}

impl Default for PermitScope {
    fn default() -> Self {
        Self {
            currencies: vec![Currency::iusd()],
            recipients: RecipientPolicy::Any,
            channels: vec![],
            compartments: vec![],
        }
    }
}

/// Policy for allowed recipients
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipientPolicy {
    /// Any recipient allowed
    Any,
    /// Only specific recipients allowed
    Allowlist(Vec<WalletId>),
    /// All except specific recipients
    Denylist(Vec<WalletId>),
    /// Must match a pattern (for programmatic filtering)
    Pattern { pattern: String },
}

impl RecipientPolicy {
    /// Check if a recipient is allowed
    pub fn is_allowed(&self, recipient: &WalletId) -> bool {
        match self {
            Self::Any => true,
            Self::Allowlist(list) => list.contains(recipient),
            Self::Denylist(list) => !list.contains(recipient),
            Self::Pattern { .. } => true, // Pattern matching would need additional logic
        }
    }
}

/// Condition that must be met to use the permit
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermitCondition {
    /// Time window when permit can be used
    TimeWindow {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    /// Require multi-sig approval
    RequiresApproval { approvers: Vec<AgentId>, threshold: u32 },
    /// Rate limit (max N uses per window)
    RateLimit { max_uses: u32, window_seconds: u64 },
    /// Minimum delay between uses
    CooldownSeconds(u64),
    /// Custom condition
    Custom { name: String, parameters: serde_json::Value },
}

/// Purpose of the spend (for categorization and audit)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendPurpose {
    /// Category of spend
    pub category: SpendCategory,
    /// Human-readable description
    pub description: String,
    /// Reference to external system
    pub external_ref: Option<String>,
}

impl Default for SpendPurpose {
    fn default() -> Self {
        Self {
            category: SpendCategory::General,
            description: String::new(),
            external_ref: None,
        }
    }
}

/// Categories of spend purposes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpendCategory {
    /// General purpose
    General,
    /// Payment for goods
    Goods,
    /// Payment for services
    Services,
    /// Subscription/recurring
    Subscription,
    /// Salary/payroll
    Payroll,
    /// Tax payment
    Tax,
    /// Investment
    Investment,
    /// Transfer to self
    SelfTransfer,
    /// Refund
    Refund,
    /// Arena stake
    ArenaStake,
    /// Marketplace fee
    MarketplaceFee,
    /// Custom category
    Custom(String),
}

/// A SpendPermit is the agent-native "currency of authority"
///
/// Permits are:
/// - Cryptographically signed
/// - Time-bounded (expiring)
/// - Amount-bounded
/// - Purpose-scoped
/// - Verifiable by third parties
///
/// Agents trade permits, not raw money.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendPermit {
    /// Unique permit ID
    pub id: PermitId,
    /// Wallet this permit draws from
    pub wallet: WalletId,
    /// Agent this permit was granted to
    pub granted_to: AgentId,
    /// Resonator that issued this permit
    pub issuer: ResonatorId,
    /// Linked budget (optional)
    pub budget: Option<BudgetId>,
    /// Scope of what's allowed
    pub scope: PermitScope,
    /// Spending limits
    pub limits: SpendingLimits,
    /// Additional conditions
    pub conditions: Vec<PermitCondition>,
    /// Purpose of this permit
    pub purpose: SpendPurpose,
    /// When the permit was issued
    pub issued_at: DateTime<Utc>,
    /// When the permit expires
    pub expires_at: DateTime<Utc>,
    /// Whether the permit has been revoked
    pub revoked: bool,
    /// Cryptographic signature
    pub signature: String,
}

impl SpendPermit {
    /// Create a new permit with default settings
    pub fn new(
        wallet: WalletId,
        granted_to: AgentId,
        issuer: ResonatorId,
        max_amount: Amount,
        validity_hours: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: PermitId::new(),
            wallet,
            granted_to,
            issuer,
            budget: None,
            scope: PermitScope::default(),
            limits: SpendingLimits::total(max_amount),
            conditions: vec![],
            purpose: SpendPurpose::default(),
            issued_at: now,
            expires_at: now + Duration::hours(validity_hours),
            revoked: false,
            signature: String::new(), // To be signed
        }
    }

    /// Check if the permit is still valid
    pub fn is_valid(&self) -> bool {
        !self.revoked && Utc::now() < self.expires_at
    }

    /// Check if the permit has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the permit can cover a payment
    pub fn can_cover(
        &self,
        amount: &Amount,
        recipient: &WalletId,
        currency: &Currency,
    ) -> Result<()> {
        // Check if revoked
        if self.revoked {
            return Err(OpeniBankError::PermitRevoked {
                permit_id: self.id.to_string(),
            });
        }

        // Check expiration
        if self.is_expired() {
            return Err(OpeniBankError::PermitExpired {
                permit_id: self.id.to_string(),
                expired_at: self.expires_at.to_rfc3339(),
            });
        }

        // Check currency
        if !self.scope.currencies.is_empty() && !self.scope.currencies.contains(currency) {
            return Err(OpeniBankError::PermitCurrencyNotAllowed {
                permit_id: self.id.to_string(),
                currency: currency.symbol(),
            });
        }

        // Check recipient
        if !self.scope.recipients.is_allowed(recipient) {
            return Err(OpeniBankError::PermitRecipientNotAllowed {
                permit_id: self.id.to_string(),
                recipient: recipient.to_string(),
            });
        }

        // Check spending limits
        if !self.limits.can_spend(amount) {
            return Err(OpeniBankError::PermitLimitExceeded {
                permit_id: self.id.to_string(),
                requested: amount.to_human(),
                remaining: self.limits.remaining().map(|a| a.to_human()).unwrap_or(0.0),
            });
        }

        Ok(())
    }

    /// Revoke this permit
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Get remaining amount that can be spent
    pub fn remaining(&self) -> Option<Amount> {
        self.limits.remaining()
    }

    /// Get the time until expiration
    pub fn time_until_expiry(&self) -> Option<Duration> {
        let now = Utc::now();
        if now >= self.expires_at {
            None
        } else {
            Some(self.expires_at - now)
        }
    }
}

/// Builder for creating SpendPermits
#[derive(Debug, Clone)]
pub struct SpendPermitBuilder {
    wallet: WalletId,
    granted_to: AgentId,
    issuer: ResonatorId,
    budget: Option<BudgetId>,
    scope: PermitScope,
    limits: SpendingLimits,
    conditions: Vec<PermitCondition>,
    purpose: SpendPurpose,
    validity_hours: i64,
}

impl SpendPermitBuilder {
    /// Create a new builder
    pub fn new(wallet: WalletId, granted_to: AgentId, issuer: ResonatorId) -> Self {
        Self {
            wallet,
            granted_to,
            issuer,
            budget: None,
            scope: PermitScope::default(),
            limits: SpendingLimits::default(),
            conditions: vec![],
            purpose: SpendPurpose::default(),
            validity_hours: 24,
        }
    }

    /// Set the budget to link to
    pub fn with_budget(mut self, budget: BudgetId) -> Self {
        self.budget = Some(budget);
        self
    }

    /// Set the allowed currencies
    pub fn with_currencies(mut self, currencies: Vec<Currency>) -> Self {
        self.scope.currencies = currencies;
        self
    }

    /// Set the recipient policy
    pub fn with_recipients(mut self, policy: RecipientPolicy) -> Self {
        self.scope.recipients = policy;
        self
    }

    /// Set the spending limits
    pub fn with_limits(mut self, limits: SpendingLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Add a condition
    pub fn with_condition(mut self, condition: PermitCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set the purpose
    pub fn with_purpose(mut self, purpose: SpendPurpose) -> Self {
        self.purpose = purpose;
        self
    }

    /// Set validity duration in hours
    pub fn valid_for_hours(mut self, hours: i64) -> Self {
        self.validity_hours = hours;
        self
    }

    /// Build the permit
    pub fn build(self) -> SpendPermit {
        let now = Utc::now();
        SpendPermit {
            id: PermitId::new(),
            wallet: self.wallet,
            granted_to: self.granted_to,
            issuer: self.issuer,
            budget: self.budget,
            scope: self.scope,
            limits: self.limits,
            conditions: self.conditions,
            purpose: self.purpose,
            issued_at: now,
            expires_at: now + Duration::hours(self.validity_hours),
            revoked: false,
            signature: String::new(),
        }
    }
}

/// Permit delegation - allows a grantee to create sub-permits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermitDelegation {
    /// The original permit
    pub parent_permit: PermitId,
    /// Maximum percentage of parent that can be delegated (0-100)
    pub max_delegation_percent: u8,
    /// Whether the delegate can further delegate
    pub allow_sub_delegation: bool,
    /// Maximum depth of delegation chain
    pub max_delegation_depth: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permit_creation() {
        let permit = SpendPermit::new(
            WalletId::new(),
            AgentId::new(),
            ResonatorId::new(),
            Amount::iusd(1000.0),
            24,
        );

        assert!(permit.is_valid());
        assert!(!permit.is_expired());
    }

    #[test]
    fn test_permit_can_cover() {
        let wallet = WalletId::new();
        let recipient = WalletId::new();
        let permit = SpendPermit::new(
            wallet,
            AgentId::new(),
            ResonatorId::new(),
            Amount::iusd(1000.0),
            24,
        );

        // Should allow valid spend
        assert!(permit
            .can_cover(&Amount::iusd(500.0), &recipient, &Currency::iusd())
            .is_ok());

        // Should reject over-limit spend
        assert!(permit
            .can_cover(&Amount::iusd(1500.0), &recipient, &Currency::iusd())
            .is_err());
    }

    #[test]
    fn test_recipient_policy() {
        let allowed = WalletId::new();
        let other = WalletId::new();

        let policy = RecipientPolicy::Allowlist(vec![allowed.clone()]);
        assert!(policy.is_allowed(&allowed));
        assert!(!policy.is_allowed(&other));

        let deny_policy = RecipientPolicy::Denylist(vec![allowed.clone()]);
        assert!(!deny_policy.is_allowed(&allowed));
        assert!(deny_policy.is_allowed(&other));
    }

    #[test]
    fn test_permit_builder() {
        let permit = SpendPermitBuilder::new(WalletId::new(), AgentId::new(), ResonatorId::new())
            .with_currencies(vec![Currency::iusd(), Currency::eth()])
            .with_limits(SpendingLimits::daily(Amount::iusd(500.0)))
            .valid_for_hours(48)
            .build();

        assert_eq!(permit.scope.currencies.len(), 2);
        assert!(permit.time_until_expiry().unwrap() > Duration::hours(47));
    }
}
