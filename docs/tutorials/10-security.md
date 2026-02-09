# Tutorial 10: Security & Compliance

> **Duration**: 60 minutes
> **Level**: Advanced
> **Prerequisites**: Tutorials 1-9, Understanding of cryptography and security principles

---

## Overview

Security is paramount in financial systems. OpeniBank implements multiple layers of protection for AI agent transactions. In this tutorial, you'll learn to:

- Implement Ed25519 cryptographic signing
- Configure spend permits and budgets
- Set up policy enforcement with Guard
- Audit transactions and generate compliance reports
- Handle security incidents

---

## Understanding OpeniBank's Security Model

```
┌─────────────────────────────────────────────────────────────────┐
│                      Security Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 5: Cryptographic Commitment ─────────────────────────┐   │
│    • Ed25519 signing                                         │   │
│    • Receipt generation                                      │   │
│    • Immutable audit log                                     │   │
│                                                              │   │
│  Layer 4: Policy Constraints ────────────────────────────────┤   │
│    • Counterparty validation                                 │   │
│    • Purpose matching                                        │   │
│    • Time window enforcement                                 │   │
│                                                              │   │
│  Layer 3: Budget Enforcement ────────────────────────────────┤   │
│    • Remaining allocation check                              │   │
│    • Spending velocity limits                                │   │
│                                                              │   │
│  Layer 2: Permit Validation ─────────────────────────────────┤   │
│    • Ed25519 signature verification                          │   │
│    • Expiration checking                                     │   │
│    • Amount bounds validation                                │   │
│                                                              │   │
│  Layer 1: LLM Output Validation ─────────────────────────────┘   │
│    • JSON schema validation                                      │
│    • Intent structure checking                                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step 1: Cryptographic Operations

### Key Generation and Management

```rust
use openibank_crypto::{KeyPair, PublicKey, Signature};
use ed25519_dalek::{SigningKey, VerifyingKey};

fn generate_agent_keypair() -> KeyPair {
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();

    KeyPair {
        private_key: signing_key,
        public_key: verifying_key,
    }
}

fn secure_key_storage() {
    // In production, use secure key management:
    // - Hardware Security Modules (HSM)
    // - AWS KMS, Google Cloud KMS, Azure Key Vault
    // - HashiCorp Vault

    // Example: Store encrypted in environment
    let keypair = generate_agent_keypair();
    let private_bytes = keypair.private_key.to_bytes();

    // Encrypt with master key
    let encrypted = encrypt_aes256(&private_bytes, &master_key);

    // Store in secure location
    std::fs::write("/secure/keys/agent.key.enc", &encrypted)?;
}
```

### Signing Transactions

```rust
use openibank_crypto::Signer;

struct TransactionSigner {
    keypair: KeyPair,
}

impl TransactionSigner {
    fn sign_transaction(&self, tx: &Transaction) -> SignedTransaction {
        // Serialize transaction deterministically
        let tx_bytes = tx.to_canonical_bytes();

        // Hash the transaction
        let hash = sha256(&tx_bytes);

        // Sign the hash
        let signature = self.keypair.private_key.sign(&hash);

        SignedTransaction {
            transaction: tx.clone(),
            signature: Signature(signature.to_bytes()),
            public_key: self.keypair.public_key.clone(),
            timestamp: Utc::now(),
        }
    }

    fn verify_signature(signed_tx: &SignedTransaction) -> Result<bool, CryptoError> {
        let tx_bytes = signed_tx.transaction.to_canonical_bytes();
        let hash = sha256(&tx_bytes);

        let verifying_key = VerifyingKey::from_bytes(&signed_tx.public_key.0)?;
        let signature = ed25519_dalek::Signature::from_bytes(&signed_tx.signature.0)?;

        Ok(verifying_key.verify(&hash, &signature).is_ok())
    }
}
```

### Multi-Signature Transactions

```rust
struct MultiSigTransaction {
    transaction: Transaction,
    required_signatures: usize,
    signatures: Vec<(PublicKey, Signature)>,
}

impl MultiSigTransaction {
    fn add_signature(&mut self, keypair: &KeyPair) -> Result<(), CryptoError> {
        if self.signatures.len() >= self.required_signatures {
            return Err(CryptoError::AlreadyComplete);
        }

        // Check if this key already signed
        if self.signatures.iter().any(|(pk, _)| pk == &keypair.public_key) {
            return Err(CryptoError::DuplicateSignature);
        }

        let tx_bytes = self.transaction.to_canonical_bytes();
        let hash = sha256(&tx_bytes);
        let signature = keypair.private_key.sign(&hash);

        self.signatures.push((
            keypair.public_key.clone(),
            Signature(signature.to_bytes()),
        ));

        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.signatures.len() >= self.required_signatures
    }

