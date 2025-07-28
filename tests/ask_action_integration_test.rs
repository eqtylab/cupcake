use std::process::Command;
use std::io::Write;
use serde_json::Value;
use tempfile::NamedTempFile;

/// Test that Ask action produces correct JSON output with "permissionDecision":"ask"
#[test]
fn test_ask_action_json_output() {
    // Create a test policy with Ask action
    let policy_yaml = r#"
# Test policy for Ask action integration
PreToolUse:
  "Bash":
    - name: "ask_confirmation_policy"
      conditions: []
      action:
        type: "ask"
        reason: "Please confirm this Bash command execution"
"#;

    // Write policy to temporary file
    let mut policy_file = NamedTempFile::new().expect("Failed to create temp policy file");
    policy_file.write_all(policy_yaml.as_bytes()).expect("Failed to write policy");
    let policy_path = policy_file.path().to_str().unwrap();

    // Create test hook event JSON for PreToolUse with Bash tool
    let hook_event_json = r#"
{
    "hook_event_name": "PreToolUse",
    "session_id": "test-session-ask",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/tmp/test",
    "tool_name": "Bash",
    "tool_input": {
        "command": "echo 'test command'",
        "description": "Test bash command"
    }
}
"#;

    // Execute cupcake run command with the test policy and hook event
    let mut child = Command::new("target/debug/cupcake")
        .args(&["run", "--event", "PreToolUse", "--config", policy_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake command");

    // Write hook event to stdin
    child.stdin.as_mut().unwrap()
        .write_all(hook_event_json.as_bytes())
        .expect("Failed to write to stdin");
    
    // Wait for process to complete and get output
    let output = child.wait_with_output().expect("Failed to wait for cupcake command");

    // The command should succeed (exit code 2 for ask in legacy, but we're using JSON now)
    println!("Exit status: {}", output.status);
    println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse the JSON output
    let json_output: Value = serde_json::from_str(&stdout_str)
        .expect("Failed to parse JSON output from cupcake");

    // Verify the JSON structure contains permissionDecision: "ask"
    assert!(json_output.is_object(), "Output should be a JSON object");
    
    if let Some(hook_specific_output) = json_output.get("hookSpecificOutput") {
        if let Some(permission_decision) = hook_specific_output.get("permissionDecision") {
            assert_eq!(
                permission_decision.as_str(),
                Some("ask"),
                "permissionDecision should be 'ask'"
            );
        } else {
            panic!("hookSpecificOutput should contain permissionDecision field");
        }
        
        // Verify the hook event name is correct
        if let Some(hook_event_name) = hook_specific_output.get("hookEventName") {
            assert_eq!(
                hook_event_name.as_str(),
                Some("PreToolUse"),
                "hookEventName should be 'PreToolUse'"
            );
        }
        
        // Verify the reason is included
        if let Some(reason) = hook_specific_output.get("permissionDecisionReason") {
            assert_eq!(
                reason.as_str(),
                Some("Please confirm this Bash command execution"),
                "permissionDecisionReason should contain our reason"
            );
        } else {
            panic!("hookSpecificOutput should contain permissionDecisionReason field");
        }
    } else {
        panic!("JSON output should contain hookSpecificOutput field");
    }
    
    // For PreToolUse Ask decisions, other fields may not be present, which is correct
    // The reason should be included in the response
    let reason_found = stdout_str.contains("Please confirm this Bash command execution");
    assert!(reason_found, "JSON output should contain the ask reason message");
}

/// Test Ask action with template substitution
#[test]
fn test_ask_action_with_template_substitution() {
    // Create a test policy with Ask action using templates
    let policy_yaml = r#"
PreToolUse:
  ".*":
    - name: "ask_with_template_policy"
      conditions: []
      action:
        type: "ask"
        reason: "Please confirm execution of {{tool_name}} with command: {{tool_input.command}}"
"#;

    // Write policy to temporary file
    let mut policy_file = NamedTempFile::new().expect("Failed to create temp policy file");
    policy_file.write_all(policy_yaml.as_bytes()).expect("Failed to write policy");
    let policy_path = policy_file.path().to_str().unwrap();

    // Create test hook event JSON
    let hook_event_json = r#"
{
    "hook_event_name": "PreToolUse",
    "session_id": "test-session-template",
    "transcript_path": "/tmp/transcript.jsonl", 
    "cwd": "/tmp/test",
    "tool_name": "Edit",
    "tool_input": {
        "file_path": "src/main.rs",
        "command": "Add logging functionality"
    }
}
"#;

    // Execute cupcake run command
    let mut child = Command::new("target/debug/cupcake")
        .args(&["run", "--event", "PreToolUse", "--config", policy_path])
        .stdin(std::process::Stdio::piped()) 
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake command");

    // Write hook event to stdin
    child.stdin.as_mut().unwrap()
        .write_all(hook_event_json.as_bytes())
        .expect("Failed to write to stdin");
    
    // Wait for process to complete and get output
    let output = child.wait_with_output().expect("Failed to wait for cupcake command");

    println!("Exit status: {}", output.status);
    println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));

    let stdout_str = String::from_utf8_lossy(&output.stdout);

    // Parse JSON and verify template substitution occurred
    let json_output: Value = serde_json::from_str(&stdout_str)
        .expect("Failed to parse JSON output");

    // Check that templates were substituted in the reason
    assert!(stdout_str.contains("Edit"), "Should contain tool name from template");
    assert!(stdout_str.contains("Add logging functionality"), "Should contain command from template");
    
    // Verify it's still an ask decision
    if let Some(hook_specific_output) = json_output.get("hookSpecificOutput") {
        if let Some(permission_decision) = hook_specific_output.get("permissionDecision") {
            assert_eq!(permission_decision.as_str(), Some("ask"));
        }
    }
}