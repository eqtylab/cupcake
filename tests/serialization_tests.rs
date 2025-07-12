use cupcake::config::{
    actions::{Action, OnFailureBehavior},
    conditions::{Condition, StateQueryFilter},
    types::{ComposedPolicy, HookEventType, PolicyFragment, RootConfig, Settings, YamlPolicy},
};
use serde_json;
use std::collections::HashMap;

#[test]
fn test_root_config_yaml_serialization() {
    let root_config = RootConfig {
        settings: Settings {
            audit_logging: true,
            debug_mode: false,
        },
        imports: vec![
            "policies/*.yaml".to_string(),
            "policies/security/*.yaml".to_string(),
        ],
    };

    // Test YAML serialization
    let yaml_str = serde_yaml_ng::to_string(&root_config).expect("Failed to serialize to YAML");
    let deserialized: RootConfig =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize from YAML");

    assert_eq!(
        deserialized.settings.audit_logging,
        root_config.settings.audit_logging
    );
    assert_eq!(
        deserialized.settings.debug_mode,
        root_config.settings.debug_mode
    );
    assert_eq!(deserialized.imports.len(), root_config.imports.len());
    assert_eq!(deserialized.imports[0], root_config.imports[0]);
    assert_eq!(deserialized.imports[1], root_config.imports[1]);
}

#[test]
fn test_yaml_policy_serialization() {
    let yaml_policy = YamlPolicy {
        name: "Test YAML Policy".to_string(),
        description: Some("A test policy for YAML".to_string()),
        conditions: vec![Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: "git\\s+commit".to_string(),
        }],
        action: Action::ProvideFeedback {
            message: "Test feedback".to_string(),
            include_context: false,
        },
    };

    // Test YAML serialization
    let yaml_str = serde_yaml_ng::to_string(&yaml_policy).expect("Failed to serialize to YAML");
    let deserialized: YamlPolicy =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize from YAML");

    assert_eq!(deserialized.name, yaml_policy.name);
    assert_eq!(deserialized.description, yaml_policy.description);
    assert_eq!(deserialized.conditions.len(), yaml_policy.conditions.len());
}

#[test]
fn test_policy_fragment_yaml_serialization() {
    let mut policy_fragment: PolicyFragment = HashMap::new();

    // Create PreToolUse policies
    let mut pre_tool_use = HashMap::new();
    pre_tool_use.insert(
        "Bash".to_string(),
        vec![YamlPolicy {
            name: "Git Commit Reminder".to_string(),
            description: Some("Reminds to run tests".to_string()),
            conditions: vec![Condition::Pattern {
                field: "tool_input.command".to_string(),
                regex: "git\\s+commit".to_string(),
            }],
            action: Action::ProvideFeedback {
                message: "Remember to run tests!".to_string(),
                include_context: false,
            },
        }],
    );

    // Create PostToolUse policies
    let mut post_tool_use = HashMap::new();
    post_tool_use.insert(
        "Write".to_string(),
        vec![YamlPolicy {
            name: "File Created".to_string(),
            description: None,
            conditions: vec![Condition::Match {
                field: "tool_name".to_string(),
                value: "Write".to_string(),
            }],
            action: Action::ProvideFeedback {
                message: "File created successfully".to_string(),
                include_context: false,
            },
        }],
    );

    policy_fragment.insert("PreToolUse".to_string(), pre_tool_use);
    policy_fragment.insert("PostToolUse".to_string(), post_tool_use);

    // Test YAML serialization
    let yaml_str = serde_yaml_ng::to_string(&policy_fragment).expect("Failed to serialize to YAML");
    let deserialized: PolicyFragment =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize from YAML");

    assert!(deserialized.contains_key("PreToolUse"));
    assert!(deserialized.contains_key("PostToolUse"));

    let pre_policies = deserialized.get("PreToolUse").unwrap();
    assert!(pre_policies.contains_key("Bash"));

    let bash_policies = pre_policies.get("Bash").unwrap();
    assert_eq!(bash_policies.len(), 1);
    assert_eq!(bash_policies[0].name, "Git Commit Reminder");

    let post_policies = deserialized.get("PostToolUse").unwrap();
    assert!(post_policies.contains_key("Write"));

    let write_policies = post_policies.get("Write").unwrap();
    assert_eq!(write_policies.len(), 1);
    assert_eq!(write_policies[0].name, "File Created");
}