    fn verify_all(&self) -> Result<bool, CryptoError> {
        let tx_bytes = self.transaction.to_canonical_bytes();
        let hash = sha256(&tx_bytes);

        for (public_key, signature) in &self.signatures {
            let verifying_key = VerifyingKey::from_bytes(&public_key.0)?;
            let sig = ed25519_dalek::Signature::from_bytes(&signature.0)?;

            if verifying_key.verify(&hash, &sig).is_err() {
                return Ok(false);
            }
        }

        Ok(self.signatures.len() >= self.required_signatures)
    }
}
```

---

## Step 2: Spend Permits

### Creating Permits

```rust
use openibank_permits::{SpendPermit, PermitBuilder, PermitConstraints};

fn create_spend_permit(
    wallet_id: &str,
    keypair: &KeyPair,
) -> Result<SpendPermit, PermitError> {
    let permit = PermitBuilder::new()
        .wallet(wallet_id)
        .max_amount(dec!(10000))  // Max $10,000
        .counterparty(Some("seller_datacorp"))  // Only this seller
        .purpose("Data Analysis service")
        .valid_from(Utc::now())
        .expires_at(Utc::now() + Duration::hours(24))
        .constraints(PermitConstraints {
            max_single_transaction: Some(dec!(5000)),
            cooldown_seconds: Some(60),  // 1 minute between transactions
            allowed_purposes: vec!["Data Analysis".to_string()],
        })
        .build()?;

    // Sign the permit
    let signed = permit.sign(keypair)?;

    Ok(signed)
}
```

### Validating Permits

```rust
use openibank_permits::PermitValidator;

struct PermitValidationService {
    validator: PermitValidator,
}

impl PermitValidationService {
    fn validate_permit(&self, permit: &SpendPermit, transaction: &Transaction) -> Result<(), ValidationError> {
        // Layer 1: Signature verification
        if !self.validator.verify_signature(permit)? {
            return Err(ValidationError::InvalidSignature);
        }

        // Layer 2: Expiration check
        if permit.expires_at < Utc::now() {
            return Err(ValidationError::Expired);
        }

        if permit.valid_from > Utc::now() {
            return Err(ValidationError::NotYetValid);
        }

        // Layer 3: Amount validation
        if transaction.amount > permit.max_amount {
            return Err(ValidationError::AmountExceeded {
                requested: transaction.amount,
                permitted: permit.max_amount,
            });
        }

        // Layer 4: Counterparty validation
        if let Some(allowed_counterparty) = &permit.counterparty {
            if &transaction.counterparty != allowed_counterparty {
                return Err(ValidationError::UnauthorizedCounterparty {
                    requested: transaction.counterparty.clone(),
                    permitted: allowed_counterparty.clone(),
                });
            }
        }

        // Layer 5: Purpose matching
        if !permit.constraints.allowed_purposes.contains(&transaction.purpose) {
            return Err(ValidationError::PurposeMismatch);
        }

        // Layer 6: Constraint checks
        if let Some(max_single) = permit.constraints.max_single_transaction {
            if transaction.amount > max_single {
                return Err(ValidationError::SingleTransactionLimitExceeded);
            }
        }

        Ok(())
    }
}
```

---

## Step 3: Budget Enforcement

### Budget Configuration

```rust
use openibank_guard::{Budget, BudgetConfig, SpendingLimit};

fn configure_agent_budget() -> BudgetConfig {
    BudgetConfig {
        // Total budget
        total_allocation: dec!(100000),

        // Time-based limits
        daily_limit: dec!(10000),
        hourly_limit: dec!(1000),

        // Transaction limits
        max_single_transaction: dec!(5000),
        min_transaction: dec!(1),

        // Velocity limits
        max_transactions_per_hour: 100,
        max_transactions_per_day: 1000,

        // Category limits
        category_limits: hashmap! {
            "trading".to_string() => dec!(50000),
            "services".to_string() => dec!(30000),
            "operations".to_string() => dec!(20000),
        },

        // Alert thresholds
        alert_at_percent: 80,
        block_at_percent: 95,
    }
}
```

### Real-Time Budget Tracking

```rust
struct BudgetTracker {
    config: BudgetConfig,
    spent: RwLock<SpendingRecord>,
}

