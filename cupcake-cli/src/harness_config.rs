//! Agent harness configuration for automated integration setup
//!
//! Provides trait-based architecture for configuring various agent harnesses
//! (Claude Code, Cursor, etc.) with Cupcake policy evaluation.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Trait for agent harness configuration
pub trait HarnessConfig {
    /// Get the harness name for display
    fn name(&self) -> &str;

    /// Get the settings file path relative to project root or user home
    fn settings_path(&self, global: bool) -> PathBuf;

    /// Generate the hook configuration JSON for this harness
    fn generate_hooks(&self, policy_dir: &Path, global: bool) -> Result<Value>;

    /// Merge hooks into existing settings without destroying other configuration
    fn merge_settings(&self, existing: Value, new_hooks: Value) -> Result<Value>;
}

/// Claude Code harness implementation
pub struct ClaudeHarness;

/// Cursor harness implementation
pub struct CursorHarness;

/// Factory AI harness implementation
pub struct FactoryHarness;

/// OpenCode harness implementation
pub struct OpenCodeHarness;

/// GitHub repository for downloading plugins
const GITHUB_REPO: &str = "eqtylab/cupcake";

/// Get the current Cupcake version (embedded at compile time)
fn get_cupcake_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

impl HarnessConfig for ClaudeHarness {
    fn name(&self) -> &str {
        "Claude Code"
    }

    fn settings_path(&self, global: bool) -> PathBuf {
        if global {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(".claude")
                .join("settings.json")
        } else {
            Path::new(".claude").join("settings.json")
        }
    }

    fn generate_hooks(&self, policy_dir: &Path, global: bool) -> Result<Value> {
        // Determine the policy path to use in commands
        let policy_path = if global {
            // Global config - use absolute path
            let abs_path =
                fs::canonicalize(policy_dir).unwrap_or_else(|_| policy_dir.to_path_buf());
            abs_path.display().to_string()
        } else {
            // Project config - use environment variable for portability
            "$CLAUDE_PROJECT_DIR/.cupcake".to_string()
        };

        Ok(json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness claude --policy-dir {}", policy_path)
                    }]
                }],
                "PostToolUse": [{
                    "matcher": "Edit|MultiEdit|Write",
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness claude --policy-dir {}", policy_path)
                    }]
                }],
                "UserPromptSubmit": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness claude --policy-dir {}", policy_path)
                    }]
                }],
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness claude --policy-dir {}", policy_path)
                    }]
                }]
            }
        }))
    }

    fn merge_settings(&self, mut existing: Value, new_hooks: Value) -> Result<Value> {
        merge_hooks(&mut existing, new_hooks)?;
        Ok(existing)
    }
}

impl HarnessConfig for CursorHarness {
    fn name(&self) -> &str {
        "Cursor"
    }

    fn settings_path(&self, _global: bool) -> PathBuf {
        // Cursor hooks MUST always be in ~/.cursor/hooks.json (global)
        // Cursor does not support project-level hooks like Claude Code does.
        // The hooks are always read from the user's home directory.
        // Reference: https://cursor.com/docs/agent/hooks.md
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".cursor")
            .join("hooks.json")
    }

    fn generate_hooks(&self, policy_dir: &Path, global: bool) -> Result<Value> {
        // Determine the policy path to use in commands
        let policy_path = if global {
            // Global config - use absolute path
            let abs_path =
                fs::canonicalize(policy_dir).unwrap_or_else(|_| policy_dir.to_path_buf());
            abs_path.display().to_string()
        } else {
            // Project config - use relative path from workspace root
            ".cupcake".to_string()
        };

        // Cursor's hook configuration format - official hooks.json structure
        // Reference: https://cursor.com/docs/agent/hooks.md
        Ok(json!({
            "version": 1,
            "hooks": {
                "beforeShellExecution": [{
                    "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
                }],
                "beforeMCPExecution": [{
                    "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
                }],
                "afterFileEdit": [{
                    "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
                }],
                "beforeReadFile": [{
                    "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
                }],
                "beforeSubmitPrompt": [{
                    "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
                }],
                "stop": [{
                    "command": format!("cupcake eval --harness cursor --policy-dir {}", policy_path)
                }]
            }
        }))
    }

    fn merge_settings(&self, mut existing: Value, new_hooks: Value) -> Result<Value> {
        // For Cursor hooks.json, merge using the same hooks structure as Claude
        merge_hooks(&mut existing, new_hooks)?;
        Ok(existing)
    }
}

impl HarnessConfig for FactoryHarness {
    fn name(&self) -> &str {
        "Factory AI"
    }

