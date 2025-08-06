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
fn test_run_command_stdin_parsing() {
    // Test that the run command can parse hook events from stdin
    let hook_event_json = EventFactory::pre_tool_use()
        .session_id("test-session-integration")
        .transcript_path("/tmp/test-transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("echo 'Integration test'")
        .tool_input_description("Test command for integration")
        .build_json();

    // Create an empty config file for this test
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test-stdin-parsing-config.yaml");
    std::fs::write(&config_path, "# Empty config\n").expect("Failed to write test config");

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args([
            "run",
            "--debug",
            "--event",
            "PreToolUse",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .env("RUST_LOG", "cupcake=debug")
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
    // stdin is dropped here, which closes the pipe

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // The command should succeed (exit code 0 = allow operation)
    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );

    // Convert stderr to string for debugging
    let _stderr_output = String::from_utf8_lossy(&output.stderr);
    let stdout_output = String::from_utf8_lossy(&output.stdout);

    // The test should succeed
    assert!(
        output.status.success(),
        "Command failed with status: {}",
        output.status
    );

    // With an empty config and valid input, it should allow by default
    // In the new spec-compliant behavior, we output JSON even for allow
    assert!(
        !stdout_output.is_empty(),
        "Expected JSON output for allow decision"
    );

    // Parse and verify the JSON response
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout_output).expect("stdout should be valid JSON");

    // Should be an allow decision
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "allow",
        "JSON response should have permissionDecision: allow"
    );

    // Default allow should have null reason
    let reason = &response_json["hookSpecificOutput"]["permissionDecisionReason"];
    assert!(reason.is_null(), "Default allow should have null reason");

    // Cleanup
    std::fs::remove_file(&config_path).ok();
}

#[test]
fn test_run_command_with_policy_evaluation() {
    // Create a test policy file in YAML format
    let test_policy = r#"
PreToolUse:
  "Bash":
    - name: "Test Block Policy"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "^rm\\s"
      action:
        type: "block_with_feedback"
        feedback_message: "Dangerous command blocked!"
        include_context: false
    "#;

    // Write test policy to temp file
    let temp_dir = std::env::temp_dir();
    let policy_path = temp_dir.join("test-eval-policy.yaml");
    std::fs::write(&policy_path, test_policy).expect("Failed to write test policy");

    // Test 1: Command that should be blocked
    let hook_event_json = EventFactory::pre_tool_use()
        .session_id("test-eval-session")
        .transcript_path("/tmp/test-transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("rm -rf /")
        .tool_input_description("Dangerous command")
        .build_json();

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args([
            "run",
            "--debug",
            "--event",
            "PreToolUse",
            "--config",
            policy_path.to_str().unwrap(),
        ])
        .env("RUST_LOG", "cupcake=debug")
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

    // Should exit with code 0 (success) but provide JSON response for block
    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 with JSON response for blocked operation"
    );

    // Should provide JSON response with block decision
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout was not valid JSON");

    // Should be a block decision in JSON format
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "deny",
        "JSON response should have permissionDecision: deny"
    );

    let stderr_output = String::from_utf8_lossy(&output.stderr);
    // The blocking feedback should still appear in stderr as it's not a debug message
    assert!(
        stderr_output.contains("Dangerous command blocked!"),
        "Expected blocking feedback message"
    );

    // Test 2: Command that should be allowed
    let safe_command_json = EventFactory::pre_tool_use()
        .session_id("test-eval-session-2")
        .transcript_path("/tmp/test-transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("echo 'safe command'")
        .tool_input_description("Safe command")
        .build_json();

    let mut child2 = Command::new(&cupcake_binary)
        .args([
            "run",
            "--debug",
            "--event",
            "PreToolUse",
            "--config",
            policy_path.to_str().unwrap(),
        ])
        .env("RUST_LOG", "cupcake=debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    // Write to stdin and explicitly close it
    {
        let stdin2 = child2.stdin.as_mut().expect("Failed to open stdin");
        stdin2
            .write_all(safe_command_json.as_bytes())
            .expect("Failed to write to stdin");
        stdin2.flush().expect("Failed to flush stdin");
    }

    let output2 = child2
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should exit with code 0 (allowed)
    assert!(
        output2.status.success(),
        "Expected success for allowed operation"
    );

    let stderr_output2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr_output2.contains("Sending response") || stderr_output2.contains("Allow response"),
        "Expected allow operation message"
    );

    // Cleanup
    std::fs::remove_file(&policy_path).ok();
}

#[test]
fn test_run_command_invalid_json() {
    // Test that the run command handles invalid JSON gracefully
    let invalid_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "invalid_field": 
    }
    "#;

    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(["run", "--debug", "--event", "PreToolUse"])
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

    // The command should succeed with exit code 0 (fail-closed behavior)
    assert!(
        output.status.success(),
        "Command should succeed with fail-closed behavior"
    );

    // Should output blocking JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON for fail-closed");

    // For PreToolUse, blocking response should have permissionDecision: deny
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "deny",
        "Expected permissionDecision: deny for fail-closed behavior"
    );

    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Cupcake error (failing closed)"),
        "Expected fail-closed error message in stderr"
    );
}

#[test]
fn test_run_command_empty_stdin() {
    // Test that the run command handles empty stdin gracefully
    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(["run", "--debug", "--event", "PreToolUse"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    // Don't write anything to stdin, just close it
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // The command should succeed with exit code 0 (fail-closed behavior)
    assert!(
        output.status.success(),
        "Command should succeed with fail-closed behavior"
    );

    // Should output blocking JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON for fail-closed");

    // For PreToolUse, blocking response should have permissionDecision: deny
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(
        decision, "deny",
        "Expected permissionDecision: deny for fail-closed behavior"
    );

    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Cupcake error (failing closed)"),
        "Expected fail-closed error message in stderr"
    );
}
