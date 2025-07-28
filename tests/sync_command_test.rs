/// Integration tests for the sync command
/// These tests verify that the sync command generates correct JSON structure for Claude Code July 20

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use serde_json::Value;

#[test]
fn test_sync_generates_correct_json_structure() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run sync command in the temporary directory
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("sync")
        .arg("--settings-path")
        .arg(temp_dir.path().join(".claude/settings.local.json").to_str().unwrap())
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync");

    // Should succeed
    assert_eq!(output.status.code(), Some(0), "Sync command should succeed");

    // Read the generated settings file
    let settings_path = temp_dir.path().join(".claude/settings.local.json");
    assert!(settings_path.exists(), "Settings file should be created");

    let settings_content = fs::read_to_string(&settings_path)
        .expect("Should be able to read settings file");
    
    let settings: Value = serde_json::from_str(&settings_content)
        .expect("Settings file should contain valid JSON");

    // Verify the JSON structure matches July 20 specification
    assert!(settings.get("hooks").is_some(), "Should have hooks object");
    
    let hooks = settings["hooks"].as_object().expect("hooks should be an object");
    
    // Test PreToolUse structure
    let pre_tool_use = hooks.get("PreToolUse").expect("Should have PreToolUse hook");
    let pre_tool_use_array = pre_tool_use.as_array().expect("PreToolUse should be an array");
    assert_eq!(pre_tool_use_array.len(), 1, "PreToolUse should have one matcher");
    
    let matcher = &pre_tool_use_array[0];
    assert_eq!(matcher["matcher"], "*", "Should have wildcard matcher");
    
    let hooks_array = matcher["hooks"].as_array().expect("Should have hooks array");
    assert_eq!(hooks_array.len(), 1, "Should have one hook command");
    
    let hook_cmd = &hooks_array[0];
    assert_eq!(hook_cmd["type"], "command", "Should be command type");
    assert_eq!(hook_cmd["command"], "cupcake run --event PreToolUse", "Should have correct command format");
    assert_eq!(hook_cmd["timeout"], 5, "Should have timeout in seconds");

    // Test UserPromptSubmit structure (no matcher)
    let user_prompt_submit = hooks.get("UserPromptSubmit").expect("Should have UserPromptSubmit hook");
    let user_prompt_submit_array = user_prompt_submit.as_array().expect("UserPromptSubmit should be an array");
    assert_eq!(user_prompt_submit_array.len(), 1, "UserPromptSubmit should have one entry");
    
    let ups_entry = &user_prompt_submit_array[0];
    assert!(ups_entry.get("matcher").is_none(), "UserPromptSubmit should not have matcher");
    
    let ups_hooks = ups_entry["hooks"].as_array().expect("Should have hooks array");
    let ups_hook_cmd = &ups_hooks[0];
    assert_eq!(ups_hook_cmd["command"], "cupcake run --event UserPromptSubmit", "Should have correct UPS command");
    assert_eq!(ups_hook_cmd["timeout"], 1, "Should have timeout in seconds");

    // Test all expected hook events exist
    let expected_events = ["PreToolUse", "PostToolUse", "UserPromptSubmit", "Notification", "Stop", "SubagentStop", "PreCompact"];
    for event in &expected_events {
        assert!(hooks.contains_key(*event), "Should have {} hook", event);
        
        let event_hooks = hooks[*event].as_array().expect(&format!("{} should be an array", event));
        assert!(!event_hooks.is_empty(), "{} should not be empty", event);
        
        // Verify each entry has correct structure
        for entry in event_hooks {
            let entry_hooks = entry["hooks"].as_array().expect("Should have hooks array");
            for hook_cmd in entry_hooks {
                assert_eq!(hook_cmd["type"], "command", "All hooks should be command type");
                assert!(hook_cmd["command"].as_str().unwrap().starts_with("cupcake run --event"), 
                       "All commands should use --event format");
                assert!(hook_cmd["timeout"].is_number(), "All hooks should have numeric timeout");
            }
        }
    }
}