    fn settings_path(&self, global: bool) -> PathBuf {
        if global {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(".factory")
                .join("settings.json")
        } else {
            Path::new(".factory").join("settings.json")
        }
    }

    fn generate_hooks(&self, policy_dir: &Path, global: bool) -> Result<Value> {
        // Determine the policy path to use in commands
        let policy_path = if global {
            // Global config - use absolute path
            let abs_path =
                fs::canonicalize(policy_dir).unwrap_or_else(|_| policy_dir.to_path_buf());
            abs_path.display().to_string()
        } else {
            // Project config - use environment variable for portability
            "\"$FACTORY_PROJECT_DIR\"/.cupcake".to_string()
        };

        Ok(json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness factory --policy-dir {}", policy_path)
                    }]
                }],
                "PostToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness factory --policy-dir {}", policy_path)
                    }]
                }],
                "UserPromptSubmit": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness factory --policy-dir {}", policy_path)
                    }]
                }],
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness factory --policy-dir {}", policy_path)
                    }]
                }],
                "Stop": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness factory --policy-dir {}", policy_path)
                    }]
                }],
                "SubagentStop": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!("cupcake eval --harness factory --policy-dir {}", policy_path)
                    }]
                }]
            }
        }))
    }

    fn merge_settings(&self, mut existing: Value, new_hooks: Value) -> Result<Value> {
        merge_hooks(&mut existing, new_hooks)?;
        Ok(existing)
    }
}

impl OpenCodeHarness {
    /// Download the OpenCode plugin from GitHub releases
    ///
    /// Downloads cupcake.js to .opencode/plugin/ directory
    pub async fn download_plugin(target_dir: &Path, global: bool) -> Result<()> {
        // Determine plugin directory
        let plugin_dir = if global {
            dirs::config_dir()
                .ok_or_else(|| anyhow!("Could not determine config directory"))?
                .join("opencode")
                .join("plugin")
        } else {
            target_dir.join(".opencode").join("plugin")
        };

        // Create plugin directory
        fs::create_dir_all(&plugin_dir)
            .with_context(|| format!("Failed to create plugin directory: {:?}", plugin_dir))?;

        let plugin_path = plugin_dir.join("cupcake.js");

        // Use latest release to avoid version sync issues between CLI and plugin
        // The plugin is forward-compatible, so latest is always safe
        let plugin_url = format!(
            "https://github.com/{}/releases/latest/download/opencode-plugin.js",
            GITHUB_REPO
        );
        let checksum_url = format!(
            "https://github.com/{}/releases/latest/download/opencode-plugin.js.sha256",
            GITHUB_REPO
        );

        println!("   Downloading OpenCode plugin (latest release)...");

        // Download the plugin
        let plugin_content = download_file(&plugin_url).await.with_context(|| {
            format!(
                "Failed to download OpenCode plugin from {}. \
                 This may happen if the release doesn't exist yet. \
                 Try installing from source: cd cupcake-plugins/opencode && npm ci && npm run build",
                plugin_url
            )
        })?;

        // Download and verify checksum
        match download_file(&checksum_url).await {
            Ok(checksum_content) => {
                let checksum_str = String::from_utf8_lossy(&checksum_content);
                let expected_hash = checksum_str
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| anyhow!("Invalid checksum format"))?;

                // Calculate actual hash
                let mut hasher = Sha256::new();
                hasher.update(&plugin_content);
                let actual_hash = hex::encode(hasher.finalize());

                if actual_hash != expected_hash {
                    return Err(anyhow!(
                        "Checksum verification failed!\n  Expected: {}\n  Actual: {}",
                        expected_hash,
                        actual_hash
                    ));
                }

                println!("   Checksum verified");
            }
            Err(e) => {
                eprintln!(
                    "   Warning: Could not verify checksum: {}. Proceeding anyway.",
                    e
                );
            }
        }

        // Write plugin to disk
        let mut file = fs::File::create(&plugin_path)
            .with_context(|| format!("Failed to create plugin file: {:?}", plugin_path))?;
        file.write_all(&plugin_content)
            .with_context(|| "Failed to write plugin content")?;

        println!("   Plugin installed to: {:?}", plugin_path);

