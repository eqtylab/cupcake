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
    let hooks = json!({
        "PreToolUse": {
            "command": "cupcake run PreToolUse",
            "timeout": 5000,
            "description": "Cupcake policy evaluation before tool execution"
        },
        "PostToolUse": {
            "command": "cupcake run PostToolUse",
            "timeout": 2000,
            "description": "Cupcake policy evaluation after tool execution"
        },
        "UserPromptSubmit": {
            "command": "cupcake run UserPromptSubmit",
            "timeout": 1000,
            "description": "Cupcake context injection for user prompts"
        },
        "Notification": {
            "command": "cupcake run Notification",
            "timeout": 1000,
            "description": "Cupcake notification handling"
        },
        "Stop": {
            "command": "cupcake run Stop",
            "timeout": 1000,
            "description": "Cupcake session cleanup"
        },
        "SubagentStop": {
            "command": "cupcake run SubagentStop",
            "timeout": 1000,
            "description": "Cupcake subagent cleanup"
        },
        "PreCompact": {
            "command": "cupcake run PreCompact",
            "timeout": 1000,
            "description": "Cupcake pre-compaction handling"
        }
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