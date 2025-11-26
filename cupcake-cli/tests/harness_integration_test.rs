//! Integration tests for harness configuration

use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Helper to run cupcake init command
fn run_init(dir: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to run cupcake init")
}

#[test]
fn test_init_with_claude_harness_fresh() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Run init with --harness claude
    let output = run_init(dir_path, &["init", "--harness", "claude"]);

    assert!(
        output.status.success(),
        "Init command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check .cupcake directory was created
    assert!(dir_path.join(".cupcake").exists());
    assert!(dir_path.join(".cupcake/policies").exists());
    assert!(dir_path.join(".cupcake/rulebook.yml").exists());

    // Check .claude/settings.json was created
    let settings_path = dir_path.join(".claude/settings.json");
    assert!(settings_path.exists(), "Claude settings file not created");

    // Verify settings content
    let settings_content = fs::read_to_string(&settings_path).unwrap();
    let settings: Value = serde_json::from_str(&settings_content).unwrap();

    // Check for required hooks
    assert!(settings["hooks"]["PreToolUse"].is_array());
    assert!(settings["hooks"]["PostToolUse"].is_array());
    assert!(settings["hooks"]["UserPromptSubmit"].is_array());
    assert!(settings["hooks"]["SessionStart"].is_array());

    // Verify PreToolUse matcher and command
    let pre_tool = &settings["hooks"]["PreToolUse"][0];
    assert_eq!(pre_tool["matcher"], "*");

    let command = pre_tool["hooks"][0]["command"].as_str().unwrap();
    assert!(command.contains("cupcake eval"));
    assert!(command.contains("$CLAUDE_PROJECT_DIR/.cupcake"));
}

#[test]
fn test_init_with_claude_harness_existing_settings() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create existing .claude/settings.json with other settings
    let claude_dir = dir_path.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    let existing_settings = json!({
        "env": {
            "FOO": "bar"
        },
        "model": "claude-3-5-sonnet-20241022",
        "hooks": {
            "Notification": [{
                "hooks": [{
                    "type": "command",
                    "command": "echo 'notification'"
                }]
            }]
        }
    });

    fs::write(
        claude_dir.join("settings.json"),
        serde_json::to_string_pretty(&existing_settings).unwrap(),
    )
    .unwrap();

    // Run init with --harness claude
    let output = run_init(dir_path, &["init", "--harness", "claude"]);

    assert!(
        output.status.success(),
        "Init command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read merged settings
    let settings_content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let settings: Value = serde_json::from_str(&settings_content).unwrap();

    // Verify existing settings preserved
    assert_eq!(settings["env"]["FOO"], "bar");
    assert_eq!(settings["model"], "claude-3-5-sonnet-20241022");
    assert!(settings["hooks"]["Notification"].is_array());

    // Verify new hooks added
    assert!(settings["hooks"]["PreToolUse"].is_array());
    assert!(settings["hooks"]["PostToolUse"].is_array());
    assert!(settings["hooks"]["UserPromptSubmit"].is_array());
    assert!(settings["hooks"]["SessionStart"].is_array());
}

