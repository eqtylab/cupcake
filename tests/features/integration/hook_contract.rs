use serde_json::Value;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

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

/// Integration tests that verify Cupcake correctly implements the Claude Code hook contract
/// These tests run the actual cupcake binary and verify its JSON input/output

#[test]
fn test_pretooluse_allow_json_output() {
    let temp_dir = tempdir().unwrap();

    // Create a simple allow policy
    let policy = r#"
PreToolUse:
  "Bash":
    - name: allow-ls
      description: Allow ls commands
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^ls\\b"
      action:
        type: allow
        reason: "ls is safe"
"#;

    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();

    // Create hook event using EventFactory
    let hook_event = EventFactory::pre_tool_use()
        .session_id("test-123")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("ls -la")
        .build_value();

    // Run cupcake and capture output
    let output = run_cupcake_with_json(&policy_path, "PreToolUse", &hook_event);

    // Parse and verify JSON response
    let response: Value = serde_json::from_str(&output).expect("Invalid JSON output");

    // Verify it has the correct permission decision
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecision"],
        "allow"
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecisionReason"],
        "ls is safe"
    );
}

#[test]
fn test_pretooluse_deny_json_output() {
    let temp_dir = tempdir().unwrap();

    // Create a block policy
    let policy = r#"
PreToolUse:
  "Bash":
    - name: block-rm
      description: Block rm commands
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^rm\\b"
      action:
        type: block_with_feedback
        feedback_message: "rm command blocked for safety"
"#;

    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();

    // Create hook event using EventFactory
    let hook_event = EventFactory::pre_tool_use()
        .session_id("test-456")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("rm -rf /")
        .build_value();

    // Run cupcake and capture output
    let output = run_cupcake_with_json(&policy_path, "PreToolUse", &hook_event);

    // Parse and verify JSON response
    let response: Value = serde_json::from_str(&output).expect("Invalid JSON output");

    // Verify it has the correct permission decision
    assert_eq!(response["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecisionReason"],
        "rm command blocked for safety"
    );
}

#[test]
fn test_pretooluse_ask_json_output() {
    let temp_dir = tempdir().unwrap();

    // Create an ask policy
    let policy = r#"
PreToolUse:
  "Bash":
    - name: ask-sudo
      description: Ask for sudo commands
      conditions:
        - type: pattern
          field: tool_input.command
          regex: "^sudo\\b"
      action:
        type: ask
        reason: "This command requires sudo. Are you sure?"
"#;

    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();

    // Create hook event using EventFactory
    let hook_event = EventFactory::pre_tool_use()
        .session_id("test-789")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("sudo apt update")
        .build_value();

    // Run cupcake and capture output
    let output = run_cupcake_with_json(&policy_path, "PreToolUse", &hook_event);

    // Parse and verify JSON response
    let response: Value = serde_json::from_str(&output).expect("Invalid JSON output");

    // Verify it has the correct permission decision
    assert_eq!(response["hookSpecificOutput"]["permissionDecision"], "ask");
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecisionReason"],
        "This command requires sudo. Are you sure?"
    );
}

#[test]
fn test_userpromptsubmit_context_injection_stdout() {
    let temp_dir = tempdir().unwrap();

    // Create a context injection policy with stdout mode
    let policy = r#"
UserPromptSubmit:
  "*":
    - name: inject-test
      description: Inject test context
      conditions:
        - type: pattern
          field: prompt
          regex: "test"
      action:
        type: inject_context
        context: "TEST CONTEXT INJECTED"
        use_stdout: true
"#;

    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();

    // Create hook event using EventFactory
    let hook_event = EventFactory::user_prompt_submit()
        .session_id("test-999")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/tmp")
        .prompt("Run a test please")
        .build_value();

    // Run cupcake and capture output
    let output = run_cupcake_with_json(&policy_path, "UserPromptSubmit", &hook_event);

    // For stdout mode, the output should be the raw context (not JSON)
    assert_eq!(output.trim(), "TEST CONTEXT INJECTED");
}

#[test]
fn test_userpromptsubmit_block_json_output() {
    let temp_dir = tempdir().unwrap();

    // Create a block policy for prompts
    let policy = r#"
UserPromptSubmit:
  "*":
    - name: block-secrets
      description: Block secrets in prompts
      conditions:
        - type: pattern
          field: prompt
          regex: "password.*=.*\\w+"
      action:
        type: block_with_feedback
        feedback_message: "Detected potential secret in prompt"
"#;

    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();

    // Create hook event using EventFactory
    let hook_event = EventFactory::user_prompt_submit()
        .session_id("test-888")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/tmp")
        .prompt("Set password = supersecret123")
        .build_value();

    // Run cupcake and capture output
    let output = run_cupcake_with_json(&policy_path, "UserPromptSubmit", &hook_event);

    // Parse and verify JSON response
    let response: Value = serde_json::from_str(&output).expect("Invalid JSON output");

    // For UserPromptSubmit blocks, it now uses decision: "block" format
    assert_eq!(response["hookSpecificOutput"]["decision"], "block");
    assert_eq!(
        response["hookSpecificOutput"]["decisionReason"],
        "Detected potential secret in prompt"
    );
}

#[test]
fn test_no_matching_policy_allows_by_default() {
    let temp_dir = tempdir().unwrap();

    // Create a policy that won't match
    let policy = r#"
PreToolUse:
  "Write":
    - name: some-write-policy
      conditions:
        - type: match
          field: tool_name
          value: "Write"
      action:
        type: allow
"#;

    let policy_path = temp_dir.path().join("policy.yaml");
    fs::write(&policy_path, policy).unwrap();

    // Create hook event using EventFactory for a different tool
    let hook_event = EventFactory::pre_tool_use()
        .session_id("test-777")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("echo hello")
        .build_value();

    // Run cupcake and capture output
    let output = run_cupcake_with_json(&policy_path, "PreToolUse", &hook_event);

    // Should get an empty JSON response (which means allow)
    let response: Value = serde_json::from_str(&output).expect("Invalid JSON output");

    // Empty response or no permissionDecision means allow
    if let Some(hook_output) = response.get("hookSpecificOutput") {
        if let Some(decision) = hook_output.get("permissionDecision") {
            assert_eq!(decision, "allow");
        }
    }
    // If hookSpecificOutput is not present, that also means allow
}

// Helper function to run cupcake with JSON input
fn run_cupcake_with_json(policy_path: &std::path::Path, event: &str, hook_json: &Value) -> String {
    let cupcake_binary = get_cupcake_binary();
    let mut cmd = Command::new(&cupcake_binary)
        .args(["run"])
        .arg("--event")
        .arg(event)
        .arg("--config")
        .arg(policy_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    // Write JSON to stdin
    let stdin = cmd.stdin.as_mut().expect("Failed to get stdin");
    stdin
        .write_all(hook_json.to_string().as_bytes())
        .expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Get output
    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");

    if !output.status.success() && output.status.code() != Some(0) {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Cupcake exited with non-zero status: {:?}", output.status);
    }

    String::from_utf8(output.stdout).expect("Invalid UTF-8 output")
}
