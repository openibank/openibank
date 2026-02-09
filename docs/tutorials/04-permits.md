# Tutorial 4: Building with Permits

> **Duration**: 30 minutes
> **Difficulty**: Intermediate
> **Prerequisites**: Completed [Tutorial 2: Making Payments](./02-payments.md)

In this comprehensive tutorial, you will master OpeniBank's permit system - the foundation of safe AI agent spending. You will learn to design permit hierarchies, implement budget policies, set velocity limits, configure counterparty constraints, and apply real-world authorization patterns.

---

## Learning Objectives

By the end of this tutorial, you will be able to:

1. Design effective permit hierarchies for different use cases
2. Create and manage BudgetPolicies with spending controls
3. Implement velocity limits to prevent rapid fund depletion
4. Configure counterparty constraints for secure payments
5. Apply real-world patterns for agent authorization

---

## The Permit Philosophy

Before diving into implementation, let's understand why permits exist.

### The Problem: Unbounded AI Spending

```
Without Permits:
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  AI Agent: "Transfer $10,000 to unknown-recipient"                  │
│       │                                                              │
│       ▼                                                              │
│  [EXECUTED] ──▶ Money gone, possibly to malicious actor             │
│                                                                      │
│  Problems:                                                           │
│  • No spending limits                                               │
│  • No recipient validation                                          │
│  • No purpose tracking                                              │
│  • No audit trail                                                   │
│  • Compromised LLM = drained account                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### The Solution: Bounded Permits

```
With Permits:
┌─────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  AI Agent: "Transfer $10,000 to unknown-recipient"                  │
│       │                                                              │
│       ▼                                                              │
│  Commitment Gate: "Let me check the permit..."                      │
│       │                                                              │
│       ├─ ✗ Permit max is $500                                       │
│       ├─ ✗ Recipient not in allow list                              │
│       └─ ✗ Purpose doesn't match                                    │
│       │                                                              │
│       ▼                                                              │
│  [REJECTED] ──▶ Money safe, attack thwarted                         │
│                                                                      │
│  Benefits:                                                           │
│  • Hard spending limits                                             │
│  • Validated recipients                                             │
│  • Tracked purpose                                                  │
│  • Complete audit trail                                             │
│  • Compromised LLM = limited damage                                 │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Part 1: Permit Hierarchy Design

Real-world agent operations require multiple levels of spending authorization.

### Hierarchy Concept

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Permit Hierarchy                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Organization Budget ($100,000/month)                               │
│  └─────────────────────────────────────────────────────────────────┤
│        │                                                             │
│        ├── Team Budget: Engineering ($30,000/month)                 │
│        │   │                                                         │
│        │   ├── Agent Budget: build-agent ($5,000/month)             │
│        │   │   ├── Permit: Cloud Services ($2,000, AWS/GCP only)    │
│        │   │   ├── Permit: SaaS Tools ($1,000, approved list)       │
│        │   │   └── Permit: General ($500, any verified vendor)      │
│        │   │                                                         │
│        │   └── Agent Budget: test-agent ($3,000/month)              │
│        │       └── Permit: Test Infrastructure ($3,000, test envs)  │
│        │                                                             │
│        ├── Team Budget: Data ($20,000/month)                        │
│        │   └── Agent Budget: data-agent ($10,000/month)             │
│        │       ├── Permit: Data Providers ($8,000, approved list)   │
│        │       └── Permit: Processing ($2,000, compute services)    │
│        │                                                             │
│        └── Team Budget: Operations ($10,000/month)                  │
│            └── ...                                                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Implementing a Hierarchy

```rust
use openibank_guard::{
    BudgetPolicy, SpendPermit, BudgetId, PermitId,
    CounterpartyConstraint, SpendPurpose, VelocityLimit,
};
use openibank_core::{Amount, ResonatorId};
use chrono::{Utc, Duration};

/// Create a complete budget hierarchy for an organization
async fn create_organization_hierarchy(
    org_id: &ResonatorId,
) -> Result<OrganizationHierarchy, HierarchyError> {

    // Level 1: Organization Master Budget
    let org_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("org"),
        owner: org_id.clone(),
        max_total: Amount::from_dollars(100_000),
        max_single: Amount::from_dollars(10_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(20_000),
            max_per_day: Amount::from_dollars(50_000),
        }),
        allow_negative: false,
        parent_budget: None, // Top level
    };

    // Level 2: Team Budgets (children of org budget)
    let engineering_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("team-eng"),
        owner: org_id.clone(),
        max_total: Amount::from_dollars(30_000),
        max_single: Amount::from_dollars(5_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(10_000),
            max_per_day: Amount::from_dollars(20_000),
        }),
        allow_negative: false,
        parent_budget: Some(org_budget.budget_id.clone()),
    };

    let data_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("team-data"),
        owner: org_id.clone(),
        max_total: Amount::from_dollars(20_000),
        max_single: Amount::from_dollars(8_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(5_000),
            max_per_day: Amount::from_dollars(15_000),
        }),
        allow_negative: false,
        parent_budget: Some(org_budget.budget_id.clone()),
    };

    // Level 3: Agent Budgets (children of team budgets)
    let build_agent_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("agent-build"),
        owner: ResonatorId::from_string("agent-build"),
        max_total: Amount::from_dollars(5_000),
        max_single: Amount::from_dollars(2_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(2_000),
            max_per_day: Amount::from_dollars(5_000),
        }),
        allow_negative: false,
        parent_budget: Some(engineering_budget.budget_id.clone()),
    };

    Ok(OrganizationHierarchy {
        org_budget,
        team_budgets: vec![engineering_budget, data_budget],
        agent_budgets: vec![build_agent_budget],
    })
}
```

