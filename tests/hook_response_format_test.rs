//! Integration tests verifying correct JSON response format for all Claude Code hook types
//! Per July 20 specification, each hook type has specific JSON output requirements

use std::process::{Command, Stdio};
use std::io::Write;
use serde_json::{json, Value};
use tempfile::tempdir;
use std::fs;

/// Helper function to run cupcake with JSON input and capture output
fn run_cupcake_with_hook_event(
    policy_path: &std::path::Path,
    hook_event: &str,
    hook_json: &Value,
) -> (String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("run")
        .arg("--event")
        .arg(hook_event)
        .arg("--config")
        .arg(policy_path.to_str().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    // Send JSON input
    let stdin = cmd.stdin.as_mut().expect("Failed to get stdin");
    stdin.write_all(hook_json.to_string().as_bytes()).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");
    let _ = stdin;

    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    
    // Debug output if test is failing
    if stdout.is_empty() {
        eprintln!("Debug: No stdout from cupcake");
        eprintln!("Stderr: {}", stderr);
        eprintln!("Exit code: {}", exit_code);
    }
    
    (stdout, exit_code)
}

#[test]
fn test_pretooluse_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create a block policy for PreToolUse
    let policy = r#"
PreToolUse:
  "Bash":
    - name: block-rm
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^rm\\b"
      action:
        type: block_with_feedback
        feedback_message: "rm commands are not allowed"
"#;
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-123",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        }
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "PreToolUse", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // PreToolUse MUST use hookSpecificOutput.permissionDecision format
    assert!(response.get("hookSpecificOutput").is_some(), "PreToolUse must have hookSpecificOutput");
    assert_eq!(
        response["hookSpecificOutput"]["hookEventName"].as_str(),
        Some("PreToolUse")
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("deny")
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecisionReason"].as_str(),
        Some("rm commands are not allowed")
    );
}

#[test]
fn test_posttooluse_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create a block policy for PostToolUse
    let policy = r#"
PostToolUse:
  "Write":
    - name: block-sensitive-files
      conditions:
        - type: pattern
          field: tool_input.file_path
          regex: "\\.env$"
      action:
        type: block_with_feedback
        feedback_message: "Writing to .env files is not allowed"
"#;
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "PostToolUse",
        "session_id": "test-456",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "/app/.env",
            "content": "SECRET=xyz"
        },
        "tool_response": {
            "success": true
        }
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "PostToolUse", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // PostToolUse MUST NOT use hookSpecificOutput, should use continue/stopReason
    assert!(response.get("hookSpecificOutput").is_none(), "PostToolUse must NOT have hookSpecificOutput");
    assert_eq!(response["continue"].as_bool(), Some(false));
    assert_eq!(
        response["stopReason"].as_str(),
        Some("Writing to .env files is not allowed")
    );
}

#[test]
fn test_stop_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create an empty policy file (no policies = all allowed)
    let policy = "{}";
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "Stop",
        "session_id": "test-789",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "stop_hook_active": false
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "Stop", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // Stop event with Allow should output empty JSON object (no special fields)
    assert!(response.get("hookSpecificOutput").is_none(), "Stop must NOT have hookSpecificOutput");
    assert!(response.get("continue").is_none(), "Allow should not set continue field");
    assert!(response.get("stopReason").is_none(), "Allow should not set stopReason");
}

#[test]
fn test_notification_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create an empty policy file (no policies = all allowed)
    let policy = "{}";
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "Notification",
        "session_id": "test-notif",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "message": "Claude needs permission to use Bash"
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "Notification", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // Notification should have minimal response
    assert!(response.get("hookSpecificOutput").is_none(), "Notification must NOT have hookSpecificOutput");
    assert!(response.get("continue").is_none());
    assert!(response.get("stopReason").is_none());
}

#[test]
fn test_precompact_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create an empty policy file (no policies = all allowed)
    let policy = "{}";
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "PreCompact",
        "session_id": "test-compact",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "trigger": "manual",
        "custom_instructions": ""
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "PreCompact", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // PreCompact should have minimal response
    assert!(response.get("hookSpecificOutput").is_none(), "PreCompact must NOT have hookSpecificOutput");
    assert!(response.get("continue").is_none());
    assert!(response.get("stopReason").is_none());
}

#[test]
fn test_subagent_stop_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create an empty policy file (no policies = all allowed)
    let policy = "{}";
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "SubagentStop",
        "session_id": "test-subagent",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "stop_hook_active": false
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "SubagentStop", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // SubagentStop event should use decision/reason format like Stop
    assert!(response.get("hookSpecificOutput").is_none(), "SubagentStop must NOT have hookSpecificOutput");
    assert!(response.get("continue").is_none());
    assert!(response.get("stopReason").is_none());
}

#[test]
fn test_userpromptsubmit_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create a policy that injects context (as PolicyFragment, not RootConfig)
    let policy = r#"
UserPromptSubmit:
  "*":
    - name: add-context
      conditions:
        - type: pattern
          field: prompt
          regex: "weather"
      action:
        type: inject_context
        context: "Note: I don't have access to real-time weather data"
        use_stdout: true
"#;
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-prompt",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "prompt": "What's the weather like?"
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "UserPromptSubmit", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    // UserPromptSubmit uses special stdout injection for context
    assert_eq!(stdout.trim(), "Note: I don't have access to real-time weather data");
}

#[test]
fn test_ask_action_response_format() {
    let temp_dir = tempdir().unwrap();
    
    // Create an ask policy for PreToolUse
    let policy = r#"
PreToolUse:
  "Bash":
    - name: ask-for-sudo
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^sudo\\b"
      action:
        type: ask
        reason: "This command requires sudo. Please confirm you want to run it."
"#;
    
    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();
    
    let hook_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-ask",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "sudo apt update"
        }
    });
    
    let (stdout, exit_code) = run_cupcake_with_hook_event(&policy_path, "PreToolUse", &hook_event);
    assert_eq!(exit_code, 0, "Should exit with 0");
    
    let response: Value = serde_json::from_str(&stdout)
        .expect("Should output valid JSON");
    
    // Ask action for PreToolUse should use permissionDecision: "ask"
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecision"].as_str(),
        Some("ask")
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecisionReason"].as_str(),
        Some("This command requires sudo. Please confirm you want to run it.")
    );
}