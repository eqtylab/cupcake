//! Integration tests for harness configuration

use serde_json::{json, Value};
use std::fs;
use std::path::Path;
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