#[test]
fn test_composed_policy_serialization() {
    let composed_policy = ComposedPolicy {
        name: "Test Composed Policy".to_string(),
        description: Some("A test composed policy".to_string()),
        hook_event: HookEventType::PreToolUse,
        matcher: "Bash".to_string(),
        conditions: vec![Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: "rm\\s".to_string(),
        }],
        action: Action::BlockWithFeedback {
            feedback_message: "Dangerous command blocked!".to_string(),
            include_context: true,
        },
    };

    // Test JSON serialization (for debugging/inspection)
    let json_str = serde_json::to_string(&composed_policy).expect("Failed to serialize to JSON");
    let deserialized: ComposedPolicy =
        serde_json::from_str(&json_str).expect("Failed to deserialize from JSON");

    assert_eq!(deserialized.name, composed_policy.name);
    assert_eq!(deserialized.description, composed_policy.description);
    assert_eq!(deserialized.hook_event, composed_policy.hook_event);
    assert_eq!(deserialized.matcher, composed_policy.matcher);
    assert_eq!(
        deserialized.conditions.len(),
        composed_policy.conditions.len()
    );
}

#[test]
fn test_condition_variants_yaml_serialization() {
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
        let yaml_str =
            serde_yaml_ng::to_string(&condition).expect("Failed to serialize condition to YAML");
        let _deserialized: Condition =
            serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize condition from YAML");
    }
}

#[test]
fn test_action_variants_yaml_serialization() {
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
        let yaml_str =
            serde_yaml_ng::to_string(&action).expect("Failed to serialize action to YAML");
        let _deserialized: Action =
            serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize action from YAML");
    }
}

#[test]
fn test_hook_event_types_yaml_serialization() {
    // Test that HookEventType serializes correctly as part of a ComposedPolicy
    let policy = ComposedPolicy {
        name: "Test Policy".to_string(),
        description: None,
        hook_event: HookEventType::PreToolUse,
        matcher: "Bash".to_string(),
        conditions: vec![Condition::Match {
            field: "tool_name".to_string(),
            value: "Bash".to_string(),
        }],
        action: Action::Approve { reason: None },
    };

    let yaml_str =
        serde_yaml_ng::to_string(&policy).expect("Failed to serialize policy with hook event");
    let _deserialized: ComposedPolicy =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize policy with hook event");

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

        let yaml_str = serde_yaml_ng::to_string(&test_policy).expect("Failed to serialize policy");
        let deserialized: ComposedPolicy =
            serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize policy");

        match (&event_type, &deserialized.hook_event) {
            (HookEventType::PreToolUse, HookEventType::PreToolUse) => {}
            (HookEventType::PostToolUse, HookEventType::PostToolUse) => {}
            (HookEventType::Notification, HookEventType::Notification) => {}
            (HookEventType::Stop, HookEventType::Stop) => {}
            (HookEventType::SubagentStop, HookEventType::SubagentStop) => {}
            (HookEventType::PreCompact, HookEventType::PreCompact) => {}
            _ => panic!("Hook event type did not round-trip correctly"),
        }
    }
}

#[test]
fn test_state_query_filter_yaml_serialization() {
    let query = StateQueryFilter {
        tool: "Bash".to_string(),
        command_contains: Some("npm test".to_string()),
        result: Some("success".to_string()),
        within_minutes: Some(30),
    };

    let yaml_str =
        serde_yaml_ng::to_string(&query).expect("Failed to serialize state query filter");
    let deserialized: StateQueryFilter =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize state query filter");

    assert_eq!(deserialized.tool, query.tool);
    assert_eq!(deserialized.command_contains, query.command_contains);
    assert_eq!(deserialized.result, query.result);
    assert_eq!(deserialized.within_minutes, query.within_minutes);
}

