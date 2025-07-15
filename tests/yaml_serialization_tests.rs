use cupcake::config::{
    actions::{Action, OnFailureBehavior},
    conditions::Condition,
    types::{ComposedPolicy, HookEventType, PolicyFragment, RootConfig, Settings, YamlPolicy},
};
use std::collections::HashMap;

#[test]
fn test_root_config_yaml_serialization() {
    let root_config = RootConfig {
        settings: Settings {
            audit_logging: true,
            debug_mode: false,
            allow_shell: false,
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
}

#[test]
fn test_yaml_policy_serialization() {
    let yaml_policy = YamlPolicy {
        name: "Test YAML Policy".to_string(),
        description: Some("A test policy in YAML format".to_string()),
        conditions: vec![Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: "git\\s+commit".to_string(),
        }],
        action: Action::ProvideFeedback {
            message: "Consider running tests before committing".to_string(),
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
    let mut fragment: PolicyFragment = HashMap::new();

    // Create a nested structure: PreToolUse -> Bash -> [policies]
    let mut bash_policies = HashMap::new();
    bash_policies.insert(
        "Bash".to_string(),
        vec![
            YamlPolicy {
                name: "Block Dangerous Commands".to_string(),
                description: None,
                conditions: vec![Condition::Pattern {
                    field: "tool_input.command".to_string(),
                    regex: "^(rm|dd)\\s".to_string(),
                }],
                action: Action::BlockWithFeedback {
                    feedback_message: "Dangerous command blocked!".to_string(),
                    include_context: false,
                },
            },
            YamlPolicy {
                name: "Git Commit Reminder".to_string(),
                description: Some("Reminds about running tests".to_string()),
                conditions: vec![Condition::Pattern {
                    field: "tool_input.command".to_string(),
                    regex: "git\\s+commit".to_string(),
                }],
                action: Action::ProvideFeedback {
                    message: "Don't forget to run tests!".to_string(),
                    include_context: false,
                },
            },
        ],
    );

    fragment.insert("PreToolUse".to_string(), bash_policies);

    // Test YAML serialization
    let yaml_str =
        serde_yaml_ng::to_string(&fragment).expect("Failed to serialize fragment to YAML");
    let deserialized: PolicyFragment =
        serde_yaml_ng::from_str(&yaml_str).expect("Failed to deserialize fragment from YAML");

    assert!(deserialized.contains_key("PreToolUse"));
    let pre_tool_use = deserialized.get("PreToolUse").unwrap();
    assert!(pre_tool_use.contains_key("Bash"));
    let bash_policies = pre_tool_use.get("Bash").unwrap();
    assert_eq!(bash_policies.len(), 2);
    assert_eq!(bash_policies[0].name, "Block Dangerous Commands");
    assert_eq!(bash_policies[1].name, "Git Commit Reminder");
}

#[test]
fn test_composed_policy_structure() {
    // Test that ComposedPolicy has all the fields needed for the engine
    let composed = ComposedPolicy {
        name: "Test Composed Policy".to_string(),
        description: Some("Composed from YAML fragments".to_string()),
        hook_event: HookEventType::PreToolUse,
        matcher: "Bash".to_string(),
        conditions: vec![Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: "echo.*".to_string(),
        }],
        action: Action::ProvideFeedback {
            message: "Echo command detected".to_string(),
            include_context: false,
        },
    };

    // ComposedPolicy doesn't implement Serialize/Deserialize by design
    // (it's an internal engine type), so just test structure
    assert_eq!(composed.name, "Test Composed Policy");
    assert!(composed.description.is_some());
    assert_eq!(composed.matcher, "Bash");
    assert_eq!(composed.conditions.len(), 1);
}

#[test]
fn test_root_config_default() {
    let default_config = RootConfig::default();

    assert!(!default_config.settings.audit_logging);
    assert!(!default_config.settings.debug_mode);
    assert_eq!(default_config.imports.len(), 1);
    assert_eq!(default_config.imports[0], "policies/*.yaml");
}

#[test]
fn test_complex_yaml_fragment() {
    // Test a more complex fragment with multiple hook events and matchers
    let yaml_content = r#"
PreToolUse:
  "Bash":
    - name: "Security Check"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^sudo\\s"
      action:
        type: "block_with_feedback"
        feedback_message: "Sudo commands require approval"
        include_context: true
  "Edit|Write":
    - name: "Rust File Formatting"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "\\.rs$"
      action:
        type: "provide_feedback"
        message: "Remember to run cargo fmt"
        include_context: false

PostToolUse:
  "Write":
    - name: "File Created Notification"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Write"
      action:
        type: "provide_feedback"
        message: "File successfully created"
        include_context: false
"#;

    let fragment: PolicyFragment =
        serde_yaml_ng::from_str(yaml_content).expect("Failed to parse complex YAML fragment");

    // Verify structure
    assert!(fragment.contains_key("PreToolUse"));
    assert!(fragment.contains_key("PostToolUse"));

    let pre_tool_use = fragment.get("PreToolUse").unwrap();
    assert!(pre_tool_use.contains_key("Bash"));
    assert!(pre_tool_use.contains_key("Edit|Write"));

    let bash_policies = pre_tool_use.get("Bash").unwrap();
    assert_eq!(bash_policies.len(), 1);
    assert_eq!(bash_policies[0].name, "Security Check");

    let edit_policies = pre_tool_use.get("Edit|Write").unwrap();
    assert_eq!(edit_policies.len(), 1);
    assert_eq!(edit_policies[0].name, "Rust File Formatting");

    let post_tool_use = fragment.get("PostToolUse").unwrap();
    assert!(post_tool_use.contains_key("Write"));
    let write_policies = post_tool_use.get("Write").unwrap();
    assert_eq!(write_policies.len(), 1);
    assert_eq!(write_policies[0].name, "File Created Notification");
}