### Hierarchical Budget Validation

When a payment is made, the system validates the entire hierarchy:

```rust
/// Validate a payment against budget hierarchy
async fn validate_against_hierarchy(
    budget_service: &BudgetService,
    permit: &SpendPermit,
    amount: Amount,
) -> Result<(), BudgetError> {
    // Get the permit's bound budget
    let budget = budget_service.get_budget(&permit.bound_budget).await?;

    // Check this level
    validate_single_budget(&budget, amount)?;

    // Check parent budgets recursively
    let mut current = budget;
    while let Some(parent_id) = &current.parent_budget {
        let parent = budget_service.get_budget(parent_id).await?;
        validate_single_budget(&parent, amount)?;
        current = parent;
    }

    Ok(())
}

fn validate_single_budget(budget: &BudgetPolicy, amount: Amount) -> Result<(), BudgetError> {
    // Check single transaction limit
    if amount > budget.max_single {
        return Err(BudgetError::SingleTransactionExceeded {
            budget_id: budget.budget_id.clone(),
            amount,
            max: budget.max_single,
        });
    }

    // Check total remaining
    if amount > budget.remaining() {
        return Err(BudgetError::InsufficientBudget {
            budget_id: budget.budget_id.clone(),
            amount,
            remaining: budget.remaining(),
        });
    }

    // Check velocity (if configured)
    if let Some(velocity) = &budget.velocity_limit {
        let spent_this_hour = budget.spent_in_period(Duration::hours(1));
        if spent_this_hour + amount > velocity.max_per_hour {
            return Err(BudgetError::VelocityExceeded {
                budget_id: budget.budget_id.clone(),
                period: "hour".into(),
                current: spent_this_hour,
                attempted: amount,
                max: velocity.max_per_hour,
            });
        }
    }

    Ok(())
}
```

---

## Part 2: Budget Policies Deep Dive

Budget policies define spending constraints at a higher level than individual permits.

### BudgetPolicy Structure

```rust
pub struct BudgetPolicy {
    // Identity
    pub budget_id: BudgetId,
    pub owner: ResonatorId,

    // Spending limits
    pub max_total: Amount,         // Total budget allocation
    pub max_single: Amount,        // Max per transaction

    // Rate limiting
    pub velocity_limit: Option<VelocityLimit>,

    // Hierarchy
    pub parent_budget: Option<BudgetId>,

    // Behavioral flags
    pub allow_negative: bool,      // Can go below zero (credit)

    // Tracking (internal)
    spent_total: Amount,
    spending_history: Vec<SpendingRecord>,
}

pub struct VelocityLimit {
    pub max_per_hour: Amount,
    pub max_per_day: Amount,
    pub max_per_week: Option<Amount>,
    pub max_per_month: Option<Amount>,
}

struct SpendingRecord {
    amount: Amount,
    timestamp: DateTime<Utc>,
    permit_id: PermitId,
    receipt_id: String,
}
```

### Creating Budget Policies via API

```bash
# Create an agent budget
curl -X POST http://localhost:8080/api/budgets \
  -H "Content-Type: application/json" \
  -d '{
    "owner": "agent-alice",
    "max_total": 500000,
    "max_single": 50000,
    "velocity_limit": {
      "max_per_hour": 100000,
      "max_per_day": 300000
    },
    "allow_negative": false
  }'
```

**Response:**
```json
{
  "budget_id": "budget-a1b2c3",
  "owner": "agent-alice",
  "max_total": 500000,
  "max_single": 50000,
  "remaining": 500000,
  "spent": 0,
  "velocity_limit": {
    "max_per_hour": 100000,
    "max_per_day": 300000
  },
  "created_at": "2025-02-08T10:00:00Z"
}
```

### Budget State Tracking