impl BudgetTracker {
    async fn check_and_reserve(&self, amount: Decimal, category: &str) -> Result<ReservationId, BudgetError> {
        let mut spent = self.spent.write().await;

        // Check total remaining
        let total_remaining = self.config.total_allocation - spent.total;
        if amount > total_remaining {
            return Err(BudgetError::InsufficientBudget {
                requested: amount,
                remaining: total_remaining,
            });
        }

        // Check daily limit
        let daily_remaining = self.config.daily_limit - spent.today;
        if amount > daily_remaining {
            return Err(BudgetError::DailyLimitExceeded {
                requested: amount,
                remaining: daily_remaining,
            });
        }

        // Check hourly limit
        let hourly_remaining = self.config.hourly_limit - spent.this_hour;
        if amount > hourly_remaining {
            return Err(BudgetError::HourlyLimitExceeded {
                requested: amount,
                remaining: hourly_remaining,
            });
        }

        // Check category limit
        if let Some(category_limit) = self.config.category_limits.get(category) {
            let category_spent = spent.by_category.get(category).copied().unwrap_or_default();
            let category_remaining = *category_limit - category_spent;
            if amount > category_remaining {
                return Err(BudgetError::CategoryLimitExceeded {
                    category: category.to_string(),
                    requested: amount,
                    remaining: category_remaining,
                });
            }
        }

        // Check velocity
        if spent.transactions_this_hour >= self.config.max_transactions_per_hour {
            return Err(BudgetError::VelocityLimitExceeded);
        }

        // Reserve the amount
        let reservation_id = ReservationId::new();
        spent.reservations.insert(reservation_id.clone(), Reservation {
            amount,
            category: category.to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
        });

        // Update counters
        spent.reserved += amount;

        Ok(reservation_id)
    }

    async fn commit_reservation(&self, reservation_id: ReservationId) -> Result<(), BudgetError> {
        let mut spent = self.spent.write().await;

        let reservation = spent.reservations.remove(&reservation_id)
            .ok_or(BudgetError::ReservationNotFound)?;

        spent.total += reservation.amount;
        spent.today += reservation.amount;
        spent.this_hour += reservation.amount;
        spent.reserved -= reservation.amount;
        spent.transactions_this_hour += 1;

        *spent.by_category.entry(reservation.category).or_default() += reservation.amount;

        // Check alert thresholds
        let usage_percent = (spent.total / self.config.total_allocation * dec!(100)).to_u32().unwrap_or(0);
        if usage_percent >= self.config.alert_at_percent as u32 {
            self.send_alert(usage_percent).await;
        }

        Ok(())
    }
}
```

---

## Step 4: Policy Enforcement with Guard

### Policy Definition

```rust
use openibank_guard::{Policy, PolicyRule, PolicyAction};

fn define_security_policies() -> Vec<Policy> {
    vec![
        // Transaction amount limits
        Policy {
            name: "max_transaction_amount".to_string(),
            rules: vec![
                PolicyRule {
                    condition: "transaction.amount > 10000".to_string(),
                    action: PolicyAction::RequireApproval {
                        approver: "risk_manager".to_string(),
                    },
                },
                PolicyRule {
                    condition: "transaction.amount > 100000".to_string(),
                    action: PolicyAction::Deny {
                        reason: "Amount exceeds maximum allowed".to_string(),
                    },
                },
            ],
        },

        // Counterparty restrictions
        Policy {
            name: "counterparty_verification".to_string(),
            rules: vec![
                PolicyRule {
                    condition: "counterparty.verified == false".to_string(),
                    action: PolicyAction::Deny {
                        reason: "Counterparty not verified".to_string(),
                    },
                },
                PolicyRule {
                    condition: "counterparty.risk_score > 0.7".to_string(),
                    action: PolicyAction::RequireApproval {
                        approver: "compliance_officer".to_string(),
                    },
                },
            ],
        },

        // Time-based restrictions
        Policy {
            name: "trading_hours".to_string(),
            rules: vec![
                PolicyRule {
                    condition: "time.hour < 9 || time.hour > 17".to_string(),
                    action: PolicyAction::RequireApproval {
                        approver: "duty_manager".to_string(),
                    },
                },
            ],
        },

        // Velocity controls
        Policy {
            name: "velocity_limits".to_string(),
            rules: vec![
                PolicyRule {
                    condition: "agent.transactions_last_minute > 10".to_string(),
                    action: PolicyAction::RateLimit {
                        delay_seconds: 60,
                    },
                },
                PolicyRule {
                    condition: "agent.transactions_last_hour > 100".to_string(),
                    action: PolicyAction::Deny {
                        reason: "Hourly transaction limit exceeded".to_string(),
                    },
                },
            ],
        },
    ]
}
```

### Policy Enforcement Engine

```rust
struct PolicyEngine {
    policies: Vec<Policy>,
    evaluator: PolicyEvaluator,
}

