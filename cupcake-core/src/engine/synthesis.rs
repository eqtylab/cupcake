//! Decision Synthesis Layer - Transforms [`DecisionSet`] into [`FinalDecision`].
//!
//! Applies strict priority: Halt > Deny/Block > Ask > Allow.

use anyhow::Result;
use std::time::Instant;
use tracing::{debug, info, instrument, trace};

use super::decision::{DecisionObject, DecisionSet, FinalDecision};

/// The Decision Synthesis Engine.
///
/// Implements strict priority: Halt > Deny/Block > Ask > Allow.
pub struct SynthesisEngine;

impl SynthesisEngine {
    /// Synthesize a DecisionSet into a single FinalDecision
    ///
    /// This is the primary function of the Intelligence Layer.
    /// It applies the strict priority hierarchy and aggregates reasons.
    #[instrument(
        name = "synthesize",
        skip(decision_set),
        fields(
            total_decisions = decision_set.decision_count(),
            halts = decision_set.halts.len(),
            denials = decision_set.denials.len(),
            blocks = decision_set.blocks.len(),
            asks = decision_set.asks.len(),
            final_decision_type = tracing::field::Empty,
            synthesis_time_us = tracing::field::Empty
        )
    )]
    pub fn synthesize(decision_set: &DecisionSet) -> Result<FinalDecision> {
        let start = Instant::now();
        info!(
            "Synthesizing decision from {} total decisions",
            decision_set.decision_count()
        );

        debug!("Synthesis input - Halts: {}, Denials: {}, Blocks: {}, Asks: {}, Allow Overrides: {}, Context Items: {}",
            decision_set.halts.len(),
            decision_set.denials.len(),
            decision_set.blocks.len(),
            decision_set.asks.len(),
            decision_set.allow_overrides.len(),
            decision_set.add_context.len());

        // Apply strict priority hierarchy

        // Helper to record decision type and duration
        let record_and_return = |decision_type: &str, decision: FinalDecision| {
            let duration = start.elapsed();
            let current_span = tracing::Span::current();
            current_span.record("final_decision_type", decision_type);
            current_span.record("synthesis_time_us", duration.as_micros());
            trace!(
                decision_type = decision_type,
                duration_us = duration.as_micros(),
                "Synthesis complete"
            );
            Ok(decision)
        };

        // Priority 1: Halt (Highest - immediate cessation)
        if decision_set.has_halts() {
            let reason = Self::aggregate_reasons(&decision_set.halts);
            let agent_messages = Self::collect_agent_messages(&decision_set.halts);
            debug!("Synthesized HALT decision: {}", reason);
            return record_and_return(
                "Halt",
                FinalDecision::Halt {
                    reason,
                    agent_messages,
                },
            );
        }

        // Priority 2: Deny/Block (High - blocking actions)
        if decision_set.has_denials() {
            let reason = Self::aggregate_reasons(&decision_set.denials);
            let agent_messages = Self::collect_agent_messages(&decision_set.denials);
            debug!("Synthesized DENY decision: {}", reason);
            return record_and_return(
                "Deny",
                FinalDecision::Deny {
                    reason,
                    agent_messages,
                },
            );
        }

        if decision_set.has_blocks() {
            let reason = Self::aggregate_reasons(&decision_set.blocks);
            let agent_messages = Self::collect_agent_messages(&decision_set.blocks);
            debug!("Synthesized BLOCK decision: {}", reason);
            return record_and_return(
                "Block",
                FinalDecision::Block {
                    reason,
                    agent_messages,
                },
            );
        }

        // Priority 3: Ask (Medium - user confirmation required)
        if decision_set.has_asks() {
            let reason = Self::aggregate_reasons(&decision_set.asks);
            let agent_messages = Self::collect_agent_messages(&decision_set.asks);
            debug!("Synthesized ASK decision: {}", reason);
            return record_and_return(
                "Ask",
                FinalDecision::Ask {
                    reason,
                    agent_messages,
                },
            );
        }

        // Priority 4: Allow Override (Low - explicit permission)
        if decision_set.has_allow_overrides() {
            let reason = Self::aggregate_reasons(&decision_set.allow_overrides);
            let agent_messages = Self::collect_agent_messages(&decision_set.allow_overrides);
            debug!("Synthesized ALLOW OVERRIDE decision: {}", reason);
            return record_and_return(
                "AllowOverride",
                FinalDecision::AllowOverride {
                    reason,
                    agent_messages,
                },
            );
        }

        // Priority 5: Allow (Default - with optional context)
        let context = decision_set.add_context.clone();
        if !context.is_empty() {
            debug!(
                "Synthesized ALLOW decision with {} context items",
                context.len()
            );
        } else {
            debug!("Synthesized ALLOW decision (no policies triggered)");
        }

        record_and_return("Allow", FinalDecision::Allow { context })
    }

    /// Collect agent-specific messages from decisions
    ///
    /// Extracts all agent_context fields from DecisionObjects.
    /// Used by Cursor harness for separate user/agent messaging.
    fn collect_agent_messages(decisions: &[DecisionObject]) -> Vec<String> {
        decisions
            .iter()
            .filter_map(|d| d.agent_context.clone())
            .collect()
    }

    /// Aggregate multiple decision reasons into a single, clear message
    ///
    /// This handles the case where multiple policies of the same priority
    /// fire simultaneously, providing a coherent explanation to the user.
    fn aggregate_reasons(decisions: &[DecisionObject]) -> String {
        if decisions.is_empty() {
            return "Policy evaluation completed".to_string();
        }

        if decisions.len() == 1 {
            return decisions[0].reason.clone();
        }

        // Multiple decisions - group by severity and create a structured message
        let mut high_decisions = Vec::new();
        let mut medium_decisions = Vec::new();
        let mut low_decisions = Vec::new();

        for decision in decisions {
            match decision.severity.to_uppercase().as_str() {
                "HIGH" | "CRITICAL" => high_decisions.push(decision),
                "MEDIUM" | "MODERATE" => medium_decisions.push(decision),
                _ => low_decisions.push(decision),
            }
        }

        let mut parts = Vec::new();

        // Start with highest severity
        if !high_decisions.is_empty() {
            if high_decisions.len() == 1 {
                parts.push(high_decisions[0].reason.clone());
            } else {
                parts.push(format!(
                    "Multiple high-severity policy violations detected: {}",
                    high_decisions
                        .iter()
                        .map(|d| format!("[{}] {}", d.rule_id, d.reason))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }

        // Add medium severity if no high severity
        if high_decisions.is_empty() && !medium_decisions.is_empty() {
            if medium_decisions.len() == 1 {
                parts.push(medium_decisions[0].reason.clone());
            } else {
                parts.push(format!(
                    "Multiple policy violations detected: {}",
                    medium_decisions
                        .iter()
                        .map(|d| format!("[{}] {}", d.rule_id, d.reason))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }

        // Add low severity only if no higher priorities
        if high_decisions.is_empty() && medium_decisions.is_empty() && !low_decisions.is_empty() {
            if low_decisions.len() == 1 {
                parts.push(low_decisions[0].reason.clone());
            } else {
                parts.push(format!(
                    "Policy guidelines: {}",
                    low_decisions
                        .iter()
                        .map(|d| d.reason.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }

        if parts.is_empty() {
            format!("Multiple policies triggered ({})", decisions.len())
        } else {
            parts.join(" ")
        }
    }

    /// Get a summary of the decision set for logging/debugging
    pub fn summarize_decision_set(decision_set: &DecisionSet) -> String {
        let mut summary_parts = Vec::new();

        if !decision_set.halts.is_empty() {
            summary_parts.push(format!("{} halt(s)", decision_set.halts.len()));
        }
        if !decision_set.denials.is_empty() {
            summary_parts.push(format!("{} denial(s)", decision_set.denials.len()));
        }
        if !decision_set.blocks.is_empty() {
            summary_parts.push(format!("{} block(s)", decision_set.blocks.len()));
        }
        if !decision_set.asks.is_empty() {
            summary_parts.push(format!("{} ask(s)", decision_set.asks.len()));
        }
        if !decision_set.allow_overrides.is_empty() {
            summary_parts.push(format!(
                "{} override(s)",
                decision_set.allow_overrides.len()
            ));
        }
        if !decision_set.add_context.is_empty() {
            summary_parts.push(format!(
                "{} context item(s)",
                decision_set.add_context.len()
            ));
        }

        if summary_parts.is_empty() {
            "No decisions".to_string()
        } else {
            summary_parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesis_priority_hierarchy() {
        // Test halt has highest priority
        let mut decision_set = DecisionSet {
            halts: vec![DecisionObject {
                reason: "Emergency stop".to_string(),
                severity: "CRITICAL".to_string(),
                rule_id: "HALT-001".to_string(),
                agent_context: None,
            }],
            denials: vec![DecisionObject {
                reason: "Denied".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "DENY-001".to_string(),
                agent_context: None,
            }],
            ..Default::default()
        };

        let result = SynthesisEngine::synthesize(&decision_set).unwrap();
        assert!(result.is_halt());
        assert_eq!(result.reason(), Some("Emergency stop"));

        // Remove halt - should now be deny
        decision_set.halts.clear();
        let result = SynthesisEngine::synthesize(&decision_set).unwrap();
        assert!(result.is_blocking());
        assert_eq!(result.reason(), Some("Denied"));
    }

    #[test]
    fn test_synthesis_empty_decision_set() {
        let decision_set = DecisionSet::default();
        let result = SynthesisEngine::synthesize(&decision_set).unwrap();

        match result {
            FinalDecision::Allow { context } => {
                assert!(context.is_empty());
            }
            _ => panic!("Expected Allow decision for empty set"),
        }
    }

    #[test]
    fn test_synthesis_with_context() {
        let decision_set = DecisionSet {
            add_context: vec![
                "Reminder: You're on main branch".to_string(),
                "Tests are failing".to_string(),
            ],
            ..Default::default()
        };

        let result = SynthesisEngine::synthesize(&decision_set).unwrap();

        match result {
            FinalDecision::Allow { context } => {
                assert_eq!(context.len(), 2);
                assert!(context.contains(&"Reminder: You're on main branch".to_string()));
            }
            _ => panic!("Expected Allow decision with context"),
        }
    }

    #[test]
    fn test_aggregate_reasons_single() {
        let decisions = vec![DecisionObject {
            reason: "Single reason".to_string(),
            severity: "HIGH".to_string(),
            rule_id: "TEST-001".to_string(),
            agent_context: None,
        }];

        let result = SynthesisEngine::aggregate_reasons(&decisions);
        assert_eq!(result, "Single reason");
    }

    #[test]
    fn test_aggregate_reasons_multiple_high_severity() {
        let decisions = vec![
            DecisionObject {
                reason: "First violation".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-001".to_string(),
                agent_context: None,
            },
            DecisionObject {
                reason: "Second violation".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-002".to_string(),
                agent_context: None,
            },
        ];

        let result = SynthesisEngine::aggregate_reasons(&decisions);
        assert!(result.contains("Multiple high-severity policy violations"));
        assert!(result.contains("[TEST-001]"));
        assert!(result.contains("[TEST-002]"));
    }

    #[test]
    fn test_collect_agent_messages() {
        let decisions = vec![
            DecisionObject {
                reason: "User message".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-001".to_string(),
                agent_context: Some("Technical details for agent".to_string()),
            },
            DecisionObject {
                reason: "Another message".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-002".to_string(),
                agent_context: None,
            },
            DecisionObject {
                reason: "Third message".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-003".to_string(),
                agent_context: Some("More agent context".to_string()),
            },
        ];

        let agent_messages = SynthesisEngine::collect_agent_messages(&decisions);
        assert_eq!(agent_messages.len(), 2);
        assert_eq!(agent_messages[0], "Technical details for agent");
        assert_eq!(agent_messages[1], "More agent context");
    }

    #[test]
    fn test_decision_set_summary() {
        let decision_set = DecisionSet {
            denials: vec![DecisionObject {
                reason: "Test".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-001".to_string(),
                agent_context: None,
            }],
            asks: vec![DecisionObject {
                reason: "Test ask".to_string(),
                severity: "MEDIUM".to_string(),
                rule_id: "TEST-002".to_string(),
                agent_context: None,
            }],
            add_context: vec!["Context message".to_string()],
            ..Default::default()
        };

        let summary = SynthesisEngine::summarize_decision_set(&decision_set);
        assert!(summary.contains("1 denial(s)"));
        assert!(summary.contains("1 ask(s)"));
        assert!(summary.contains("1 context item(s)"));
    }
}

