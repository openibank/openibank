//! Visible reasoning types for AI agent decision-making
//!
//! This module provides structured reasoning that can be displayed to users,
//! making AI agent decisions transparent and verifiable.

use serde::{Deserialize, Serialize};

/// A structured reasoning step showing how an agent arrived at a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// Step number in the reasoning chain
    pub step: u32,
    /// Brief category of this step (e.g., "Risk Assessment", "Budget Check")
    pub category: String,
    /// The reasoning content
    pub reasoning: String,
    /// Optional confidence score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Complete reasoning trace for an agent decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    /// Unique ID for this reasoning trace
    pub trace_id: String,
    /// Agent that made this decision
    pub agent_id: String,
    /// The decision context (what prompted this reasoning)
    pub context: String,
    /// Sequential reasoning steps
    pub steps: Vec<ReasoningStep>,
    /// Final decision
    pub decision: AgentDecision,
    /// Total time taken for reasoning (milliseconds)
    pub duration_ms: u64,
    /// Which LLM model was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl ReasoningTrace {
    pub fn new(agent_id: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            context: context.into(),
            steps: vec![],
            decision: AgentDecision::Pending,
            duration_ms: 0,
            model: None,
        }
    }

    pub fn add_step(&mut self, category: impl Into<String>, reasoning: impl Into<String>) {
        self.steps.push(ReasoningStep {
            step: self.steps.len() as u32 + 1,
            category: category.into(),
            reasoning: reasoning.into(),
            confidence: None,
        });
    }

    pub fn add_step_with_confidence(
        &mut self,
        category: impl Into<String>,
        reasoning: impl Into<String>,
        confidence: f32,
    ) {
        self.steps.push(ReasoningStep {
            step: self.steps.len() as u32 + 1,
            category: category.into(),
            reasoning: reasoning.into(),
            confidence: Some(confidence),
        });
    }

    pub fn decide(&mut self, decision: AgentDecision) {
        self.decision = decision;
    }
}

/// The final decision from agent reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentDecision {
    /// Reasoning in progress
    Pending,
    /// Accept an offer or proposal
    Accept {
        reason: String,
    },
    /// Reject an offer or proposal
    Reject {
        reason: String,
    },
    /// Counter-propose with modifications
    CounterOffer {
        reason: String,
        proposed_amount: Option<u64>,
        proposed_terms: Option<String>,
    },
    /// Request more information
    RequestInfo {
        questions: Vec<String>,
    },
    /// Escalate to human/higher authority
    Escalate {
        reason: String,
    },
    /// Generic action decision
    Action {
        action_type: String,
        parameters: serde_json::Value,
    },
}

/// Prompt templates for structured reasoning
pub struct ReasoningPrompts;

impl ReasoningPrompts {
    /// Generate a system prompt that instructs the LLM to show reasoning
    pub fn buyer_evaluation_prompt() -> String {
        r#"You are an AI agent evaluating a service offer. Your decision must be transparent and verifiable.

IMPORTANT: Think step by step and show your reasoning clearly. Each step should be labeled.

Respond in JSON format with the following structure:
{
  "steps": [
    {"category": "Step Category", "reasoning": "Your reasoning here", "confidence": 0.0-1.0}
  ],
  "decision": {
    "type": "accept" | "reject" | "counter_offer" | "request_info",
    "reason": "Brief explanation",
    "proposed_amount": null or number (for counter_offer),
    "questions": [] (for request_info)
  }
}

Consider these factors:
1. Budget Constraints: Do I have sufficient funds? What's my remaining budget?
2. Value Assessment: Is the price fair for the service offered?
3. Risk Analysis: What could go wrong? Are there delivery guarantees?
4. Priority Check: Is this service aligned with my current objectives?
5. Trust Evaluation: What's the seller's reputation and track record?"#.to_string()
    }

    /// Generate a prompt for seller pricing decisions
    pub fn seller_pricing_prompt() -> String {
        r#"You are an AI agent determining pricing for a service. Your reasoning must be transparent.

IMPORTANT: Think step by step and show your reasoning clearly.

Respond in JSON format with the following structure:
{
  "steps": [
    {"category": "Step Category", "reasoning": "Your reasoning here", "confidence": 0.0-1.0}
  ],
  "decision": {
    "type": "action",
    "action_type": "set_price",
    "parameters": {
      "price": number,
      "currency": "IUSD",
      "valid_until": "ISO datetime"
    }
  }
}

Consider these factors:
1. Cost Analysis: What are my costs to deliver this service?
2. Market Position: What do competitors charge?
3. Value Delivered: What's the value to the buyer?
4. Demand Assessment: How urgent is buyer demand?
5. Relationship Factor: Is this a new or returning customer?"#.to_string()
    }

