//! Decision types for the Hybrid Model
//! 
//! Implements the NEW_GUIDING_FINAL.md DecisionSet architecture.
//! This replaces the deprecated decision object model with the modern
//! decision verb aggregation system.

use serde::{Deserialize, Serialize};

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
    
    /// Explicit permission override decisions (low priority)
    #[serde(default)]
    pub allow_overrides: Vec<DecisionObject>,
    
    /// Context injection decisions (informational)
    #[serde(default)]
    pub add_context: Vec<String>,
}

/// The final decision after synthesis by the Rust Intelligence Layer
#[derive(Debug, Clone, PartialEq)]
pub enum FinalDecision {
    /// Immediate halt - highest priority
    Halt {
        reason: String,
    },
    
    /// Deny the action - high priority
    Deny {
        reason: String,
    },
    
    /// Block progression - high priority
    Block {
        reason: String,
    },
    
    /// Ask user for confirmation - medium priority
    Ask {
        reason: String,
    },
    
    /// Allow with explicit override - low priority
    AllowOverride {
        reason: String,
    },
    
    /// Allow with optional context - default
    Allow {
        context: Vec<String>,
    },
}

impl FinalDecision {
    /// Check if this decision should halt execution entirely
    pub fn is_halt(&self) -> bool {
        matches!(self, FinalDecision::Halt { .. })
    }
    
    /// Check if this decision blocks the action
    pub fn is_blocking(&self) -> bool {
        matches!(self, FinalDecision::Deny { .. } | FinalDecision::Block { .. })
    }
    
    /// Check if this decision requires user confirmation
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, FinalDecision::Ask { .. })
    }
    
    /// Get the primary reason for this decision
    pub fn reason(&self) -> Option<&str> {
        match self {
            FinalDecision::Halt { reason } => Some(reason),
            FinalDecision::Deny { reason } => Some(reason),
            FinalDecision::Block { reason } => Some(reason),
            FinalDecision::Ask { reason } => Some(reason),
            FinalDecision::AllowOverride { reason } => Some(reason),
            FinalDecision::Allow { .. } => None,
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
    
    /// Check if the decision set is completely empty (no decisions)
    pub fn is_empty(&self) -> bool {
        self.halts.is_empty()
            && self.denials.is_empty()
            && self.blocks.is_empty()
            && self.asks.is_empty()
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
            + self.allow_overrides.len()
    }
}

// Legacy compatibility - will be removed in Phase 3
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViolationObject {
    pub id: String,
    pub msg: String,
    pub meta: serde_json::Value,
    pub feedback: ViolationFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViolationFeedback {
    #[serde(rename = "permissionDecision")]
    pub permission_decision: String,
    #[serde(rename = "permissionDecisionReason")]
    pub permission_decision_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionObjectLegacy {
    #[serde(default)]
    pub deny: Vec<ViolationObject>,
    #[serde(default)]
    pub additional_context: Vec<String>,
    #[serde(default)]
    pub missing_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Ask,
    Deny,
}

impl Default for PermissionDecision {
    fn default() -> Self {
        PermissionDecision::Allow
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregatedDecision {
    pub final_decision: PermissionDecision,
    pub violations: Vec<ViolationObject>,
    pub additional_context: Vec<String>,
    pub missing_signals: Vec<String>,
    pub primary_reason: Option<String>,
}

impl AggregatedDecision {
    pub fn from_decision(decision: DecisionObjectLegacy) -> Self {
        let final_decision = if !decision.deny.is_empty() {
            PermissionDecision::Deny
        } else {
            PermissionDecision::Allow
        };
        
        let primary_reason = decision.deny.first()
            .map(|v| v.feedback.permission_decision_reason.clone());
        
        Self {
            final_decision,
            violations: decision.deny,
            additional_context: decision.additional_context,
            missing_signals: decision.missing_signals,
            primary_reason,
        }
    }
    
    pub fn from_decisions(decisions: Vec<DecisionObjectLegacy>) -> Self {
        let mut aggregated = DecisionObjectLegacy::default();
        
        for decision in decisions {
            aggregated.deny.extend(decision.deny);
            aggregated.additional_context.extend(decision.additional_context);
            aggregated.missing_signals.extend(decision.missing_signals);
        }
        
        aggregated.missing_signals.sort();
        aggregated.missing_signals.dedup();
        
        Self::from_decision(aggregated)
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
        };
        
        assert!(halt.is_halt());
        assert!(!halt.is_blocking());
        assert!(!halt.requires_confirmation());
        assert_eq!(halt.reason(), Some("Emergency stop"));
        
        let deny = FinalDecision::Deny {
            reason: "Policy violation".to_string(),
        };
        
        assert!(!deny.is_halt());
        assert!(deny.is_blocking());
        assert!(!deny.requires_confirmation());
        
        let ask = FinalDecision::Ask {
            reason: "Confirmation needed".to_string(),
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
    }
}

// Aligns with NEW_GUIDING_FINAL.md:
// - DecisionSet mirrors the exact WASM output structure
// - FinalDecision represents synthesized outcomes from Rust Intelligence Layer
// - Clear separation between Rego aggregation and Rust synthesis
// - Implements strict priority hierarchy: Halt > Deny/Block > Ask > Allow
// - Foundation for the Hybrid Model's two-layer decision architecture