impl PolicyEngine {
    async fn evaluate(&self, context: &TransactionContext) -> PolicyResult {
        let mut result = PolicyResult::Allow;

        for policy in &self.policies {
            for rule in &policy.rules {
                if self.evaluator.matches(&rule.condition, context)? {
                    match &rule.action {
                        PolicyAction::Allow => {
                            // Continue checking other rules
                        }
                        PolicyAction::Deny { reason } => {
                            return PolicyResult::Deny {
                                policy: policy.name.clone(),
                                reason: reason.clone(),
                            };
                        }
                        PolicyAction::RequireApproval { approver } => {
                            result = PolicyResult::RequireApproval {
                                policy: policy.name.clone(),
                                approver: approver.clone(),
                            };
                        }
                        PolicyAction::RateLimit { delay_seconds } => {
                            return PolicyResult::RateLimit {
                                delay_seconds: *delay_seconds,
                            };
                        }
                        PolicyAction::Log { level, message } => {
                            self.log(level, message, context);
                        }
                    }
                }
            }
        }

        result
    }

    async fn enforce(&self, transaction: &Transaction) -> Result<(), PolicyError> {
        let context = TransactionContext::from_transaction(transaction);

        match self.evaluate(&context).await {
            PolicyResult::Allow => Ok(()),
            PolicyResult::Deny { policy, reason } => {
                Err(PolicyError::Denied { policy, reason })
            }
            PolicyResult::RequireApproval { policy, approver } => {
                // Queue for approval
                self.queue_for_approval(transaction, &policy, &approver).await?;
                Err(PolicyError::PendingApproval { policy, approver })
            }
            PolicyResult::RateLimit { delay_seconds } => {
                tokio::time::sleep(Duration::from_secs(delay_seconds)).await;
                // Retry
                self.enforce(transaction).await
            }
        }
    }
}
```

---

## Step 5: Audit & Compliance

### Audit Logging

```rust
use openibank_audit::{AuditLog, AuditEvent, AuditLevel};

struct AuditService {
    log: AuditLog,
}

impl AuditService {
    async fn log_transaction(&self, tx: &SignedTransaction, result: &TransactionResult) {
        let event = AuditEvent {
            timestamp: Utc::now(),
            event_type: "transaction".to_string(),
            level: match result {
                TransactionResult::Success => AuditLevel::Info,
                TransactionResult::Failure(_) => AuditLevel::Warning,
            },
            actor: tx.transaction.from.clone(),
            action: format!("transfer {} to {}", tx.transaction.amount, tx.transaction.to),
            resource: tx.transaction.id.clone(),
            result: result.clone(),
            metadata: json!({
                "amount": tx.transaction.amount.to_string(),
                "currency": tx.transaction.currency,
                "purpose": tx.transaction.purpose,
                "permit_id": tx.transaction.permit_id,
            }),
            ip_address: None,  // Not applicable for agents
            user_agent: Some("OpeniBank Agent".to_string()),
        };

        self.log.append(event).await;
    }

    async fn log_security_event(&self, event_type: &str, details: serde_json::Value, level: AuditLevel) {
        let event = AuditEvent {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            level,
            actor: "system".to_string(),
            action: event_type.to_string(),
            resource: "security".to_string(),
            result: TransactionResult::Success,
            metadata: details,
            ip_address: None,
            user_agent: None,
        };

        self.log.append(event).await;

        // For critical events, send alert
        if level == AuditLevel::Critical {
            self.send_security_alert(&event).await;
        }
    }
}
```

### Compliance Reporting

```rust
struct ComplianceReporter {
    audit_log: AuditLog,
}

