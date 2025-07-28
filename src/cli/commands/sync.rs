use super::CommandHandler;
use crate::Result;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

/// Handler for the `sync` command
pub struct SyncCommand {
    pub settings_path: Option<String>,
    pub dry_run: bool,
    pub force: bool,
}

impl CommandHandler for SyncCommand {
    fn execute(&self) -> Result<()> {
        // 1. Locate Claude Code settings file
        let settings_path = self.locate_settings_file()?;
        
        println!("🔄 Syncing Cupcake hooks to Claude Code settings...");
        println!("📍 Settings file: {}", settings_path.display());
        
        // 2. Read existing settings or create default
        let mut settings = self.read_or_create_settings(&settings_path)?;
        
        // 3. Prepare Cupcake hook configuration
        let cupcake_hooks = self.build_cupcake_hooks();
        
        // 4. Merge hooks into settings
        let updated = self.merge_hooks(&mut settings, cupcake_hooks)?;
        
        if !updated && !self.force {
            println!("✅ Hooks are already up to date!");
            return Ok(());
        }
        
        // 5. Write back or display (dry run)
        if self.dry_run {
            println!("\n🔍 Dry run mode - would write:");
            println!("{}", serde_json::to_string_pretty(&settings)?);
        } else {
            self.write_settings(&settings_path, &settings)?;
            println!("✅ Successfully updated Claude Code settings!");
            println!("\n📝 Registered hooks:");
            if let Some(hooks) = settings.get("hooks").and_then(|h| h.as_object()) {
                for (event, hook_array) in hooks {
                    if let Some(array) = hook_array.as_array() {
                        let hook_count = array.iter()
                            .map(|item| item.get("hooks").and_then(|h| h.as_array()).map(|a| a.len()).unwrap_or(0))
                            .sum::<usize>();
                        println!("   - {} ({} hook commands)", event, hook_count);
                    } else {
                        println!("   - {} (legacy format)", event);
                    }
                }
            }
        }
        
        Ok(())
    }

    fn name(&self) -> &'static str {
        "sync"
    }

    fn requires_privileges(&self) -> bool {
        // Sync only modifies .claude/settings.local.json in the current directory
        // which doesn't require special privileges
        false
    }
}

impl SyncCommand {
    /// Create new sync command
    pub fn new(settings_path: Option<String>, dry_run: bool, force: bool) -> Self {
        Self {
            settings_path,
            dry_run,
            force,
        }
    }
    
    /// Locate the Claude Code settings file
    fn locate_settings_file(&self) -> Result<PathBuf> {
        if let Some(path) = &self.settings_path {
            // User specified explicit path
            Ok(PathBuf::from(path))
        } else {
            // Auto-discover in standard location
            let cwd = std::env::current_dir()
                .map_err(|e| crate::CupcakeError::Config(format!("Failed to get current directory: {}", e)))?;
            Ok(cwd.join(".claude").join("settings.local.json"))
        }
    }
    
