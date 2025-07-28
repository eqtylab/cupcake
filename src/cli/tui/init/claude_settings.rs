//! Claude Code settings.local.json updater for TUI

use std::fs;
use std::path::PathBuf;
use serde_json::json;
use crate::Result;
use std::io::Write;

/// Update Claude Code settings with hook configuration
pub fn update_claude_settings() -> Result<()> {
    let settings_path = get_claude_settings_path();
    
    // Create modern hook configuration matching July 20 updates
    // Uses the correct nested array format per Claude Code spec
    let hooks = json!({
        "PreToolUse": [
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PreToolUse",
                        "timeout": 5  // seconds per Claude Code spec
                    }
                ]
            }
        ],
        "PostToolUse": [
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PostToolUse",
                        "timeout": 2  // seconds per Claude Code spec
                    }
                ]
            }
        ],
        "UserPromptSubmit": [
            {
                // No matcher for non-tool events
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event UserPromptSubmit",
                        "timeout": 1  // seconds per Claude Code spec
                    }
                ]
            }
        ],
        "Notification": [
            {
                // No matcher for non-tool events
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event Notification",
                        "timeout": 1  // seconds per Claude Code spec
                    }
                ]
            }
        ],
        "Stop": [
            {
                // No matcher for non-tool events
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event Stop",
                        "timeout": 1  // seconds per Claude Code spec
                    }
                ]
            }
        ],
        "SubagentStop": [
            {
                // No matcher for non-tool events
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event SubagentStop",
                        "timeout": 1  // seconds per Claude Code spec
                    }
                ]
            }
        ],
        "PreCompact": [
            {
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PreCompact",
                        "timeout": 1  // seconds per Claude Code spec
                    }
                ]
            }
        ]
    });
    
    // Read existing settings if present
    let mut settings = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| {
            // If parsing fails, create new settings
            json!({
                "_comment": "Claude Code local settings - configured by Cupcake",
                "hooks": {}
            })
        })
    } else {
        json!({
            "_comment": "Claude Code local settings - configured by Cupcake",
            "hooks": {}
        })
    };
    
    // Update hooks section
    settings["hooks"] = hooks;
    
    // Ensure directory exists
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write settings with pretty formatting
    let content = serde_json::to_string_pretty(&settings)?;
    let mut file = fs::File::create(&settings_path)?;
    file.write_all(content.as_bytes())?;
    
    Ok(())
}

/// Get the path to Claude Code settings
fn get_claude_settings_path() -> PathBuf {
    PathBuf::from(".claude/settings.local.json")
}