//! Integration tests for fail-closed error handling behavior
//!
//! Ensures that any error during hook processing results in a spec-compliant
//! blocking response instead of silently failing open.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::common::event_factory::EventFactory;

fn get_cupcake_binary() -> String {
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join("debug")
        .join("cupcake")
        .to_string_lossy()
        .to_string()
}

#[test]
fn test_fail_closed_with_malformed_config() {
    // Create a malformed config file
    let malformed_config = r#"
PreToolUse:
  "Bash":
    - name: "Malformed Policy"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          # Missing required 'regex' field
      action:
        type: "block_with_feedback"
        feedback_message: "This should not be reached"
    "#;

    // Write malformed config to temp file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test-malformed-config.yaml");
    std::fs::write(&config_path, malformed_config).expect("Failed to write test config");

    // Create a valid hook event
    let hook_event_json = EventFactory::pre_tool_use()
        .session_id("test-fail-closed")
        .transcript_path("/tmp/test-transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("echo 'test'")
        .tool_input_description("Test command")
        .build_json();

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args([
            "run",
            "--event",
            "PreToolUse",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    // Write to stdin and explicitly close it
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(hook_event_json.as_bytes())
            .expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should exit with code 0 (required by Claude Code spec)
    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 for fail-closed behavior"
    );

    // stdout should contain a valid blocking JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // For PreToolUse, blocking response should have permissionDecision: deny
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "deny",
        "Expected permissionDecision: deny for fail-closed behavior"
    );

    // Should have an error reason
    let reason = &response_json["hookSpecificOutput"]["permissionDecisionReason"];
    assert!(
        reason.as_str().unwrap().contains("Cupcake error"),
        "Expected error reason in blocking response"
    );

    // stderr should contain error message
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Cupcake error (failing closed)"),
        "Expected fail-closed error message in stderr"
    );

    // Cleanup
    std::fs::remove_file(&config_path).ok();
}

#[test]
fn test_fail_closed_with_invalid_event_json() {
    // Test with malformed JSON that will cause parsing to fail
    let invalid_json = r#"{"hook_event_name": "PreToolUse", "session_id": INVALID}"#;

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(["run", "--event", "PreToolUse"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    // Write to stdin and explicitly close it
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(invalid_json.as_bytes())
            .expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should exit with code 0 (required by Claude Code spec)
    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 for fail-closed behavior"
    );

    // stdout should contain a valid blocking JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // For PreToolUse, blocking response should have permissionDecision: deny
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "deny",
        "Expected permissionDecision: deny for fail-closed behavior"
    );

    // stderr should contain error message
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Cupcake error (failing closed)"),
        "Expected fail-closed error message in stderr"
    );
}

#[test]
fn test_fail_closed_user_prompt_submit() {
    // Test fail-closed behavior for UserPromptSubmit event
    let invalid_json = r#"{"hook_event_name": "UserPromptSubmit", "invalid": true}"#;

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(["run", "--event", "UserPromptSubmit"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    // Write to stdin and explicitly close it
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(invalid_json.as_bytes())
            .expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should exit with code 0 (required by Claude Code spec)
    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 for fail-closed behavior"
    );

    // stdout should contain a valid blocking JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // For UserPromptSubmit, blocking response should have continue: false
    assert_eq!(
        response_json["continue"], false,
        "Expected continue: false for fail-closed behavior"
    );

    // Should have a stopReason
    assert!(
        response_json["stopReason"]
            .as_str()
            .unwrap()
            .contains("Cupcake error"),
        "Expected error in stopReason"
    );
}

#[test]
fn test_fail_closed_with_nonexistent_config() {
    // Test loading a config file that doesn't exist
    let hook_event_json = EventFactory::pre_tool_use()
        .session_id("test-fail-closed-no-config")
        .transcript_path("/tmp/test-transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("echo 'test'")
        .tool_input_description("Test command")
        .build_json();

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args([
            "run",
            "--event",
            "PreToolUse",
            "--config",
            "/nonexistent/path/to/config.yaml",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    // Write to stdin and explicitly close it
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(hook_event_json.as_bytes())
            .expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should exit with code 0 (required by Claude Code spec)
    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 for fail-closed behavior"
    );

    // stdout should contain a valid blocking JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // For PreToolUse, blocking response should have permissionDecision: deny
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "deny",
        "Expected permissionDecision: deny for fail-closed behavior"
    );

    // stderr should contain error message about file not found
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Cupcake error (failing closed)"),
        "Expected fail-closed error message in stderr"
    );
}