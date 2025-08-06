use super::CommandHandler;
use crate::config::claude_hooks;
use crate::Result;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
                        let hook_count = array
                            .iter()
                            .map(|item| {
                                item.get("hooks")
                                    .and_then(|h| h.as_array())
                                    .map(|a| a.len())
                                    .unwrap_or(0)
                            })
                            .sum::<usize>();
                        println!("   - {event} ({hook_count} hook commands)");
                    } else {
                        println!("   - {event} (legacy format)");
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
            let cwd = std::env::current_dir().map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to get current directory: {e}"))
            })?;
            Ok(cwd.join(".claude").join("settings.local.json"))
        }
    }

    /// Read existing settings or create default structure
    fn read_or_create_settings(&self, path: &Path) -> Result<Value> {
        if path.exists() {
            let content = fs::read_to_string(path).map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to read settings: {e}"))
            })?;
            serde_json::from_str(&content)
                .map_err(|e| crate::CupcakeError::Config(format!("Invalid JSON in settings: {e}")))
        } else {
            // Create default structure - just hooks, no comment
            Ok(json!({
                "hooks": {}
            }))
        }
    }

    /// Build the Cupcake hook configuration using July 20 structure
    fn build_cupcake_hooks(&self) -> Value {
        claude_hooks::build_cupcake_hooks()
    }

    /// Merge Cupcake hooks into existing settings using July 20 structure
    /// Implements idempotent "remove-then-append" strategy using managed_by marker
    fn merge_hooks(&self, settings: &mut Value, cupcake_hooks: Value) -> Result<bool> {
        let mut updated = false;

        // Ensure hooks object exists
        if settings.get("hooks").is_none() {
            settings["hooks"] = json!({});
            updated = true;
        }

        // Get mutable reference to hooks
        let hooks = settings["hooks"].as_object_mut().ok_or_else(|| {
            crate::CupcakeError::Config("Invalid hooks structure in settings".to_string())
        })?;

        // Process each event type
        if let Some(cupcake_obj) = cupcake_hooks.as_object() {
            for (event_name, cupcake_hook_array) in cupcake_obj {
                // Step 1: Surgical removal - filter out existing hooks with managed_by: "cupcake"
                let filtered_array = if let Some(existing_value) = hooks.get(event_name) {
                    if let Some(existing_array) = existing_value.as_array() {
                        // Filter out cupcake-managed hooks
                        let filtered: Vec<Value> = existing_array
                            .iter()
                            .filter(|hook| {
                                // Keep only hooks that are NOT managed by cupcake
                                hook.get("managed_by")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s != "cupcake")
                                    .unwrap_or(true)  // Keep if no managed_by field
                            })
                            .cloned()
                            .collect();
                        
                        // If we removed any hooks, we've updated
                        if filtered.len() != existing_array.len() {
                            updated = true;
                        }
                        
                        Some(filtered)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Update with filtered array if needed
                if let Some(filtered) = filtered_array {
                    hooks.insert(event_name.clone(), json!(filtered));
                }
                
                // Step 2: Clean append - add our new cupcake hooks
                if let Some(new_hooks) = cupcake_hook_array.as_array() {
                    if let Some(existing_array) = hooks.get_mut(event_name).and_then(|v| v.as_array_mut()) {
                        // Append to existing array
                        for hook in new_hooks {
                            existing_array.push(hook.clone());
                            updated = true;
                        }
                    } else {
                        // No existing array, create new one
                        hooks.insert(event_name.clone(), cupcake_hook_array.clone());
                        updated = true;
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
            fs::create_dir_all(parent).map_err(|e| {
                crate::CupcakeError::Config(format!("Failed to create .claude directory: {e}"))
            })?;
        }

        // Write with pretty formatting
        let content = serde_json::to_string_pretty(settings)?;
        let mut file = fs::File::create(path).map_err(|e| {
            crate::CupcakeError::Config(format!("Failed to create settings file: {e}"))
        })?;
        file.write_all(content.as_bytes())
            .map_err(|e| crate::CupcakeError::Config(format!("Failed to write settings: {e}")))?;

        Ok(())
    }
}