#[test]
fn test_complex_yaml_policy_serialization() {
    let yaml_policy = YamlPolicy {
        name: "Complex YAML Policy".to_string(),
        description: Some("A complex policy with nested conditions and actions".to_string()),
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

    let yaml_str =
        serde_yaml_ng::to_string(&yaml_policy).expect("Failed to serialize complex YAML policy");
    let deserialized: YamlPolicy =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize complex YAML policy");

    assert_eq!(deserialized.name, yaml_policy.name);
    assert_eq!(deserialized.description, yaml_policy.description);
    assert_eq!(deserialized.conditions.len(), yaml_policy.conditions.len());
}

#[test]
fn test_round_trip_yaml_serialization() {
    // Create a complete policy fragment with all possible variants
    let mut original: PolicyFragment = HashMap::new();

    let mut pre_tool_use = HashMap::new();
    pre_tool_use.insert(
        "Bash".to_string(),
        vec![YamlPolicy {
            name: "Test YAML Policy 1".to_string(),
            description: None,
            conditions: vec![Condition::Pattern {
                field: "tool_input.command".to_string(),
                regex: "git\\s+push".to_string(),
            }],
            action: Action::BlockWithFeedback {
                feedback_message: "Push blocked".to_string(),
                include_context: false,
            },
        }],
    );

    let mut post_tool_use = HashMap::new();
    post_tool_use.insert(
        "Write".to_string(),
        vec![YamlPolicy {
            name: "Test YAML Policy 2".to_string(),
            description: Some("Another test policy".to_string()),
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
        }],
    );

    original.insert("PreToolUse".to_string(), pre_tool_use);
    original.insert("PostToolUse".to_string(), post_tool_use);

    // Serialize to YAML
    let yaml_str = serde_yaml_ng::to_string(&original).expect("Failed to serialize to YAML");

    // Deserialize from YAML
    let deserialized: PolicyFragment =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize from YAML");

    // Verify round trip
    assert_eq!(deserialized.len(), original.len());

    assert!(deserialized.contains_key("PreToolUse"));
    assert!(deserialized.contains_key("PostToolUse"));

    let orig_pre = original.get("PreToolUse").unwrap();
    let deser_pre = deserialized.get("PreToolUse").unwrap();
    assert_eq!(deser_pre.len(), orig_pre.len());

    let orig_post = original.get("PostToolUse").unwrap();
    let deser_post = deserialized.get("PostToolUse").unwrap();
    assert_eq!(deser_post.len(), orig_post.len());

    // Check specific policies
    let orig_bash_policies = orig_pre.get("Bash").unwrap();
    let deser_bash_policies = deser_pre.get("Bash").unwrap();
    assert_eq!(deser_bash_policies[0].name, orig_bash_policies[0].name);

    let orig_write_policies = orig_post.get("Write").unwrap();
    let deser_write_policies = deser_post.get("Write").unwrap();
    assert_eq!(deser_write_policies[0].name, orig_write_policies[0].name);
    assert_eq!(
        deser_write_policies[0].description,
        orig_write_policies[0].description
    );
}

#[test]
fn test_real_world_yaml_fragment() {
    // Test parsing a real-world YAML fragment like what would be in policies/00-base.yaml
    let yaml_content = r#"
PreToolUse:
  "Bash":
    - name: "Git Commit Reminder"
      description: "Reminds to run tests before committing"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "git\\s+commit"
      action:
        type: "provide_feedback"
        message: "💡 Remember to run tests before committing!"
        include_context: false

    - name: "Dangerous Command Warning"
      description: "Warns about potentially destructive commands"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^(rm|dd|format)\\s.*(-rf|--force)"
      action:
        type: "provide_feedback"
        message: "⚠️  Potentially destructive command detected. Please review carefully."
        include_context: true

  "Edit|Write":
    - name: "Rust File Formatting Reminder"
      description: "Suggests running cargo fmt on Rust files"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "\\.rs$"
      action:
        type: "provide_feedback"
        message: "📝 Consider running 'cargo fmt' after editing Rust files"
        include_context: false

PostToolUse:
  "Write":
    - name: "File Creation Confirmation"
      description: "Confirms successful file creation"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Write"
      action:
        type: "provide_feedback"
        message: "✅ File successfully created"
        include_context: false
"#;

    let fragment: PolicyFragment =
        serde_yaml_ng::from_str(yaml_content).expect("Failed to parse real-world YAML fragment");

    // Verify structure
    assert!(fragment.contains_key("PreToolUse"));
    assert!(fragment.contains_key("PostToolUse"));

    let pre_tool_use = fragment.get("PreToolUse").unwrap();
    assert!(pre_tool_use.contains_key("Bash"));
    assert!(pre_tool_use.contains_key("Edit|Write"));

    let bash_policies = pre_tool_use.get("Bash").unwrap();
    assert_eq!(bash_policies.len(), 2);
    assert_eq!(bash_policies[0].name, "Git Commit Reminder");
    assert_eq!(bash_policies[1].name, "Dangerous Command Warning");

    let edit_write_policies = pre_tool_use.get("Edit|Write").unwrap();
    assert_eq!(edit_write_policies.len(), 1);
    assert_eq!(edit_write_policies[0].name, "Rust File Formatting Reminder");

    let post_tool_use = fragment.get("PostToolUse").unwrap();
    assert!(post_tool_use.contains_key("Write"));

    let write_policies = post_tool_use.get("Write").unwrap();
    assert_eq!(write_policies.len(), 1);
    assert_eq!(write_policies[0].name, "File Creation Confirmation");
}
