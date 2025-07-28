//! Claude Code settings.local.json updater for TUI

use std::fs;
use std::path::PathBuf;
use serde_json::json;
use crate::Result;
use crate::config::claude_hooks;
use std::io::Write;

/// Update Claude Code settings with hook configuration
pub fn update_claude_settings() -> Result<()> {
    let settings_path = get_claude_settings_path();
    
    // Get the standard Cupcake hook configuration
    let hooks = claude_hooks::build_cupcake_hooks();
    
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