use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Once;

// Ensure we only build the binary once for all tests
static BUILD_ONCE: Once = Once::new();
static mut BINARY_PATH: Option<String> = None;

fn get_cupcake_binary() -> String {
    unsafe {
        BUILD_ONCE.call_once(|| {
            // Build the binary
            let output = Command::new("cargo")
                .args(&["build"])
                .output()
                .expect("Failed to build cupcake");
            
            if !output.status.success() {
                panic!("Failed to build cupcake binary: {}", String::from_utf8_lossy(&output.stderr));
            }
            
            let path = std::env::current_dir()
                .unwrap()
                .join("target")
                .join("debug")
                .join("cupcake");
            
            BINARY_PATH = Some(path.to_string_lossy().to_string());
        });
        
        BINARY_PATH.clone().unwrap()
    }
}

#[test]
fn test_run_command_stdin_parsing() {
    // Test that the run command can parse hook events from stdin
    let hook_event_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session-integration",
        "transcript_path": "/tmp/test-transcript.jsonl",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo 'Integration test'",
            "description": "Test command for integration"
        }
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
    let hook_event_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-eval-session",
        "transcript_path": "/tmp/test-transcript.jsonl",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /",
            "description": "Dangerous command"
        }
    }
    "#;

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
    let response_json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout was not valid JSON");

    // Should be a block decision in JSON format
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(decision, "deny", "JSON response should have permissionDecision: deny");

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
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo 'safe command'",
            "description": "Safe command"
        }
    }
    "#;

    let mut child2 = Command::new(&cupcake_binary)
        .args([
            "run",
            "--debug",
            "--event",
            "PreToolUse",
            "--config",
            policy_path.to_str().unwrap(),
        ])
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
