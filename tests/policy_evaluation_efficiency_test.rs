use std::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;

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
fn test_policy_evaluation_occurs_only_once_per_policy() {
    // Create a simple test policy file with 2 policies
    let mut policy_file = NamedTempFile::new().unwrap();
    writeln!(policy_file, r#"
PreToolUse:
  "Bash":
    - name: "Test Policy 1"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "echo"
      action:
        type: "provide_feedback"
        message: "Test feedback 1"
    
    - name: "Test Policy 2" 
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "echo"
      action:
        type: "provide_feedback"
        message: "Test feedback 2"
"#).unwrap();
    policy_file.flush().unwrap();

    // Create test hook event
    let hook_event = r#"{"hook_event_name": "PreToolUse", "session_id": "test", "transcript_path": "/tmp/test", "cwd": "/tmp", "tool_name": "Bash", "tool_input": {"command": "echo hello"}}"#;

    // Run cupcake with debug logging, piping hook event to stdin
    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(&["run", "--event", "PreToolUse", "--config", policy_file.path().to_str().unwrap(), "--debug"])
        .env("RUST_LOG", "debug")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Send hook event via stdin
    child.stdin.as_mut().unwrap().write_all(hook_event.as_bytes()).unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    
    let result = child.wait_with_output().unwrap();
    let debug_output = String::from_utf8_lossy(&result.stderr);

    // Count policy evaluations in debug output
    // Count "Evaluating policy conditions" which shows each condition evaluation
    let evaluation_count = debug_output.matches("Evaluating policy conditions").count();
    
    // Verify efficient policy evaluation: exactly 1 evaluation per policy
    assert_eq!(evaluation_count, 2, 
        "Expected 2 policy evaluations (1 per policy), but found {}. Debug output:\n{}", 
        evaluation_count, debug_output);
}

#[test]
fn test_single_policy_evaluation_efficiency() {
    // Create a simple test policy file with just 1 policy
    let mut policy_file = NamedTempFile::new().unwrap();
    writeln!(policy_file, r#"
PreToolUse:
  "Bash":
    - name: "Single Test Policy"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "echo"
      action:
        type: "provide_feedback"
        message: "Single policy feedback"
"#).unwrap();
    policy_file.flush().unwrap();

    // Create test hook event
    let hook_event = r#"{"hook_event_name": "PreToolUse", "session_id": "test", "transcript_path": "/tmp/test", "cwd": "/tmp", "tool_name": "Bash", "tool_input": {"command": "echo hello"}}"#;

    // Run cupcake with debug logging
    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(&["run", "--event", "PreToolUse", "--config", policy_file.path().to_str().unwrap(), "--debug"])
        .env("RUST_LOG", "debug")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Send hook event via stdin
    child.stdin.as_mut().unwrap().write_all(hook_event.as_bytes()).unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    
    let result = child.wait_with_output().unwrap();
    let debug_output = String::from_utf8_lossy(&result.stderr);

    // Count policy evaluations in debug output
    let evaluation_count = debug_output.matches("Evaluating policy conditions").count();
    
    // With 1 policy, we expect exactly 1 evaluation
    assert_eq!(evaluation_count, 1, 
        "Expected 1 policy evaluation for single policy, but found {}. Debug output:\n{}", 
        evaluation_count, debug_output);
}

#[test]
fn test_complex_policies_evaluation_efficiency() {
    // Create a complex test policy file with different conditions and actions
    let mut policy_file = NamedTempFile::new().unwrap();
    writeln!(policy_file, r#"
PreToolUse:
  "Bash":
    - name: "Complex Policy 1"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "git.*commit"
      action:
        type: "provide_feedback"
        message: "Git commit reminder"
    
    - name: "Complex Policy 2"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "rm.*-rf"
      action:
        type: "block_with_feedback"
        feedback_message: "Dangerous command blocked"
        
    - name: "Complex Policy 3"
      conditions:
        - type: "pattern"
          field: "tool_input.command"
          regex: "echo.*"
      action:
        type: "provide_feedback"
        message: "Echo command detected"

  "Edit|Write":
    - name: "File Policy 1"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "\\.rs$"
      action:
        type: "provide_feedback"
        message: "Rust file editing"
        
    - name: "File Policy 2"
      conditions:
        - type: "pattern"
          field: "tool_input.file_path"
          regex: "\\.md$"
      action:
        type: "provide_feedback"
        message: "Markdown file editing"
"#).unwrap();
    policy_file.flush().unwrap();

    // Create test hook event that matches some policies
    let hook_event = r#"{"hook_event_name": "PreToolUse", "session_id": "test", "transcript_path": "/tmp/test", "cwd": "/tmp", "tool_name": "Bash", "tool_input": {"command": "echo hello world"}}"#;

    // Run cupcake with debug logging
    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args(&["run", "--event", "PreToolUse", "--config", policy_file.path().to_str().unwrap(), "--debug"])
        .env("RUST_LOG", "debug")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Send hook event via stdin
    child.stdin.as_mut().unwrap().write_all(hook_event.as_bytes()).unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    
    let result = child.wait_with_output().unwrap();
    let debug_output = String::from_utf8_lossy(&result.stderr);

    // Count policy evaluations in debug output  
    let evaluation_count = debug_output.matches("Evaluating policy conditions").count();
    
    // Should evaluate each applicable policy exactly once
    // For Bash tool: Complex Policy 1, 2, 3 = 3 evaluations
    // Edit|Write policies don't apply to Bash tool, so not evaluated
    assert_eq!(evaluation_count, 3, 
        "Expected 3 policy evaluations (matching Bash policies only), but found {}. Debug output:\n{}", 
        evaluation_count, debug_output);
}