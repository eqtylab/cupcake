use std::io::Write;
use tempfile::tempdir;
use std::fs;
use std::process::{Command, Stdio};

#[test]
fn test_user_prompt_submit_blocking() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000
  debug: false

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file that blocks prompts with API keys
    let policy_yaml = r#"
UserPromptSubmit:
  "":  # Empty string matcher for UserPromptSubmit
    - name: "Block API keys in prompts"
      description: "Prevent accidental exposure of API keys"
      conditions:
        - type: pattern
          field: prompt
          regex: "(sk-|api_key|API_KEY)[a-zA-Z0-9_-]{16,}"
      action:
        type: block_with_feedback
        feedback_message: "API key detected in prompt! Please remove sensitive information."
        include_context: false
"#;
    fs::write(policies_dir.join("api-key-policy.yaml"), policy_yaml).unwrap();

    // Create hook event JSON with API key in prompt
    let hook_event_json = r#"
{
    "hook_event_name": "UserPromptSubmit",
    "session_id": "test-session-123",
    "transcript_path": "/tmp/test-transcript.md",
    "cwd": "/home/test/project",
    "prompt": "Here's my API key: sk-1234567890abcdef1234567890abcdef"
}
"#;

    // Run cupcake with the hook event
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut cmd = Command::new(cupcake_binary)
        .args(&["run", "--event", "UserPromptSubmit", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    // Write hook event to stdin
    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    // Get the output
    let output = cmd.wait_with_output().unwrap();

    // Should exit with code 2 (blocked)
    assert_eq!(output.status.code(), Some(2));

    // Should have feedback on stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("API key detected in prompt"));
}

#[test]
fn test_user_prompt_submit_allowed() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000
  debug: false

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file that provides feedback but doesn't block
    let policy_yaml = r#"
UserPromptSubmit:
  "":
    - name: "Check for todos"
      conditions:
        - type: pattern
          field: prompt
          regex: "TODO|FIXME"
      action:
        type: provide_feedback
        message: "Reminder: You have TODO items to address"
        include_context: false
"#;
    fs::write(policies_dir.join("todo-policy.yaml"), policy_yaml).unwrap();

    // Create hook event JSON with TODO but no sensitive data
    let hook_event_json = r#"
{
    "hook_event_name": "UserPromptSubmit",
    "session_id": "test-session-456",
    "transcript_path": "/tmp/test-transcript.md",
    "cwd": "/home/test/project",
    "prompt": "TODO: Implement the new feature"
}
"#;

    // Run cupcake with the hook event
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut cmd = Command::new(cupcake_binary)
        .args(&["run", "--event", "UserPromptSubmit", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    // Write hook event to stdin
    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    // Get the output
    let output = cmd.wait_with_output().unwrap();

    // Should exit with code 0 (allowed)
    assert_eq!(output.status.code(), Some(0));

    // Should have feedback on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Reminder: You have TODO items"));
}

#[test]
fn test_user_prompt_submit_no_match() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000
  debug: false

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file with conditions that won't match
    let policy_yaml = r#"
UserPromptSubmit:
  "":
    - name: "Check for specific pattern"
      conditions:
        - type: pattern
          field: prompt
          regex: "NEVER_MATCH_THIS_PATTERN_XYZ123"
      action:
        type: block_with_feedback
        feedback_message: "Should not see this"
        include_context: false
"#;
    fs::write(policies_dir.join("no-match-policy.yaml"), policy_yaml).unwrap();

    // Create hook event JSON with normal prompt
    let hook_event_json = r#"
{
    "hook_event_name": "UserPromptSubmit",
    "session_id": "test-session-789",
    "transcript_path": "/tmp/test-transcript.md",
    "cwd": "/home/test/project",
    "prompt": "Write a function to calculate factorial"
}
"#;

    // Run cupcake with the hook event
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut cmd = Command::new(cupcake_binary)
        .args(&["run", "--event", "UserPromptSubmit", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");

    // Write hook event to stdin
    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    // Get the output
    let output = cmd.wait_with_output().unwrap();

    // Should exit with code 0 (allowed - no policy matched)
    assert_eq!(output.status.code(), Some(0));

    // Should have no output (except possibly state tracking messages)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Debug what we got
    if !stderr.trim().is_empty() {
        eprintln!("Unexpected stderr: {}", stderr);
    }
    
    assert!(stdout.is_empty() || stdout.trim().is_empty());
    // Allow state tracking messages on stderr
    assert!(!stderr.contains("Should not see this"));
}