    /// Read existing settings or create default structure
    fn read_or_create_settings(&self, path: &Path) -> Result<Value> {
        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| crate::CupcakeError::Config(format!("Failed to read settings: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| crate::CupcakeError::Config(format!("Invalid JSON in settings: {}", e)))
        } else {
            // Create default structure
            Ok(json!({
                "_comment": "Claude Code local settings - managed by Cupcake",
                "hooks": {}
            }))
        }
    }
    
    /// Build the Cupcake hook configuration using July 20 structure
    fn build_cupcake_hooks(&self) -> Value {
        json!({
            "PreToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "cupcake run --event PreToolUse",
                            "timeout": 5  // timeout in seconds per Claude Code spec
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
                            "timeout": 2  // timeout in seconds per Claude Code spec
                        }
                    ]
                }
            ],
            "UserPromptSubmit": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": "cupcake run --event UserPromptSubmit",
                            "timeout": 1  // timeout in seconds per Claude Code spec
                        }
                    ]
                }
            ],
            "Notification": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": "cupcake run --event Notification",
                            "timeout": 1  // timeout in seconds per Claude Code spec
                        }
                    ]
                }
            ],
            "Stop": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": "cupcake run --event Stop",
                            "timeout": 1  // timeout in seconds per Claude Code spec
                        }
                    ]
                }
            ],
            "SubagentStop": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": "cupcake run --event SubagentStop",
                            "timeout": 1  // timeout in seconds per Claude Code spec
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
                            "timeout": 1  // timeout in seconds per Claude Code spec
                        }
                    ]
                }
            ]
        })
    }
    
    /// Merge Cupcake hooks into existing settings using July 20 structure
    fn merge_hooks(&self, settings: &mut Value, cupcake_hooks: Value) -> Result<bool> {
        let mut updated = false;
        
        // Ensure hooks object exists
        if !settings.get("hooks").is_some() {
            settings["hooks"] = json!({});
            updated = true;
        }
        
        // Get mutable reference to hooks
        let hooks = settings["hooks"].as_object_mut()
            .ok_or_else(|| crate::CupcakeError::Config("Invalid hooks structure in settings".to_string()))?;
        
        // Merge each Cupcake hook (now arrays)
        if let Some(cupcake_obj) = cupcake_hooks.as_object() {
            for (event_name, hook_array) in cupcake_obj {
                // Check if this event already has hooks
                if let Some(existing_array) = hooks.get(event_name) {
                    if !self.force {
                        // Check if the Cupcake hook already exists
                        let needs_update = if let Some(existing_hooks) = existing_array.as_array() {
                            !self.cupcake_hook_exists_in_array(existing_hooks, hook_array)
                        } else {
                            // Existing is not an array, need to replace
                            true
                        };
                        
                        if needs_update {
                            eprintln!("⚠️  Hook '{}' already exists. Use --force to add/update Cupcake hooks.", event_name);
                            continue;
                        } else {
                            // Cupcake hook already exists and is up to date
                            continue;
                        }
                    }
                }
                
                // Add or replace the hook array
                if self.force || !hooks.contains_key(event_name) {
                    // For force mode or new events, just replace the entire array
                    hooks.insert(event_name.clone(), hook_array.clone());
                    updated = true;
                } else {
                    // Append to existing array (this case shouldn't be reached due to check above)
                    if let Some(existing_array) = hooks.get_mut(event_name).and_then(|v| v.as_array_mut()) {
                        if let Some(new_hooks) = hook_array.as_array() {
                            for hook in new_hooks {
                                existing_array.push(hook.clone());
                            }
                            updated = true;
                        }
                    }
                }
            }
        }
        
        Ok(updated)
    }
    
    /// Check if a Cupcake hook already exists in the hook array
    fn cupcake_hook_exists_in_array(&self, existing_hooks: &[Value], cupcake_hook_array: &Value) -> bool {
        if let Some(cupcake_hooks) = cupcake_hook_array.as_array() {
            for cupcake_hook in cupcake_hooks {
                // Look for a hook with the same matcher and command structure
                let found = existing_hooks.iter().any(|existing_hook| {
                    self.hooks_are_equivalent(existing_hook, cupcake_hook)
                });
                if !found {
                    return false; // This cupcake hook doesn't exist
                }
            }
            true // All cupcake hooks already exist
        } else {
            false
        }
    }
    
    /// Check if two hook objects are equivalent (same matcher and command)
    fn hooks_are_equivalent(&self, hook1: &Value, hook2: &Value) -> bool {
        // Compare matcher (if present)
        let matcher1 = hook1.get("matcher").and_then(|m| m.as_str());
        let matcher2 = hook2.get("matcher").and_then(|m| m.as_str());
        
        if matcher1 != matcher2 {
            return false;
        }
        
        // Compare hooks arrays
        let hooks1 = hook1.get("hooks").and_then(|h| h.as_array());
        let hooks2 = hook2.get("hooks").and_then(|h| h.as_array());
        
        match (hooks1, hooks2) {
            (Some(h1), Some(h2)) => {
                // Check if any command in hooks2 exists in hooks1
                h2.iter().any(|cmd2| {
                    h1.iter().any(|cmd1| {
                        cmd1.get("command") == cmd2.get("command")
                    })
                })
            }
            _ => false,
        }
    }
    
    /// Write settings back to file with proper formatting
    fn write_settings(&self, path: &Path, settings: &Value) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| crate::CupcakeError::Config(format!("Failed to create .claude directory: {}", e)))?;
        }
        
        // Write with pretty formatting
        let content = serde_json::to_string_pretty(settings)?;
        let mut file = fs::File::create(path)
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to create settings file: {}", e)))?;
        file.write_all(content.as_bytes())
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to write settings: {}", e)))?;
        
        Ok(())
    }
}