```rust
impl BudgetPolicy {
    /// Get remaining budget
    pub fn remaining(&self) -> Amount {
        if self.spent_total >= self.max_total {
            Amount::zero()
        } else {
            self.max_total - self.spent_total
        }
    }

    /// Get amount spent in a time period
    pub fn spent_in_period(&self, period: Duration) -> Amount {
        let cutoff = Utc::now() - period;
        self.spending_history
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .map(|r| r.amount)
            .fold(Amount::zero(), |a, b| a + b)
    }

    /// Record a spend
    pub fn record_spend(&mut self, amount: Amount, permit_id: PermitId, receipt_id: String) {
        self.spent_total = self.spent_total + amount;
        self.spending_history.push(SpendingRecord {
            amount,
            timestamp: Utc::now(),
            permit_id,
            receipt_id,
        });
    }

    /// Check if a spend is allowed
    pub fn can_spend(&self, amount: Amount) -> Result<(), BudgetError> {
        // Check total
        if amount > self.remaining() {
            return Err(BudgetError::InsufficientBudget {
                remaining: self.remaining(),
                requested: amount,
            });
        }

        // Check single limit
        if amount > self.max_single {
            return Err(BudgetError::SingleTransactionExceeded {
                max: self.max_single,
                requested: amount,
            });
        }

        // Check velocity
        if let Some(velocity) = &self.velocity_limit {
            let hourly = self.spent_in_period(Duration::hours(1));
            if hourly + amount > velocity.max_per_hour {
                return Err(BudgetError::HourlyVelocityExceeded {
                    current: hourly,
                    requested: amount,
                    max: velocity.max_per_hour,
                });
            }

            let daily = self.spent_in_period(Duration::days(1));
            if daily + amount > velocity.max_per_day {
                return Err(BudgetError::DailyVelocityExceeded {
                    current: daily,
                    requested: amount,
                    max: velocity.max_per_day,
                });
            }
        }

        Ok(())
    }
}
```

---

## Part 3: Velocity Limits

Velocity limits prevent rapid fund depletion, even when individual transactions are within limits.

### Why Velocity Limits?

```
Scenario: Agent with $10,000 budget, $100 max per transaction

Without Velocity Limits:
  10:00 - Transfer $100 ✓
  10:01 - Transfer $100 ✓
  10:02 - Transfer $100 ✓
  ...
  10:99 - Transfer $100 ✓

  Result: $10,000 drained in 100 minutes!

With Velocity Limits ($500/hour):
  10:00 - Transfer $100 ✓ (hourly: $100)
  10:01 - Transfer $100 ✓ (hourly: $200)
  10:02 - Transfer $100 ✓ (hourly: $300)
  10:03 - Transfer $100 ✓ (hourly: $400)
  10:04 - Transfer $100 ✓ (hourly: $500)
  10:05 - Transfer $100 ✗ VELOCITY EXCEEDED

  Result: Max $500/hour, attack contained!
```

### Implementing Velocity Controls

```rust
use std::collections::VecDeque;
use chrono::{DateTime, Utc, Duration};

pub struct VelocityTracker {
    window_size: Duration,
    max_amount: Amount,
    transactions: VecDeque<(DateTime<Utc>, Amount)>,
}

impl VelocityTracker {
    pub fn new(window_size: Duration, max_amount: Amount) -> Self {
        Self {
            window_size,
            max_amount,
            transactions: VecDeque::new(),
        }
    }

    /// Clean up old transactions outside the window
    fn cleanup(&mut self) {
        let cutoff = Utc::now() - self.window_size;
        while let Some((timestamp, _)) = self.transactions.front() {
            if *timestamp < cutoff {
                self.transactions.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get current total in window
    pub fn current_total(&mut self) -> Amount {
        self.cleanup();
        self.transactions.iter().map(|(_, amt)| *amt).fold(Amount::zero(), |a, b| a + b)
    }

    /// Check if amount is allowed
    pub fn can_spend(&mut self, amount: Amount) -> bool {
        self.current_total() + amount <= self.max_amount
    }

    /// Record a transaction
    pub fn record(&mut self, amount: Amount) {
        self.cleanup();
        self.transactions.push_back((Utc::now(), amount));
    }
}

// Usage in budget
pub struct BudgetWithVelocity {
    budget: BudgetPolicy,
    hourly_tracker: VelocityTracker,
    daily_tracker: VelocityTracker,
}

impl BudgetWithVelocity {
    pub fn new(budget: BudgetPolicy) -> Self {
        let velocity = budget.velocity_limit.clone().unwrap_or_default();
        Self {
            budget,
            hourly_tracker: VelocityTracker::new(
                Duration::hours(1),
                velocity.max_per_hour,
            ),
            daily_tracker: VelocityTracker::new(
                Duration::days(1),
                velocity.max_per_day,
            ),
        }
    }

    pub fn can_spend(&mut self, amount: Amount) -> Result<(), BudgetError> {
        // Check budget limits
        self.budget.can_spend(amount)?;

        // Check hourly velocity
        if !self.hourly_tracker.can_spend(amount) {
            return Err(BudgetError::HourlyVelocityExceeded {
                current: self.hourly_tracker.current_total(),
                requested: amount,
                max: self.hourly_tracker.max_amount,
            });
        }

        // Check daily velocity
        if !self.daily_tracker.can_spend(amount) {
            return Err(BudgetError::DailyVelocityExceeded {
                current: self.daily_tracker.current_total(),
                requested: amount,
                max: self.daily_tracker.max_amount,
            });
        }

        Ok(())
    }

    pub fn record_spend(&mut self, amount: Amount, permit_id: PermitId, receipt_id: String) {
        self.budget.record_spend(amount, permit_id, receipt_id);
        self.hourly_tracker.record(amount);
        self.daily_tracker.record(amount);
    }
}
```

