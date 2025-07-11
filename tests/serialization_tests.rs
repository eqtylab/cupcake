use cupcake::config::{
    actions::{Action, OnFailureBehavior},
    conditions::{Condition, StateQueryFilter},
    types::{HookEventType, Policy, PolicyFile, Settings},
};
use serde_json;
use std::collections::HashMap;

#[test]
fn test_policy_file_toml_serialization() {
    let policy_file = PolicyFile {
        schema_version: "1.0".to_string(),
        settings: Settings {
            audit_logging: true,
            debug_mode: false,
        },
        policies: vec![Policy {
            name: "Test Policy".to_string(),
            description: Some("A test policy".to_string()),
            hook_event: HookEventType::PreToolUse,
            matcher: Some("Bash".to_string()),
            conditions: vec![Condition::Pattern {
                field: "tool_input.command".to_string(),
                regex: "git\\s+commit".to_string(),
            }],
            action: Action::ProvideFeedback {
                message: "Test feedback".to_string(),
                include_context: false,
            },
        }],
    };

    // Test TOML serialization
    let toml_str = toml::to_string(&policy_file).expect("Failed to serialize to TOML");
    let deserialized: PolicyFile =
        toml::from_str(&toml_str).expect("Failed to deserialize from TOML");

    assert_eq!(deserialized.schema_version, policy_file.schema_version);
    assert_eq!(
        deserialized.settings.audit_logging,
        policy_file.settings.audit_logging
    );
    assert_eq!(deserialized.policies.len(), policy_file.policies.len());
    assert_eq!(deserialized.policies[0].name, policy_file.policies[0].name);
}

#[test]
fn test_condition_variants_serialization() {
    let conditions = vec![
        Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: "test".to_string(),
        },
        Condition::Pattern {
            field: "tool_input.file_path".to_string(),
            regex: "\\.rs$".to_string(),
        },
        Condition::Check {
            command: "echo '{{tool_input.file_path}}' | grep -q 'src/.*\\.rs$'".to_string(),
            expect_success: true,
        },
        Condition::Pattern {
            field: "tool_input.content".to_string(),
            regex: "TODO:".to_string(),
        },
        Condition::Match {
            field: "tool_name".to_string(),
            value: "Read".to_string(),
        },
        Condition::And {
            conditions: vec![
                Condition::Pattern {
                    field: "tool_input.command".to_string(),
                    regex: "git".to_string(),
                },
                Condition::Not {
                    condition: Box::new(Condition::Pattern {
                        field: "tool_input.file_path".to_string(),
                        regex: "test/".to_string(),
                    }),
                },
            ],
        },
        Condition::Check {
            command: "[ $(date +%H) -ge 09 ] && [ $(date +%H) -le 17 ]".to_string(),
            expect_success: true,
        },
        Condition::Check {
            command: "case $(date +%a) in Mon|Tue|Wed) exit 0 ;; *) exit 1 ;; esac".to_string(),
            expect_success: true,
        },
    ];

    for condition in conditions {
        let toml_str = toml::to_string(&condition).expect("Failed to serialize condition to TOML");
        let _deserialized: Condition =
            toml::from_str(&toml_str).expect("Failed to deserialize condition from TOML");
    }
}

