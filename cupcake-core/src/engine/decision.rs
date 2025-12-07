//! Decision types - [`DecisionSet`] from WASM and [`FinalDecision`] after synthesis.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single decision object from a policy rule
/// This is the standard format returned by decision verb rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionObject {
    /// Human-readable explanation of the decision
    pub reason: String,

    /// Severity level (HIGH, MEDIUM, LOW)
    pub severity: String,

    /// Unique identifier for the rule that generated this decision
    pub rule_id: String,

    /// Optional agent-specific context (technical details for the agent)
    /// Separate from `reason` which is user-facing
    /// Currently used by Cursor harness to populate `agentMessage` field
    #[serde(default)]
    pub agent_context: Option<String>,
}

/// A modification decision that transforms tool input before execution
/// This is used by the `modify` verb to sanitize or transform commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModificationObject {
    /// Human-readable explanation of the modification
    pub reason: String,

    /// Severity level (HIGH, MEDIUM, LOW)
    #[serde(default = "default_severity")]
    pub severity: String,

    /// Unique identifier for the rule that generated this modification
    pub rule_id: String,

    /// Priority for conflict resolution when multiple policies modify (1-100, higher wins)
    /// Default is 50 (medium priority)
    #[serde(default = "default_priority")]
    pub priority: u8,

    /// The modified input parameters to use instead of the original
    /// This will be passed as `updatedInput` in the response
    pub updated_input: Value,

    /// Optional agent-specific context
    #[serde(default)]
    pub agent_context: Option<String>,
}

fn default_severity() -> String {
    "MEDIUM".to_string()
}

fn default_priority() -> u8 {
    50
}

/// The complete set of all decisions from Rego aggregation
/// This is what the single cupcake.system.evaluate entrypoint returns
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionSet {
    /// Immediate cessation decisions (highest priority)
    #[serde(default)]
    pub halts: Vec<DecisionObject>,

    /// Policy violation decisions (high priority)
    #[serde(default)]
    pub denials: Vec<DecisionObject>,

    /// Blocking decisions for specific events (high priority)
    #[serde(default)]
    pub blocks: Vec<DecisionObject>,

    /// User confirmation required decisions (medium priority)
    #[serde(default)]
    pub asks: Vec<DecisionObject>,

    /// Input modification decisions (medium-low priority)
    /// Allows policies to transform tool input before execution
    #[serde(default)]
    pub modifications: Vec<ModificationObject>,

    /// Explicit permission override decisions (low priority)
    #[serde(default)]
    pub allow_overrides: Vec<DecisionObject>,

    /// Context injection decisions (informational)
    #[serde(default)]
    pub add_context: Vec<String>,

    /// Agent-specific messages collected from agent_context fields
    /// Used by Cursor harness for separate user/agent messaging
    #[serde(default, skip_deserializing)]
    pub agent_messages: Vec<String>,
}

/// The final decision after synthesis by the Rust Intelligence Layer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FinalDecision {
    /// Immediate halt - highest priority
    Halt {
        reason: String,
        agent_messages: Vec<String>,
    },

    /// Deny the action - high priority
    Deny {
        reason: String,
        agent_messages: Vec<String>,
    },

    /// Block progression - high priority
    Block {
        reason: String,
        agent_messages: Vec<String>,
    },

    /// Ask user for confirmation - medium priority
    Ask {
        reason: String,
        agent_messages: Vec<String>,
    },

    /// Modify input and allow - medium-low priority
    /// The action proceeds with the modified input parameters
    Modify {
        reason: String,
        updated_input: Value,
        agent_messages: Vec<String>,
    },

    /// Allow with explicit override - low priority
    AllowOverride {
        reason: String,
        agent_messages: Vec<String>,
    },

    /// Allow with optional context - default
    Allow { context: Vec<String> },
}

impl FinalDecision {
    /// Check if this decision should halt execution entirely
    pub fn is_halt(&self) -> bool {
        matches!(self, FinalDecision::Halt { .. })
    }