        Ok(())
    }

    /// Print manual installation instructions as fallback
    pub fn print_manual_instructions() {
        eprintln!();
        eprintln!("   To manually install the OpenCode plugin:");
        eprintln!();
        eprintln!("   1. Build the plugin:");
        eprintln!("      cd cupcake-plugins/opencode && npm ci && npm run build");
        eprintln!();
        eprintln!("   2. Copy to your project:");
        eprintln!("      mkdir -p .opencode/plugin");
        eprintln!("      cp cupcake-plugins/opencode/dist/cupcake.js .opencode/plugin/");
        eprintln!();
        eprintln!("   Or download from GitHub releases:");
        eprintln!(
            "      curl -fsSL https://github.com/{}/releases/latest/download/opencode-plugin.js \\",
            GITHUB_REPO
        );
        eprintln!("        -o .opencode/plugin/cupcake.js");
        eprintln!();
    }
}

/// Download a file from a URL
async fn download_file(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .user_agent(format!("cupcake/{}", get_cupcake_version()))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to send request to {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "HTTP request failed with status {}: {}",
            response.status(),
            url
        ));
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| "Failed to read response body")?;

    Ok(bytes.to_vec())
}

/// Merge hooks into existing settings without duplicates
fn merge_hooks(existing: &mut Value, new: Value) -> Result<()> {
    // Ensure existing is an object
    if !existing.is_object() {
        *existing = json!({});
    }

    // Get or create hooks object
    let hooks = existing
        .as_object_mut()
        .ok_or_else(|| anyhow!("Invalid settings format"))?
        .entry("hooks")
        .or_insert_with(|| json!({}));

    // Ensure hooks is an object
    if !hooks.is_object() {
        *hooks = json!({});
    }

    let new_hooks = new["hooks"]
        .as_object()
        .ok_or_else(|| anyhow!("Invalid hooks format"))?;

    // For each event type in new hooks
    for (event_name, new_matchers) in new_hooks {
        let event_array = hooks
            .as_object_mut()
            .unwrap()
            .entry(event_name)
            .or_insert_with(|| json!([]));

        // Ensure it's an array
        if !event_array.is_array() {
            *event_array = json!([]);
        }

        let event_array = event_array
            .as_array_mut()
            .ok_or_else(|| anyhow!("Invalid event array"))?;

        // Check if this exact configuration already exists
        if let Some(new_matcher_array) = new_matchers.as_array() {
            for new_matcher in new_matcher_array {
                if !contains_matcher(event_array, new_matcher) {
                    event_array.push(new_matcher.clone());
                }
            }
        }
    }

    Ok(())
}

/// Check if a matcher configuration already exists in the array
fn contains_matcher(array: &[Value], matcher: &Value) -> bool {
    array.iter().any(|existing| {
        // Check if matcher patterns are the same
        let same_matcher = existing.get("matcher") == matcher.get("matcher");

        // Check if hook commands are the same
        let existing_hooks = existing.get("hooks").and_then(|h| h.as_array());
        let new_hooks = matcher.get("hooks").and_then(|h| h.as_array());

        if let (Some(existing_hooks), Some(new_hooks)) = (existing_hooks, new_hooks) {
            // Check if any new hook command already exists
            let has_duplicate = new_hooks.iter().any(|new_hook| {
                existing_hooks.iter().any(|existing_hook| {
                    // Compare commands to detect duplicates
                    existing_hook.get("command") == new_hook.get("command")
                })
            });

            same_matcher && has_duplicate
        } else {
            false
        }
    })
}

