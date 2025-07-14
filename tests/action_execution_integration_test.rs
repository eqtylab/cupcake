#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Create a test policy file with UpdateState action
    fn create_update_state_policy(dir: &TempDir) -> PathBuf {
        let guardrails_dir = dir.path().join("guardrails");
        let policies_dir = guardrails_dir.join("policies");
        fs::create_dir_all(&policies_dir).unwrap();

        // Create root config
        let root_config = r#"
settings:
  audit_logging: false
  debug_mode: false

imports:
  - "policies/*.yaml"
"#;
        fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

        // Create policy with UpdateState action
        let policy = r#"
PostToolUse:
  "Read":
    - name: "Track file reads"
      description: "Remember which files were read"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Read"
      action:
        type: "update_state"
        event: "FileRead"
        data:
          file_path: "{{tool_input.file_path}}"
          timestamp: "{{now}}"
          tool: "{{tool_name}}"
"#;
        fs::write(policies_dir.join("track-reads.yaml"), policy).unwrap();

        guardrails_dir
    }

    /// Create a test policy file with RunCommand action
    fn create_run_command_policy(dir: &TempDir, expect_success: bool) -> PathBuf {
        let guardrails_dir = dir.path().join("guardrails");
        let policies_dir = guardrails_dir.join("policies");
        fs::create_dir_all(&policies_dir).unwrap();

        // Create root config
        let root_config = r#"
settings:
  audit_logging: false
  debug_mode: false

imports:
  - "policies/*.yaml"
"#;
        fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

        // Create policy with RunCommand action
        let command = if expect_success { "true" } else { "false" };
        let policy = format!(
            r#"
PreToolUse:
  "Bash":
    - name: "Run test command"
      description: "Execute a test command"
      conditions:
        - type: "match"
          field: "tool_name"
          value: "Bash"
      action:
        type: "run_command"
        command: "{}"
        on_failure: "block"
        on_failure_feedback: "Test command failed"
"#,
            command
        );
        fs::write(policies_dir.join("run-command.yaml"), policy).unwrap();

        guardrails_dir
    }

    #[test]
    fn test_update_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        create_update_state_policy(&temp_dir);

        // Create .cupcake/state directory
        let cupcake_dir = temp_dir.path().join(".cupcake");
        let state_dir = cupcake_dir.join("state");
        fs::create_dir_all(&state_dir).unwrap();

        // Create test hook event
        let hook_event = r#"{
            "hook_event_name": "PostToolUse",
            "session_id": "test-session-update-state",
            "transcript_path": "/tmp/transcript.jsonl",
            "tool_name": "Read",
            "tool_input": {
                "file_path": "/tmp/test.txt"
            },
            "tool_response": {
                "content": "file content"
            }
        }"#;

        // Get the path to the cupcake binary
        let cupcake_bin = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("cupcake");

        // Run cupcake with the hook event
        let output = std::process::Command::new(&cupcake_bin)
            .arg("run")
            .arg("--event")
            .arg("PostToolUse")
            .arg("--config")
            .arg("guardrails/cupcake.yaml")
            .arg("--debug")
            .current_dir(temp_dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn cupcake");

        // Write hook event to stdin
        let mut child = output;
        std::io::Write::write_all(
            child.stdin.as_mut().unwrap(),
            hook_event.as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();
        
        // Print debug info if test fails
        if output.status.code() != Some(0) {
            eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Should exit with code 0 (allow)
        assert_eq!(output.status.code(), Some(0), "Expected exit code 0");

        // Check that state file was created
        let state_file = state_dir.join("test-session-update-state.json");
        assert!(
            state_file.exists(),
            "State file should exist at {:?}",
            state_file
        );

        // Read and verify state file content
        let state_content = fs::read_to_string(&state_file).unwrap();
        let state_json: serde_json::Value = serde_json::from_str(&state_content).unwrap();

        // Verify session ID
        assert_eq!(
            state_json["session_id"].as_str().unwrap(),
            "test-session-update-state"
        );

        // Verify that our custom event was recorded
        let entries = state_json["entries"].as_array().unwrap();
        
        // Should have at least 2 entries: automatic PostToolUse tracking + our custom event
        assert!(entries.len() >= 2, "Expected at least 2 state entries, got {}", entries.len());

        // Find our custom FileRead event
        let file_read_event = entries
            .iter()
            .find(|e| {
                e["event"]["type"].as_str() == Some("custom_event") &&
                e["event"]["name"].as_str() == Some("FileRead")
            })
            .expect("FileRead event not found in state");

        // Verify the custom event data
        let custom_data = &file_read_event["event"]["data"];
        assert_eq!(
            custom_data["file_path"].as_str().unwrap(),
            "/tmp/test.txt"
        );
        assert_eq!(custom_data["tool"].as_str().unwrap(), "Read");
        assert!(custom_data["timestamp"].as_str().is_some());
    }

    #[test]
    fn test_run_command_execution_success() {
        let temp_dir = TempDir::new().unwrap();
        create_run_command_policy(&temp_dir, true); // true command always succeeds

        // Create test hook event
        let hook_event = r#"{
            "hook_event_name": "PreToolUse",
            "session_id": "test-session-run-success",
            "transcript_path": "/tmp/transcript.jsonl",
            "tool_name": "Bash",
            "tool_input": {
                "command": "echo test"
            }
        }"#;

        // Get the path to the cupcake binary
        let cupcake_bin = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("cupcake");

        // Run cupcake with the hook event
        let output = std::process::Command::new(&cupcake_bin)
            .arg("run")
            .arg("--event")
            .arg("PreToolUse")
            .arg("--config")
            .arg("guardrails/cupcake.yaml")
            .arg("--debug")
            .current_dir(temp_dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn cupcake");

        // Write hook event to stdin
        let mut child = output;
        std::io::Write::write_all(
            child.stdin.as_mut().unwrap(),
            hook_event.as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();

        // Print debug info if test fails
        if output.status.code() != Some(0) {
            eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Should exit with code 0 (allow) because the command succeeds
        assert_eq!(output.status.code(), Some(0), "Expected exit code 0");

        // Debug output should show command execution
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Executing action for policy 'Run test command'"),
            "Expected action execution message in stderr"
        );
    }

    #[test]
    fn test_run_command_execution_failure() {
        let temp_dir = TempDir::new().unwrap();
        create_run_command_policy(&temp_dir, false); // false command always fails

        // Create test hook event
        let hook_event = r#"{
            "hook_event_name": "PreToolUse",
            "session_id": "test-session-run-failure",
            "transcript_path": "/tmp/transcript.jsonl",
            "tool_name": "Bash",
            "tool_input": {
                "command": "echo test"
            }
        }"#;

        // Get the path to the cupcake binary
        let cupcake_bin = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("cupcake");

        // Run cupcake with the hook event
        let output = std::process::Command::new(&cupcake_bin)
            .arg("run")
            .arg("--event")
            .arg("PreToolUse")
            .arg("--config")
            .arg("guardrails/cupcake.yaml")
            .arg("--debug")
            .current_dir(temp_dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn cupcake");

        // Write hook event to stdin
        let mut child = output;
        std::io::Write::write_all(
            child.stdin.as_mut().unwrap(),
            hook_event.as_bytes(),
        )
        .unwrap();

        let output = child.wait_with_output().unwrap();

        // Should exit with code 2 (block) because the command fails and on_failure: block
        assert_eq!(output.status.code(), Some(2), "Expected exit code 2");

        // Should show the failure feedback
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Test command failed") || stderr.contains("Policy requires command execution"),
            "Expected failure feedback in stderr"
        );
    }
}