impl ComplianceReporter {
    async fn generate_daily_report(&self, date: NaiveDate) -> ComplianceReport {
        let events = self.audit_log.query(
            date.and_hms(0, 0, 0),
            date.and_hms(23, 59, 59),
            None,  // All event types
        ).await.unwrap();

        let mut report = ComplianceReport {
            date,
            total_transactions: 0,
            total_volume: dec!(0),
            denied_transactions: 0,
            policy_violations: Vec::new(),
            suspicious_activities: Vec::new(),
            agent_statistics: HashMap::new(),
        };

        for event in &events {
            if event.event_type == "transaction" {
                report.total_transactions += 1;

                if let Some(amount) = event.metadata.get("amount") {
                    report.total_volume += amount.as_str()
                        .and_then(|s| Decimal::from_str(s).ok())
                        .unwrap_or_default();
                }

                if matches!(event.result, TransactionResult::Failure(_)) {
                    report.denied_transactions += 1;
                }

                // Update agent statistics
                let stats = report.agent_statistics
                    .entry(event.actor.clone())
                    .or_insert(AgentStats::default());
                stats.transaction_count += 1;
            }

            if event.event_type == "policy_violation" {
                report.policy_violations.push(PolicyViolation {
                    timestamp: event.timestamp,
                    actor: event.actor.clone(),
                    policy: event.metadata.get("policy")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    details: event.metadata.clone(),
                });
            }

            if event.level == AuditLevel::Warning || event.level == AuditLevel::Critical {
                if self.is_suspicious(&event) {
                    report.suspicious_activities.push(SuspiciousActivity {
                        timestamp: event.timestamp,
                        event: event.clone(),
                        risk_score: self.calculate_risk_score(&event),
                    });
                }
            }
        }

        report
    }

    fn is_suspicious(&self, event: &AuditEvent) -> bool {
        // Check for suspicious patterns
        let suspicious_patterns = [
            event.event_type == "failed_authentication",
            event.metadata.get("velocity_exceeded").is_some(),
            event.metadata.get("unusual_counterparty").is_some(),
            event.metadata.get("off_hours_transaction").is_some(),
        ];

        suspicious_patterns.iter().any(|&p| p)
    }

    fn calculate_risk_score(&self, event: &AuditEvent) -> f64 {
        let mut score = 0.0;

        // Factor in various risk indicators
        if event.level == AuditLevel::Critical {
            score += 0.5;
        }

        if event.metadata.get("high_value").is_some() {
            score += 0.2;
        }

        if event.metadata.get("new_counterparty").is_some() {
            score += 0.1;
        }

        score.min(1.0)
    }
}
```

---

## Step 6: Security Incident Response

### Incident Detection

```rust
struct IncidentDetector {
    audit_log: AuditLog,
    alert_service: AlertService,
}

impl IncidentDetector {
    async fn monitor(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = self.audit_log.subscribe().await?;

        while let Some(event) = stream.next().await {
            if let Some(incident) = self.detect_incident(&event) {
                self.handle_incident(incident).await?;
            }
        }

        Ok(())
    }

    fn detect_incident(&self, event: &AuditEvent) -> Option<SecurityIncident> {
        // Pattern 1: Multiple failed authentications
        if event.event_type == "failed_authentication" {
            if let Some(count) = event.metadata.get("consecutive_failures") {
                if count.as_u64().unwrap_or(0) >= 5 {
                    return Some(SecurityIncident {
                        incident_type: IncidentType::BruteForceAttempt,
                        severity: Severity::High,
                        actor: event.actor.clone(),
                        details: event.metadata.clone(),
                    });
                }
            }
        }

        // Pattern 2: Unusual transaction patterns
        if event.event_type == "transaction" {
            if event.metadata.get("anomaly_detected").is_some() {
                return Some(SecurityIncident {
                    incident_type: IncidentType::AnomalousTransaction,
                    severity: Severity::Medium,
                    actor: event.actor.clone(),
                    details: event.metadata.clone(),
                });
            }
        }

        // Pattern 3: Policy bypass attempt
        if event.event_type == "policy_bypass_attempt" {
            return Some(SecurityIncident {
                incident_type: IncidentType::PolicyBypassAttempt,
                severity: Severity::Critical,
                actor: event.actor.clone(),
                details: event.metadata.clone(),
            });
        }

        None
    }

    async fn handle_incident(&self, incident: SecurityIncident) -> Result<(), Box<dyn std::error::Error>> {
        // Log the incident
        self.audit_log.log_security_event(
            "security_incident",
            json!({
                "type": format!("{:?}", incident.incident_type),
                "severity": format!("{:?}", incident.severity),
                "actor": incident.actor,
                "details": incident.details,
            }),
            AuditLevel::Critical,
        ).await;

        // Take automated response based on severity
        match incident.severity {
            Severity::Critical => {
                // Immediately freeze the agent
                self.freeze_agent(&incident.actor).await?;

                // Notify security team
                self.alert_service.send_critical_alert(&incident).await?;

                // Create incident ticket
                self.create_incident_ticket(&incident).await?;
            }
            Severity::High => {
                // Increase monitoring
                self.increase_monitoring(&incident.actor).await?;

                // Notify on-call
                self.alert_service.send_high_alert(&incident).await?;
            }
            Severity::Medium => {
                // Log for review
                self.alert_service.send_medium_alert(&incident).await?;
            }
            Severity::Low => {
                // Just log
            }
        }

        Ok(())
    }