    /// Generate a prompt for arbiter dispute resolution
    pub fn arbiter_resolution_prompt() -> String {
        r#"You are an AI arbiter resolving a dispute between a buyer and seller. Your decision must be fair, transparent, and well-reasoned.

IMPORTANT: Examine all evidence carefully and show your reasoning step by step.

Respond in JSON format with the following structure:
{
  "steps": [
    {"category": "Step Category", "reasoning": "Your reasoning here", "confidence": 0.0-1.0}
  ],
  "decision": {
    "type": "action",
    "action_type": "resolve_dispute",
    "parameters": {
      "ruling": "buyer_wins" | "seller_wins" | "split",
      "refund_percentage": 0-100,
      "explanation": "Detailed ruling explanation"
    }
  }
}

Consider these factors:
1. Evidence Review: What proof has each party provided?
2. Contract Terms: What did the original agreement specify?
3. Delivery Verification: Was the service actually delivered as promised?
4. Quality Assessment: Did the delivery meet reasonable expectations?
5. Precedent Check: How have similar disputes been resolved?"#.to_string()
    }
}

/// Parse LLM response into a ReasoningTrace
pub fn parse_reasoning_response(
    response: &str,
    agent_id: &str,
    context: &str,
) -> Result<ReasoningTrace, String> {
    // Try to parse as JSON
    let json: serde_json::Value =
        serde_json::from_str(response).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut trace = ReasoningTrace::new(agent_id, context);

    // Parse steps
    if let Some(steps) = json.get("steps").and_then(|s| s.as_array()) {
        for step in steps {
            let category = step
                .get("category")
                .and_then(|c| c.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let reasoning = step
                .get("reasoning")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            let confidence = step.get("confidence").and_then(|c| c.as_f64()).map(|c| c as f32);

            trace.steps.push(ReasoningStep {
                step: trace.steps.len() as u32 + 1,
                category,
                reasoning,
                confidence,
            });
        }
    }

    // Parse decision
    if let Some(decision) = json.get("decision") {
        let decision_type = decision
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("pending");

        trace.decision = match decision_type {
            "accept" => AgentDecision::Accept {
                reason: decision
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "reject" => AgentDecision::Reject {
                reason: decision
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "counter_offer" => AgentDecision::CounterOffer {
                reason: decision
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string(),
                proposed_amount: decision.get("proposed_amount").and_then(|a| a.as_u64()),
                proposed_terms: decision
                    .get("proposed_terms")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string()),
            },
            "request_info" => AgentDecision::RequestInfo {
                questions: decision
                    .get("questions")
                    .and_then(|q| q.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|q| q.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            "escalate" => AgentDecision::Escalate {
                reason: decision
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            "action" => AgentDecision::Action {
                action_type: decision
                    .get("action_type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                parameters: decision
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            },
            _ => AgentDecision::Pending,
        };
    }

    Ok(trace)
}

/// Display formatter for terminal output
pub fn format_reasoning_trace(trace: &ReasoningTrace) -> String {
    let mut output = String::new();

    output.push_str(&format!("┌─ Reasoning Trace: {} ─┐\n", trace.trace_id));
    output.push_str(&format!("│ Agent: {}\n", trace.agent_id));
    output.push_str(&format!("│ Context: {}\n", trace.context));
    if let Some(ref model) = trace.model {
        output.push_str(&format!("│ Model: {}\n", model));
    }
    output.push_str("├────────────────────────────────────────┤\n");

    for step in &trace.steps {
        let confidence_str = step
            .confidence
            .map(|c| format!(" [{:.0}%]", c * 100.0))
            .unwrap_or_default();

        output.push_str(&format!(
            "│ {}. {}{}\n",
            step.step, step.category, confidence_str
        ));

        // Wrap reasoning text
        for line in step.reasoning.lines() {
            output.push_str(&format!("│    {}\n", line));
        }
    }

    output.push_str("├────────────────────────────────────────┤\n");
    output.push_str(&format!("│ Decision: {:?}\n", trace.decision));
    output.push_str(&format!("│ Duration: {}ms\n", trace.duration_ms));
    output.push_str("└────────────────────────────────────────┘\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_trace() {
        let mut trace = ReasoningTrace::new("buyer_alice", "Evaluating DataCorp offer");
        trace.add_step("Budget Check", "Have $500, offer is $100. Within budget.");
        trace.add_step_with_confidence("Value Assessment", "Service appears fairly priced.", 0.85);
        trace.decide(AgentDecision::Accept {
            reason: "Good value within budget".to_string(),
        });

        assert_eq!(trace.steps.len(), 2);
        matches!(trace.decision, AgentDecision::Accept { .. });
    }

    #[test]
    fn test_parse_reasoning_response() {
        let json = r#"{
            "steps": [
                {"category": "Budget", "reasoning": "Sufficient funds", "confidence": 0.9}
            ],
            "decision": {
                "type": "accept",
                "reason": "Good deal"
            }
        }"#;

        let trace = parse_reasoning_response(json, "agent_1", "Test").unwrap();
        assert_eq!(trace.steps.len(), 1);
        matches!(trace.decision, AgentDecision::Accept { .. });
    }
}
