//! End-to-end parsing canary test
//!
//! This test serves as our early warning system. It runs against the current
//! codebase and must continue to pass throughout the refactoring operation.
//! If this test fails, we've broken something fundamental.

use crate::common::EventFactory;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Get the path to the cupcake binary
fn get_cupcake_binary() -> String {
    // Use cargo to build and get the binary path
    let output = Command::new("cargo")
        .args(["build", "--bin", "cupcake", "--quiet"])
        .output()
        .expect("Failed to build cupcake");

    if !output.status.success() {
        panic!("Failed to build cupcake binary");
    }

    // The binary will be in target/debug/cupcake
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/target/debug/cupcake")
}

#[test]
fn test_end_to_end_pre_tool_use_parsing() {
    // Build a PreToolUse event using our factory
    let event_json = EventFactory::pre_tool_use()
        .tool_name("Bash")
        .tool_input_command("echo 'Hello, World!'")
        .session_id("canary-test-session")
        .build_json();

    // Create a temporary policy file that allows everything
    let policy_content = r#"
settings:
  debug_mode: false

imports: []
"#;

    let mut policy_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(policy_file, "{policy_content}").expect("Failed to write policy");

    // Run cupcake with the event
    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args([
            "run",
            "--event",
            "PreToolUse",
            "--config",
            policy_file.path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake process");

    // Send the event JSON to stdin
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(event_json.as_bytes())
            .expect("Failed to write to stdin");
    }

    // Wait for the process to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for cupcake process");

    // The process should exit with code 0 (allow by default)
    assert!(
        output.status.success(),
        "Cupcake should exit successfully. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Optional: Verify we got some JSON output (even if empty)
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        // Try to parse as JSON to ensure valid response
        let _: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("Invalid JSON output: {e} - Output was: {stdout}"));
    }
}

#[test]
fn test_end_to_end_user_prompt_submit_parsing() {
    // Test another hook type to ensure our parsing is robust
    let event_json = EventFactory::user_prompt_submit()
        .prompt("Write a function to calculate factorial")
        .session_id("canary-prompt-session")
        .build_json();

    // Create a temporary policy file
    let policy_content = r#"
settings:
  debug_mode: false

imports: []
"#;

    let mut policy_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(policy_file, "{policy_content}").expect("Failed to write policy");

    // Run cupcake with the event
    let cupcake_binary = get_cupcake_binary();
    let mut child = Command::new(&cupcake_binary)
        .args([
            "run",
            "--event",
            "UserPromptSubmit",
            "--config",
            policy_file.path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake process");

    // Send the event JSON to stdin
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(event_json.as_bytes())
            .expect("Failed to write to stdin");
    }

    // Wait for the process to complete
    let output = child
        .wait_with_output()
        .expect("Failed to wait for cupcake process");

    // The process should exit with code 0
    assert!(
        output.status.success(),
        "Cupcake should exit successfully. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