### Velocity Configuration Patterns

```rust
// Conservative: For new or untrusted agents
let conservative = VelocityLimit {
    max_per_hour: Amount::from_dollars(100),
    max_per_day: Amount::from_dollars(500),
    max_per_week: Some(Amount::from_dollars(2_000)),
    max_per_month: Some(Amount::from_dollars(5_000)),
};

// Standard: For established agents
let standard = VelocityLimit {
    max_per_hour: Amount::from_dollars(1_000),
    max_per_day: Amount::from_dollars(5_000),
    max_per_week: Some(Amount::from_dollars(20_000)),
    max_per_month: Some(Amount::from_dollars(50_000)),
};

// High-volume: For trusted automated systems
let high_volume = VelocityLimit {
    max_per_hour: Amount::from_dollars(10_000),
    max_per_day: Amount::from_dollars(50_000),
    max_per_week: None, // No weekly limit
    max_per_month: Some(Amount::from_dollars(500_000)),
};

// Burst-friendly: For occasional large transactions
let burst_friendly = VelocityLimit {
    max_per_hour: Amount::from_dollars(5_000),  // Allow bursts
    max_per_day: Amount::from_dollars(10_000),  // But limit daily
    max_per_week: Some(Amount::from_dollars(30_000)),
    max_per_month: Some(Amount::from_dollars(100_000)),
};
```

---

## Part 4: Counterparty Constraints

Counterparty constraints control **who** can receive payments from a permit.

### Constraint Types

```rust
pub enum CounterpartyConstraint {
    /// Any recipient is allowed
    Any,

    /// Only this specific recipient
    Specific(ResonatorId),

    /// Any member of a verified category
    Category(String),

    /// Only recipients in this list
    AllowList(Vec<ResonatorId>),

    /// Any recipient except these
    DenyList(Vec<ResonatorId>),

    /// Recipients matching a pattern
    Pattern(String),

    /// Composite constraints
    And(Box<CounterpartyConstraint>, Box<CounterpartyConstraint>),
    Or(Box<CounterpartyConstraint>, Box<CounterpartyConstraint>),
    Not(Box<CounterpartyConstraint>),
}
```

### Implementing Constraint Validation

```rust
impl CounterpartyConstraint {
    /// Check if a recipient is allowed
    pub fn allows(&self, recipient: &ResonatorId, registry: &VendorRegistry) -> bool {
        match self {
            Self::Any => true,

            Self::Specific(allowed) => recipient == allowed,

            Self::Category(category) => {
                registry.is_member(recipient, category)
            }

            Self::AllowList(list) => {
                list.contains(recipient)
            }

            Self::DenyList(list) => {
                !list.contains(recipient)
            }

            Self::Pattern(pattern) => {
                // Match against ResonatorId pattern
                let regex = Regex::new(pattern).unwrap();
                regex.is_match(&recipient.id)
            }

            Self::And(a, b) => {
                a.allows(recipient, registry) && b.allows(recipient, registry)
            }

            Self::Or(a, b) => {
                a.allows(recipient, registry) || b.allows(recipient, registry)
            }

            Self::Not(inner) => {
                !inner.allows(recipient, registry)
            }
        }
    }
}
```

### Common Constraint Patterns

```rust
// Pattern 1: Single vendor relationship
let single_vendor = CounterpartyConstraint::Specific(
    ResonatorId::from_string("vendor-acme-corp")
);

// Pattern 2: Approved vendor list
let approved_vendors = CounterpartyConstraint::AllowList(vec![
    ResonatorId::from_string("vendor-aws"),
    ResonatorId::from_string("vendor-gcp"),
    ResonatorId::from_string("vendor-azure"),
]);

// Pattern 3: Category-based (verified merchants)
let verified_merchants = CounterpartyConstraint::Category(
    "verified-merchant".to_string()
);

// Pattern 4: Exclude known bad actors
let exclude_suspicious = CounterpartyConstraint::DenyList(vec![
    ResonatorId::from_string("suspicious-001"),
    ResonatorId::from_string("blacklisted-002"),
]);

// Pattern 5: Internal transfers only
let internal_only = CounterpartyConstraint::Pattern(
    r"^agent-internal-.*$".to_string()
);

// Pattern 6: Composite - verified AND not blacklisted
let safe_merchants = CounterpartyConstraint::And(
    Box::new(CounterpartyConstraint::Category("verified-merchant".into())),
    Box::new(CounterpartyConstraint::Not(
        Box::new(CounterpartyConstraint::DenyList(blacklist.clone()))
    )),
);

// Pattern 7: Either specific vendor OR verified category
let flexible = CounterpartyConstraint::Or(
    Box::new(CounterpartyConstraint::Specific(primary_vendor.clone())),
    Box::new(CounterpartyConstraint::Category("backup-vendor".into())),
);
```

