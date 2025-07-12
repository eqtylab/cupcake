use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn test_run_command_stdin_parsing() {
    // Test that the run command can parse hook events from stdin
    let hook_event_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session-integration",
        "transcript_path": "/tmp/test-transcript.jsonl",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo 'Integration test'",
            "description": "Test command for integration"
        }
    }
    "#;

    let mut child = Command::new("cargo")
        .args(["run", "--", "run", "--debug", "--event", "PreToolUse"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin
        .write_all(hook_event_json.as_bytes())
        .expect("Failed to write to stdin");
    child.stdin.take(); // Close stdin to signal end of input

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
    let stderr_output = String::from_utf8_lossy(&output.stderr);

    // Check that debug output contains expected information
    assert!(
        stderr_output.contains("Debug: Event: PreToolUse"),
        "Expected event debug output"
    );
    assert!(
        stderr_output.contains("Debug: Parsed hook event:"),
        "Expected parsed event debug output"
    );
    assert!(
        stderr_output.contains("test-session-integration"),
        "Expected session ID in debug output"
    );
    assert!(
        stderr_output.contains("Debug: Allowing operation")
            || stderr_output.contains("Debug: Evaluation complete"),
        "Expected evaluation or allow operation debug output"
    );
}

#[test]
fn test_run_command_with_policy_evaluation() {
    // Create a test policy file
    let test_policy = r#"
schema_version = "1.0"

[[policies]]
name = "Test Block Policy"
hook_event = "PreToolUse"
matcher = "Bash"
conditions = [
  { type = "pattern", field = "tool_input.command", regex = "^rm\\s" }
]
action = { type = "block_with_feedback", feedback_message = "Dangerous command blocked!", include_context = false }
    "#;

    // Write test policy to temp file
    let temp_dir = std::env::temp_dir();
    let policy_path = temp_dir.join("test-eval-policy.toml");
    std::fs::write(&policy_path, test_policy).expect("Failed to write test policy");

    // Test 1: Command that should be blocked
    let hook_event_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-eval-session",
        "transcript_path": "/tmp/test-transcript.jsonl",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /",
            "description": "Dangerous command"
        }
    }
    "#;

    let mut child = Command::new("cargo")
        .args([
            "run",
            "--",
            "run",
            "--debug",
            "--event",
            "PreToolUse",
            "--policy-file",
            policy_path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin
        .write_all(hook_event_json.as_bytes())
        .expect("Failed to write to stdin");
    child.stdin.take();

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // Should exit with code 2 (blocked)
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 for blocked operation"
    );

    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Dangerous command blocked!"),
        "Expected blocking feedback message"
    );
    assert!(
        stderr_output.contains("Debug: Evaluation complete"),
        "Expected evaluation complete message"
    );

    // Test 2: Command that should be allowed
    let safe_command_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-eval-session-2",
        "transcript_path": "/tmp/test-transcript.jsonl",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo 'safe command'",
            "description": "Safe command"
        }
    }
    "#;

    let mut child2 = Command::new("cargo")
        .args([
            "run",
            "--",
            "run",
            "--debug",
            "--event",
            "PreToolUse",
            "--policy-file",
            policy_path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    let stdin2 = child2.stdin.as_mut().expect("Failed to open stdin");
    stdin2
        .write_all(safe_command_json.as_bytes())
        .expect("Failed to write to stdin");
    child2.stdin.take();

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
        stderr_output2.contains("Debug: Allowing operation"),
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

    let mut child = Command::new("cargo")
        .args(["run", "--", "run", "--debug", "--event", "PreToolUse"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake run command");

    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    stdin
        .write_all(invalid_json.as_bytes())
        .expect("Failed to write to stdin");
    child.stdin.take(); // Close stdin to signal end of input

    let output = child
        .wait_with_output()
        .expect("Failed to wait for command");

    // The command should succeed (graceful degradation) but show error message
    assert!(
        output.status.success(),
        "Command should succeed with graceful degradation"
    );

    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Error reading hook event"),
        "Expected hook event error message"
    );
}

#[test]
fn test_run_command_empty_stdin() {
    // Test that the run command handles empty stdin gracefully
    let mut child = Command::new("cargo")
        .args(["run", "--", "run", "--debug", "--event", "PreToolUse"])
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

    // The command should succeed (graceful degradation) but show error message
    assert!(
        output.status.success(),
        "Command should succeed with graceful degradation"
    );

    let stderr_output = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_output.contains("Error reading hook event"),
        "Expected hook event error message"
    );
}
