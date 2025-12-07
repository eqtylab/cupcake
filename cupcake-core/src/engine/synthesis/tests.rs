//! Tests for the synthesis engine

use super::super::decision::ModificationObject;
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

#[test]
fn test_modify_decision_basic() {
    use serde_json::json;

    let decision_set = DecisionSet {
        modifications: vec![ModificationObject {
            reason: "Sanitized dangerous command".to_string(),
            severity: "HIGH".to_string(),
            rule_id: "SANITIZE-001".to_string(),
            priority: 50,
            updated_input: json!({"command": "echo safe"}),
            agent_context: None,
        }],
        ..Default::default()
    };

    let result = SynthesisEngine::synthesize(&decision_set).unwrap();

    match result {
        FinalDecision::Modify {
            reason,
            updated_input,
            agent_messages,
        } => {
            assert!(reason.contains("Sanitized dangerous command"));
            assert_eq!(updated_input["command"], "echo safe");
            assert!(agent_messages.is_empty());
        }
        _ => panic!("Expected Modify decision"),
    }
}

#[test]
fn test_modify_priority_below_ask() {
    use serde_json::json;

    // Ask should take precedence over Modify
    let decision_set = DecisionSet {
        asks: vec![DecisionObject {
            reason: "Confirm this action".to_string(),
            severity: "MEDIUM".to_string(),
            rule_id: "ASK-001".to_string(),
            agent_context: None,
        }],
        modifications: vec![ModificationObject {
            reason: "Would have modified".to_string(),
            severity: "HIGH".to_string(),
            rule_id: "MOD-001".to_string(),
            priority: 50,
            updated_input: json!({"command": "echo safe"}),
            agent_context: None,
        }],
        ..Default::default()
    };

    let result = SynthesisEngine::synthesize(&decision_set).unwrap();
    assert!(result.is_ask(), "Ask should have higher priority than Modify");
}

#[test]
fn test_modify_merge_multiple_modifications() {
    use serde_json::json;

    // Multiple modifications should be merged
    let decision_set = DecisionSet {
        modifications: vec![
            ModificationObject {
                reason: "Sanitize path".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "SANITIZE-PATH".to_string(),
                priority: 80, // Higher priority
                updated_input: json!({"path": "/safe/path", "nested": {"key1": "value1"}}),
                agent_context: Some("Path was sanitized".to_string()),
            },
            ModificationObject {
                reason: "Add timeout".to_string(),
                severity: "MEDIUM".to_string(),
                rule_id: "ADD-TIMEOUT".to_string(),
                priority: 50, // Lower priority
                updated_input: json!({"timeout": 30, "nested": {"key2": "value2"}}),
                agent_context: None,
            },
        ],
        ..Default::default()
    };

    let result = SynthesisEngine::synthesize(&decision_set).unwrap();

    match result {
        FinalDecision::Modify {
            reason,
            updated_input,
            agent_messages,
        } => {
            // Both modifications should be mentioned in reason
            assert!(reason.contains("Sanitize path") || reason.contains("Add timeout"));

            // Higher priority wins for conflicting nested key
            assert_eq!(updated_input["path"], "/safe/path");
            assert_eq!(updated_input["timeout"], 30);

            // Nested objects: higher priority's key1 should be present,
            // lower priority's key2 should be merged in
            assert_eq!(updated_input["nested"]["key1"], "value1");
            assert_eq!(updated_input["nested"]["key2"], "value2");

            // Agent context from first modification
            assert!(agent_messages.contains(&"Path was sanitized".to_string()));
        }
        _ => panic!("Expected Modify decision"),
    }
}

#[test]
fn test_modify_priority_conflict_resolution() {
    use serde_json::json;

    // When two modifications have conflicting keys, higher priority wins
    let decision_set = DecisionSet {
        modifications: vec![
            ModificationObject {
                reason: "High priority mod".to_string(),
                severity: "HIGH".to_string(),
                rule_id: "HIGH-PRIORITY".to_string(),
                priority: 100,
                updated_input: json!({"command": "high-priority-command"}),
                agent_context: None,
            },
            ModificationObject {
                reason: "Low priority mod".to_string(),
                severity: "LOW".to_string(),
                rule_id: "LOW-PRIORITY".to_string(),
                priority: 10,
                updated_input: json!({"command": "low-priority-command"}),
                agent_context: None,
            },
        ],
        ..Default::default()
    };

    let result = SynthesisEngine::synthesize(&decision_set).unwrap();

    match result {
        FinalDecision::Modify { updated_input, .. } => {
            // Higher priority (100) should win over lower priority (10)
            assert_eq!(updated_input["command"], "high-priority-command");
        }
        _ => panic!("Expected Modify decision"),
    }
}