### Creating Permits with Constraints via API

```bash
# Permit for specific vendor
curl -X POST http://localhost:8080/api/permits \
  -H "Content-Type: application/json" \
  -d '{
    "issuer": "agent-alice",
    "max_amount": 10000,
    "counterparty": {
      "type": "specific",
      "target": "vendor-aws"
    },
    "purpose": "AWS cloud services"
  }'

# Permit for allow list
curl -X POST http://localhost:8080/api/permits \
  -H "Content-Type: application/json" \
  -d '{
    "issuer": "agent-alice",
    "max_amount": 50000,
    "counterparty": {
      "type": "allow_list",
      "targets": ["vendor-aws", "vendor-gcp", "vendor-azure"]
    },
    "purpose": "Cloud infrastructure"
  }'

# Permit for verified category
curl -X POST http://localhost:8080/api/permits \
  -H "Content-Type: application/json" \
  -d '{
    "issuer": "agent-alice",
    "max_amount": 5000,
    "counterparty": {
      "type": "category",
      "category": "verified-merchant"
    },
    "purpose": "General SaaS services"
  }'
```

---

## Part 5: Real-World Patterns

Let's look at complete patterns for common use cases.

### Pattern 1: Subscription Management Agent

```rust
/// Agent that manages recurring subscriptions
fn create_subscription_agent_permits(
    agent_id: &ResonatorId,
    budget: &BudgetPolicy,
) -> Vec<SpendPermit> {
    let mut permits = Vec::new();

    // Permit for monthly subscriptions (auto-renewal)
    permits.push(SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(500),
        remaining: Amount::from_dollars(500),
        counterparty: CounterpartyConstraint::Category("subscription-service".into()),
        purpose: SpendPurpose::RecurringPayment("Monthly SaaS subscriptions".into()),
        expires_at: Utc::now() + Duration::days(30),
        auto_renew: true,
        signature: String::new(),
    }.sign(&agent_keypair));

    // Permit for one-time purchases (manual approval required)
    permits.push(SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(100),
        remaining: Amount::from_dollars(100),
        counterparty: CounterpartyConstraint::Category("verified-merchant".into()),
        purpose: SpendPurpose::OneTimePurchase("Discretionary tools".into()),
        expires_at: Utc::now() + Duration::days(7),
        auto_renew: false,
        signature: String::new(),
    }.sign(&agent_keypair));

    permits
}
```

### Pattern 2: Multi-Stage Project Agent

```rust
/// Agent working on a project with phased funding
fn create_project_agent_permits(
    agent_id: &ResonatorId,
    project_id: &str,
    phases: &[ProjectPhase],
) -> Result<ProjectPermits, PermitError> {
    let mut phase_permits = Vec::new();

    for phase in phases {
        // Create budget for this phase
        let phase_budget = BudgetPolicy {
            budget_id: BudgetId::new_with_prefix(&format!("project-{}-phase-{}", project_id, phase.number)),
            owner: agent_id.clone(),
            max_total: phase.budget,
            max_single: phase.max_single_transaction,
            velocity_limit: Some(VelocityLimit {
                max_per_hour: phase.budget / 10,
                max_per_day: phase.budget / 3,
            }),
            allow_negative: false,
            parent_budget: None,
        };

        // Create permits for approved vendors
        let vendor_permit = SpendPermit {
            permit_id: PermitId::new(),
            issuer: agent_id.clone(),
            bound_budget: phase_budget.budget_id.clone(),
            asset_class: AssetClass::IUSD,
            max_amount: phase.budget,
            remaining: phase.budget,
            counterparty: CounterpartyConstraint::AllowList(phase.approved_vendors.clone()),
            purpose: SpendPurpose::ProjectExpense(format!("Phase {}: {}", phase.number, phase.description)),
            expires_at: phase.deadline,
            auto_renew: false,
            signature: String::new(),
        }.sign(&agent_keypair);

        phase_permits.push(PhasePermit {
            phase: phase.clone(),
            budget: phase_budget,
            permit: vendor_permit,
        });
    }

    Ok(ProjectPermits {
        project_id: project_id.to_string(),
        phases: phase_permits,
    })
}

struct ProjectPhase {
    number: u32,
    description: String,
    budget: Amount,
    max_single_transaction: Amount,
    approved_vendors: Vec<ResonatorId>,
    deadline: DateTime<Utc>,
}
```

### Pattern 3: Trading Agent with Risk Controls