/// Configure harness integration with error recovery
pub async fn configure_harness(
    harness_type: super::HarnessType,
    policy_dir: &Path,
    global: bool,
) -> Result<()> {
    use super::HarnessType;

    match harness_type {
        HarnessType::Claude => {
            let harness = ClaudeHarness;
            let settings_path = harness.settings_path(global);

            // Try to configure, fallback to manual instructions on error
            if let Err(e) =
                setup_harness_settings(&harness, &settings_path, policy_dir, global).await
            {
                eprintln!(
                    "⚠️  Could not automatically configure {}: {}",
                    harness.name(),
                    e
                );
                print_manual_instructions(&harness, policy_dir, global);
                // Don't fail the entire init - just warn
            } else {
                println!(
                    "✅ Configured {} integration in {}",
                    harness.name(),
                    settings_path.display()
                );
                println!("   - Added PreToolUse hook for all tools");
                println!("   - Added PostToolUse hook for file modifications");
                println!("   - Added UserPromptSubmit hook for prompt validation");
                println!("   - Added SessionStart hook for initial context");
                println!();
                println!("   {} will now evaluate all tool uses and prompts against your Cupcake policies.",
                    harness.name());
            }
        }
        HarnessType::Cursor => {
            let harness = CursorHarness;
            let settings_path = harness.settings_path(global);

            // Try to configure, fallback to manual instructions on error
            if let Err(e) =
                setup_harness_settings(&harness, &settings_path, policy_dir, global).await
            {
                eprintln!(
                    "⚠️  Could not automatically configure {}: {}",
                    harness.name(),
                    e
                );
                print_cursor_manual_instructions(policy_dir, global);
                // Don't fail the entire init - just warn
            } else {
                println!(
                    "✅ Configured {} integration in {}",
                    harness.name(),
                    settings_path.display()
                );
                println!("   - Added beforeShellExecution hook for shell commands");
                println!("   - Added beforeMCPExecution hook for MCP tools");
                println!("   - Added afterFileEdit hook for post-edit validation");
                println!("   - Added beforeReadFile hook for file access control");
                println!("   - Added beforeSubmitPrompt hook for prompt validation");
                println!("   - Added stop hook for cleanup");
                println!();
                println!(
                    "   {} will now evaluate all actions against your Cupcake policies.",
                    harness.name()
                );
            }
        }
        HarnessType::Factory => {
            let harness = FactoryHarness;
            let settings_path = harness.settings_path(global);

            // Try to configure, fallback to manual instructions on error
            if let Err(e) =
                setup_harness_settings(&harness, &settings_path, policy_dir, global).await
            {
                eprintln!(
                    "⚠️  Could not automatically configure {}: {}",
                    harness.name(),
                    e
                );
                print_manual_instructions(&harness, policy_dir, global);
                // Don't fail the entire init - just warn
            } else {
                println!(
                    "✅ Configured {} integration in {}",
                    harness.name(),
                    settings_path.display()
                );
                println!("   - Added PreToolUse hook for all tools");
                println!("   - Added PostToolUse hook for all tools");
                println!("   - Added UserPromptSubmit hook for prompt validation");
                println!("   - Added SessionStart hook for initial context");
                println!("   - Added Stop/SubagentStop hooks for cleanup");
                println!();
                println!("   {} will now evaluate all tool uses and prompts against your Cupcake policies.",
                    harness.name());
            }
        }
        HarnessType::OpenCode => {
            // OpenCode uses a plugin model - download the plugin from GitHub releases
            println!("   Configuring OpenCode integration...");

            // Determine the target directory for the plugin
            let target_dir = if global {
                // For global, use config directory
                dirs::config_dir().ok_or_else(|| anyhow!("Could not determine config directory"))?
            } else {
                // For project, use the policy_dir parent (project root)
                policy_dir.parent().unwrap_or(Path::new(".")).to_path_buf()
            };

            // Try to download the plugin from GitHub releases
            match OpenCodeHarness::download_plugin(&target_dir, global).await {
                Ok(()) => {
                    let plugin_location = if global {
                        "~/.config/opencode/plugin/cupcake.js".to_string()
                    } else {
                        ".opencode/plugin/cupcake.js".to_string()
                    };

                    println!("✅ Configured OpenCode integration");
                    println!("   - Plugin installed to: {}", plugin_location);
                    println!();
                    println!(
                        "   OpenCode will automatically load the Cupcake plugin and enforce policies."
                    );
                    println!();
                    println!(
                        "   Optional: Create .cupcake/opencode.json to customize plugin behavior:"
                    );
                    println!("   {{");
                    println!("     \"enabled\": true,");
                    println!("     \"logLevel\": \"info\",");
                    println!("     \"failMode\": \"closed\"");
                    println!("   }}");
                }
                Err(e) => {
                    eprintln!(
                        "⚠️  Could not automatically download OpenCode plugin: {}",
                        e
                    );
                    OpenCodeHarness::print_manual_instructions();
                    // Don't fail the entire init - just warn
                }
            }
        }
    }

    Ok(())
}

/// Setup harness settings file (create or merge)
async fn setup_harness_settings(
    harness: &dyn HarnessConfig,
    settings_path: &Path,
    policy_dir: &Path,
    global: bool,
) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Generate hook configuration
    let new_hooks = harness.generate_hooks(policy_dir, global)?;

    // Check if settings file exists
    let final_settings = if settings_path.exists() {
        // Read existing settings
        let content = fs::read_to_string(settings_path)?;
        let existing: Value = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Invalid JSON in existing settings: {}", e))?;

        println!("⚠️  Found existing {}", settings_path.display());
        println!("   Merging Cupcake hooks into existing configuration...");

        // Merge hooks
        harness.merge_settings(existing, new_hooks)?
    } else {
        // Create new settings with just hooks
        new_hooks
    };

    // Write settings with pretty formatting
    let json_str = serde_json::to_string_pretty(&final_settings)?;
    fs::write(settings_path, json_str)?;

    Ok(())
}