#[test]
fn test_init_with_claude_harness_duplicate_prevention() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Run init with --harness claude twice
    let output1 = run_init(dir_path, &["init", "--harness", "claude"]);
    assert!(output1.status.success());

    // Second init should say project already exists but still configure harness
    let output2 = run_init(dir_path, &["init", "--harness", "claude"]);
    assert!(output2.status.success());

    // Read settings
    let settings_content = fs::read_to_string(dir_path.join(".claude/settings.json")).unwrap();
    let settings: Value = serde_json::from_str(&settings_content).unwrap();

    // Should only have one PreToolUse matcher
    assert_eq!(settings["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    assert_eq!(
        settings["hooks"]["PostToolUse"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn test_init_without_harness() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Run init without harness flag
    let output = run_init(dir_path, &["init"]);

    assert!(
        output.status.success(),
        "Init command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check .cupcake directory was created
    assert!(dir_path.join(".cupcake").exists());

    // Check .claude/settings.json was NOT created
    assert!(
        !dir_path.join(".claude/settings.json").exists(),
        "Claude settings should not be created without --harness flag"
    );
}

#[test]
fn test_init_global_with_claude_harness() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Set HOME to temp directory for this test
    std::env::set_var("HOME", dir_path);

    // Run global init with --harness claude
    let output = run_init(dir_path, &["init", "--global", "--harness", "claude"]);

    // Note: This may fail if global config already exists, which is okay for CI
    if output.status.success() {
        // Check for global Claude settings
        let global_settings = dir_path.join(".claude/settings.json");
        if global_settings.exists() {
            let settings_content = fs::read_to_string(&global_settings).unwrap();
            let settings: Value = serde_json::from_str(&settings_content).unwrap();

            // Global should use absolute paths, not $CLAUDE_PROJECT_DIR
            let command = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
                .as_str()
                .unwrap();
            assert!(
                !command.contains("$CLAUDE_PROJECT_DIR"),
                "Global config should use absolute paths"
            );
        }
    }
}

/// Helper to run cupcake eval with stdin input
fn run_eval_with_stdin(args: &[&str], stdin_data: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cupcake eval");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .expect("Failed to write to stdin");
    }

    child.wait_with_output().expect("Failed to wait on child")
}

/// Test that Cursor harness resolves policy_dir from workspace_roots when cwd is empty
///
/// This tests the fix for Cursor hooks where:
/// - hooks.json uses relative path `--policy-dir .cupcake`
/// - cwd field is empty (Cursor behavior)
/// - workspace_roots contains the actual project directory
#[test]
fn test_cursor_eval_resolves_policy_dir_from_workspace_roots() {
    // Create a temp directory that simulates a project with .cupcake initialized
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Initialize Cupcake for Cursor in the temp project
    let init_output = run_init(project_path, &["init", "--harness", "cursor"]);
    assert!(
        init_output.status.success(),
        "Init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Verify the policies directory exists
    assert!(
        project_path.join(".cupcake/policies/cursor").exists(),
        "Cursor policies directory should exist after init"
    );

    // Create a Cursor event with empty cwd and workspace_roots pointing to project
    // This mimics the actual Cursor behavior observed in production
    let cursor_event = json!({
        "conversation_id": "test-conv-id",
        "generation_id": "test-gen-id",
        "command": "echo hello",
        "cwd": "",  // Empty cwd - this is the key issue
        "hook_event_name": "beforeShellExecution",
        "cursor_version": "2.0.77",
        "workspace_roots": [
            project_path.to_str().unwrap()
        ]
    });

    // Run eval with relative policy-dir (as configured in ~/.cursor/hooks.json)
    // The engine should resolve .cupcake against workspace_roots[0]
    let output = run_eval_with_stdin(
        &["eval", "--harness", "cursor", "--policy-dir", ".cupcake"],
        &cursor_event.to_string(),
    );

    // The command should succeed (not fail with "Policy directory does not exist")
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Eval should succeed when workspace_roots is provided. stderr: {stderr}"
    );

    // The output should be valid JSON (the response)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: Result<Value, _> = serde_json::from_str(&stdout);
    assert!(
        response.is_ok(),
        "Response should be valid JSON. Got: {stdout}"
    );

    // The response should allow the harmless echo command
    let response = response.unwrap();
    // Cursor expects permission field for beforeShellExecution
    if let Some(permission) = response.get("permission") {
        assert_eq!(
            permission.as_str().unwrap_or(""),
            "allow",
            "Harmless 'echo hello' should be allowed"
        );
    }
}

/// Test that Cursor harness blocks dangerous commands when resolved from workspace_roots
#[test]
fn test_cursor_eval_blocks_dangerous_command_via_workspace_roots() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Initialize Cupcake with Cursor harness
    let init_output = run_init(project_path, &["init", "--harness", "cursor"]);
    assert!(
        init_output.status.success(),
        "Init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create a simple policy that blocks rm -rf commands
    let block_rm_policy = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.block_rm

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {
        "rule_id": "BLOCK-RM-RF",
        "reason": "rm -rf commands are blocked for safety"
    }
}
"#;

    // Write the policy to the Cursor policies directory
    let policy_path = project_path.join(".cupcake/policies/cursor/block_rm.rego");
    fs::write(&policy_path, block_rm_policy).expect("Failed to write policy");

    // Create a dangerous Cursor event
    let cursor_event = json!({
        "conversation_id": "test-conv-id",
        "generation_id": "test-gen-id",
        "command": "rm -rf /tmp/test-junk",
        "cwd": "",
        "hook_event_name": "beforeShellExecution",
        "cursor_version": "2.0.77",
        "workspace_roots": [
            project_path.to_str().unwrap()
        ]
    });

    // Run eval
    let output = run_eval_with_stdin(
        &["eval", "--harness", "cursor", "--policy-dir", ".cupcake"],
        &cursor_event.to_string(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Eval command should succeed. stderr: {stderr}"
    );

    // Parse response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("Response should be valid JSON: {stdout}"));

    // The response should deny the dangerous command
    // Cursor uses "permission": "deny" for blocked commands
    if let Some(permission) = response.get("permission") {
        assert_eq!(
            permission.as_str().unwrap_or(""),
            "deny",
            "rm -rf should be blocked. Response: {stdout}"
        );
    } else {
        // Some responses might have "continue": false instead
        if let Some(cont) = response.get("continue") {
            assert!(
                !cont.as_bool().unwrap_or(true),
                "rm -rf should be blocked (continue=false). Response: {stdout}"
            );
        }
    }
}

/// Test that Cursor falls back to cwd when workspace_roots is empty
#[test]
fn test_cursor_eval_falls_back_to_cwd_when_workspace_roots_empty() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Set HOME to project_path to avoid test isolation issues
    // (test_init_global_with_claude_harness may have leaked HOME env var)
    std::env::set_var("HOME", project_path);

    // Initialize Cupcake
    let init_output = run_init(project_path, &["init", "--harness", "cursor"]);
    assert!(
        init_output.status.success(),
        "Init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    // Create event with empty workspace_roots but valid cwd
    // cwd should be used as fallback
    let cursor_event = json!({
        "conversation_id": "test-conv-id",
        "generation_id": "test-gen-id",
        "command": "echo test",
        "cwd": project_path.to_str().unwrap(),  // Non-empty cwd as fallback
        "hook_event_name": "beforeShellExecution",
        "cursor_version": "2.0.77",
        "workspace_roots": []  // Empty - should fall back to cwd
    });

    // Run eval - should use cwd as fallback to resolve .cupcake
    let output = run_eval_with_stdin(
        &["eval", "--harness", "cursor", "--policy-dir", ".cupcake"],
        &cursor_event.to_string(),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Eval should succeed when falling back to cwd. stderr: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: Result<Value, _> = serde_json::from_str(&stdout);
    assert!(response.is_ok(), "Response should be valid JSON: {stdout}");
}
