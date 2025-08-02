use cupcake::config::actions::{Action, CommandSpec, ArrayCommandSpec, OnFailureBehavior};
use cupcake::config::loader::PolicyLoader;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_inject_context_edge_case_empty_context() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

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

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test prompt"
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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Empty context should result in empty output or just newline
    assert!(stdout.trim().is_empty() || stdout == "\n");
}

#[test]
fn test_inject_context_very_long_output() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a script that outputs a very long context
    let script_path = temp_dir.path().join("long-output.sh");
    fs::write(&script_path, r#"#!/bin/bash
# Generate 1000 lines of output
for i in {1..1000}; do
    echo "Line $i: This is a test of very long context injection output"
done
"#).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let policy_content = format!(r#"
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
"#, script_path.to_str().unwrap());

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "SessionStart",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "source": "startup"
    });

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
    assert!(line_count >= 1000, "Expected at least 1000 lines, got {}", line_count);
    assert!(stdout.contains("Line 1:"));
    assert!(stdout.contains("Line 1000:"));
}

#[test]
fn test_inject_context_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

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

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test special chars"
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
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a script that sleeps for a long time
    let script_path = temp_dir.path().join("slow-script.sh");
    fs::write(&script_path, r#"#!/bin/bash
sleep 30
echo "This should timeout"
"#).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let policy_content = format!(r#"
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
"#, script_path.to_str().unwrap());

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test timeout"
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

    // Should continue despite timeout
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Timeout should result in empty output
    assert!(!stdout.contains("This should timeout"));
}

#[test]
fn test_inject_context_combined_with_other_actions() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

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

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Build a web app"
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
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

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
    let security_event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session-1",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "How to implement security checks?"
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
            stdin.write_all(security_event.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠️ Security Alert"));
    assert!(!stdout.contains("💡 Tip"));

    // Test with non-security prompt
    let general_event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session-2",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Build a fast API"
    });

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
            stdin.write_all(general_event.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    assert_eq!(output2.status.code(), Some(0));
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(!stdout2.contains("⚠️ Security Alert"));
    assert!(stdout2.contains("💡 Tip"));
}

#[test]
fn test_inject_context_with_env_substitution() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a script that uses environment variables
    let script_path = temp_dir.path().join("env-test.sh");
    fs::write(&script_path, r#"#!/bin/bash
echo "Running as user: $USER"
echo "Home directory: $HOME"
echo "Session: $1"
echo "Custom env: $CUPCAKE_TEST"
"#).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let policy_content = format!(r#"
SessionStart:
  "*":
    - name: env-context
      description: Test environment variable substitution
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
            args: ["{{{{session_id}}}}"]
            env:
              CUPCAKE_TEST: "test-value-123"
          on_failure: continue
        use_stdout: true
"#, script_path.to_str().unwrap());

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "SessionStart",
        "session_id": "env-test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "source": "startup"
    });

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
    
    assert!(stdout.contains("Running as user:"));
    assert!(stdout.contains("Session: env-test-session"));
    assert!(stdout.contains("Custom env: test-value-123"));
}

#[test]
fn test_inject_context_from_command_with_working_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();
    
    let work_dir = temp_dir.path().join("workdir");
    fs::create_dir(&work_dir).unwrap();
    
    // Create a file in the work directory
    fs::write(work_dir.join("context.txt"), "Special project context").unwrap();

    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: workdir-context
      description: Test working directory
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["cat", "context.txt"]
            working_dir: "./workdir"
          on_failure: block
        use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Load project context"
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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Special project context"));
}

#[test]
fn test_inject_context_suppress_output_modes() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Test both use_stdout and suppress_output combinations
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: json-mode-suppressed
      description: JSON mode with suppress_output
      conditions:
        - type: pattern
          field: prompt
          regex: "mode1"
      action:
        type: inject_context
        context: "Mode 1: JSON with suppress"
        use_stdout: false  # JSON mode
        suppress_output: true  # Suppress output
        
    - name: stdout-mode-suppressed
      description: Stdout mode with suppress_output
      conditions:
        - type: pattern
          field: prompt
          regex: "mode2"
      action:
        type: inject_context
        context: "Mode 2: Stdout with suppress"
        use_stdout: true   # Stdout mode
        suppress_output: true  # Suppress output
        
    - name: json-mode-normal
      description: JSON mode without suppress
      conditions:
        - type: pattern
          field: prompt
          regex: "mode3"
      action:
        type: inject_context
        context: "Mode 3: JSON normal"
        use_stdout: false  # JSON mode
        suppress_output: false  # Normal output
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    // Test mode1: JSON with suppress
    let event1 = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-1",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test mode1"
    });

    let output1 = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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
            stdin.write_all(event1.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    // Should output JSON with suppressOutput: true
    assert!(stdout1.contains("\"suppressOutput\":true") || stdout1.contains("\"suppressOutput\": true"));
    assert!(stdout1.contains("Mode 1: JSON with suppress"));

    // Test mode2: Stdout with suppress
    let event2 = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-2",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test mode2"
    });

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
            stdin.write_all(event2.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    // Should switch to JSON when suppress_output is true
    assert!(stdout2.contains("\"suppressOutput\":true") || stdout2.contains("\"suppressOutput\": true"));
}

#[test]
fn test_inject_context_error_propagation() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Test command that exits with error and on_failure: block
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: error-block
      description: Test error with block
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["sh", "-c", "echo 'Error occurred' >&2; exit 1"]
          on_failure: block
        use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test error"
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

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should output JSON with block decision
    assert!(stdout.contains("\"continue\":false") || stdout.contains("\"continue\": false"));
    assert!(stdout.contains("Dynamic context generation failed"));
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