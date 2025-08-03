use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[test]
fn test_minimal_session_start_yaml() {
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

    // Create the MINIMAL SessionStart policy - just inject a simple string
    let minimal_policy_yaml = r#"
SessionStart:
  "*":
    - name: "Welcome message"
      conditions: []
      action:
        type: inject_context  
        context: "Welcome! Remember to run tests before committing."
"#;
    fs::write(
        policies_dir.join("minimal-session-start.yaml"),
        minimal_policy_yaml,
    )
    .unwrap();

    // Create hook event JSON for startup
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

    // Verify the output
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should inject the context via stdout
    assert!(stdout.contains("Welcome! Remember to run tests before committing."));
}

#[test]
fn test_session_start_source_specific_yaml() {
    // Test that SessionStart works with basic conditions - simplified version
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Simple SessionStart policy that always matches
    let always_match_yaml = r#"
SessionStart:
  "*":
    - name: "Always show guidance" 
      conditions: []
      action:
        type: inject_context
        context: "SessionStart guidance works!"
"#;
    fs::write(policies_dir.join("always-match.yaml"), always_match_yaml).unwrap();

    let session_event = r#"
{
    "hook_event_name": "SessionStart",
    "session_id": "test-session",
    "transcript_path": "/tmp/transcript.jsonl",
    "cwd": "/home/test",
    "source": "startup"
}
"#;

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
        .write_all(session_event.as_bytes())
        .unwrap();
    let output = cmd.wait_with_output().expect("Failed to wait for cupcake");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain the guidance message
    assert!(stdout.contains("SessionStart guidance works!"));
}

#[test]
fn test_session_start_with_builder_pattern() {
    // Demonstrate that we can now use the minimal YAML thanks to our builder pattern
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    let root_config = r#"
settings:
  timeout_ms: 5000

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Ultra-minimal YAML - only specify what's needed, defaults handle the rest
    let ultra_minimal_yaml = r#"
SessionStart:
  "*":
    - name: "Quick tip"
      conditions: []
      action:
        type: inject_context
        context: "💡 Tip: Use 'just test' for fast testing"
        # use_stdout defaults to true
        # suppress_output defaults to false  
"#;
    fs::write(policies_dir.join("ultra-minimal.yaml"), ultra_minimal_yaml).unwrap();

    let hook_event_json = r#"
{
    "hook_event_name": "SessionStart",
    "session_id": "test-session",
    "transcript_path": "/tmp/transcript.jsonl", 
    "cwd": "/home/test",
    "source": "startup"
}
"#;

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
    assert!(stdout.contains("💡 Tip: Use 'just test' for fast testing"));
}
