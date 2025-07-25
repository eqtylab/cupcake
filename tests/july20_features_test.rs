use cupcake::config::actions::Action;
use cupcake::config::conditions::Condition;
use cupcake::config::loader::PolicyLoader;
use std::fs;
use tempfile::tempdir;

/// Integration tests for Claude Code July 20 features
/// Focus on verifying the core functionality works end-to-end

#[test]
fn test_inject_context_action_parsing() {
    let temp_dir = tempdir().unwrap();
    
    // Create a policy with InjectContext action
    let policy_yaml = r#"# Test InjectContext action
UserPromptSubmit:
  "*":
    - name: test-inject
      description: Test context injection
      conditions:
        - type: pattern
          field: prompt
          regex: "test"
      action:
        type: inject_context
        context: "This is injected context"
        use_stdout: true
"#;
    
    let policy_path = temp_dir.path().join("inject.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();
    
    // Load and verify the policy
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();
    
    assert_eq!(config.policies.len(), 1);
    assert_eq!(config.policies[0].name, "test-inject");
    assert_eq!(config.policies[0].hook_event.to_string(), "UserPromptSubmit");
    
    // Verify the action is InjectContext
    match &config.policies[0].action {
        Action::InjectContext { context, use_stdout } => {
            assert_eq!(context, "This is injected context");
            assert_eq!(*use_stdout, true);
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_state_query_condition_parsing() {
    let temp_dir = tempdir().unwrap();
    
    // Create a policy with StateQuery condition
    let policy_yaml = r#"# Test StateQuery condition
PreToolUse:
  "Bash":
    - name: test-state-query
      description: Test state query condition
      conditions:
        - type: state_query
          filter:
            tool: Bash
            command_contains: "test"
            result: success
            within_minutes: 30
          expect_exists: true
      action:
        type: allow
"#;
    
    let policy_path = temp_dir.path().join("state-query.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();
    
    // Load and verify the policy
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();
    
    assert_eq!(config.policies.len(), 1);
    assert_eq!(config.policies[0].name, "test-state-query");
    
    // Verify the condition is StateQuery
    assert_eq!(config.policies[0].conditions.len(), 1);
    match &config.policies[0].conditions[0] {
        Condition::StateQuery { filter, expect_exists } => {
            assert_eq!(filter.tool, "Bash");
            assert_eq!(filter.command_contains, Some("test".to_string()));
            assert_eq!(filter.result, Some("success".to_string()));
            assert_eq!(filter.within_minutes, Some(30));
            assert_eq!(*expect_exists, true);
        }
        _ => panic!("Expected StateQuery condition"),
    }
}

#[test]
fn test_claude_project_dir_support() {
    // Create two directories - one for CLAUDE_PROJECT_DIR, one for current
    let claude_dir = tempdir().unwrap();
    let current_dir = tempdir().unwrap();
    
    // Create policy in CLAUDE_PROJECT_DIR
    let claude_guardrails = claude_dir.path().join("guardrails");
    fs::create_dir(&claude_guardrails).unwrap();
    
    // Create RootConfig that imports policies
    let claude_root = r#"# CLAUDE_PROJECT_DIR root config
imports:
  - "policy.yaml"
"#;
    
    fs::write(claude_guardrails.join("cupcake.yaml"), claude_root).unwrap();
    
    // Create the actual policy file
    let claude_policy = r#"PreToolUse:
  "*":
    - name: claude-policy
      description: From CLAUDE_PROJECT_DIR
      conditions: []
      action:
        type: allow
"#;
    
    fs::write(claude_guardrails.join("policy.yaml"), claude_policy).unwrap();
    
    // Create different policy in current directory
    let current_guardrails = current_dir.path().join("guardrails");
    fs::create_dir(&current_guardrails).unwrap();
    
    // Create RootConfig for current directory
    let current_root = r#"# Current directory root config
imports:
  - "policy.yaml"
"#;
    
    fs::write(current_guardrails.join("cupcake.yaml"), current_root).unwrap();
    
    // Create the actual policy file
    let current_policy = r#"PreToolUse:
  "*":
    - name: current-policy
      description: From current directory
      conditions: []
      action:
        type: block_with_feedback
        feedback_message: "Blocked"
"#;
    
    fs::write(current_guardrails.join("policy.yaml"), current_policy).unwrap();
    
    // Test with CLAUDE_PROJECT_DIR set
    std::env::set_var("CLAUDE_PROJECT_DIR", claude_dir.path());
    
    let mut loader = PolicyLoader::new();
    let result = loader.load_configuration_from_directory(current_dir.path());
    
    match result {
        Ok(config) => {
            // Should use policy from CLAUDE_PROJECT_DIR
            assert_eq!(config.policies.len(), 1, "Expected 1 policy but got {}", config.policies.len());
            assert_eq!(config.policies[0].name, "claude-policy");
        }
        Err(e) => panic!("Failed to load config: {}", e),
    }
    
    // Clean up
    std::env::remove_var("CLAUDE_PROJECT_DIR");
}

#[test]
fn test_mcp_tool_pattern_matching() {
    let temp_dir = tempdir().unwrap();
    
    // Create policies with various MCP patterns
    let policy_yaml = r#"# MCP pattern matching test
PreToolUse:
  "mcp__.*":
    - name: all-mcp
      description: Match all MCP tools
      conditions: []
      action:
        type: allow
  
  "mcp__github__.*":
    - name: github-mcp
      description: Match GitHub MCP tools
      conditions: []
      action:
        type: allow
  
  "mcp__.*(delete|remove).*":
    - name: dangerous-mcp
      description: Match dangerous MCP operations
      conditions: []
      action:
        type: block_with_feedback
        feedback_message: "Blocked"
"#;
    
    let policy_path = temp_dir.path().join("mcp.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();
    
    // Load policies
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();
    
    // Should have 3 policies
    assert_eq!(config.policies.len(), 3);
    
    // Verify matchers
    let matchers: Vec<&str> = config.policies.iter()
        .map(|p| p.matcher.as_str())
        .collect();
    
    assert!(matchers.contains(&"mcp__.*"));
    assert!(matchers.contains(&"mcp__github__.*"));
    assert!(matchers.contains(&"mcp__.*(delete|remove).*"));
}

#[test]
fn test_wildcard_matcher_for_all_events() {
    let temp_dir = tempdir().unwrap();
    
    // Create policy with "*" matcher for UserPromptSubmit
    let policy_yaml = r#"# Wildcard matcher test
UserPromptSubmit:
  "*":
    - name: all-prompts
      description: Match all user prompts
      conditions: []
      action:
        type: allow

PreToolUse:
  "*":
    - name: all-tools
      description: Match all tools
      conditions: []
      action:
        type: allow
"#;
    
    let policy_path = temp_dir.path().join("wildcard.yaml");
    fs::write(&policy_path, policy_yaml).unwrap();
    
    // Load policies
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();
    
    assert_eq!(config.policies.len(), 2);
    
    // Check the policies loaded correctly
    let policy_names: Vec<&str> = config.policies.iter()
        .map(|p| p.name.as_str())
        .collect();
    
    assert!(policy_names.contains(&"all-prompts"));
    assert!(policy_names.contains(&"all-tools"));
    
    // Check matchers
    for policy in &config.policies {
        assert_eq!(policy.matcher, "*");
    }
}

// TODO: Add test for Ask action once it's implemented

#[test]
fn test_complex_policy_with_imports() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&guardrails_dir).unwrap();
    
    // Create root config with imports
    let root_config = r#"# Root configuration
imports:
  - "policies/*.yaml"

settings:
  audit_logging: true
  timeout_ms: 3000
"#;
    
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();
    
    // Create policies directory
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir(&policies_dir).unwrap();
    
    // Create first policy file
    let policy1 = r#"PreToolUse:
  "Write":
    - name: policy-1
      description: First policy
      conditions: []
      action:
        type: allow
"#;
    
    fs::write(policies_dir.join("policy1.yaml"), policy1).unwrap();
    
    // Create second policy file with advanced features
    let policy2 = r#"UserPromptSubmit:
  "*":
    - name: policy-2
      description: Second policy with state query
      conditions:
        - type: state_query
          filter:
            tool: "Bash"
            within_minutes: 10
          expect_exists: false
      action:
        type: inject_context
        context: "No recent bash commands"
        use_stdout: true
"#;
    
    fs::write(policies_dir.join("policy2.yaml"), policy2).unwrap();
    
    // Load from directory
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration_from_directory(temp_dir.path()).unwrap();
    
    // Should have both policies
    assert_eq!(config.policies.len(), 2);
    
    // Verify settings were loaded
    assert_eq!(config.settings.audit_logging, true);
    assert_eq!(config.settings.timeout_ms, 3000);
    
    // Verify policies
    let policy_names: Vec<&str> = config.policies.iter()
        .map(|p| p.name.as_str())
        .collect();
    
    assert!(policy_names.contains(&"policy-1"));
    assert!(policy_names.contains(&"policy-2"));
}