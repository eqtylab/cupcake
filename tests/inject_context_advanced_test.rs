use serde_json::json;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_inject_context_with_pipeline_commands() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test command execution with static output
    let policy_content = r#"
SessionStart:
  "*":
    - name: pipeline-context
      description: Test command execution
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["echo", "Files in project:\nCurrent time: 2025-08-02"]
          on_failure: continue
        use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "SessionStart",
        "session_id": "pipeline-test",
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
    assert!(stdout.contains("Files in project:"));
    assert!(stdout.contains("Current time:"));
}

#[test]
fn test_inject_context_with_multiple_template_vars() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Create a script that uses multiple template variables
    let script_path = temp_dir.path().join("multi-template.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
echo "=== Context Report ==="
echo "Prompt: $1"
echo "Session: $2"
echo "Directory: $3"
echo "User: $4"
echo "===================="
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
UserPromptSubmit:
  "*":
    - name: multi-template
      description: Test multiple template variables
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
            args: ["{{{{prompt}}}}", "{{{{session_id}}}}", "{{{{cwd}}}}", "{{{{env.USER}}}}"]
          on_failure: continue
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "multi-var-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Complex template test"
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

    assert!(stdout.contains("Prompt: Complex template test"));
    assert!(stdout.contains("Session: multi-var-session"));
    assert!(stdout.contains(&format!("Directory: {}", temp_dir.path().display())));
    assert!(stdout.contains("User:")); // Will contain actual username
}

#[test]
fn test_inject_context_nested_conditionals() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test nested conditional context injection
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: nested-conditional
      description: Nested conditional logic
      conditions: []
      action:
        type: conditional
        if:
          type: pattern
          field: prompt
          regex: "(?i)database"
        then:
          type: conditional
          if:
            type: pattern
            field: prompt
            regex: "(?i)production"
          then:
            type: inject_context
            context: "🔴 PRODUCTION DATABASE - Use extreme caution!"
            use_stdout: true
          else:
            type: inject_context
            context: "🟡 Development database - Test freely"
            use_stdout: true
        else:
          type: inject_context
          context: "💚 General development context"
          use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    // Test production database prompt
    let prod_event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-prod",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Connect to production database"
    });

    let output_prod = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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
            stdin.write_all(prod_event.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout_prod = String::from_utf8_lossy(&output_prod.stdout);
    assert!(stdout_prod.contains("🔴 PRODUCTION DATABASE"));

    // Test dev database prompt
    let dev_event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-dev",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Setup test database"
    });

    let output_dev = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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
            stdin.write_all(dev_event.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout_dev = String::from_utf8_lossy(&output_dev.stdout);
    assert!(stdout_dev.contains("🟡 Development database"));

    // Test general prompt
    let general_event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-general",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Build a REST API"
    });

    let output_general = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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

    let stdout_general = String::from_utf8_lossy(&output_general.stdout);
    assert!(stdout_general.contains("💚 General development context"));
}

#[test]
fn test_inject_context_with_multiple_policies() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();
    let policies_dir = config_dir.join("policies");
    fs::create_dir(&policies_dir).unwrap();

    // Root config with imports
    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - "policies/*.yaml"
"#;
    fs::write(config_dir.join("cupcake.yaml"), root_config).unwrap();

    // Policy 1: Security context
    let security_policy = r#"
UserPromptSubmit:
  "*":
    - name: security-context
      description: Inject security context
      conditions:
        - type: pattern
          field: prompt
          regex: "(?i)(auth|login|password|token)"
      action:
        type: inject_context
        context: |
          🔐 Security Context:
          - Use bcrypt for passwords
          - Implement rate limiting
          - Never log sensitive data
        use_stdout: true
"#;
    fs::write(policies_dir.join("10-security.yaml"), security_policy).unwrap();

    // Policy 2: Performance context
    let performance_policy = r#"
UserPromptSubmit:
  "*":
    - name: performance-context
      description: Inject performance context
      conditions:
        - type: pattern
          field: prompt
          regex: "(?i)(optimize|performance|fast|speed)"
      action:
        type: inject_context
        context: |
          ⚡ Performance Context:
          - Profile before optimizing
          - Consider caching strategies
          - Use async where appropriate
        use_stdout: true
"#;
    fs::write(policies_dir.join("20-performance.yaml"), performance_policy).unwrap();

    // Policy 3: Default context (always applies)
    let default_policy = r#"
UserPromptSubmit:
  "*":
    - name: default-context
      description: Default project context
      conditions: []
      action:
        type: inject_context
        context: "📋 Project: Follow SOLID principles and write tests"
        use_stdout: true
"#;
    fs::write(policies_dir.join("99-default.yaml"), default_policy).unwrap();

    // Test prompt that triggers multiple contexts
    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "multi-policy",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Optimize the login performance"
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
            stdin.write_all(event.to_string().as_bytes()).unwrap();
        })
        .wait_with_output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain all three contexts
    assert!(stdout.contains("🔐 Security Context"));
    assert!(stdout.contains("⚡ Performance Context"));
    assert!(stdout.contains("📋 Project: Follow SOLID principles"));
}

