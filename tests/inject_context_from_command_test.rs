use cupcake::config::actions::{Action, ArrayCommandSpec, CommandSpec, OnFailureBehavior};
use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_inject_context_from_command_yaml_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a test script that outputs context
    let script_path = temp_dir.path().join("get-context.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
echo "Dynamic context: User asked about '$1'"
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Create a test policy with from_command
    let policy_content = format!(
        r#"
UserPromptSubmit:
  "*":
    - name: inject-dynamic-context
      description: Inject dynamic context from command
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
            args: ["{{{{prompt}}}}"]
          on_failure: continue
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    // Create UserPromptSubmit event JSON
    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "How to implement authentication?"
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

    // Should output the dynamic context to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dynamic context: User asked about 'How to implement authentication?'"));
}

#[test]
fn test_inject_context_from_command_failure_continue() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a policy with from_command that will fail but continue
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: inject-failing-command
      description: Command that fails but continues
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["/nonexistent/command"]
          on_failure: continue
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

    // Should exit with code 0 (continue despite failure)
    assert_eq!(output.status.code(), Some(0));

    // Should output empty context (command failed but on_failure: continue)
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Empty output or just newline is expected
    assert!(stdout.trim().is_empty() || stdout == "\n");
}

#[test]
fn test_inject_context_from_command_failure_block() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a policy with from_command that will fail and block
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: inject-failing-command
      description: Command that fails and blocks
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["false"]  # Command that always fails
          on_failure: block
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

    // Should exit with code 0 but provide JSON response for block
    assert_eq!(output.status.code(), Some(0));

    // Should output JSON with block decision
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"continue\":false") || stdout.contains("\"continue\": false"));
    assert!(stdout.contains("Dynamic context generation failed"));
}

#[test]
fn test_inject_context_from_command_with_template_substitution() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a test script that uses multiple template variables
    let script_path = temp_dir.path().join("get-context.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
echo "Session: $1"
echo "User: $2"
echo "Prompt: $3"
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Create a policy with template substitution in args
    let policy_content = format!(
        r#"
UserPromptSubmit:
  "*":
    - name: inject-with-templates
      description: Use multiple template variables
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
            args: ["{{{{session_id}}}}", "{{{{env.USER}}}}", "{{{{prompt}}}}"]
          on_failure: continue
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-123",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Help with testing"
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
    assert!(stdout.contains("Session: test-123"));
    assert!(stdout.contains("User:")); // Will have actual username
    assert!(stdout.contains("Prompt: Help with testing"));
}

#[test]
fn test_inject_context_action_builder() {
    // Test static context builder
    let static_action = Action::inject_context("Static context");
    match static_action {
        Action::InjectContext {
            context,
            from_command,
            use_stdout,
            suppress_output,
        } => {
            assert_eq!(context, Some("Static context".to_string()));
            assert!(from_command.is_none());
            assert!(use_stdout);
            assert!(!suppress_output);
        }
        _ => panic!("Expected InjectContext action"),
    }

    // Test from_command builder
    let command_action = Action::inject_context_from_command(
        CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["test".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        })),
        OnFailureBehavior::Continue,
    );

    match command_action {
        Action::InjectContext {
            context,
            from_command,
            ..
        } => {
            assert!(context.is_none());
            assert!(from_command.is_some());
        }
        _ => panic!("Expected InjectContext action"),
    }
}

#[test]
fn test_inject_context_from_command_with_session_start() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("guardrails");
    fs::create_dir(&config_dir).unwrap();

    // Create a test script that outputs session-specific context
    let script_path = temp_dir.path().join("get-session-context.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
echo "Welcome to session $1!"
echo "Current directory: $PWD"
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Create a test policy for SessionStart with from_command
    let policy_content = format!(
        r#"
SessionStart:
  "*":
    - name: inject-session-context
      description: Inject dynamic context at session start
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
            args: ["{{{{session_id}}}}"]
          on_failure: continue
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    // Create SessionStart event JSON
    let event_json = json!({
        "hook_event_name": "SessionStart",
        "session_id": "session-456",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "source": "startup"
    });

    // Run cupcake with the test configuration
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

    // Should exit with code 0 (allow)
    assert_eq!(output.status.code(), Some(0));

    // Should output the dynamic context to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Welcome to session session-456!"));
    assert!(stdout.contains("Current directory:"));
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