```rust
/// Agent that trades with strict risk management
fn create_trading_agent_permits(
    agent_id: &ResonatorId,
    risk_profile: &RiskProfile,
) -> TradingPermits {
    // Master trading budget
    let trading_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("trading"),
        owner: agent_id.clone(),
        max_total: risk_profile.max_portfolio_value,
        max_single: risk_profile.max_position_size,
        velocity_limit: Some(VelocityLimit {
            max_per_hour: risk_profile.max_hourly_volume,
            max_per_day: risk_profile.max_daily_volume,
        }),
        allow_negative: false,
        parent_budget: None,
    };

    // Permit for market orders (tighter limits)
    let market_order_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: trading_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: risk_profile.max_position_size / 2, // Half size for market orders
        remaining: risk_profile.max_position_size / 2,
        counterparty: CounterpartyConstraint::Category("exchange".into()),
        purpose: SpendPurpose::Trading("Market orders".into()),
        expires_at: Utc::now() + Duration::hours(24), // Daily refresh
        auto_renew: true,
        signature: String::new(),
    }.sign(&agent_keypair);

    // Permit for limit orders (larger limits)
    let limit_order_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: trading_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: risk_profile.max_position_size,
        remaining: risk_profile.max_position_size,
        counterparty: CounterpartyConstraint::Category("exchange".into()),
        purpose: SpendPurpose::Trading("Limit orders".into()),
        expires_at: Utc::now() + Duration::hours(24),
        auto_renew: true,
        signature: String::new(),
    }.sign(&agent_keypair);

    // Permit for fee payments (small, for exchange fees)
    let fee_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: trading_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(100), // $100 daily fee budget
        remaining: Amount::from_dollars(100),
        counterparty: CounterpartyConstraint::Category("exchange-fee".into()),
        purpose: SpendPurpose::Fee("Trading fees".into()),
        expires_at: Utc::now() + Duration::hours(24),
        auto_renew: true,
        signature: String::new(),
    }.sign(&agent_keypair);

    TradingPermits {
        budget: trading_budget,
        market_order_permit,
        limit_order_permit,
        fee_permit,
    }
}

struct RiskProfile {
    max_portfolio_value: Amount,
    max_position_size: Amount,
    max_hourly_volume: Amount,
    max_daily_volume: Amount,
    max_drawdown_percent: f64,
}
```

### Pattern 4: Service Provider Agent

```rust
/// Agent that provides services and receives payments
fn create_service_provider_permits(
    agent_id: &ResonatorId,
) -> ServiceProviderConfig {
    // Operating expense budget
    let opex_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("opex"),
        owner: agent_id.clone(),
        max_total: Amount::from_dollars(10_000),
        max_single: Amount::from_dollars(1_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(2_000),
            max_per_day: Amount::from_dollars(5_000),
        }),
        allow_negative: false,
        parent_budget: None,
    };

    // Permit for infrastructure costs
    let infra_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: opex_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(5_000),
        remaining: Amount::from_dollars(5_000),
        counterparty: CounterpartyConstraint::AllowList(vec![
            ResonatorId::from_string("vendor-aws"),
            ResonatorId::from_string("vendor-cloudflare"),
            ResonatorId::from_string("vendor-datadog"),
        ]),
        purpose: SpendPurpose::Infrastructure("Cloud and monitoring".into()),
        expires_at: Utc::now() + Duration::days(30),
        auto_renew: true,
        signature: String::new(),
    }.sign(&agent_keypair);

    // Permit for data sources
    let data_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: opex_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(3_000),
        remaining: Amount::from_dollars(3_000),
        counterparty: CounterpartyConstraint::Category("data-provider".into()),
        purpose: SpendPurpose::DataAcquisition("External data sources".into()),
        expires_at: Utc::now() + Duration::days(30),
        auto_renew: true,
        signature: String::new(),
    }.sign(&agent_keypair);

    ServiceProviderConfig {
        budget: opex_budget,
        permits: vec![infra_permit, data_permit],
    }
}
```

---

## Complete Working Example