#[test]
fn test_action_variants_serialization() {
    let actions = vec![
        Action::ProvideFeedback {
            message: "Test feedback".to_string(),
            include_context: true,
        },
        Action::BlockWithFeedback {
            feedback_message: "Blocked".to_string(),
            include_context: false,
        },
        Action::Approve {
            reason: Some("Auto-approved".to_string()),
        },
        Action::RunCommand {
            command: "echo test".to_string(),
            on_failure: OnFailureBehavior::Block,
            on_failure_feedback: Some("Command failed".to_string()),
            background: false,
            timeout_seconds: Some(30),
        },
        Action::UpdateState {
            event: Some("TestEvent".to_string()),
            key: Some("test_key".to_string()),
            value: Some(serde_json::json!("test_value")),
            data: Some({
                let mut map = HashMap::new();
                map.insert("key1".to_string(), serde_json::json!("value1"));
                map.insert("key2".to_string(), serde_json::json!(42));
                map
            }),
        },
        Action::Conditional {
            if_condition: Condition::Match {
                field: "event_type".to_string(),
                value: "TestEvent".to_string(),
            },
            then_action: Box::new(Action::Approve { reason: None }),
            else_action: Some(Box::new(Action::ProvideFeedback {
                message: "Condition not met".to_string(),
                include_context: false,
            })),
        },
    ];

    for action in actions {
        let toml_str = toml::to_string(&action).expect("Failed to serialize action to TOML");
        let _deserialized: Action =
            toml::from_str(&toml_str).expect("Failed to deserialize action from TOML");
    }
}

#[test]
fn test_hook_event_types_serialization() {
    // Test that HookEventType serializes correctly as part of a Policy
    // (TOML cannot serialize bare enums, they must be part of a table/struct)
    let policy = Policy {
        name: "Test Policy".to_string(),
        description: None,
        hook_event: HookEventType::PreToolUse,
        matcher: Some("Bash".to_string()),
        conditions: vec![Condition::Match {
            field: "tool_name".to_string(),
            value: "Bash".to_string(),
        }],
        action: Action::Approve { reason: None },
    };

    let toml_str = toml::to_string(&policy).expect("Failed to serialize policy with hook event");
    let _deserialized: Policy =
        toml::from_str(&toml_str).expect("Failed to deserialize policy with hook event");

    // Test all hook event types serialize correctly
    let hook_events = vec![
        HookEventType::PreToolUse,
        HookEventType::PostToolUse,
        HookEventType::Notification,
        HookEventType::Stop,
        HookEventType::SubagentStop,
        HookEventType::PreCompact,
    ];

    for event_type in hook_events {
        let mut test_policy = policy.clone();
        test_policy.hook_event = event_type.clone();
        
        let toml_str = toml::to_string(&test_policy).expect("Failed to serialize policy");
        let deserialized: Policy = toml::from_str(&toml_str).expect("Failed to deserialize policy");
        
        match (&event_type, &deserialized.hook_event) {
            (HookEventType::PreToolUse, HookEventType::PreToolUse) => {},
            (HookEventType::PostToolUse, HookEventType::PostToolUse) => {},
            (HookEventType::Notification, HookEventType::Notification) => {},
            (HookEventType::Stop, HookEventType::Stop) => {},
            (HookEventType::SubagentStop, HookEventType::SubagentStop) => {},
            (HookEventType::PreCompact, HookEventType::PreCompact) => {},
            _ => panic!("Hook event type did not round-trip correctly"),
        }
    }
}

#[test]
fn test_state_query_filter_serialization() {
    let query = StateQueryFilter {
        tool: "Bash".to_string(),
        command_contains: Some("npm test".to_string()),
        result: Some("success".to_string()),
        within_minutes: Some(30),
    };

    let toml_str = toml::to_string(&query).expect("Failed to serialize state query filter");
    let deserialized: StateQueryFilter =
        toml::from_str(&toml_str).expect("Failed to deserialize state query filter");

    assert_eq!(deserialized.tool, query.tool);
    assert_eq!(deserialized.command_contains, query.command_contains);
    assert_eq!(deserialized.result, query.result);
    assert_eq!(deserialized.within_minutes, query.within_minutes);
}