#[test]
fn test_inject_context_with_binary_output() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Create a script that outputs binary data
    let script_path = temp_dir.path().join("binary-output.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
# Output some text followed by binary data
echo "Text context"
# Output null bytes and other binary data
printf '\x00\x01\x02\x03\xff\xfe'
echo "More text"
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
UserPromptSubmit:
  "*":
    - name: binary-test
      description: Test binary output handling
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

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "binary-test",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test binary"
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
    // Binary data should be handled gracefully
    let stdout = output.stdout;
    assert!(stdout.windows(4).any(|w| w == b"Text"));
}

#[test]
fn test_inject_context_concurrent_execution() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Create a slow script to test timeout/concurrency
    let script_path = temp_dir.path().join("slow-context.sh");
    fs::write(
        &script_path,
        r#"#!/bin/bash
sleep 0.5
echo "Context loaded after delay"
"#,
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Multiple policies with inject_context
    let policy_content = format!(
        r#"
UserPromptSubmit:
  "*":
    - name: context-1
      conditions: []
      action:
        type: inject_context
        context: "Static context 1"
        use_stdout: true
        
    - name: context-2
      conditions: []
      action:
        type: inject_context
        from_command:
          spec:
            mode: array
            command: ["{}"]
          on_failure: continue
        use_stdout: true
        
    - name: context-3
      conditions: []
      action:
        type: inject_context
        context: "Static context 3"
        use_stdout: true
"#,
        script_path.to_str().unwrap()
    );

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    let event_json = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "concurrent-test",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "prompt": "Test concurrency"
    });

    let start = std::time::Instant::now();
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
    let duration = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Exit code: {:?}", output.status.code());
    println!("STDOUT: {stdout}");
    println!("STDERR: {stderr}");

    assert_eq!(output.status.code(), Some(0));

    // All contexts should appear
    assert!(stdout.contains("Static context 1"));
    assert!(stdout.contains("Context loaded after delay"));
    assert!(stdout.contains("Static context 3"));

    // Should complete reasonably quickly despite delay
    assert!(
        duration.as_secs() < 2,
        "Execution took too long: {duration:?}"
    );
}

#[test]
fn test_inject_context_session_source_matching() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test SessionStart with different source conditions
    // FIXME: Cupcake doesn't support source-specific matching for SessionStart yet
    // Claude Code allows matching "startup", "resume", "clear" but Cupcake only supports "*"
    // For now, use "*" matcher to make test pass
    let policy_content = r#"
SessionStart:
  "*":
    - name: startup-context
      conditions:
        - type: pattern
          field: source
          regex: "^startup$"
      action:
        type: inject_context
        context: "🚀 Fresh session started"
        use_stdout: true
        
    - name: resume-context
      conditions:
        - type: pattern
          field: source
          regex: "^resume$"
      action:
        type: inject_context
        context: "📂 Resuming previous session"
        use_stdout: true
        
    - name: clear-context
      conditions:
        - type: pattern
          field: source
          regex: "^clear$"
      action:
        type: inject_context
        context: "🧹 Session cleared and restarted"
        use_stdout: true
"#;

    fs::write(config_dir.join("cupcake.yaml"), policy_content).unwrap();

    // Test startup source
    let startup_event = json!({
        "hook_event_name": "SessionStart",
        "session_id": "startup-test",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "source": "startup"
    });

    let output_startup = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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
            stdin
                .write_all(startup_event.to_string().as_bytes())
                .unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout_startup = String::from_utf8_lossy(&output_startup.stdout);
    let stderr_startup = String::from_utf8_lossy(&output_startup.stderr);

    println!(
        "Startup test - Exit code: {:?}",
        output_startup.status.code()
    );
    println!("STDOUT: {stdout_startup}");
    println!("STDERR: {stderr_startup}");

    assert_eq!(output_startup.status.code(), Some(0));
    assert!(stdout_startup.contains("🚀 Fresh session started"));
    assert!(!stdout_startup.contains("📂 Resuming"));

    // Test resume source
    let resume_event = json!({
        "hook_event_name": "SessionStart",
        "session_id": "resume-test",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": temp_dir.path().to_str().unwrap(),
        "source": "resume"
    });

    let output_resume = Command::new(env!("CARGO_BIN_EXE_cupcake"))
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
            stdin
                .write_all(resume_event.to_string().as_bytes())
                .unwrap();
        })
        .wait_with_output()
        .unwrap();

    let stdout_resume = String::from_utf8_lossy(&output_resume.stdout);
    let stderr_resume = String::from_utf8_lossy(&output_resume.stderr);

    println!("Resume test - Exit code: {:?}", output_resume.status.code());
    println!("STDOUT: {stdout_resume}");
    println!("STDERR: {stderr_resume}");

    assert_eq!(output_resume.status.code(), Some(0));
    assert!(stdout_resume.contains("📂 Resuming previous session"));
    assert!(!stdout_resume.contains("🚀 Fresh session"));
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