```rust
//! Complete Permits Tutorial
//!
//! Run with: cargo run --example permits

use openibank_guard::{
    BudgetPolicy, SpendPermit, BudgetId, PermitId,
    CounterpartyConstraint, SpendPurpose, VelocityLimit,
    CommitmentGate,
};
use openibank_core::{Amount, ResonatorId, PaymentIntent};
use openibank_issuer::Issuer;
use openibank_ledger::Ledger;
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpeniBank Permits Tutorial ===\n");

    // Initialize
    let ledger = Ledger::new_in_memory();
    let issuer = Issuer::new(ledger.clone());
    let gate = CommitmentGate::new();

    let agent_id = ResonatorId::from_string("agent-alice");
    let vendor_aws = ResonatorId::from_string("vendor-aws");
    let vendor_gcp = ResonatorId::from_string("vendor-gcp");
    let vendor_random = ResonatorId::from_string("vendor-random");

    // Fund the agent
    issuer.mint(agent_id.clone(), Amount::from_dollars(10_000)).await?;

    // ========================================
    // Part 1: Create Budget Hierarchy
    // ========================================
    println!("Part 1: Creating Budget Hierarchy\n");

    let main_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("main"),
        owner: agent_id.clone(),
        max_total: Amount::from_dollars(5_000),
        max_single: Amount::from_dollars(1_000),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(2_000),
            max_per_day: Amount::from_dollars(4_000),
        }),
        allow_negative: false,
        parent_budget: None,
    };

    println!("Created budget: {}", main_budget.budget_id);
    println!("  Max total:  ${:.2}", main_budget.max_total.as_dollars());
    println!("  Max single: ${:.2}", main_budget.max_single.as_dollars());
    println!("  Velocity:   ${:.2}/hour, ${:.2}/day",
        main_budget.velocity_limit.as_ref().unwrap().max_per_hour.as_dollars(),
        main_budget.velocity_limit.as_ref().unwrap().max_per_day.as_dollars()
    );

    // ========================================
    // Part 2: Create Permits with Different Constraints
    // ========================================
    println!("\nPart 2: Creating Permits\n");

    // Permit 1: Specific vendor (AWS only)
    let aws_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: main_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(500),
        remaining: Amount::from_dollars(500),
        counterparty: CounterpartyConstraint::Specific(vendor_aws.clone()),
        purpose: SpendPurpose::ServicePayment("AWS cloud services".into()),
        expires_at: Utc::now() + Duration::days(30),
        signature: String::new(),
    }.sign(&agent_keypair);

    println!("Permit 1 (AWS only): {}", aws_permit.permit_id);
    println!("  Max: ${:.2}", aws_permit.max_amount.as_dollars());
    println!("  Constraint: Specific(vendor-aws)");

    // Permit 2: Allow list (AWS or GCP)
    let cloud_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: main_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(1_000),
        remaining: Amount::from_dollars(1_000),
        counterparty: CounterpartyConstraint::AllowList(vec![
            vendor_aws.clone(),
            vendor_gcp.clone(),
        ]),
        purpose: SpendPurpose::ServicePayment("Cloud infrastructure".into()),
        expires_at: Utc::now() + Duration::days(30),
        signature: String::new(),
    }.sign(&agent_keypair);

    println!("\nPermit 2 (Cloud providers): {}", cloud_permit.permit_id);
    println!("  Max: ${:.2}", cloud_permit.max_amount.as_dollars());
    println!("  Constraint: AllowList([vendor-aws, vendor-gcp])");

    // ========================================
    // Part 3: Test Constraint Validation
    // ========================================
    println!("\nPart 3: Testing Constraints\n");

    // Test 1: AWS permit -> AWS vendor (should pass)
    println!("Test 1: AWS permit -> AWS vendor");
    let result = aws_permit.counterparty.allows(&vendor_aws, &registry);
    println!("  Result: {}", if result { "ALLOWED" } else { "DENIED" });

    // Test 2: AWS permit -> GCP vendor (should fail)
    println!("\nTest 2: AWS permit -> GCP vendor");
    let result = aws_permit.counterparty.allows(&vendor_gcp, &registry);
    println!("  Result: {}", if result { "ALLOWED" } else { "DENIED" });

    // Test 3: Cloud permit -> AWS vendor (should pass)
    println!("\nTest 3: Cloud permit -> AWS vendor");
    let result = cloud_permit.counterparty.allows(&vendor_aws, &registry);
    println!("  Result: {}", if result { "ALLOWED" } else { "DENIED" });

    // Test 4: Cloud permit -> random vendor (should fail)
    println!("\nTest 4: Cloud permit -> random vendor");
    let result = cloud_permit.counterparty.allows(&vendor_random, &registry);
    println!("  Result: {}", if result { "ALLOWED" } else { "DENIED" });

    // ========================================
    // Part 4: Execute Payment with Permit
    // ========================================
    println!("\nPart 4: Executing Payments\n");

    // Valid payment (AWS permit -> AWS)
    let intent = PaymentIntent {
        intent_id: "intent-001".into(),
        permit_id: aws_permit.permit_id.clone(),
        sender: agent_id.clone(),
        recipient: vendor_aws.clone(),
        amount: Amount::from_dollars(100),
        memo: "EC2 instance payment".into(),
        created_at: Utc::now(),
    };

    println!("Attempting valid payment (AWS permit -> AWS)...");
    match gate.create_commitment(&intent, &aws_permit, &main_budget, ConsequenceRef::DirectPayment).await {
        Ok((receipt, _)) => {
            println!("  SUCCESS: {}", receipt.receipt_id);
            println!("  Amount: ${:.2}", receipt.amount.as_dollars());
        }
        Err(e) => println!("  FAILED: {:?}", e),
    }

    // Invalid payment (AWS permit -> GCP - should fail)
    let bad_intent = PaymentIntent {
        intent_id: "intent-002".into(),
        permit_id: aws_permit.permit_id.clone(),
        sender: agent_id.clone(),
        recipient: vendor_gcp.clone(), // Wrong recipient!
        amount: Amount::from_dollars(100),
        memo: "GCP attempt".into(),
        created_at: Utc::now(),
    };

    println!("\nAttempting invalid payment (AWS permit -> GCP)...");
    match gate.create_commitment(&bad_intent, &aws_permit, &main_budget, ConsequenceRef::DirectPayment).await {
        Ok(_) => println!("  UNEXPECTED SUCCESS (this should not happen)"),
        Err(e) => println!("  CORRECTLY REJECTED: {:?}", e),
    }

    // ========================================
    // Part 5: Velocity Limit Test
    // ========================================
    println!("\nPart 5: Testing Velocity Limits\n");

    // Create a budget with tight velocity limits
    let tight_budget = BudgetPolicy {
        budget_id: BudgetId::new_with_prefix("tight"),
        owner: agent_id.clone(),
        max_total: Amount::from_dollars(1_000),
        max_single: Amount::from_dollars(500),
        velocity_limit: Some(VelocityLimit {
            max_per_hour: Amount::from_dollars(200), // Only $200/hour!
            max_per_day: Amount::from_dollars(500),
        }),
        allow_negative: false,
        parent_budget: None,
    };

    let tight_permit = SpendPermit {
        permit_id: PermitId::new(),
        issuer: agent_id.clone(),
        bound_budget: tight_budget.budget_id.clone(),
        asset_class: AssetClass::IUSD,
        max_amount: Amount::from_dollars(500),
        remaining: Amount::from_dollars(500),
        counterparty: CounterpartyConstraint::Any,
        purpose: SpendPurpose::ServicePayment("Testing".into()),
        expires_at: Utc::now() + Duration::hours(1),
        signature: String::new(),
    }.sign(&agent_keypair);

    println!("Tight budget: ${:.2}/hour velocity limit", 200.0);

    // Try to make multiple payments
    for i in 1..=4 {
        let intent = PaymentIntent {
            intent_id: format!("velocity-test-{}", i),
            permit_id: tight_permit.permit_id.clone(),
            sender: agent_id.clone(),
            recipient: vendor_aws.clone(),
            amount: Amount::from_dollars(75),
            memo: format!("Payment {}", i),
            created_at: Utc::now(),
        };

        print!("  Payment {} ($75): ", i);
        match gate.create_commitment(&intent, &tight_permit, &tight_budget, ConsequenceRef::DirectPayment).await {
            Ok(_) => println!("SUCCESS (hourly total: ${:.2})", 75.0 * i as f64),
            Err(BudgetError::HourlyVelocityExceeded { current, max, .. }) => {
                println!("VELOCITY LIMIT (current: ${:.2}, max: ${:.2})",
                    current.as_dollars(), max.as_dollars());
            }
            Err(e) => println!("ERROR: {:?}", e),
        }
    }

    println!("\n=== Permits Tutorial Complete ===");

    Ok(())
}
```

