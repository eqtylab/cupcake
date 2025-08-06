use crate::common::event_factory::EventFactory;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

#[test]
fn test_inject_context_use_stdout_true() {
    // VERIFICATION: use_stdout: true results in raw text output to stdout
    
    let temp_dir = TempDir::new().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    fs::create_dir_all(&guardrails_dir).unwrap();
    
    // Create a policy with use_stdout: true
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: "Inject with stdout"
      conditions: []
      action:
        type: "inject_context"
        context: |
          This is injected context.
          It should appear as raw text on stdout.
        use_stdout: true
"#;
    
    fs::write(guardrails_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    let event = EventFactory::user_prompt_submit()
        .session_id("test-stdout-mode")
        .transcript_path("/tmp/test.jsonl")
        .cwd("/tmp")
        .prompt("Test message")
        .build_json();
    
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut child = Command::new(cupcake_binary)
        .args(["run", "--event", "UserPromptSubmit", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");
    
    child.stdin.as_mut().unwrap().write_all(event.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    
    // Should exit with code 0
    assert_eq!(output.status.code(), Some(0));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Stdout should contain raw text, not JSON
    assert!(stdout.contains("This is injected context."));
    assert!(stdout.contains("It should appear as raw text on stdout."));
    assert!(!stdout.contains("\"additionalContext\""), "Should not contain JSON structure");
    assert!(!stdout.contains("\"hookSpecificOutput\""), "Should not contain JSON structure");
    
    // Debug output
    if stdout.contains("\"additionalContext\"") {
        eprintln!("ERROR: Got JSON when expecting raw text");
        eprintln!("STDOUT: {}", stdout);
        eprintln!("STDERR: {}", stderr);
    }
}

#[test]
fn test_inject_context_use_stdout_false() {
    // VERIFICATION: use_stdout: false results in spec-compliant JSON response with additionalContext
    
    let temp_dir = TempDir::new().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    fs::create_dir_all(&guardrails_dir).unwrap();
    
    // Create a policy with use_stdout: false
    let policy_content = r#"
UserPromptSubmit:
  "*":
    - name: "Inject with JSON"
      conditions: []
      action:
        type: "inject_context"
        context: |
          This is JSON-injected context.
          It should appear in additionalContext field.
        use_stdout: false
"#;
    
    fs::write(guardrails_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    let event = EventFactory::user_prompt_submit()
        .session_id("test-json-mode")
        .transcript_path("/tmp/test.jsonl")
        .cwd("/tmp")
        .prompt("Test message")
        .build_json();
    
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut child = Command::new(cupcake_binary)
        .args(["run", "--event", "UserPromptSubmit", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");
    
    child.stdin.as_mut().unwrap().write_all(event.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    
    // Should exit with code 0
    assert_eq!(output.status.code(), Some(0));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Stdout should contain JSON with additionalContext
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    
    // Verify JSON structure - additionalContext is nested inside hookSpecificOutput
    let hook_output = response_json.get("hookSpecificOutput")
        .expect("Should have hookSpecificOutput field");
    let additional_context = hook_output.get("additionalContext")
        .expect("Should have additionalContext field inside hookSpecificOutput");
    
    // The context should be a string (already joined)
    let context_str = additional_context.as_str()
        .expect("additionalContext should be a string");
    
    // Verify context content
    assert!(context_str.contains("This is JSON-injected context."));
    assert!(context_str.contains("It should appear in additionalContext field."));
}

#[test]
fn test_inject_context_multiple_policies_last_wins() {
    // Per tactical advisory: When multiple InjectContext policies match,
    // the last matching policy's use_stdout preference wins
    
    let temp_dir = TempDir::new().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();
    
    // Create root config with imports
    let root_config = r#"
imports:
  - "policies/*.yaml"
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();
    
    // First policy: use_stdout: true
    let policy1 = r#"
UserPromptSubmit:
  "*":
    - name: "First inject - stdout mode"
      conditions: []
      action:
        type: "inject_context"
        context: "First context"
        use_stdout: true
"#;
    fs::write(policies_dir.join("1_stdout.yaml"), policy1).unwrap();
    
    // Second policy: use_stdout: false (this should win)
    let policy2 = r#"
UserPromptSubmit:
  "*":
    - name: "Second inject - JSON mode"
      conditions: []
      action:
        type: "inject_context"
        context: "Second context"
        use_stdout: false
"#;
    fs::write(policies_dir.join("2_json.yaml"), policy2).unwrap();
    
    let event = EventFactory::user_prompt_submit()
        .session_id("test-last-wins")
        .transcript_path("/tmp/test.jsonl")
        .cwd("/tmp")
        .prompt("Test message")
        .build_json();
    
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut child = Command::new(cupcake_binary)
        .args(["run", "--event", "UserPromptSubmit", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");
    
    child.stdin.as_mut().unwrap().write_all(event.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    
    // Should exit with code 0
    assert_eq!(output.status.code(), Some(0));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should use JSON mode (last policy's preference)
    let response_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON - last policy set use_stdout: false");
    
    // Both contexts should be included in hookSpecificOutput.additionalContext
    let hook_output = response_json.get("hookSpecificOutput")
        .expect("Should have hookSpecificOutput field");
    let additional_context = hook_output.get("additionalContext")
        .expect("Should have additionalContext field inside hookSpecificOutput");
    
    // The context should be a string with both contexts
    let context_str = additional_context.as_str()
        .expect("additionalContext should be a string");
    
    assert!(context_str.contains("First context"));
    assert!(context_str.contains("Second context"));
}

#[test]
fn test_precompact_always_uses_stdout() {
    // PreCompact is special - it always outputs to stdout regardless of use_stdout setting
    
    let temp_dir = TempDir::new().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    fs::create_dir_all(&guardrails_dir).unwrap();
    
    // Create a policy with use_stdout: false (should be ignored for PreCompact)
    let policy_content = r#"
PreCompact:
  "manual":
    - name: "PreCompact injection"
      conditions: []
      action:
        type: "inject_context"
        context: |
          # Instructions for compaction
          This should appear on stdout even with use_stdout: false
        use_stdout: false
"#;
    
    fs::write(guardrails_dir.join("cupcake.yaml"), policy_content).unwrap();
    
    let event = EventFactory::pre_compact()
        .session_id("test-precompact")
        .transcript_path("/tmp/test.jsonl")
        .cwd("/tmp")
        .trigger("manual")
        .build_json();
    
    let cupcake_binary = env!("CARGO_BIN_EXE_cupcake");
    let mut child = Command::new(cupcake_binary)
        .args(["run", "--event", "PreCompact", "--config", guardrails_dir.join("cupcake.yaml").to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake");
    
    child.stdin.as_mut().unwrap().write_all(event.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    
    // Should exit with code 0
    assert_eq!(output.status.code(), Some(0));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should contain raw text, not JSON (PreCompact special behavior)
    assert!(stdout.contains("# Instructions for compaction"));
    assert!(stdout.contains("This should appear on stdout even with use_stdout: false"));
    assert!(!stdout.contains("\"additionalContext\""), "PreCompact should not output JSON");
}