/// Print manual configuration instructions as fallback
fn print_manual_instructions(harness: &dyn HarnessConfig, policy_dir: &Path, global: bool) {
    let policy_path = if global {
        policy_dir.display().to_string()
    } else {
        "$CLAUDE_PROJECT_DIR/.cupcake".to_string()
    };

    eprintln!();
    eprintln!(
        "   To manually configure, add this to your {}:",
        harness.settings_path(global).display()
    );
    eprintln!();
    eprintln!("   {{");
    eprintln!("     \"hooks\": {{");
    eprintln!("       \"PreToolUse\": [{{");
    eprintln!("         \"matcher\": \"*\",");
    eprintln!("         \"hooks\": [{{");
    eprintln!("           \"type\": \"command\",");
    eprintln!(
        "           \"command\": \"cupcake eval --harness claude --policy-dir {policy_path}\""
    );
    eprintln!("         }}]");
    eprintln!("       }}]");
    eprintln!("     }}");
    eprintln!("   }}");
    eprintln!();
}

/// Print manual configuration instructions for Cursor
fn print_cursor_manual_instructions(policy_dir: &Path, global: bool) {
    let policy_path = if global {
        policy_dir.display().to_string()
    } else {
        ".cupcake".to_string()
    };

    // Cursor hooks are always in ~/.cursor/hooks.json
    let settings_path = "~/.cursor/hooks.json";

    eprintln!();
    eprintln!("   To manually configure, add this to your {settings_path}:");
    eprintln!();
    eprintln!("   {{");
    eprintln!("     \"version\": 1,");
    eprintln!("     \"hooks\": {{");
    eprintln!("       \"beforeShellExecution\": [{{");
    eprintln!("         \"command\": \"cupcake eval --harness cursor --policy-dir {policy_path}\"");
    eprintln!("       }}],");
    eprintln!("       \"beforeMCPExecution\": [{{");
    eprintln!("         \"command\": \"cupcake eval --harness cursor --policy-dir {policy_path}\"");
    eprintln!("       }}],");
    eprintln!("       \"afterFileEdit\": [{{");
    eprintln!("         \"command\": \"cupcake eval --harness cursor --policy-dir {policy_path}\"");
    eprintln!("       }}],");
    eprintln!("       \"beforeReadFile\": [{{");
    eprintln!("         \"command\": \"cupcake eval --harness cursor --policy-dir {policy_path}\"");
    eprintln!("       }}],");
    eprintln!("       \"beforeSubmitPrompt\": [{{");
    eprintln!("         \"command\": \"cupcake eval --harness cursor --policy-dir {policy_path}\"");
    eprintln!("       }}],");
    eprintln!("       \"stop\": [{{");
    eprintln!("         \"command\": \"cupcake eval --harness cursor --policy-dir {policy_path}\"");
    eprintln!("       }}]");
    eprintln!("     }}");
    eprintln!("   }}");
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_empty_settings() {
        let mut existing = json!({});
        let new = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": "cupcake eval --harness claude"
                    }]
                }]
            }
        });

        merge_hooks(&mut existing, new.clone()).unwrap();
        assert_eq!(existing["hooks"]["PreToolUse"][0]["matcher"], "*");
    }

    #[test]
    fn test_merge_preserves_existing() {
        let mut existing = json!({
            "env": {"FOO": "bar"},
            "hooks": {
                "PostToolUse": [{
                    "matcher": "Write",
                    "hooks": [{
                        "type": "command",
                        "command": "echo done"
                    }]
                }]
            }
        });

        let new = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": "cupcake eval --harness claude"
                    }]
                }]
            }
        });

        merge_hooks(&mut existing, new).unwrap();

        // Existing settings preserved
        assert_eq!(existing["env"]["FOO"], "bar");
        assert_eq!(existing["hooks"]["PostToolUse"][0]["matcher"], "Write");

        // New hooks added
        assert_eq!(existing["hooks"]["PreToolUse"][0]["matcher"], "*");
    }

    #[test]
    fn test_duplicate_detection() {
        let mut existing = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": "cupcake eval --policy-dir /path"
                    }]
                }]
            }
        });

        let new = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "*",
                    "hooks": [{
                        "type": "command",
                        "command": "cupcake eval --policy-dir /path"
                    }]
                }]
            }
        });

        merge_hooks(&mut existing, new).unwrap();

        // Should not duplicate
        assert_eq!(existing["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    }
}
