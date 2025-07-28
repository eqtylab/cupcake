use cupcake::config::loader::PolicyLoader;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_claude_project_dir_discovery() {
    // Create two temp directories - one for project, one for Claude project
    let current_dir = tempdir().unwrap();
    let claude_dir = tempdir().unwrap();
    
    // Create policy in Claude project directory
    let claude_guardrails = claude_dir.path().join("guardrails");
    fs::create_dir(&claude_guardrails).unwrap();
    
    // Create a RootConfig that imports a policy file
    let claude_config = r#"# Claude project directory config
imports:
  - "policies.yaml"

settings:
  timeout_ms: 3000
"#;
    
    fs::write(claude_guardrails.join("cupcake.yaml"), claude_config).unwrap();
    
    // Create the actual policy file
    let claude_policies = r#"PreToolUse:
  "Bash":
    - name: claude-project-policy
      description: Policy from CLAUDE_PROJECT_DIR
      conditions: []
      action:
        type: allow
"#;
    
    fs::write(claude_guardrails.join("policies.yaml"), claude_policies).unwrap();
    
    // Also create a policy in current directory (should be ignored)
    let current_guardrails = current_dir.path().join("guardrails");
    fs::create_dir(&current_guardrails).unwrap();
    
    // Create config that imports policies
    let current_config = r#"imports:
  - "policies.yaml"
"#;
    
    fs::write(current_guardrails.join("cupcake.yaml"), current_config).unwrap();
    
    let current_policies = r#"PreToolUse:
  "Edit":
    - name: current-dir-policy
      description: Policy from current directory
      conditions: []
      action:
        type: allow
"#;
    
    fs::write(current_guardrails.join("policies.yaml"), current_policies).unwrap();
    
    // Set CLAUDE_PROJECT_DIR environment variable
    std::env::set_var("CLAUDE_PROJECT_DIR", claude_dir.path());
    
    // Ensure cleanup happens even on panic using RAII
    struct EnvGuard;
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("CLAUDE_PROJECT_DIR");
        }
    }
    let _guard = EnvGuard;
    
    // Load configuration - should use Claude project directory
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration_from_directory(current_dir.path()).unwrap();
    
    // Verify we got the policy from Claude project directory
    assert_eq!(config.policies.len(), 1);
    assert_eq!(config.policies[0].name, "claude-project-policy");
    assert_eq!(config.policies[0].matcher, "Bash");
}

#[test]
fn test_claude_project_dir_fallback() {
    // Create temp directory
    let current_dir = tempdir().unwrap();
    
    // Create policy in current directory
    let current_guardrails = current_dir.path().join("guardrails");
    fs::create_dir(&current_guardrails).unwrap();
    
    let config = r#"imports:
  - "policies.yaml"
"#;
    
    fs::write(current_guardrails.join("cupcake.yaml"), config).unwrap();
    
    let policy = r#"PreToolUse:
  "*":
    - name: fallback-policy
      description: Policy from current directory
      conditions: []
      action:
        type: allow
"#;
    
    fs::write(current_guardrails.join("policies.yaml"), policy).unwrap();
    
    // Set CLAUDE_PROJECT_DIR to non-existent directory
    let non_existent = tempdir().unwrap();
    std::env::set_var("CLAUDE_PROJECT_DIR", non_existent.path());
    
    // Ensure cleanup happens even on panic using RAII
    struct EnvGuard;
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("CLAUDE_PROJECT_DIR");
        }
    }
    let _guard = EnvGuard;
    
    // Load configuration - should fall back to current directory
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration_from_directory(current_dir.path()).unwrap();
    
    // Verify we got the policy from current directory (fallback)
    assert_eq!(config.policies.len(), 1);
    assert_eq!(config.policies[0].name, "fallback-policy");
}

#[test]
fn test_no_claude_project_dir() {
    // Ensure CLAUDE_PROJECT_DIR is not set
    std::env::remove_var("CLAUDE_PROJECT_DIR");
    
    // Create temp directory with policy
    let current_dir = tempdir().unwrap();
    let guardrails = current_dir.path().join("guardrails");
    fs::create_dir(&guardrails).unwrap();
    
    let config = r#"imports:
  - "policy.yaml"
"#;
    
    fs::write(guardrails.join("cupcake.yaml"), config).unwrap();
    
    let policy = r#"PreToolUse:
  "Read":
    - name: regular-policy
      description: Regular policy discovery
      conditions: []
      action:
        type: allow
"#;
    
    fs::write(guardrails.join("policy.yaml"), policy).unwrap();
    
    // Load configuration - should use regular discovery
    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration_from_directory(current_dir.path()).unwrap();
    
    // Verify regular discovery works
    assert_eq!(config.policies.len(), 1);
    assert_eq!(config.policies[0].name, "regular-policy");
}