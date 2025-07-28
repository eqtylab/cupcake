#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;


    /// Create a test policy file with RunCommand action
    fn create_run_command_policy(dir: &TempDir, expect_success: bool) -> PathBuf {
        let guardrails_dir = dir.path().join("guardrails");
        let policies_dir = guardrails_dir.join("policies");
        fs::create_dir_all(&policies_dir).unwrap();

        // Create root config
        let root_config = r#"
settings:
  debug_mode: false

imports:
  - "policies/*.yaml"
"#;
        fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

        // Create policy with RunCommand action using secure array format
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
        spec:
          mode: "array"
          command: ["{}"]
        on_failure: "block"
        on_failure_feedback: "Test command failed"
"#,
            command
        );
        fs::write(policies_dir.join("run-command.yaml"), policy).unwrap();

        guardrails_dir
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
            "cwd": "/tmp",
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
            "cwd": "/tmp",
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

        // Should exit with code 0 (success) but provide JSON response for block
        assert_eq!(output.status.code(), Some(0), "Expected exit code 0 with JSON response");

        // Should provide JSON response with block decision
        let stdout = String::from_utf8_lossy(&output.stdout);
        let response_json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("stdout was not valid JSON");

        // Should be a block decision in JSON format
        let decision = &response_json["hookSpecificOutput"]["permissionDecision"];
        assert_eq!(decision, "deny", "JSON response should have permissionDecision: deny");

        // Should contain the failure feedback
        let reason = &response_json["hookSpecificOutput"]["permissionDecisionReason"];
        assert!(
            reason.as_str().unwrap().contains("Test command failed"),
            "JSON should contain the failure feedback message"
        );
    }
}