    async fn freeze_agent(&self, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Revoke all permits
        // Cancel pending transactions
        // Disable agent

        println!("SECURITY: Agent {} has been frozen", agent_id);

        Ok(())
    }
}
```

---

## Complete Security Implementation

```rust
use openibank_crypto::*;
use openibank_permits::*;
use openibank_guard::*;
use openibank_audit::*;

struct SecureAgentSystem {
    crypto: CryptoService,
    permits: PermitService,
    budget_tracker: BudgetTracker,
    policy_engine: PolicyEngine,
    audit: AuditService,
    incident_detector: IncidentDetector,
}

impl SecureAgentSystem {
    async fn execute_secure_transaction(
        &self,
        transaction: Transaction,
        permit: SpendPermit,
        agent_keypair: &KeyPair,
    ) -> Result<Receipt, SecurityError> {
        // Step 1: Validate permit
        self.permits.validate(&permit, &transaction)?;

        // Step 2: Check budget
        let reservation = self.budget_tracker.check_and_reserve(
            transaction.amount,
            &transaction.category,
        ).await?;

        // Step 3: Evaluate policies
        self.policy_engine.enforce(&transaction).await?;

        // Step 4: Sign transaction
        let signed = self.crypto.sign_transaction(&transaction, agent_keypair)?;

        // Step 5: Execute
        let result = self.execute_internal(&signed).await;

        // Step 6: Finalize budget
        match &result {
            Ok(_) => {
                self.budget_tracker.commit_reservation(reservation).await?;
            }
            Err(_) => {
                self.budget_tracker.release_reservation(reservation).await?;
            }
        }

        // Step 7: Audit log
        self.audit.log_transaction(&signed, &result.clone().into()).await;

        // Step 8: Generate receipt
        let receipt = self.generate_receipt(&signed, &result)?;

        result.map(|_| receipt)
    }

    fn generate_receipt(
        &self,
        signed_tx: &SignedTransaction,
        result: &Result<(), TransactionError>,
    ) -> Result<Receipt, SecurityError> {
        let receipt = Receipt {
            receipt_id: Uuid::new_v4().to_string(),
            transaction_id: signed_tx.transaction.id.clone(),
            timestamp: Utc::now(),
            status: match result {
                Ok(_) => ReceiptStatus::Completed,
                Err(e) => ReceiptStatus::Failed { reason: e.to_string() },
            },
            amount: signed_tx.transaction.amount,
            from: signed_tx.transaction.from.clone(),
            to: signed_tx.transaction.to.clone(),
            signature: signed_tx.signature.clone(),
            public_key: signed_tx.public_key.clone(),
        };

        // Sign the receipt
        let signed_receipt = self.crypto.sign_receipt(&receipt)?;

        Ok(signed_receipt)
    }
}
```

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `INVALID_SIGNATURE` | Key mismatch or tampering | Verify key pair, check message integrity |
| `PERMIT_EXPIRED` | Permit past expiration | Generate new permit |
| `BUDGET_EXCEEDED` | Over allocation | Request budget increase or wait for reset |
| `POLICY_DENIED` | Rule violation | Review policy, request exception |
| `FROZEN_AGENT` | Security incident | Contact security team |

---

## Best Practices

1. **Rotate keys regularly** - Implement key rotation schedule
2. **Least privilege permits** - Only grant necessary permissions
3. **Defense in depth** - Multiple security layers
4. **Monitor everything** - Comprehensive audit logging
5. **Incident response plan** - Have procedures ready
6. **Regular security audits** - External review of security controls

---

## Conclusion

Congratulations! You've completed the OpeniBank tutorial series. You now have the knowledge to:

- Create and manage AI agents
- Execute secure transactions
- Deploy agent fleets at scale
- Build multi-agent systems
- Implement robust security controls

## Next Steps

- [API Reference](../api/README.md)
- [SDK Documentation](../sdk/README.md)
- [Deployment Guide](../deployment/README.md)
- [Architecture Overview](../architecture/SERVICES.md)

---

**OpeniBank**: Where AI agents bank securely.
