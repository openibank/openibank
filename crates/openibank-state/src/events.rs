//! System events for SSE streaming and activity tracking
//!
//! Events are broadcast to all subscribers (dashboard via SSE, CLI poll, etc.)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// System-wide events emitted during OpeniBank operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SystemEvent {
    /// A new agent was created and registered as a Maple Resonator
    AgentCreated {
        agent_id: String,
        name: String,
        role: String,
        has_resonator: bool,
        timestamp: DateTime<Utc>,
    },

    /// Agent state changed (presence, resonator status)
    AgentStateChanged {
        agent_id: String,
        old_state: String,
        new_state: String,
        timestamp: DateTime<Utc>,
    },

    /// Wallet balance updated
    BalanceUpdated {
        agent_id: String,
        agent_name: String,
        old_balance: u64,
        new_balance: u64,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// Trade started (buyer initiated purchase)
    TradeStarted {
        trade_id: String,
        buyer_id: String,
        seller_id: String,
        service_name: String,
        amount: u64,
        timestamp: DateTime<Utc>,
    },

    /// Trade completed successfully
    TradeCompleted {
        trade_id: String,
        buyer_id: String,
        seller_id: String,
        service_name: String,
        amount: u64,
        receipt_id: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Trade failed
    TradeFailed {
        trade_id: String,
        buyer_id: String,
        seller_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// LLM reasoning occurred (agent brain activity)
    LLMReasoning {
        agent_id: String,
        agent_name: String,
        action: String,
        model: Option<String>,
        reasoning_summary: String,
        latency_ms: Option<u64>,
        timestamp: DateTime<Utc>,
    },

    /// Receipt generated (commitment, escrow, etc.)
    ReceiptGenerated {
        receipt_id: String,
        receipt_type: String,
        actor: String,
        description: String,
        timestamp: DateTime<Utc>,
    },

    /// Issuer event (mint/burn)
    IssuerEvent {
        event_type: IssuerEventType,
        amount: u64,
        account: String,
        total_supply: u64,
        timestamp: DateTime<Utc>,
    },

    /// Ledger entry recorded
    LedgerEntry {
        entry_id: String,
        from: String,
        to: String,
        amount: u64,
        memo: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Escrow state change
    EscrowEvent {
        escrow_id: String,
        payer: String,
        payee: String,
        amount: u64,
        state: String,
        timestamp: DateTime<Utc>,
    },

    /// Dispute opened/resolved
    DisputeEvent {
        case_id: String,
        escrow_id: String,
        event_type: DisputeEventType,
        decision: Option<String>,
        reasoning: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// System status update (periodic or on-demand)
    SystemStatus {
        agent_count: usize,
        trade_count: u32,
        total_supply: u64,
        uptime_seconds: u64,
        timestamp: DateTime<Utc>,
    },

    /// System reset
    SystemReset {
        timestamp: DateTime<Utc>,
    },

    /// Maple runtime event
    MapleRuntimeEvent {
        event_type: String,
        description: String,
        timestamp: DateTime<Utc>,
    },

    // ====================================================================
    // Maple Deep Integration Events
    // ====================================================================

    /// Buyer↔Seller coupling established for a trade
    CouplingEstablished {
        coupling_id: String,
        buyer_id: String,
        seller_id: String,
        strength: f64,
        timestamp: DateTime<Utc>,
    },

    /// Coupling strengthened after successful trade
    CouplingStrengthened {
        coupling_id: String,
        old_strength: f64,
        new_strength: f64,
        timestamp: DateTime<Utc>,
    },

    /// Coupling weakened after trade failure
    CouplingWeakened {
        coupling_id: String,
        old_strength: f64,
        new_strength: f64,
        timestamp: DateTime<Utc>,
    },

    /// Coupling dissolved after trade completion
    Decoupled {
        coupling_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// RcfCommitment submitted to AAS pipeline
    CommitmentSubmitted {
        commitment_id: String,
        buyer_name: String,
        seller_name: String,
        amount: u64,
        service_name: String,
        timestamp: DateTime<Utc>,
    },

    /// Commitment approved by AAS
    CommitmentApproved {
        commitment_id: String,
        decision: String,
        timestamp: DateTime<Utc>,
    },

    /// Commitment rejected by AAS
    CommitmentRejected {
        commitment_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// Trade outcome recorded in AAS ledger
    CommitmentOutcomeRecorded {
        commitment_id: String,
        success: bool,
        details: String,
        timestamp: DateTime<Utc>,
    },

    /// Attention budget allocated for a trade
    AttentionAllocated {
        agent_id: String,
        agent_name: String,
        amount: u64,
        remaining: u64,
        timestamp: DateTime<Utc>,
    },

    /// Agent attention exhausted (cannot trade)
    AttentionExhausted {
        agent_id: String,
        agent_name: String,
        timestamp: DateTime<Utc>,
    },

    /// Capability granted to an agent
    CapabilityGranted {
        agent_id: String,
        agent_name: String,
        capability: String,
        domain: String,
        timestamp: DateTime<Utc>,
    },

    /// Capability denied for an agent
    CapabilityDenied {
        agent_id: String,
        agent_name: String,
        capability: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

/// Types of issuer events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssuerEventType {
    /// IUSD minted to an account
    Mint,
    /// IUSD burned from an account
    Burn,
    /// Reserve cap changed
    ReserveCapChanged,
}

/// Types of dispute events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeEventType {
    /// Dispute opened
    Opened,
    /// Evidence submitted
    EvidenceSubmitted,
    /// Decision made
    Decided,
    /// Appeal filed
    Appealed,
}

impl SystemEvent {
    /// Get the timestamp of this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SystemEvent::AgentCreated { timestamp, .. } => *timestamp,
            SystemEvent::AgentStateChanged { timestamp, .. } => *timestamp,
            SystemEvent::BalanceUpdated { timestamp, .. } => *timestamp,
            SystemEvent::TradeStarted { timestamp, .. } => *timestamp,
            SystemEvent::TradeCompleted { timestamp, .. } => *timestamp,
            SystemEvent::TradeFailed { timestamp, .. } => *timestamp,
            SystemEvent::LLMReasoning { timestamp, .. } => *timestamp,
            SystemEvent::ReceiptGenerated { timestamp, .. } => *timestamp,
            SystemEvent::IssuerEvent { timestamp, .. } => *timestamp,
            SystemEvent::LedgerEntry { timestamp, .. } => *timestamp,
            SystemEvent::EscrowEvent { timestamp, .. } => *timestamp,
            SystemEvent::DisputeEvent { timestamp, .. } => *timestamp,
            SystemEvent::SystemStatus { timestamp, .. } => *timestamp,
            SystemEvent::SystemReset { timestamp, .. } => *timestamp,
            SystemEvent::MapleRuntimeEvent { timestamp, .. } => *timestamp,
            SystemEvent::CouplingEstablished { timestamp, .. } => *timestamp,
            SystemEvent::CouplingStrengthened { timestamp, .. } => *timestamp,
            SystemEvent::CouplingWeakened { timestamp, .. } => *timestamp,
            SystemEvent::Decoupled { timestamp, .. } => *timestamp,
            SystemEvent::CommitmentSubmitted { timestamp, .. } => *timestamp,
            SystemEvent::CommitmentApproved { timestamp, .. } => *timestamp,
            SystemEvent::CommitmentRejected { timestamp, .. } => *timestamp,
            SystemEvent::CommitmentOutcomeRecorded { timestamp, .. } => *timestamp,
            SystemEvent::AttentionAllocated { timestamp, .. } => *timestamp,
            SystemEvent::AttentionExhausted { timestamp, .. } => *timestamp,
            SystemEvent::CapabilityGranted { timestamp, .. } => *timestamp,
            SystemEvent::CapabilityDenied { timestamp, .. } => *timestamp,
        }
    }

    /// Get a short description for logging
    pub fn summary(&self) -> String {
        match self {
            SystemEvent::AgentCreated { name, role, .. } => {
                format!("Agent created: {} ({})", name, role)
            }
            SystemEvent::AgentStateChanged { agent_id, new_state, .. } => {
                format!("Agent {} → {}", agent_id, new_state)
            }
            SystemEvent::BalanceUpdated { agent_name, old_balance, new_balance, .. } => {
                format!("{}: ${:.2} → ${:.2}", agent_name, *old_balance as f64 / 100.0, *new_balance as f64 / 100.0)
            }
            SystemEvent::TradeStarted { service_name, amount, .. } => {
                format!("Trade started: {} (${:.2})", service_name, *amount as f64 / 100.0)
            }
            SystemEvent::TradeCompleted { service_name, amount, .. } => {
                format!("Trade completed: {} (${:.2})", service_name, *amount as f64 / 100.0)
            }
            SystemEvent::TradeFailed { reason, .. } => {
                format!("Trade failed: {}", reason)
            }
            SystemEvent::LLMReasoning { agent_name, action, .. } => {
                format!("LLM: {} - {}", agent_name, action)
            }
            SystemEvent::ReceiptGenerated { receipt_type, actor, .. } => {
                format!("Receipt: {} by {}", receipt_type, actor)
            }
            SystemEvent::IssuerEvent { event_type, amount, .. } => {
                format!("Issuer {:?}: ${:.2}", event_type, *amount as f64 / 100.0)
            }
            SystemEvent::LedgerEntry { from, to, amount, .. } => {
                format!("Ledger: {} → {} (${:.2})", from, to, *amount as f64 / 100.0)
            }
            SystemEvent::EscrowEvent { escrow_id, state, .. } => {
                format!("Escrow {}: {}", &escrow_id[..8.min(escrow_id.len())], state)
            }
            SystemEvent::DisputeEvent { case_id, event_type, .. } => {
                format!("Dispute {}: {:?}", &case_id[..8.min(case_id.len())], event_type)
            }
            SystemEvent::SystemStatus { agent_count, trade_count, .. } => {
                format!("Status: {} agents, {} trades", agent_count, trade_count)
            }
            SystemEvent::SystemReset { .. } => "System reset".to_string(),
            SystemEvent::MapleRuntimeEvent { description, .. } => {
                format!("Maple: {}", description)
            }
            SystemEvent::CouplingEstablished { buyer_id, seller_id, strength, .. } => {
                format!("Coupling: {} ↔ {} (strength: {:.1})", buyer_id, seller_id, strength)
            }
            SystemEvent::CouplingStrengthened { coupling_id, new_strength, .. } => {
                format!("Coupling {} strengthened → {:.1}", &coupling_id[..8.min(coupling_id.len())], new_strength)
            }
            SystemEvent::CouplingWeakened { coupling_id, new_strength, .. } => {
                format!("Coupling {} weakened → {:.1}", &coupling_id[..8.min(coupling_id.len())], new_strength)
            }
            SystemEvent::Decoupled { coupling_id, reason, .. } => {
                format!("Decoupled {}: {}", &coupling_id[..8.min(coupling_id.len())], reason)
            }
            SystemEvent::CommitmentSubmitted { buyer_name, seller_name, amount, .. } => {
                format!("Commitment: {} → {} (${:.2})", buyer_name, seller_name, *amount as f64 / 100.0)
            }
            SystemEvent::CommitmentApproved { commitment_id, .. } => {
                format!("Commitment approved: {}", &commitment_id[..8.min(commitment_id.len())])
            }
            SystemEvent::CommitmentRejected { commitment_id, reason, .. } => {
                format!("Commitment rejected: {} ({})", &commitment_id[..8.min(commitment_id.len())], reason)
            }
            SystemEvent::CommitmentOutcomeRecorded { commitment_id, success, .. } => {
                format!("Outcome {}: {}", &commitment_id[..8.min(commitment_id.len())], if *success { "success" } else { "failed" })
            }
            SystemEvent::AttentionAllocated { agent_name, amount, remaining, .. } => {
                format!("Attention: {} allocated {} (remaining: {})", agent_name, amount, remaining)
            }
            SystemEvent::AttentionExhausted { agent_name, .. } => {
                format!("Attention exhausted: {}", agent_name)
            }
            SystemEvent::CapabilityGranted { agent_name, capability, .. } => {
                format!("Capability granted: {} → {}", agent_name, capability)
            }
            SystemEvent::CapabilityDenied { agent_name, capability, reason, .. } => {
                format!("Capability denied: {} → {} ({})", agent_name, capability, reason)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = SystemEvent::AgentCreated {
            agent_id: "res_alice".to_string(),
            name: "Alice".to_string(),
            role: "Buyer".to_string(),
            has_resonator: true,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("AgentCreated"));
        assert!(json.contains("Alice"));

        let back: SystemEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.summary(), event.summary());
    }

    #[test]
    fn test_event_summary() {
        let event = SystemEvent::TradeCompleted {
            trade_id: "trade_1".to_string(),
            buyer_id: "res_alice".to_string(),
            seller_id: "res_datacorp".to_string(),
            service_name: "Data Analysis".to_string(),
            amount: 10000,
            receipt_id: Some("rcpt_1".to_string()),
            timestamp: Utc::now(),
        };

        let summary = event.summary();
        assert!(summary.contains("Data Analysis"));
        assert!(summary.contains("$100.00"));
    }
}
