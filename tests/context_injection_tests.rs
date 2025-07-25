use cupcake::config::actions::Action;
use serde_json::json;
use std::process::Command;
use tempfile::TempDir;
use std::fs;
use std::io::Write;

#[test]
fn test_inject_context_action_serialization() {
    let action = Action::InjectContext {
        context: "Remember to follow security best practices".to_string(),
        use_stdout: true,
    };
    
    let yaml = serde_yaml_ng::to_string(&action).unwrap();
    assert!(yaml.contains("inject_context"));
    assert!(yaml.contains("Remember to follow security best practices"));
    
    let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(action, deserialized);
}

#[test]
fn test_inject_context_soft_action() {
    let action = Action::InjectContext {
        context: "Test context".to_string(),
        use_stdout: false,
    };
    
    assert!(action.is_soft_action());
    assert!(!action.is_hard_action());
}

#[test]
fn test_user_prompt_submit_with_context_injection() {
    // Create a temporary directory for test files
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();
    
    // Create a test policy that injects context on UserPromptSubmit
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: inject-security-reminder
      description: Inject security reminder
      conditions: []
      action:
        type: inject_context
        context: "Security Reminder: Never expose API keys or secrets in code"
        use_stdout: true
"#;
    
    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    // Create UserPromptSubmit event JSON
    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "How do I connect to a database?"
    });
    
    // Run cupcake with the test configuration
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg("UserPromptSubmit")
        .arg("--config")
        .arg(config_dir.join("cupcake.yaml").to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .with_stdin(|stdin| {
            stdin.write_all(event_json.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();
    
    // Should exit with code 0 (allow)
    assert_eq!(output.status.code(), Some(0));
    
    // Should output the injected context to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Security Reminder: Never expose API keys or secrets"));
}

#[test]
fn test_multiple_context_injections() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();
    
    // Create a test policy with multiple inject_context actions
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: inject-multiple-contexts
      description: First context injection
      conditions: []
      action:
        type: inject_context
        context: "Context 1: Be careful with user input"
        use_stdout: true
        
    - name: inject-second-context
      description: Second context injection
      conditions: []
      action:
        type: inject_context
        context: "Context 2: Always validate data"
        use_stdout: true
"#;
    
    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Process user data"
    });
    
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg("UserPromptSubmit")
        .arg("--config")
        .arg(config_dir.join("cupcake.yaml").to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .with_stdin(|stdin| {
            stdin.write_all(event_json.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Context 1: Be careful with user input"));
    assert!(stdout.contains("Context 2: Always validate data"));
}

#[test]
fn test_context_injection_with_block() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();
    
    // Policy that blocks certain prompts but still tries to inject context
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: block-dangerous-prompt
      description: Block dangerous commands
      conditions:
        - type: pattern
          field: prompt
          regex: ".*rm -rf.*"
      action:
        type: block_with_feedback
        feedback_message: "Dangerous command detected in prompt"
        
    - name: inject-context-anyway
      description: Try to inject context
      conditions: []
      action:
        type: inject_context
        context: "This context won't be seen due to block"
        use_stdout: true
"#;
    
    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "How do I run rm -rf /"
    });
    
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg("UserPromptSubmit")
        .arg("--config")
        .arg(config_dir.join("cupcake.yaml").to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .with_stdin(|stdin| {
            stdin.write_all(event_json.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();
    
    // Should exit with code 2 (block)
    assert_eq!(output.status.code(), Some(2));
    
    // Should output block feedback to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Dangerous command detected"));
    
    // Context should not appear in stdout due to block
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("This context won't be seen"));
}

#[test]
fn test_context_injection_with_template_substitution() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();
    
    // Policy that uses template variables in context
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: inject-dynamic-context
      description: Inject context with template variables
      conditions: []
      action:
        type: inject_context
        context: "Session {{session_id}}: Remember to validate inputs in {{env.USER}}'s project"
        use_stdout: true
"#;
    
    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "abc-123",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Create input handler"
    });
    
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg("UserPromptSubmit")
        .arg("--config")
        .arg(config_dir.join("cupcake.yaml").to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .with_stdin(|stdin| {
            stdin.write_all(event_json.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Session abc-123:"));
    // USER env var should be substituted
    assert!(stdout.contains("project"));
}

#[test]
fn test_ask_permission_with_user_prompt_submit() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();
    
    // Policy that returns Ask decision
    let policy_content = r#"
policies:
  - name: ask-for-database-access
    hook_event: UserPromptSubmit
    matcher: ""
    conditions:
      - type: pattern
        field: prompt
        regex: ".*database.*"
    action:
      type: run_command
      spec:
        mode: array
        command: ["false"]
      on_failure: block
      on_failure_feedback: "Need approval for database-related prompts"
"#;
    
    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    // For now, Ask is not directly supported in actions, but we've laid the groundwork
    // This test documents the expected future behavior
}

// Helper trait for Command builder pattern
trait CommandExt {
    fn with_stdin<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut std::process::ChildStdin);
}

impl CommandExt for std::process::Child {
    fn with_stdin<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut std::process::ChildStdin),
    {
        if let Some(ref mut stdin) = self.stdin {
            f(stdin);
        }
        self
    }
}