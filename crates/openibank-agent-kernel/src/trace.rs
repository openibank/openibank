//! Kernel trace artifacts for replay and audit

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KernelStage {
    Policy,
    Propose,
    Gate,
    Decision,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelTraceEvent {
    pub timestamp: DateTime<Utc>,
    pub stage: KernelStage,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelTrace {
    pub agent_id: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub events: Vec<KernelTraceEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_entries: Option<usize>,
}

impl KernelTrace {
    pub fn new(agent_id: impl Into<String>, role: impl Into<String>, max_entries: Option<usize>) -> Self {
        Self {
            agent_id: agent_id.into(),
            role: role.into(),
            created_at: Utc::now(),
            events: Vec::new(),
            max_entries,
        }
    }

    pub fn record(&mut self, stage: KernelStage, message: impl Into<String>, data: Option<serde_json::Value>) {
        self.events.push(KernelTraceEvent {
            timestamp: Utc::now(),
            stage,
            message: message.into(),
            data,
        });
        if let Some(max) = self.max_entries {
            if self.events.len() > max {
                let overflow = self.events.len() - max;
                self.events.drain(0..overflow);
            }
        }
    }

    pub fn is_replayable_with(&self, other: &KernelTrace) -> bool {
        if self.events.len() != other.events.len() {
            return false;
        }
        self.events.iter().zip(other.events.iter()).all(|(a, b)| {
            a.stage == b.stage && a.message == b.message && a.data == b.data
        })
    }
}
