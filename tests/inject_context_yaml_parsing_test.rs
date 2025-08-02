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
        Action::InjectContext { context, from_command, use_stdout, suppress_output } => {
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
        Action::InjectContext { context, from_command, use_stdout, .. } => {
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
        Action::InjectContext { context, from_command, use_stdout, .. } => {
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
        Action::InjectContext { context, from_command, use_stdout, suppress_output } => {
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
        Action::InjectContext { from_command, suppress_output, .. } => {
            assert!(from_command.is_some());
            assert!(*suppress_output);
            
            let dynamic_spec = from_command.as_ref().unwrap();
            assert_eq!(dynamic_spec.on_failure, OnFailureBehavior::Block);
        }
        _ => panic!("Expected InjectContext action"),
    }
}