#[test] 
fn test_sync_preserves_existing_user_settings() {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    
    let settings_path = claude_dir.join("settings.local.json");
    
    // Create existing settings with user hooks and other settings
    let existing_settings = serde_json::json!({
        "model": "claude-3-5-sonnet-20241022",
        "customInstructions": "Always be helpful",
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Write",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo 'User hook for Write'",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "PostToolUse": [
                {
                    "matcher": "Read",
                    "hooks": [
                        {
                            "type": "command", 
                            "command": "/path/to/user/script.sh"
                        }
                    ]
                }
            ]
        }
    });
    
    fs::write(&settings_path, serde_json::to_string_pretty(&existing_settings).unwrap())
        .expect("Should write existing settings");

    // Run sync command
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("sync")
        .arg("--settings-path")
        .arg(settings_path.to_str().unwrap())
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync");

    // Should warn about existing hooks but not fail
    assert_eq!(output.status.code(), Some(0), "Sync should succeed even with existing hooks");
    
    // Verify output contains warning
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"), "Should warn about existing hooks");

    // Read updated settings
    let updated_content = fs::read_to_string(&settings_path)
        .expect("Should read updated settings");
    let updated_settings: Value = serde_json::from_str(&updated_content)
        .expect("Updated settings should be valid JSON");

    // Verify user settings are preserved
    assert_eq!(updated_settings["model"], "claude-3-5-sonnet-20241022", "Should preserve model setting");
    assert_eq!(updated_settings["customInstructions"], "Always be helpful", "Should preserve custom instructions");
    
    // Verify existing hooks are preserved (not overwritten without --force)
    let hooks = updated_settings["hooks"].as_object().expect("Should have hooks");
    let pre_tool_use = hooks["PreToolUse"].as_array().expect("PreToolUse should be array");
    
    // Should still have the user's Write hook
    let user_hook_exists = pre_tool_use.iter().any(|entry| {
        entry.get("matcher").and_then(|m| m.as_str()) == Some("Write")
    });
    assert!(user_hook_exists, "Should preserve user's existing Write hook");
}

#[test]
fn test_sync_force_mode_overwrites_hooks() {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    
    let settings_path = claude_dir.join("settings.local.json");
    
    // Create existing settings with conflicting hooks
    let existing_settings = serde_json::json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "old-command-to-be-replaced"
                        }
                    ]
                }
            ]
        }
    });
    
    fs::write(&settings_path, serde_json::to_string_pretty(&existing_settings).unwrap())
        .expect("Should write existing settings");

    // Run sync command with --force
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("sync")
        .arg("--force")
        .arg("--settings-path")
        .arg(settings_path.to_str().unwrap())
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync --force");

    // Should succeed
    assert_eq!(output.status.code(), Some(0), "Sync --force should succeed");

    // Read updated settings
    let updated_content = fs::read_to_string(&settings_path)
        .expect("Should read updated settings");
    let updated_settings: Value = serde_json::from_str(&updated_content)
        .expect("Updated settings should be valid JSON");

    // Verify Cupcake hooks were installed (replacing old ones)
    let hooks = updated_settings["hooks"].as_object().expect("Should have hooks");
    let pre_tool_use = hooks["PreToolUse"].as_array().expect("PreToolUse should be array");
    
    // Should have Cupcake's hook command now
    let cupcake_hook_exists = pre_tool_use.iter().any(|entry| {
        entry.get("hooks").and_then(|h| h.as_array()).map(|hooks_array| {
            hooks_array.iter().any(|cmd| {
                cmd.get("command").and_then(|c| c.as_str()) == Some("cupcake run --event PreToolUse")
            })
        }).unwrap_or(false)
    });
    assert!(cupcake_hook_exists, "Should have Cupcake's PreToolUse hook after --force");
}

#[test]
fn test_sync_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run sync command with --dry-run
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("sync")
        .arg("--dry-run")
        .arg("--settings-path")
        .arg(temp_dir.path().join(".claude/settings.local.json").to_str().unwrap())
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync --dry-run");

    // Should succeed
    assert_eq!(output.status.code(), Some(0), "Sync --dry-run should succeed");

    // Should not create settings file
    let settings_path = temp_dir.path().join(".claude/settings.local.json");
    assert!(!settings_path.exists(), "Settings file should not be created in dry-run mode");

    // Should show what would be written
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry run mode"), "Should mention dry run mode");
    assert!(stdout.contains("PreToolUse"), "Should show hook configuration");
}