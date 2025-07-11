use cupcake::config::{
    types::{PolicyFile, Settings, Policy, HookEventType},
    conditions::{Condition, StateQuery},
    actions::{Action, OnFailureBehavior},
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
            conditions: vec![Condition::CommandRegex {
                value: "git\\s+commit".to_string(),
                flags: vec!["multiline".to_string()],
            }],
            action: Action::ProvideFeedback {
                message: "Test feedback".to_string(),
                include_context: false,
            },
        }],
    };
    
    // Test TOML serialization
    let toml_str = toml::to_string(&policy_file).expect("Failed to serialize to TOML");
    let deserialized: PolicyFile = toml::from_str(&toml_str).expect("Failed to deserialize from TOML");
    
    assert_eq!(deserialized.schema_version, policy_file.schema_version);
    assert_eq!(deserialized.settings.audit_logging, policy_file.settings.audit_logging);
    assert_eq!(deserialized.policies.len(), policy_file.policies.len());
    assert_eq!(deserialized.policies[0].name, policy_file.policies[0].name);
}

#[test]
fn test_condition_variants_serialization() {
    let conditions = vec![
        Condition::CommandRegex {
            value: "test".to_string(),
            flags: vec![],
        },
        Condition::FilepathRegex {
            value: "\\.rs$".to_string(),
            flags: vec!["case_insensitive".to_string()],
        },
        Condition::FilepathGlob {
            value: "src/**/*.rs".to_string(),
        },
        Condition::FileContentRegex {
            value: "TODO:".to_string(),
            flags: vec!["multiline".to_string()],
        },
        Condition::StateExists {
            query: StateQuery {
                tool: Some("Read".to_string()),
                path: Some("README.md".to_string()),
                ..Default::default()
            },
        },
        Condition::And {
            conditions: vec![
                Condition::CommandRegex {
                    value: "git".to_string(),
                    flags: vec![],
                },
                Condition::Not {
                    condition: Box::new(Condition::FilepathGlob {
                        value: "test/**".to_string(),
                    }),
                },
            ],
        },
        Condition::TimeWindow {
            start: "09:00".to_string(),
            end: "17:00".to_string(),
            timezone: Some("UTC".to_string()),
        },
        Condition::DayOfWeek {
            days: vec!["Mon".to_string(), "Tue".to_string(), "Wed".to_string()],
        },
    ];
    
    for condition in conditions {
        let toml_str = toml::to_string(&condition).expect("Failed to serialize condition to TOML");
        let _deserialized: Condition = toml::from_str(&toml_str).expect("Failed to deserialize condition from TOML");
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
            if_condition: Condition::StateExists {
                query: StateQuery {
                    event: Some("TestEvent".to_string()),
                    ..Default::default()
                },
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
        let _deserialized: Action = toml::from_str(&toml_str).expect("Failed to deserialize action from TOML");
    }
}

#[test]
fn test_hook_event_types_serialization() {
    let hook_events = vec![
        HookEventType::PreToolUse,
        HookEventType::PostToolUse,
        HookEventType::Notification,
        HookEventType::Stop,
        HookEventType::SubagentStop,
        HookEventType::PreCompact,
    ];
    
    for event_type in hook_events {
        let toml_str = toml::to_string(&event_type).expect("Failed to serialize hook event type");
        let _deserialized: HookEventType = toml::from_str(&toml_str).expect("Failed to deserialize hook event type");
    }
}

#[test]
fn test_state_query_serialization() {
    let query = StateQuery {
        tool: Some("Bash".to_string()),
        path: Some("/path/to/file".to_string()),
        event: Some("CustomEvent".to_string()),
        command_contains: Some("npm test".to_string()),
        result: Some("success".to_string()),
        within_minutes: Some(30),
        since: Some("last_commit".to_string()),
        additional: {
            let mut map = HashMap::new();
            map.insert("custom_field".to_string(), serde_json::json!("custom_value"));
            map
        },
    };
    
    let toml_str = toml::to_string(&query).expect("Failed to serialize state query");
    let deserialized: StateQuery = toml::from_str(&toml_str).expect("Failed to deserialize state query");
    
    assert_eq!(deserialized.tool, query.tool);
    assert_eq!(deserialized.path, query.path);
    assert_eq!(deserialized.event, query.event);
    assert_eq!(deserialized.command_contains, query.command_contains);
    assert_eq!(deserialized.result, query.result);
    assert_eq!(deserialized.within_minutes, query.within_minutes);
    assert_eq!(deserialized.since, query.since);
    assert_eq!(deserialized.additional.len(), query.additional.len());
}

#[test]
fn test_complex_policy_serialization() {
    let policy = Policy {
        name: "Complex Policy".to_string(),
        description: Some("A complex policy with nested conditions and actions".to_string()),
        hook_event: HookEventType::PreToolUse,
        matcher: Some("Edit|Write".to_string()),
        conditions: vec![
            Condition::And {
                conditions: vec![
                    Condition::FilepathRegex {
                        value: "\\.rs$".to_string(),
                        flags: vec![],
                    },
                    Condition::Or {
                        conditions: vec![
                            Condition::FileContentRegex {
                                value: "unsafe\\s*\\{".to_string(),
                                flags: vec!["multiline".to_string()],
                            },
                            Condition::Not {
                                condition: Box::new(Condition::StateExists {
                                    query: StateQuery {
                                        tool: Some("Read".to_string()),
                                        path: Some("SAFETY.md".to_string()),
                                        within_minutes: Some(60),
                                        ..Default::default()
                                    },
                                }),
                            },
                        ],
                    },
                ],
            },
        ],
        action: Action::Conditional {
            if_condition: Condition::TimeWindow {
                start: "09:00".to_string(),
                end: "17:00".to_string(),
                timezone: Some("America/New_York".to_string()),
            },
            then_action: Box::new(Action::BlockWithFeedback {
                feedback_message: "Unsafe code requires safety review during business hours".to_string(),
                include_context: true,
            }),
            else_action: Some(Box::new(Action::ProvideFeedback {
                message: "Consider adding safety comments for unsafe code".to_string(),
                include_context: false,
            })),
        },
    };
    
    let toml_str = toml::to_string(&policy).expect("Failed to serialize complex policy");
    let deserialized: Policy = toml::from_str(&toml_str).expect("Failed to deserialize complex policy");
    
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
                conditions: vec![
                    Condition::CommandRegex {
                        value: "git\\s+push".to_string(),
                        flags: vec!["case_insensitive".to_string()],
                    },
                ],
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
                conditions: vec![
                    Condition::FilepathGlob {
                        value: "*.rs".to_string(),
                    },
                ],
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
    let deserialized: PolicyFile = toml::from_str(&toml_str).expect("Failed to deserialize from TOML");
    
    // Verify round trip
    assert_eq!(deserialized.schema_version, original.schema_version);
    assert_eq!(deserialized.settings.audit_logging, original.settings.audit_logging);
    assert_eq!(deserialized.settings.debug_mode, original.settings.debug_mode);
    assert_eq!(deserialized.policies.len(), original.policies.len());
    
    for (i, (original_policy, deserialized_policy)) in original.policies.iter().zip(deserialized.policies.iter()).enumerate() {
        assert_eq!(deserialized_policy.name, original_policy.name, "Policy {} name mismatch", i);
        assert_eq!(deserialized_policy.description, original_policy.description, "Policy {} description mismatch", i);
        assert_eq!(deserialized_policy.matcher, original_policy.matcher, "Policy {} matcher mismatch", i);
        assert_eq!(deserialized_policy.conditions.len(), original_policy.conditions.len(), "Policy {} conditions length mismatch", i);
    }
}