/// Integration tests to verify the new JSON-based communication protocol
/// These tests ensure that the Claude Code July 20 hook specification is correctly implemented

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn test_pre_tool_use_block_produces_correct_json() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"imports: ["policies/*.yaml"]"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create a simple policy that always blocks
    let policy_yaml = r#"
PreToolUse:
  "*":
    - name: "Always Block"
      conditions: []
      action:
        type: "block_with_feedback"
        feedback_message: "Blocked by test policy"
"#;
    fs::write(policies_dir.join("block_policy.yaml"), policy_yaml).unwrap();

    // Create a PreToolUse hook event
    let hook_event_json = r#"
{
    "hook_event_name": "PreToolUse",
    "session_id": "test-session-json-block",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/tmp",
    "tool_name": "Bash",
    "tool_input": { "command": "ls" }
}
"#;

    // Run cupcake
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut cmd = Command::new(cupcake_binary)
        .args(&["run", "--event", "PreToolUse"])
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    cmd.stdin.as_mut().unwrap().write_all(hook_event_json.as_bytes()).unwrap();
    let output = cmd.wait_with_output().unwrap();

    // VERIFY THE NEW CONTRACT
    // 1. Assert the process exits successfully (code 0)
    assert_eq!(output.status.code(), Some(0), "Process should exit 0 even when blocking");

    // 2. Assert the decision is communicated via JSON on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout was not valid JSON");

    // 3. Assert the JSON has the correct structure for a block decision
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(decision, "deny", "JSON response should have permissionDecision: deny");

    let reason = &response_json["hookSpecificOutput"]["permissionDecisionReason"];
    assert_eq!(reason, "Blocked by test policy", "JSON should contain the feedback message");
}

#[test]
fn test_pre_tool_use_allow_produces_correct_json() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"imports: ["policies/*.yaml"]"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create a simple policy that always allows
    let policy_yaml = r#"
PreToolUse:
  "*":
    - name: "Always Allow"
      conditions: []
      action:
        type: "allow"
        reason: "Allowed by test policy"
"#;
    fs::write(policies_dir.join("allow_policy.yaml"), policy_yaml).unwrap();

    // Create a PreToolUse hook event
    let hook_event_json = r#"
{
    "hook_event_name": "PreToolUse",
    "session_id": "test-session-json-allow",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/tmp",
    "tool_name": "Bash",
    "tool_input": { "command": "echo hello" }
}
"#;

    // Run cupcake
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut cmd = Command::new(cupcake_binary)
        .args(&["run", "--event", "PreToolUse"])
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    cmd.stdin.as_mut().unwrap().write_all(hook_event_json.as_bytes()).unwrap();
    let output = cmd.wait_with_output().unwrap();

    // VERIFY THE NEW CONTRACT
    // 1. Assert the process exits successfully (code 0)
    assert_eq!(output.status.code(), Some(0), "Process should exit 0 for allow");

    // 2. Assert the decision is communicated via JSON on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout was not valid JSON");

    // 3. Assert the JSON has the correct structure for an allow decision
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(decision, "allow", "JSON response should have permissionDecision: allow");

    let reason = &response_json["hookSpecificOutput"]["permissionDecisionReason"];
    assert_eq!(reason, "Allowed by test policy", "JSON should contain the allow reason");
}

#[test]
fn test_pre_tool_use_no_matching_policy_defaults_to_allow() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"imports: ["policies/*.yaml"]"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create a policy that doesn't match our tool
    let policy_yaml = r#"
PreToolUse:
  "Git":  # Only matches Git tool, not Bash
    - name: "Git Only Policy"
      conditions: []
      action:
        type: "block_with_feedback"
        feedback_message: "This shouldn't match"
"#;
    fs::write(policies_dir.join("nomatch_policy.yaml"), policy_yaml).unwrap();

    // Create a PreToolUse hook event for Bash (won't match Git policy)
    let hook_event_json = r#"
{
    "hook_event_name": "PreToolUse",
    "session_id": "test-session-json-default",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/tmp",
    "tool_name": "Bash",
    "tool_input": { "command": "echo hello" }
}
"#;

    // Run cupcake
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut cmd = Command::new(cupcake_binary)
        .args(&["run", "--event", "PreToolUse"])
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    cmd.stdin.as_mut().unwrap().write_all(hook_event_json.as_bytes()).unwrap();
    let output = cmd.wait_with_output().unwrap();

    // VERIFY THE NEW CONTRACT
    // 1. Assert the process exits successfully (code 0)
    assert_eq!(output.status.code(), Some(0), "Process should exit 0 for default allow");

    // 2. Assert the decision is communicated via JSON on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response_json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout was not valid JSON");

    // 3. Assert the JSON has the correct structure for a default allow decision
    let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
    assert_eq!(decision, "allow", "JSON response should have permissionDecision: allow by default");

    // Should have no reason for the default allow case
    let reason = &response_json["hookSpecificOutput"]["permissionDecisionReason"];
    assert!(reason.is_null(), "Default allow should have null reason");
}