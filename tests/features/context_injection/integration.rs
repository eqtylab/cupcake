use cupcake::config::actions::Action;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

use crate::common::event_factory::EventFactory;

#[test]
fn test_inject_context_action_serialization() {
    let action = Action::InjectContext {
        context: Some("Remember to follow security best practices".to_string()),
        from_command: None,
        use_stdout: true,
        suppress_output: false,
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
        context: Some("Test context".to_string()),
        from_command: None,
        use_stdout: false,
        suppress_output: false,
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
    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("How do I connect to a database?")
        .build_value();

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

    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Process user data")
        .build_value();

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

    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("How do I run rm -rf /")
        .build_value();

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

    // Should exit with code 0 (success) but provide JSON response for block
    assert_eq!(output.status.code(), Some(0));

    // Should output JSON with block decision to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse as JSON to check for UserPromptSubmit Block format
    let response: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout should be valid JSON");
    
    // UserPromptSubmit Block uses decision: "block" format
    assert_eq!(response["hookSpecificOutput"]["decision"], "block");
    assert!(response["hookSpecificOutput"]["decisionReason"]
        .as_str()
        .unwrap()
        .contains("Dangerous command detected"));

    // Context should not appear in stdout due to block (block overrides context injection)
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

    let event_json = EventFactory::user_prompt_submit()
        .session_id("abc-123")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Create input handler")
        .build_value();

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
fn test_inject_context_edge_case_empty_context() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test empty context injection (should be allowed)
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: empty-context
      description: Test empty context injection
      conditions: []
      action:
        type: inject_context
        context: ""
        use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Test prompt")
        .build_value();

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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Empty context should result in empty output or just newline
    assert!(stdout.trim().is_empty() || stdout == "\n");
}

#[test]
fn test_inject_context_very_long_output() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Create a script that outputs a very long context
    let script_path = temp_dir.path().join("long-output.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
# Generate 1000 lines of output
for i in {1..1000}; do
    echo "Line $i: This is a test of very long context injection output"
done
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let policy_content = format!(
        r#"
SessionStart:
  "*":
    - name: long-context
      description: Test very long context output
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
          on_failure: continue
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = EventFactory::session_start()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .source_startup()
        .build_value();

    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg("SessionStart")
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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain multiple lines
    let line_count = stdout.lines().count();
    assert!(
        line_count >= 1000,
        "Expected at least 1000 lines, got {line_count}"
    );
    assert!(stdout.contains("Line 1:"));
    assert!(stdout.contains("Line 1000:"));
}

#[test]
fn test_inject_context_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test context with special characters that need escaping
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: special-chars
      description: Test special characters in context
      conditions: []
      action:
        type: inject_context
        context: |
          Special characters test:
          - Quotes: "double" and 'single'
          - Backslashes: \ and \\
          - Unicode: 🚀 🎉 🔥
          - Newlines and	tabs
          - JSON: {"key": "value"}
          - Regex: ^[a-z]+$
        use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Test special chars")
        .build_value();

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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that special characters are preserved
    assert!(stdout.contains("\"double\""));
    assert!(stdout.contains("'single'"));
    assert!(stdout.contains("\\"));
    assert!(stdout.contains("🚀"));
    assert!(stdout.contains("{\"key\": \"value\"}"));
}

#[test]
fn test_inject_context_from_command_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Create a script that sleeps for a long time
    let script_path = temp_dir.path().join("slow-script.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
sleep 30
echo "This should timeout"
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let policy_content = format!(
        r#"
settings:
  timeout_ms: 1000  # 1 second timeout

UserPromptSubmit:
  "*":
    - name: timeout-test
      description: Test command timeout
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
          on_failure: continue
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Test timeout")
        .build_value();

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

    // Should continue despite timeout
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Timeout should result in empty output
    assert!(!stdout.contains("This should timeout"));
}

#[test]
fn test_inject_context_combined_with_other_actions() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Complex policy with multiple action types
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: provide-feedback
      description: First provide feedback
      conditions: []
      action:
        type: provide_feedback
        message: "Processing your request..."
        suppress_output: false
        
    - name: inject-context-1
      description: Inject first context
      conditions: []
      action:
        type: inject_context
        context: "Context 1: Security guidelines loaded"
        use_stdout: true
        
    - name: inject-context-2
      description: Inject second context
      conditions: []
      action:
        type: inject_context
        context: "Context 2: Performance tips loaded"
        use_stdout: true
        
    - name: final-allow
      description: Allow the operation
      conditions: []
      action:
        type: allow
        suppress_output: false
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Build a web app")
        .build_value();

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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should see feedback first, then both contexts
    assert!(stdout.contains("Processing your request..."));
    assert!(stdout.contains("Context 1: Security guidelines loaded"));
    assert!(stdout.contains("Context 2: Performance tips loaded"));
}

#[test]
fn test_inject_context_with_conditional_logic() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test conditional context injection
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: conditional-context
      description: Conditionally inject context
      conditions: []
      action:
        type: conditional
        if:
          type: pattern
          field: prompt
          regex: "(?i)security"
        then:
          type: inject_context
          context: "⚠️ Security Alert: Remember to validate all inputs and sanitize outputs"
          use_stdout: true
        else:
          type: inject_context
          context: "💡 Tip: Consider performance and scalability in your design"
          use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    // Test with security-related prompt
    let security_event = EventFactory::user_prompt_submit()
        .session_id("test-session-1")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("How to implement security checks?")
        .build_value();

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
            stdin
                .write_all(security_event.to_string().as_bytes())
                .unwrap();
        })
        .wait_with_output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠️ Security Alert"));
    assert!(!stdout.contains("💡 Tip"));

    // Test with non-security prompt
    let general_event = EventFactory::user_prompt_submit()
        .session_id("test-session-2")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd(temp_dir.path().to_str().unwrap())
        .prompt("Build a fast API")
        .build_value();

    let output2 = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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
            stdin
                .write_all(general_event.to_string().as_bytes())
                .unwrap();
        })
        .wait_with_output()
        .unwrap();

    assert_eq!(output2.status.code(), Some(0));
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(!stdout2.contains("⚠️ Security Alert"));
    assert!(stdout2.contains("💡 Tip"));
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
