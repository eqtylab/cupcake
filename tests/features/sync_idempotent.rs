use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_sync_do_no_harm() {
    // CRITICAL TEST: Verify sync preserves user settings and is idempotent

    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir(&claude_dir).unwrap();

    // Create a pre-existing settings.local.json with:
    // 1. User-defined hooks (without managed_by marker)
    // 2. Other settings that must be preserved
    let user_settings = json!({
        "model": "claude-3-opus-20240229",  // Must be preserved
        "permissions": {
            "api_access": true  // Must be preserved
        },
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo 'User hook for Bash'",
                            "timeout": 3
                        }
                    ]
                }
            ],
            "UserPromptSubmit": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo 'User hook for prompts'",
                            "timeout": 2
                        }
                    ]
                }
            ]
        },
        "custom_field": "should_be_preserved"  // Must be preserved
    });

    let settings_path = claude_dir.join("settings.local.json");
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&user_settings).unwrap(),
    )
    .unwrap();

    // First sync
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .args(["sync"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync");

    assert!(output.status.success(), "First sync should succeed");

    // Read the result
    let settings_after_first = fs::read_to_string(&settings_path).unwrap();
    let json_after_first: Value = serde_json::from_str(&settings_after_first).unwrap();

    // CRITICAL ASSERTIONS - Do No Harm

    // 1. User's original hooks are still present
    let pretooluse_hooks = json_after_first["hooks"]["PreToolUse"].as_array().unwrap();
    let user_bash_hook_exists = pretooluse_hooks.iter().any(|hook| {
        hook.get("matcher").and_then(|v| v.as_str()) == Some("Bash")
            && hook.get("managed_by").is_none() // User hook has no managed_by
    });
    assert!(
        user_bash_hook_exists,
        "User's Bash hook should be preserved"
    );

    let userprompt_hooks = json_after_first["hooks"]["UserPromptSubmit"]
        .as_array()
        .unwrap();
    let user_prompt_hook_exists = userprompt_hooks.iter().any(|hook| {
        hook["hooks"][0]["command"].as_str() == Some("echo 'User hook for prompts'")
            && hook.get("managed_by").is_none()
    });
    assert!(
        user_prompt_hook_exists,
        "User's prompt hook should be preserved"
    );

    // 2. Other settings are preserved
    assert_eq!(
        json_after_first["model"], "claude-3-opus-20240229",
        "Model setting should be preserved"
    );
    assert_eq!(
        json_after_first["permissions"]["api_access"], true,
        "Permissions should be preserved"
    );
    assert_eq!(
        json_after_first["custom_field"], "should_be_preserved",
        "Custom fields should be preserved"
    );

    // 3. Cupcake hooks are present
    let cupcake_pretool_exists = pretooluse_hooks
        .iter()
        .any(|hook| hook.get("managed_by").and_then(|v| v.as_str()) == Some("cupcake"));
    assert!(
        cupcake_pretool_exists,
        "Cupcake PreToolUse hooks should be added"
    );

    let cupcake_prompt_exists = userprompt_hooks
        .iter()
        .any(|hook| hook.get("managed_by").and_then(|v| v.as_str()) == Some("cupcake"));
    assert!(
        cupcake_prompt_exists,
        "Cupcake UserPromptSubmit hooks should be added"
    );

    // Second sync - IDEMPOTENCY TEST
    let output2 = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .args(["sync"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync second time");

    assert!(output2.status.success(), "Second sync should succeed");

    // Read the result after second sync
    let settings_after_second = fs::read_to_string(&settings_path).unwrap();

    // CRITICAL: File should be identical after second sync
    assert_eq!(
        settings_after_first, settings_after_second,
        "Second sync should produce identical file (idempotency)"
    );
}

#[test]
fn test_sync_remove_then_append() {
    // Test the surgical removal and re-append behavior

    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir(&claude_dir).unwrap();

    // Create settings with existing cupcake hooks (old version)
    let old_settings = json!({
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "*",
                    "managed_by": "cupcake",
                    "hooks": [{
                        "type": "command",
                        "command": "cupcake run --event PreToolUse --old-flag",  // Old command
                        "timeout": 5
                    }]
                },
                {
                    "matcher": "Bash",
                    // No managed_by - this is a user hook
                    "hooks": [{
                        "type": "command",
                        "command": "user-security-check",
                        "timeout": 2
                    }]
                }
            ]
        }
    });

    let settings_path = claude_dir.join("settings.local.json");
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&old_settings).unwrap(),
    )
    .unwrap();

    // Run sync
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .args(["sync"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync");

    assert!(output.status.success(), "Sync should succeed");

    // Read result
    let settings_after: Value =
        serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();

    let pretooluse_hooks = settings_after["hooks"]["PreToolUse"].as_array().unwrap();

    // Old cupcake hook should be removed
    let old_cupcake_exists = pretooluse_hooks.iter().any(|hook| {
        hook["hooks"][0]["command"]
            .as_str()
            .map(|cmd| cmd.contains("--old-flag"))
            .unwrap_or(false)
    });
    assert!(!old_cupcake_exists, "Old cupcake hook should be removed");

    // User hook should still exist
    let user_hook_exists = pretooluse_hooks.iter().any(|hook| {
        hook.get("managed_by").is_none()
            && hook["hooks"][0]["command"].as_str() == Some("user-security-check")
    });
    assert!(user_hook_exists, "User hook should be preserved");

    // New cupcake hook should exist
    let new_cupcake_exists = pretooluse_hooks.iter().any(|hook| {
        hook.get("managed_by").and_then(|v| v.as_str()) == Some("cupcake")
            && !hook["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .contains("--old-flag")
    });
    assert!(new_cupcake_exists, "New cupcake hook should be added");
}

#[test]
fn test_sync_intelligent_matchers() {
    // Test that PreCompact and SessionStart get multiple matchers

    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir(&claude_dir).unwrap();

    // Empty settings
    let settings_path = claude_dir.join("settings.local.json");
    fs::write(&settings_path, "{}").unwrap();

    // Run sync
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .args(["sync"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run cupcake sync");

    assert!(output.status.success(), "Sync should succeed");

    // Read result
    let settings: Value =
        serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();

    // Check PreCompact has manual and auto matchers
    let precompact = settings["hooks"]["PreCompact"].as_array().unwrap();
    assert_eq!(precompact.len(), 2, "PreCompact should have 2 entries");

    let precompact_matchers: Vec<&str> = precompact
        .iter()
        .filter_map(|h| h.get("matcher").and_then(|v| v.as_str()))
        .collect();
    assert!(precompact_matchers.contains(&"manual"));
    assert!(precompact_matchers.contains(&"auto"));

    // Check SessionStart has startup, resume, and clear matchers
    let session_start = settings["hooks"]["SessionStart"].as_array().unwrap();
    assert_eq!(session_start.len(), 3, "SessionStart should have 3 entries");

    let session_matchers: Vec<&str> = session_start
        .iter()
        .filter_map(|h| h.get("matcher").and_then(|v| v.as_str()))
        .collect();
    assert!(session_matchers.contains(&"startup"));
    assert!(session_matchers.contains(&"resume"));
    assert!(session_matchers.contains(&"clear"));

    // All should have managed_by marker
    for hook in precompact.iter().chain(session_start.iter()) {
        assert_eq!(
            hook.get("managed_by").and_then(|v| v.as_str()),
            Some("cupcake")
        );
    }
}
