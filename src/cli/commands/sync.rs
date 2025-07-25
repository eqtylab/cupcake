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
                for (event, _) in hooks {
                    println!("   - {}", event);
                }
            }
        }
        
        Ok(())
    }

    fn name(&self) -> &'static str {
        "sync"
    }

    fn requires_privileges(&self) -> bool {
        // May need to modify Claude Code settings
        true
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
    
    /// Build the Cupcake hook configuration
    fn build_cupcake_hooks(&self) -> Value {
        json!({
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
        })
    }
    
    /// Merge Cupcake hooks into existing settings
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
        
        // Merge each Cupcake hook
        if let Some(cupcake_obj) = cupcake_hooks.as_object() {
            for (event, config) in cupcake_obj {
                if self.force || !hooks.contains_key(event) {
                    hooks.insert(event.clone(), config.clone());
                    updated = true;
                } else {
                    // Check if existing hook is different
                    if let Some(existing) = hooks.get(event) {
                        if existing != config {
                            if self.force {
                                hooks.insert(event.clone(), config.clone());
                                updated = true;
                            } else {
                                eprintln!("⚠️  Hook '{}' already exists with different configuration. Use --force to override.", event);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(updated)
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
