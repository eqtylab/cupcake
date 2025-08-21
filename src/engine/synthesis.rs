//! Decision Synthesis Layer - The Intelligence Layer in Rust
//! 
//! Implements the NEW_GUIDING_FINAL.md synthesis logic that transforms
//! the aggregated DecisionSet from Rego into a single, prioritized FinalDecision.
//! 
//! This is where the Hybrid Model's intelligence resides - applying strict
//! prioritization and handling Claude Code API semantics.

use anyhow::Result;
use tracing::{debug, info};

use super::decision::{DecisionSet, DecisionObject, FinalDecision};

/// The Decision Synthesis Engine
/// 
/// This is the core intelligence that implements the strict priority hierarchy
/// defined in NEW_GUIDING_FINAL.md: Halt > Deny/Block > Ask > Allow
pub struct SynthesisEngine;

impl SynthesisEngine {
    /// Synthesize a DecisionSet into a single FinalDecision
    /// 
    /// This is the primary function of the Intelligence Layer.
    /// It applies the strict priority hierarchy and aggregates reasons.
    pub fn synthesize(decision_set: &DecisionSet) -> Result<FinalDecision> {
        info!("Synthesizing decision from {} total decisions", decision_set.decision_count());
        
        // Enhanced debug logging to understand what we're synthesizing
        eprintln!("==== SYNTHESIS INPUT ====");
        eprintln!("Halts: {}", decision_set.halts.len());
        eprintln!("Denials: {}", decision_set.denials.len());
        eprintln!("Blocks: {}", decision_set.blocks.len());
        eprintln!("Asks: {}", decision_set.asks.len());
        eprintln!("Allow Overrides: {}", decision_set.allow_overrides.len());
        eprintln!("Context Items: {}", decision_set.add_context.len());
        if !decision_set.denials.is_empty() {
            eprintln!("Denial reasons: {:?}", decision_set.denials.iter().map(|d| &d.reason).collect::<Vec<_>>());
        }
        if !decision_set.asks.is_empty() {
            eprintln!("Ask reasons: {:?}", decision_set.asks.iter().map(|d| &d.reason).collect::<Vec<_>>());
        }
        eprintln!("========================");
        
        // Apply strict priority hierarchy
        
        // Priority 1: Halt (Highest - immediate cessation)
        if decision_set.has_halts() {
            let reason = Self::aggregate_reasons(&decision_set.halts);
            debug!("Synthesized HALT decision: {}", reason);
            return Ok(FinalDecision::Halt { reason });
        }
        
        // Priority 2: Deny/Block (High - blocking actions)
        if decision_set.has_denials() {
            let reason = Self::aggregate_reasons(&decision_set.denials);
            debug!("Synthesized DENY decision: {}", reason);
            return Ok(FinalDecision::Deny { reason });
        }
        
        if decision_set.has_blocks() {
            let reason = Self::aggregate_reasons(&decision_set.blocks);
            debug!("Synthesized BLOCK decision: {}", reason);
            return Ok(FinalDecision::Block { reason });
        }
        
        // Priority 3: Ask (Medium - user confirmation required)
        if decision_set.has_asks() {
            let reason = Self::aggregate_reasons(&decision_set.asks);
            debug!("Synthesized ASK decision: {}", reason);
            return Ok(FinalDecision::Ask { reason });
        }
        
        // Priority 4: Allow Override (Low - explicit permission)
        if decision_set.has_allow_overrides() {
            let reason = Self::aggregate_reasons(&decision_set.allow_overrides);
            debug!("Synthesized ALLOW OVERRIDE decision: {}", reason);
            return Ok(FinalDecision::AllowOverride { reason });
        }
        
        // Priority 5: Allow (Default - with optional context)
        let context = decision_set.add_context.clone();
        if !context.is_empty() {
            debug!("Synthesized ALLOW decision with {} context items", context.len());
        } else {
            debug!("Synthesized ALLOW decision (no policies triggered)");
        }
        
        Ok(FinalDecision::Allow { context })
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
                parts.push(format!("Multiple high-severity policy violations detected: {}",
                    high_decisions.iter()
                        .map(|d| format!("[{}] {}", d.rule_id, d.reason))
                        .collect::<Vec<_>>()
                        .join("; ")));
            }
        }
        
        // Add medium severity if no high severity
        if high_decisions.is_empty() && !medium_decisions.is_empty() {
            if medium_decisions.len() == 1 {
                parts.push(medium_decisions[0].reason.clone());
            } else {
                parts.push(format!("Multiple policy violations detected: {}",
                    medium_decisions.iter()
                        .map(|d| format!("[{}] {}", d.rule_id, d.reason))
                        .collect::<Vec<_>>()
                        .join("; ")));
            }
        }
        
        // Add low severity only if no higher priorities
        if high_decisions.is_empty() && medium_decisions.is_empty() && !low_decisions.is_empty() {
            if low_decisions.len() == 1 {
                parts.push(low_decisions[0].reason.clone());
            } else {
                parts.push(format!("Policy guidelines: {}",
                    low_decisions.iter()
                        .map(|d| d.reason.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")));
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
            summary_parts.push(format!("{} override(s)", decision_set.allow_overrides.len()));
        }
        if !decision_set.add_context.is_empty() {
            summary_parts.push(format!("{} context item(s)", decision_set.add_context.len()));
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
            }],
            denials: vec![DecisionObject {
                reason: "Denied".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "DENY-001".to_string(),
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
            },
            DecisionObject {
                reason: "Second violation".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-002".to_string(),
            },
        ];
        
        let result = SynthesisEngine::aggregate_reasons(&decisions);
        assert!(result.contains("Multiple high-severity policy violations"));
        assert!(result.contains("[TEST-001]"));
        assert!(result.contains("[TEST-002]"));
    }
    
    #[test]
    fn test_decision_set_summary() {
        let decision_set = DecisionSet {
            denials: vec![DecisionObject {
                reason: "Test".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "TEST-001".to_string(),
            }],
            asks: vec![DecisionObject {
                reason: "Test ask".to_string(),
                severity: "MEDIUM".to_string(),
                rule_id: "TEST-002".to_string(),
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

// Aligns with NEW_GUIDING_FINAL.md:
// - Implements the Intelligence Layer in Rust (Hybrid Model)
// - Applies strict prioritization: Halt > Deny/Block > Ask > Allow
// - Aggregates multiple decisions of same priority into coherent messages
// - Handles edge cases (empty sets, mixed priorities)
// - Provides clear debugging and logging capabilities
// - Foundation for Claude Code API semantic mapping