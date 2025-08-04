use crate::common::event_factory::EventFactory;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn test_silent_auto_approval() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file with silent auto-approval
    let policy_yaml = r#"
PreToolUse:
  "Write|Edit":
    - name: "Silent auto-allow test files"
      conditions:
        - type: pattern
          field: tool_input.file_path
          regex: "test/"
      action:
        type: allow
        reason: "Test file auto-approved"
        suppress_output: true
"#;
    fs::write(policies_dir.join("silent-allow-policy.yaml"), policy_yaml).unwrap();

    // Create hook event JSON
    let hook_event_json = EventFactory::pre_tool_use()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/test")
        .tool_name("Write")
        .tool_input(serde_json::json!({
            "file_path": "test/example.rs",
            "content": "fn test() {}"
        }))
        .build_json();

    // Build the cupcake binary
    Command::new("cargo")
        .args(["build", "--bin", "cupcake"])
        .output()
        .expect("Failed to build cupcake");

    // Run cupcake
    let mut cmd = Command::new("./target/debug/cupcake")
        .args([
            "run",
            "--event",
            "-",
            "--config",
            guardrails_dir.join("cupcake.yaml").to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");

    // Verify the output
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON response
    let response: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify it's an allow decision with suppressOutput
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecision"],
        "allow"
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecisionReason"],
        "Test file auto-approved"
    );
    assert_eq!(response["suppressOutput"], true);
}

#[test]
fn test_silent_feedback() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy with silent feedback
    let policy_yaml = r#"
PreToolUse:
  "Bash":
    - name: "Silent feedback for tests"
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^cargo test"
      action:
        type: provide_feedback
        message: "Running tests - good practice!"
        suppress_output: true
"#;
    fs::write(policies_dir.join("silent-feedback.yaml"), policy_yaml).unwrap();

    // Create hook event JSON
    let hook_event_json = EventFactory::pre_tool_use()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/test")
        .tool_name("Bash")
        .tool_input_command("cargo test")
        .build_json();

    // Run cupcake
    let mut cmd = Command::new("./target/debug/cupcake")
        .args([
            "run",
            "--event",
            "-",
            "--config",
            guardrails_dir.join("cupcake.yaml").to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // With suppress_output, feedback should not appear in stdout
    assert!(!stdout.contains("Running tests - good practice!"));
}

#[test]
fn test_silent_context_injection() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy with silent context injection
    let policy_yaml = r#"
UserPromptSubmit:
  "*":
    - name: "Silent context injection"
      conditions:
        - type: pattern
          field: prompt
          regex: "secret"
      action:
        type: inject_context
        context: "Remember: handle secrets carefully"
        suppress_output: true
"#;
    fs::write(policies_dir.join("silent-inject.yaml"), policy_yaml).unwrap();

    // Create hook event JSON
    let hook_event_json = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/test")
        .prompt("How do I store secret keys?")
        .build_json();

    // Run cupcake
    let mut cmd = Command::new("./target/debug/cupcake")
        .args([
            "run",
            "--event",
            "-",
            "--config",
            guardrails_dir.join("cupcake.yaml").to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {stdout}");
    println!("STDERR: {stderr}");

    // Should get JSON response instead of plain text
    let response: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(response["suppressOutput"], true);

    // Context should be in additionalContext field
    assert_eq!(
        response["hookSpecificOutput"]["additionalContext"],
        "Remember: handle secrets carefully"
    );
}