---

## Key Concepts Recap

| Concept | Purpose |
|---------|---------|
| **BudgetPolicy** | High-level spending limits and tracking |
| **SpendPermit** | Bounded authorization for specific spending |
| **VelocityLimit** | Rate limiting to prevent rapid depletion |
| **CounterpartyConstraint** | Controls who can receive payments |
| **Hierarchy** | Multi-level budget organization |

---

## Troubleshooting

### "Budget exceeded" but there's money left

```rust
// Check if hitting velocity limit
println!("Spent this hour: ${:.2}", budget.spent_in_period(Duration::hours(1)).as_dollars());
println!("Hourly limit: ${:.2}", budget.velocity_limit.max_per_hour.as_dollars());
```

### "Counterparty not allowed"

```rust
// Debug constraint matching
match &permit.counterparty {
    CounterpartyConstraint::Specific(allowed) => {
        println!("Permit allows only: {}", allowed);
        println!("You're trying to pay: {}", recipient);
    }
    CounterpartyConstraint::AllowList(list) => {
        println!("Permit allows: {:?}", list);
        println!("Recipient {} in list: {}", recipient, list.contains(recipient));
    }
    _ => {}
}
```

### Permit expired

```rust
// Always check expiration before use
if permit.expires_at < Utc::now() {
    println!("Permit expired at: {}", permit.expires_at);
    println!("Current time: {}", Utc::now());
    // Create new permit
}
```

---

## Next Steps

You now have a comprehensive understanding of OpeniBank's permit system. Continue with:

1. **[Escrow Workflows](./05-escrow.md)** - Multi-party conditional payments with arbitration

---

## Quick Reference

### Permit Checklist

Before creating a permit, consider:

- [ ] What is the maximum amount needed?
- [ ] Who are the allowed recipients?
- [ ] What is the purpose of the spending?
- [ ] When should the permit expire?
- [ ] What velocity limits are appropriate?
- [ ] Should it auto-renew?

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/budgets` | POST | Create budget |
| `/api/budgets/{id}` | GET | Get budget status |
| `/api/permits` | POST | Create permit |
| `/api/permits/{id}` | GET | Get permit details |
| `/api/permits/{id}/revoke` | POST | Revoke permit |

---

**Next Tutorial**: [Escrow Workflows](./05-escrow.md) - Learn how to implement multi-party trades with conditional settlement and dispute resolution.
