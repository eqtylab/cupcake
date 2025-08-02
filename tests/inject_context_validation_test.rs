use cupcake::config::loader::PolicyLoader;
use std::fs;
use tempfile::tempdir;

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
    
    assert!(result.is_ok(), "Should allow inject_context with UserPromptSubmit");
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
    
    assert!(result.is_ok(), "Should allow inject_context with SessionStart");
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
    
    assert!(result.is_err(), "Should reject inject_context with PreToolUse");
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(error_msg.contains("inject_context action is only valid for UserPromptSubmit and SessionStart"));
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
    
    assert!(result.is_err(), "Should reject inject_context with PostToolUse");
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
    
    assert!(result.is_err(), "Should reject inject_context with Notification");
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
    
    assert!(result.is_ok(), "Should allow inject_context in conditional for UserPromptSubmit");
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
    
    assert!(result.is_err(), "Should reject inject_context in conditional for PreToolUse");
    let error = result.unwrap_err();
    assert!(error.to_string().contains("inject_context action is only valid"));
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
    
    // Invalid policy
    let invalid_yaml = r#"
PreCompact:
  "*":
    - name: invalid-policy
      conditions: []
      action:
        type: inject_context
        context: "Invalid for PreCompact"
"#;
    fs::write(policies_dir.join("02-invalid.yaml"), invalid_yaml).unwrap();
    
    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration(&temp_dir.path().join("cupcake.yaml"));
    
    assert!(result.is_err(), "Should fail when any policy has invalid inject_context");
    let error = result.unwrap_err();
    assert!(error.to_string().contains("not PreCompact"));
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
    assert!(error_msg.contains("inject_context action is only valid for UserPromptSubmit and SessionStart"));
    assert!(error_msg.contains("not SubagentStop"));
    assert!(error_msg.contains("Claude Code's specification"));
}