#[test]
fn test_complex_policy_serialization() {
    let policy = Policy {
        name: "Complex Policy".to_string(),
        description: Some("A complex policy with nested conditions and actions".to_string()),
        hook_event: HookEventType::PreToolUse,
        matcher: Some("Edit|Write".to_string()),
        conditions: vec![Condition::And {
            conditions: vec![
                Condition::Pattern {
                    field: "tool_input.file_path".to_string(),
                    regex: "\\.rs$".to_string(),
                },
                Condition::Or {
                    conditions: vec![
                        Condition::Pattern {
                            field: "tool_input.content".to_string(),
                            regex: "unsafe\\s*\\{".to_string(),
                        },
                        Condition::Not {
                            condition: Box::new(Condition::Check {
                                command: "[ -f SAFETY.md ]".to_string(),
                                expect_success: true,
                            }),
                        },
                    ],
                },
            ],
        }],
        action: Action::Conditional {
            if_condition: Condition::Check {
                command: "[ $(date +%H) -ge 09 ] && [ $(date +%H) -le 17 ]".to_string(),
                expect_success: true,
            },
            then_action: Box::new(Action::BlockWithFeedback {
                feedback_message: "Unsafe code requires safety review during business hours"
                    .to_string(),
                include_context: true,
            }),
            else_action: Some(Box::new(Action::ProvideFeedback {
                message: "Consider adding safety comments for unsafe code".to_string(),
                include_context: false,
            })),
        },
    };

    let toml_str = toml::to_string(&policy).expect("Failed to serialize complex policy");
    let deserialized: Policy =
        toml::from_str(&toml_str).expect("Failed to deserialize complex policy");

    assert_eq!(deserialized.name, policy.name);
    assert_eq!(deserialized.description, policy.description);
    assert_eq!(deserialized.conditions.len(), policy.conditions.len());
}

#[test]
fn test_round_trip_serialization() {
    // Create a complete policy file with all possible variants
    let original = PolicyFile {
        schema_version: "1.0".to_string(),
        settings: Settings {
            audit_logging: true,
            debug_mode: true,
        },
        policies: vec![
            Policy {
                name: "Test Policy 1".to_string(),
                description: None,
                hook_event: HookEventType::PreToolUse,
                matcher: Some("Bash".to_string()),
                conditions: vec![Condition::Pattern {
                    field: "tool_input.command".to_string(),
                    regex: "git\\s+push".to_string(),
                }],
                action: Action::BlockWithFeedback {
                    feedback_message: "Push blocked".to_string(),
                    include_context: false,
                },
            },
            Policy {
                name: "Test Policy 2".to_string(),
                description: Some("Another test policy".to_string()),
                hook_event: HookEventType::PostToolUse,
                matcher: Some("Write".to_string()),
                conditions: vec![Condition::Pattern {
                    field: "tool_input.file_path".to_string(),
                    regex: "\\.rs$".to_string(),
                }],
                action: Action::RunCommand {
                    command: "rustfmt {{tool_input.file_path}}".to_string(),
                    on_failure: OnFailureBehavior::Continue,
                    on_failure_feedback: None,
                    background: true,
                    timeout_seconds: None,
                },
            },
        ],
    };

    // Serialize to TOML
    let toml_str = toml::to_string(&original).expect("Failed to serialize to TOML");

    // Deserialize from TOML
    let deserialized: PolicyFile =
        toml::from_str(&toml_str).expect("Failed to deserialize from TOML");

    // Verify round trip
    assert_eq!(deserialized.schema_version, original.schema_version);
    assert_eq!(
        deserialized.settings.audit_logging,
        original.settings.audit_logging
    );
    assert_eq!(
        deserialized.settings.debug_mode,
        original.settings.debug_mode
    );
    assert_eq!(deserialized.policies.len(), original.policies.len());

    for (i, (original_policy, deserialized_policy)) in original
        .policies
        .iter()
        .zip(deserialized.policies.iter())
        .enumerate()
    {
        assert_eq!(
            deserialized_policy.name, original_policy.name,
            "Policy {} name mismatch",
            i
        );
        assert_eq!(
            deserialized_policy.description, original_policy.description,
            "Policy {} description mismatch",
            i
        );
        assert_eq!(
            deserialized_policy.matcher, original_policy.matcher,
            "Policy {} matcher mismatch",
            i
        );
        assert_eq!(
            deserialized_policy.conditions.len(),
            original_policy.conditions.len(),
            "Policy {} conditions length mismatch",
            i
        );
    }
}
