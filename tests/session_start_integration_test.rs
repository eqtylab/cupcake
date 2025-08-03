use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn test_session_start_hook_support() {
    // Create a temporary directory for the test
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file with SessionStart context injection
    let policy_yaml = r#"
SessionStart:
  "*":
    - name: "Session startup context"
      conditions: []
      action:
        type: inject_context
        context: |
          Welcome to the project!
          Key commands: cargo test, cargo fmt
        use_stdout: true
"#;
    fs::write(policies_dir.join("session-start-policy.yaml"), policy_yaml).unwrap();

    // Create hook event JSON
    let hook_event_json = r#"
{
    "hook_event_name": "SessionStart",
    "session_id": "test-session",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/home/test",
    "source": "startup"
}
"#;

    // Build the cupcake binary
    Command::new("cargo")
        .args(["build", "--bin", "cupcake"])
        .output()
        .expect("Failed to build cupcake");

    // Run cupcake with the test configuration
    let mut cmd = Command::new("./target/debug/cupcake")
        .args([
            "run",
            "--event",
            "-",
            "--config",
            guardrails_dir.join("cupcake.yaml").to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake");

    // Write the hook event to stdin
    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    // Get the output
    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");

    // Verify the output
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Welcome to the project!"));
    assert!(stdout.contains("Key commands: cargo test, cargo fmt"));
}

#[test]
fn test_session_start_source_matching() {
    // Test that SessionStart properly matches different source types
    let sources = vec!["startup", "resume", "clear"];

    for source in sources {
        let temp_dir = tempdir().unwrap();
        let guardrails_dir = temp_dir.path().join("guardrails");
        let policies_dir = guardrails_dir.join("policies");
        fs::create_dir_all(&policies_dir).unwrap();

        // Create root config
        let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
        fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

        // Create policy file
        let policy_yaml = r#"
SessionStart:
  "*":
    - name: "Session context"
      conditions: []
      action:
        type: provide_feedback
        message: "Session started"
"#;
        fs::write(policies_dir.join("session-policy.yaml"), policy_yaml).unwrap();

        // Create hook event JSON
        let hook_event_json = format!(
            r#"
{{
    "hook_event_name": "SessionStart",
    "session_id": "test-session",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/home/test",
    "source": "{source}"
}}
"#
        );

        // Run cupcake
        let mut cmd = Command::new("./target/debug/cupcake")
            .args([
                "run",
                "--event",
                "-",
                "--config",
                guardrails_dir.join("cupcake.yaml").to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start cupcake");

        cmd.stdin
            .as_mut()
            .unwrap()
            .write_all(hook_event_json.as_bytes())
            .unwrap();

        let output = cmd.wait_with_output().expect("Failed to wait for cupcake");
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Session started"));
    }
}

#[test]
fn test_session_start_with_block() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file that blocks clear sessions
    let policy_yaml = r#"
SessionStart:
  "*":
    - name: "Block clear sessions"
      conditions:
        - type: check
          spec:
            mode: array
            command: ["test", "-f", "/tmp/clear-blocked"]
          expect_success: true
      action:
        type: block_with_feedback
        feedback_message: "Clear sessions are not allowed"
"#;
    fs::write(policies_dir.join("block-policy.yaml"), policy_yaml).unwrap();

    // Create marker file to trigger the block
    fs::write("/tmp/clear-blocked", "marker").unwrap();

    // Create hook event JSON
    let hook_event_json = r#"
{
    "hook_event_name": "SessionStart",
    "session_id": "test-session",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/home/test",
    "source": "clear"
}
"#;

    // Run cupcake
    let mut cmd = Command::new("./target/debug/cupcake")
        .args([
            "run",
            "--event",
            "-",
            "--config",
            guardrails_dir.join("cupcake.yaml").to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start cupcake");

    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(hook_event_json.as_bytes())
        .unwrap();

    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");
    assert!(output.status.success());

    // Should output JSON response blocking the session
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(response["continue"], false);
    assert_eq!(response["stopReason"], "Clear sessions are not allowed");

    // Clean up
    fs::remove_file("/tmp/clear-blocked").ok();
}