    /// Check if this decision blocks the action
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            FinalDecision::Deny { .. } | FinalDecision::Block { .. }
        )
    }

    /// Check if this decision requires user confirmation
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, FinalDecision::Ask { .. })
    }

    /// Check if this is an Ask decision (alias for requires_confirmation)
    pub fn is_ask(&self) -> bool {
        self.requires_confirmation()
    }

    /// Check if this decision modifies input
    pub fn is_modify(&self) -> bool {
        matches!(self, FinalDecision::Modify { .. })
    }

    /// Get the primary reason for this decision
    pub fn reason(&self) -> Option<&str> {
        match self {
            FinalDecision::Halt { reason, .. } => Some(reason),
            FinalDecision::Deny { reason, .. } => Some(reason),
            FinalDecision::Block { reason, .. } => Some(reason),
            FinalDecision::Ask { reason, .. } => Some(reason),
            FinalDecision::Modify { reason, .. } => Some(reason),
            FinalDecision::AllowOverride { reason, .. } => Some(reason),
            FinalDecision::Allow { .. } => None,
        }
    }

    /// Get agent-specific messages if present
    pub fn agent_messages(&self) -> Option<&Vec<String>> {
        match self {
            FinalDecision::Halt { agent_messages, .. } => Some(agent_messages),
            FinalDecision::Deny { agent_messages, .. } => Some(agent_messages),
            FinalDecision::Block { agent_messages, .. } => Some(agent_messages),
            FinalDecision::Ask { agent_messages, .. } => Some(agent_messages),
            FinalDecision::Modify { agent_messages, .. } => Some(agent_messages),
            FinalDecision::AllowOverride { agent_messages, .. } => Some(agent_messages),
            FinalDecision::Allow { .. } => None,
        }
    }

    /// Get the updated input if this is a Modify decision
    pub fn updated_input(&self) -> Option<&Value> {
        match self {
            FinalDecision::Modify { updated_input, .. } => Some(updated_input),
            _ => None,
        }
    }
}

impl DecisionSet {
    /// Check if any halt decisions are present
    pub fn has_halts(&self) -> bool {
        !self.halts.is_empty()
    }

    /// Check if any denial decisions are present
    pub fn has_denials(&self) -> bool {
        !self.denials.is_empty()
    }

    /// Check if any block decisions are present
    pub fn has_blocks(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Check if any ask decisions are present
    pub fn has_asks(&self) -> bool {
        !self.asks.is_empty()
    }

    /// Check if any allow override decisions are present
    pub fn has_allow_overrides(&self) -> bool {
        !self.allow_overrides.is_empty()
    }

    /// Check if any modification decisions are present
    pub fn has_modifications(&self) -> bool {
        !self.modifications.is_empty()
    }

    /// Check if the decision set is completely empty (no decisions)
    pub fn is_empty(&self) -> bool {
        self.halts.is_empty()
            && self.denials.is_empty()
            && self.blocks.is_empty()
            && self.asks.is_empty()
            && self.modifications.is_empty()
            && self.allow_overrides.is_empty()
            && self.add_context.is_empty()
    }

    /// Get all decision objects across all categories for debugging
    pub fn all_decisions(&self) -> Vec<&DecisionObject> {
        let mut decisions = Vec::new();
        decisions.extend(&self.halts);
        decisions.extend(&self.denials);
        decisions.extend(&self.blocks);
        decisions.extend(&self.asks);
        decisions.extend(&self.allow_overrides);
        decisions
    }

    /// Count total number of decisions
    pub fn decision_count(&self) -> usize {
        self.halts.len()
            + self.denials.len()
            + self.blocks.len()
            + self.asks.len()
            + self.modifications.len()
            + self.allow_overrides.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_set_priorities() {
        let mut decision_set = DecisionSet::default();

        // Empty set should be empty
        assert!(decision_set.is_empty());
        assert_eq!(decision_set.decision_count(), 0);

        // Add a denial
        decision_set.denials.push(DecisionObject {
            reason: "Test denial".to_string(),
            severity: "HIGH".to_string(),
            rule_id: "TEST-001".to_string(),
            agent_context: None,
        });

        assert!(!decision_set.is_empty());
        assert!(decision_set.has_denials());
        assert!(!decision_set.has_halts());
        assert_eq!(decision_set.decision_count(), 1);
    }

    #[test]
    fn test_final_decision_properties() {
        let halt = FinalDecision::Halt {
            reason: "Emergency stop".to_string(),
            agent_messages: vec!["Technical details".to_string()],
        };

        assert!(halt.is_halt());
        assert!(!halt.is_blocking());
        assert!(!halt.requires_confirmation());
        assert_eq!(halt.reason(), Some("Emergency stop"));
        assert_eq!(
            halt.agent_messages(),
            Some(&vec!["Technical details".to_string()])
        );

        let deny = FinalDecision::Deny {
            reason: "Policy violation".to_string(),
            agent_messages: vec![],
        };

        assert!(!deny.is_halt());
        assert!(deny.is_blocking());
        assert!(!deny.requires_confirmation());

        let ask = FinalDecision::Ask {
            reason: "Confirmation needed".to_string(),
            agent_messages: vec![],
        };

        assert!(!ask.is_halt());
        assert!(!ask.is_blocking());
        assert!(ask.requires_confirmation());

        let allow = FinalDecision::Allow {
            context: vec!["Info message".to_string()],
        };

        assert!(!allow.is_halt());
        assert!(!allow.is_blocking());
        assert!(!allow.requires_confirmation());
        assert_eq!(allow.reason(), None);
        assert_eq!(allow.agent_messages(), None);
    }
}
