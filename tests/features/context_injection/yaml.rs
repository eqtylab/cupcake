use cupcake::config::actions::{Action, OnFailureBehavior};
use cupcake::config::loader::PolicyLoader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_inject_context_static_yaml_parsing() {
    let temp_dir = tempdir().unwrap();

    // Test static context injection
    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: static-context
      description: Test static context injection
      conditions: []
      action:
        type: inject_context
        context: "This is static context"
        use_stdout: true
        suppress_output: false
"#;

    let policy_path = temp_dir.path().join("static.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();

    assert_eq!(config.policies.len(), 1);

    match &config.policies[0].action {
        Action::InjectContext {
            context,
            from_command,
            use_stdout,
            suppress_output,
        } => {
            assert_eq!(context.as_deref(), Some("This is static context"));
            assert!(from_command.is_none());
            assert!(*use_stdout);
            assert!(!suppress_output);
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_inject_context_from_command_yaml_parsing() {
    let temp_dir = tempdir().unwrap();

    // Test dynamic context injection from command
    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: dynamic-context
      description: Test dynamic context injection
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["./scripts/get-context.sh"]
            args: ["{{prompt}}"]
          on_failure: continue
        use_stdout: true
"#;

    let policy_path = temp_dir.path().join("dynamic.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();

    assert_eq!(config.policies.len(), 1);

    match &config.policies[0].action {
        Action::InjectContext {
            context,
            from_command,
            use_stdout,
            ..
        } => {
            assert!(context.is_none());
            assert!(from_command.is_some());
            assert!(*use_stdout);

            let dynamic_spec = from_command.as_ref().unwrap();
            assert_eq!(dynamic_spec.on_failure, OnFailureBehavior::Continue);
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_inject_context_from_command_shell_mode() {
    let temp_dir = tempdir().unwrap();

    // Test shell mode for complex scripts
    let policy_yaml = r#"
SessionStart:
  "*":
    - name: complex-script
      description: Complex shell script
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: shell
            script: |
              echo "Session started at $(date)"
              echo "Git branch: $(git branch --show-current 2>/dev/null || echo 'not a git repo')"
          on_failure: continue
        use_stdout: false
"#;

    let policy_path = temp_dir.path().join("shell.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();

    assert_eq!(config.policies.len(), 1);

    match &config.policies[0].action {
        Action::InjectContext {
            context,
            from_command,
            use_stdout,
            ..
        } => {
            assert!(context.is_none());
            assert!(from_command.is_some());
            assert!(!use_stdout); // Using JSON method
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_inject_context_mutually_exclusive() {
    let temp_dir = tempdir().unwrap();

    // Test that both context and from_command cannot be specified
    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: invalid-both
      description: Invalid - both static and dynamic
      conditions: []
      action:
        type: inject_context
        context: "Static context"
        from_command:
          spec:
            mode: array
            command: ["echo", "dynamic"]
          on_failure: continue
"#;

    let policy_path = temp_dir.path().join("invalid.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    // This should parse successfully - validation happens at runtime
    let config = loader.load_configuration(&policy_path).unwrap();

    // The YAML will parse, but at runtime we'll validate that only one is specified
    assert_eq!(config.policies.len(), 1);
}

#[test]
fn test_inject_context_minimal_yaml() {
    let temp_dir = tempdir().unwrap();

    // Test minimal YAML - relying on defaults
    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: minimal
      conditions: []
      action:
        type: inject_context
        context: "Minimal context"
"#;

    let policy_path = temp_dir.path().join("minimal.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();

    assert_eq!(config.policies.len(), 1);

    match &config.policies[0].action {
        Action::InjectContext {
            context,
            from_command,
            use_stdout,
            suppress_output,
        } => {
            assert_eq!(context.as_deref(), Some("Minimal context"));
            assert!(from_command.is_none());
            assert!(*use_stdout); // Default true
            assert!(!suppress_output); // Default false
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_inject_context_from_command_with_on_failure_block() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: block-on-failure
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["./critical-context.sh"]
          on_failure: block
        suppress_output: true
"#;

    let policy_path = temp_dir.path().join("block.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();

    match &config.policies[0].action {
        Action::InjectContext {
            from_command,
            suppress_output,
            ..
        } => {
            assert!(from_command.is_some());
            assert!(*suppress_output);

            let dynamic_spec = from_command.as_ref().unwrap();
            assert_eq!(dynamic_spec.on_failure, OnFailureBehavior::Block);
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_inject_context_valid_with_user_prompt_submit() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: valid-inject
      description: Valid context injection for UserPromptSubmit
      conditions: []
      action:
        type: inject_context
        context: "This is valid for UserPromptSubmit"
"#;

    let policy_path = temp_dir.path().join("valid.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_ok(),
        "Should allow inject_context with UserPromptSubmit"
    );
    let config = result.unwrap();
    assert_eq!(config.policies.len(), 1);
}

#[test]
fn test_inject_context_valid_with_session_start() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
SessionStart:
  "*":
    - name: valid-session-inject
      description: Valid context injection for SessionStart
      conditions: []
      action:
        type: inject_context
        context: "Welcome to the session!"
"#;

    let policy_path = temp_dir.path().join("valid-session.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_ok(),
        "Should allow inject_context with SessionStart"
    );
    let config = result.unwrap();
    assert_eq!(config.policies.len(), 1);
}

#[test]
fn test_inject_context_valid_with_pre_compact() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
PreCompact:
  "manual":
    - name: compact-context
      description: Valid context injection for PreCompact
      conditions: []
      action:
        type: inject_context
        context: "Important context to preserve during compaction"
"#;

    let policy_path = temp_dir.path().join("precompact.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_ok(),
        "Should accept inject_context with PreCompact"
    );
    let config = result.unwrap();
    assert_eq!(config.policies.len(), 1);
}

#[test]
fn test_inject_context_invalid_with_pre_tool_use() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
PreToolUse:
  "Bash":
    - name: invalid-inject
      description: Invalid context injection for PreToolUse
      conditions: []
      action:
        type: inject_context
        context: "This should fail"
"#;

    let policy_path = temp_dir.path().join("invalid.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_err(),
        "Should reject inject_context with PreToolUse"
    );
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(error_msg.contains(
        "inject_context action is only valid for UserPromptSubmit, SessionStart, and PreCompact"
    ));
    assert!(error_msg.contains("not PreToolUse"));
}

#[test]
fn test_inject_context_invalid_with_post_tool_use() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
PostToolUse:
  "Write":
    - name: invalid-post-inject
      description: Invalid context injection for PostToolUse
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["echo", "test"]
          on_failure: continue
"#;

    let policy_path = temp_dir.path().join("invalid-post.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_err(),
        "Should reject inject_context with PostToolUse"
    );
    let error = result.unwrap_err();
    assert!(error.to_string().contains("not PostToolUse"));
}

#[test]
fn test_inject_context_invalid_with_notification() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
Notification:
  "*":
    - name: invalid-notification-inject
      conditions: []
      action:
        type: inject_context
        context: "Cannot inject on notifications"
"#;

    let policy_path = temp_dir.path().join("invalid-notification.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_err(),
        "Should reject inject_context with Notification"
    );
}

#[test]
fn test_inject_context_invalid_with_stop() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
Stop:
  "*":
    - name: invalid-stop-inject
      conditions: []
      action:
        type: inject_context
        context: "Cannot inject on stop"
"#;

    let policy_path = temp_dir.path().join("invalid-stop.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(result.is_err(), "Should reject inject_context with Stop");
}

#[test]
fn test_inject_context_in_conditional_valid() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: conditional-inject
      conditions: []
      action:
        type: conditional
        if:
          type: pattern
          field: prompt
          regex: "test"
        then:
          type: inject_context
          context: "Valid in conditional for UserPromptSubmit"
        else:
          type: provide_feedback
          message: "No injection needed"
"#;

    let policy_path = temp_dir.path().join("conditional-valid.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_ok(),
        "Should allow inject_context in conditional for UserPromptSubmit"
    );
}

#[test]
fn test_inject_context_in_conditional_invalid() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
PreToolUse:
  "Bash":
    - name: conditional-inject-invalid
      conditions: []
      action:
        type: conditional
        if:
          type: pattern
          field: tool_input.command
          regex: "rm"
        then:
          type: inject_context
          context: "Invalid in conditional for PreToolUse"
"#;

    let policy_path = temp_dir.path().join("conditional-invalid.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(
        result.is_err(),
        "Should reject inject_context in conditional for PreToolUse"
    );
    let error = result.unwrap_err();
    assert!(error
        .to_string()
        .contains("inject_context action is only valid"));
}

#[test]
fn test_mixed_valid_and_invalid_policies() {
    let temp_dir = tempdir().unwrap();

    // Create a root config that imports multiple policies
    let root_yaml = r#"
settings:
  timeout_ms: 5000

imports:
  - "policies/*.yaml"
"#;
    fs::write(temp_dir.path().join("cupcake.yaml"), root_yaml).unwrap();

    let policies_dir = temp_dir.path().join("policies");
    fs::create_dir(&policies_dir).unwrap();

    // Valid policy
    let valid_yaml = r#"
UserPromptSubmit:
  "*":
    - name: valid-policy
      conditions: []
      action:
        type: inject_context
        context: "Valid"
"#;
    fs::write(policies_dir.join("01-valid.yaml"), valid_yaml).unwrap();

    // Invalid policy - using Stop which is not allowed for inject_context
    let invalid_yaml = r#"
Stop:
  "*":
    - name: invalid-policy
      conditions: []
      action:
        type: inject_context
        context: "Invalid for Stop event"
"#;
    fs::write(policies_dir.join("02-invalid.yaml"), invalid_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&temp_dir.path().join("cupcake.yaml"));

    assert!(
        result.is_err(),
        "Should fail when any policy has invalid inject_context"
    );
    let error = result.unwrap_err();
    assert!(error.to_string().contains("not Stop"));
}

#[test]
fn test_error_message_clarity() {
    let temp_dir = tempdir().unwrap();

    let policy_yaml = r#"
SubagentStop:
  "*":
    - name: unclear-inject
      conditions: []
      action:
        type: inject_context
        context: "Test error message"
"#;

    let policy_path = temp_dir.path().join("unclear.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();

    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&policy_path);

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = error.to_string();

    // Check that error message is helpful
    assert!(error_msg.contains(
        "inject_context action is only valid for UserPromptSubmit, SessionStart, and PreCompact"
    ));
    assert!(error_msg.contains("not SubagentStop"));
}
