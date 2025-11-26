//! Input preprocessing to defend against adversarial patterns
//!
//! This module provides automatic normalization of input data before policy
//! evaluation, implementing defense-in-depth against bypass techniques identified
//! in TOB-EQTY-LAB-CUPCAKE-3.
//!
//! ## Architecture
//!
//! The preprocessing pipeline operates in phases:
//! - Phase 1: Whitespace normalization (implemented)
//! - Phase 2: Pattern detection (future)
//! - Phase 3: AST analysis (future)
//!
//! ## Security Model
//!
//! Rather than requiring every policy to implement secure parsing, we normalize
//! adversarial patterns at the engine level, providing automatic protection for
//! all policies (user and builtin).

use crate::harness::types::HarnessType;
use serde_json::Value;
use tracing::{debug, trace};

pub mod config;
pub mod normalizers;
pub mod script_inspector;
pub mod symlink_resolver;

pub use config::PreprocessConfig;
use normalizers::WhitespaceNormalizer;
use script_inspector::ScriptInspector;
use symlink_resolver::SymlinkResolver;

/// Preprocess input JSON to normalize adversarial patterns
///
/// This is the main entry point for input preprocessing. It:
/// 1. Identifies tool-specific fields that need normalization
/// 2. Applies appropriate normalizers based on configuration
/// 3. Logs all transformations for auditability
///
/// # Arguments
/// * `input` - Mutable reference to the input JSON
/// * `config` - Configuration controlling what normalizations to apply
/// * `harness` - The harness type (Claude Code or Cursor)
///
/// # Example
/// ```
/// # use serde_json::json;
/// # use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
/// # use cupcake_core::harness::types::HarnessType;
/// let mut input = json!({
///     "tool_name": "Bash",
///     "tool_input": {
///         "command": "rm  -rf  .cupcake"  // Double spaces
///     }
/// });
///
/// let config = PreprocessConfig::default();
/// preprocess_input(&mut input, &config, HarnessType::ClaudeCode);
///
/// // Command is now normalized: "rm -rf .cupcake"
/// ```
pub fn preprocess_input(input: &mut Value, config: &PreprocessConfig, harness: HarnessType) {
    // Extract tool/event information based on harness type
    // We copy these strings out of the JSON so we can modify the JSON later.
    // Can't modify data while something points into it.
    let (tool_name, event_name): (String, String) = match harness {
        HarnessType::ClaudeCode | HarnessType::Factory => {
            // Claude Code and Factory AI use tool_name (same structure)
            let tool = input
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let event = input
                .get("hook_event_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (tool, event)
        }
        HarnessType::OpenCode => {
            // OpenCode uses lowercase tool names that need to be mapped to Cupcake format
            // Clone fields before mutating input
            let args = input.get("args").cloned();
            let result = input.get("result").cloned();

            // Get tool and event as owned strings to avoid borrow issues
            let tool_lowercase = input
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(); // Make it owned

            // Map OpenCode tool names to Cupcake format (bash -> Bash, edit -> Edit, etc.)
            let tool_mapped = match tool_lowercase.as_str() {
                "bash" => "Bash".to_string(),
                "edit" => "Edit".to_string(),
                "write" => "Write".to_string(),
                "read" => "Read".to_string(),
                "grep" => "Grep".to_string(),
                "glob" => "Glob".to_string(),
                "list" => "List".to_string(),
                "patch" => "Patch".to_string(),
                "todowrite" => "TodoWrite".to_string(),
                "todoread" => "TodoRead".to_string(),
                "webfetch" => "WebFetch".to_string(),
                "task" => "Task".to_string(),
                _ => tool_lowercase, // Unknown tools pass through
            };

            // Now we can mutate input
            if let Some(obj) = input.as_object_mut() {
                // Add tool_name field for engine compatibility
                obj.insert(
                    "tool_name".to_string(),
                    serde_json::Value::String(tool_mapped),
                );

                // Add tool_input field by renaming args to tool_input for engine compatibility
                if let Some(args_value) = args {
                    obj.insert("tool_input".to_string(), args_value);
                }

                // Add tool_response field by renaming result to tool_response for PostToolUse events
                if let Some(result_value) = result {
                    obj.insert("tool_response".to_string(), result_value);
                }
            }

            // Re-read the values we just inserted to get proper string slices
            let tool = input
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let event = input
                .get("hook_event_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (tool, event)
        }
        HarnessType::Cursor => {
            // Cursor uses hook_event_name to determine the action type
            let event = input
                .get("hook_event_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            // For Cursor, we treat certain events as equivalent to tools
            let tool = match event.as_str() {
                "beforeShellExecution" => "Bash",
                "beforeFileEdit" | "afterFileEdit" => "Edit",
                "beforeFileWrite" | "afterFileWrite" => "Write",
                _ => "unknown",
            }
            .to_string();
            (tool, event)
        }
    };

    trace!(
        "Preprocessing input for harness: {}, tool: {}, event: {}",
        harness,
        tool_name,
        event_name
    );

    // Apply tool-specific preprocessing based on the tool type
    match tool_name.as_str() {
        "Bash" if config.normalize_whitespace => match harness {
            HarnessType::ClaudeCode => preprocess_claude_bash_command(input, config),
            HarnessType::Factory => preprocess_claude_bash_command(input, config),
            HarnessType::Cursor => preprocess_cursor_shell_command(input, config),
            HarnessType::OpenCode => preprocess_claude_bash_command(input, config), // Same format as Claude/Factory
        },
        // Future: Add other tool-specific preprocessing
        // "Task" => preprocess_task_prompt(input, config),
        // "WebFetch" => preprocess_url(input, config),
        _ => {
            trace!("No preprocessing rules for tool: {}", tool_name);
        }
    }

    // ==========================================================================
    // CONTENT FIELD NORMALIZATION FOR WRITE/EDIT UNIFICATION
    // ==========================================================================
    //
    // Problem: Write and Edit tools use different field names for content:
    //   - Write: tool_input.content (full file content)
    //   - Edit:  tool_input.new_string (replacement text)
    //
    // This forces policy authors to write duplicate rules or helper functions
    // just to handle the field name difference.
    //
    // Solution: Copy Write's `content` to `new_string` during preprocessing,
    // allowing policies to use a single field name for both tools:
    //
    //   ```rego
    //   deny contains decision if {
    //       input.tool_name in {"Write", "Edit"}
    //       content := input.tool_input.new_string  # Works for both!
    //       contains(content, "bad pattern")
    //   }
    //   ```
    //
    // This is similar to how we normalize file paths with `resolved_file_path`.
    // The original `content` field is preserved for backwards compatibility.
    //
    // Note: We copy Write→new_string (not Edit→content) because:
    //   - Edit's new_string is always the "new" content being written
    //   - Write's content is also the "new" content being written
    //   - This makes `new_string` semantically consistent across both tools
    //
    // ==========================================================================
    if harness == HarnessType::ClaudeCode {
        normalize_write_edit_content_fields(input, &tool_name);
    }

    // Apply symlink resolution for file operations (TOB-4 defense)
    if config.enable_symlink_resolution {
        resolve_and_attach_symlinks(input, harness);
    }

    // Future: Apply cross-tool normalizations
    // if config.detect_substitution {
    //     detect_command_substitution(input);
    // }
}

/// Normalize content fields between Write and Edit tools for unified policy access
///
/// This function copies Write's `content` field to `new_string`, allowing policies
/// to use a single field name when checking content for both Write and Edit operations.
///
/// # Why this matters
///
/// Without normalization, policies need duplicate rules:
/// ```rego
/// # Rule for Write
/// deny contains decision if {
///     input.tool_name == "Write"
///     content := input.tool_input.content
///     # ... check content
/// }
///
/// # Duplicate rule for Edit
/// deny contains decision if {
///     input.tool_name == "Edit"
///     content := input.tool_input.new_string
///     # ... same check
/// }
/// ```
///
/// With normalization, a single rule works:
/// ```rego
/// deny contains decision if {
///     input.tool_name in {"Write", "Edit"}
///     content := input.tool_input.new_string  # Works for both!
///     # ... check content
/// }
/// ```
///
/// # Field semantics
///
/// - `new_string`: The content being written (unified field)
/// - `content`: Original Write field (preserved for backwards compatibility)
/// - `old_string`: Edit-only field for the text being replaced
fn normalize_write_edit_content_fields(input: &mut Value, tool_name: &str) {
    // Only normalize Write tool - Edit already has new_string
    if tool_name != "Write" {
        return;
    }

    if let Some(tool_input) = input.get_mut("tool_input") {
        if let Some(tool_input_obj) = tool_input.as_object_mut() {
            // Copy content to new_string if content exists
            if let Some(content) = tool_input_obj.get("content").cloned() {
                tool_input_obj.insert("new_string".to_string(), content);
                trace!("Normalized Write content → new_string for unified policy access");
            }
        }
    }
}

/// Preprocess Claude Code Bash commands to normalize whitespace and inspect scripts
fn preprocess_claude_bash_command(input: &mut Value, config: &PreprocessConfig) {
    // Navigate to tool_input.command (Claude Code structure)
    if let Some(tool_input) = input.get_mut("tool_input") {
        if let Some(command_value) = tool_input.get_mut("command") {
            if let Some(command) = command_value.as_str() {
                let original = command.to_string();
                let mut normalized = WhitespaceNormalizer::normalize_command(&original);

                // Only update if changed
                if original != normalized {
                    *command_value = Value::String(normalized.clone());

                    if config.audit_transformations {
                        debug!(
                            "Normalized Claude Code Bash command: '{}' → '{}'",
                            original, normalized
                        );
                    }
                } else {
                    normalized = original;
                }

                // Check for script execution if enabled
                if config.enable_script_inspection {
                    inspect_and_attach_script(input, &normalized);
                }
            }
        }
    }
}

/// Preprocess Cursor shell commands to normalize whitespace and inspect scripts
fn preprocess_cursor_shell_command(input: &mut Value, config: &PreprocessConfig) {
    // Navigate to command (Cursor structure - command is at root level)
    if let Some(command_value) = input.get_mut("command") {
        if let Some(command) = command_value.as_str() {
            let original = command.to_string();
            let mut normalized = WhitespaceNormalizer::normalize_command(&original);

            // Only update if changed
            if original != normalized {
                *command_value = Value::String(normalized.clone());

                if config.audit_transformations {
                    debug!(
                        "Normalized Cursor shell command: '{}' → '{}'",
                        original, normalized
                    );
                }
            } else {
                normalized = original;
            }

            // Check for script execution if enabled
            if config.enable_script_inspection {
                inspect_and_attach_script(input, &normalized);
            }
        }
    }
}

/// Helper function to inspect command for script execution and attach script content
fn inspect_and_attach_script(input: &mut Value, command: &str) {
    // Check if the command executes a script
    if let Some(script_path) = ScriptInspector::detect_script_execution(command) {
        trace!("Detected script execution: {:?}", script_path);

        // Get the working directory from the event (if available)
        let cwd = input
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(std::path::Path::new);

        // Try to load the script content
        if let Some(script_content) = ScriptInspector::load_script_content(&script_path, cwd) {
            // Attach the script content to the event
            ScriptInspector::attach_script_to_event(input, &script_path, &script_content);

            debug!(
                "Attached script content from {:?} ({} bytes)",
                script_path,
                script_content.len()
            );
        } else {
            trace!("Could not load script content from {:?}", script_path);
        }
    }
}

/// Helper function to resolve symlinks in file paths and attach metadata
///
/// This implements TOB-4 defense by detecting when file paths are symbolic links
/// and resolving them to their canonical target paths. This prevents bypass attacks
/// where attackers create symlinks to protected directories.
fn resolve_and_attach_symlinks(input: &mut Value, harness: HarnessType) {
    // Get the working directory from the event (if available) - shared across all paths
    // Clone the path to avoid borrow checker issues
    let cwd: Option<std::path::PathBuf> = input
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);

    // Extract file path based on tool and harness type
    let file_path_opt = match harness {
        HarnessType::ClaudeCode | HarnessType::Factory => {
            // Claude Code and Factory AI structure: input.tool_input.<field>
            input.get("tool_input").and_then(|tool_input| {
                // Try different field names based on tool type
                // NOTE: Glob 'pattern' field is intentionally excluded - patterns like
                // "src/**/*.rs" are not file paths and should not be canonicalized
                tool_input
                    .get("file_path")
                    .or_else(|| tool_input.get("path"))
                    .or_else(|| tool_input.get("notebook_path"))
            })
        }
        HarnessType::Cursor => {
            // Cursor structure: input.<field> (direct at root)
            input.get("file_path").or_else(|| input.get("path"))
        }
        HarnessType::OpenCode => {
            // OpenCode uses same structure as Claude Code/Factory: input.tool_input.<field>
            input.get("tool_input").and_then(|tool_input| {
                tool_input
                    .get("file_path")
                    .or_else(|| tool_input.get("path"))
                    .or_else(|| tool_input.get("filePath")) // OpenCode may use camelCase
            })
        }
    };

    // Process single file path if present
    if let Some(path_value) = file_path_opt {
        if let Some(path_str) = path_value.as_str() {
            let path_str_owned = path_str.to_string();
            resolve_and_attach_single_path(input, &path_str_owned, cwd.as_deref());
        }
    }

    // Special handling for MultiEdit tool (Claude Code only)
    if harness == HarnessType::ClaudeCode {
        // First, collect all the file paths to avoid borrow checker issues
        let edit_paths: Vec<String> = input
            .get("tool_input")
            .and_then(|ti| ti.get("edits"))
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|edit| {
                        edit.get("file_path")
                            .and_then(|fp| fp.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Now process each edit with mutable access
        if !edit_paths.is_empty() {
            if let Some(tool_input) = input.get_mut("tool_input") {
                if let Some(edits) = tool_input.get_mut("edits") {
                    if let Some(edits_array) = edits.as_array_mut() {
                        for (i, edit) in edits_array.iter_mut().enumerate() {
                            if let Some(path_str) = edit_paths.get(i) {
                                trace!("Canonicalizing MultiEdit file_path: {}", path_str);
                                resolve_and_attach_single_path(edit, path_str, cwd.as_deref());
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Resolve and attach a single file path's canonical form
fn resolve_and_attach_single_path(
    target: &mut Value,
    path_str: &str,
    cwd: Option<&std::path::Path>,
) {
    let path = std::path::Path::new(path_str);

    // TOB-4: ALWAYS canonicalize paths (not just symlinks)
    // This provides defense-in-depth against:
    // - Symlink bypass attacks
    // - Relative path tricks (../../.cupcake/)
    // - Path normalization issues
    trace!("Canonicalizing file path: {}", path_str);

    if let Some(resolved_path) = SymlinkResolver::resolve_path(path, cwd) {
        // Check if this was a symlink
        let is_symlink = SymlinkResolver::is_symlink(path);

        // Attach canonical path metadata to the event/edit
        SymlinkResolver::attach_metadata(target, path_str, &resolved_path, is_symlink);

        debug!(
            "Canonicalized path: {} -> {:?} (symlink: {})",
            path_str, resolved_path, is_symlink
        );
    } else {
        // FALLBACK: Path doesn't exist (e.g., Write creating new file)
        // Still provide a resolved path by manually joining CWD + path
        // This ensures policies ALWAYS have resolved_file_path available
        trace!(
            "Could not canonicalize path: {} (file/parent doesn't exist)",
            path_str
        );

        let fallback_path = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(cwd) = cwd {
            cwd.join(path)
        } else {
            // No CWD provided - use current directory
            std::env::current_dir()
                .ok()
                .map(|c| c.join(path))
                .unwrap_or_else(|| path.to_path_buf())
        };

        // Attach metadata with fallback path (is_symlink = false since we couldn't verify)
        SymlinkResolver::attach_metadata(target, path_str, &fallback_path, false);

        debug!(
            "Using fallback path resolution: {} -> {:?} (non-existent)",
            path_str, fallback_path
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_preprocess_claude_bash_command() {
        let mut input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": "rm  -rf  .cupcake",
                "timeout": 5000
            }
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        assert_eq!(
            input["tool_input"]["command"].as_str().unwrap(),
            "rm -rf .cupcake"
        );
        // Other fields unchanged
        assert_eq!(input["tool_input"]["timeout"].as_i64().unwrap(), 5000);
    }

    #[test]
    fn test_write_content_normalized_to_new_string() {
        // Test that Write's content field is copied to new_string for unified access
        let mut input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": {
                "file_path": "/tmp/test.tsx",
                "content": "<input type=\"date\" />"
            }
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // new_string should be added with same value as content
        assert_eq!(
            input["tool_input"]["new_string"].as_str().unwrap(),
            "<input type=\"date\" />",
            "Write's content should be copied to new_string"
        );

        // Original content field should be preserved
        assert_eq!(
            input["tool_input"]["content"].as_str().unwrap(),
            "<input type=\"date\" />",
            "Original content field should be preserved for backwards compatibility"
        );
    }

    #[test]
    fn test_edit_new_string_unchanged() {
        // Test that Edit's existing new_string is not modified
        let mut input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/tmp/test.tsx",
                "old_string": "old content",
                "new_string": "new content"
            }
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // new_string should remain unchanged
        assert_eq!(
            input["tool_input"]["new_string"].as_str().unwrap(),
            "new content",
            "Edit's new_string should not be modified"
        );

        // old_string should also be unchanged
        assert_eq!(
            input["tool_input"]["old_string"].as_str().unwrap(),
            "old content",
            "Edit's old_string should not be modified"
        );
    }

    #[test]
    fn test_write_edit_unified_field_access() {
        // Test that both Write and Edit can be accessed via new_string
        let config = PreprocessConfig::default();

        // Write input
        let mut write_input = json!({
            "tool_name": "Write",
            "tool_input": {
                "file_path": "/tmp/test.txt",
                "content": "unified content"
            }
        });
        preprocess_input(&mut write_input, &config, HarnessType::ClaudeCode);

        // Edit input
        let mut edit_input = json!({
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/tmp/test.txt",
                "old_string": "old",
                "new_string": "unified content"
            }
        });
        preprocess_input(&mut edit_input, &config, HarnessType::ClaudeCode);

        // Both should have new_string with the same semantic meaning
        assert_eq!(
            write_input["tool_input"]["new_string"].as_str().unwrap(),
            edit_input["tool_input"]["new_string"].as_str().unwrap(),
            "Both Write and Edit should have new_string with same value for unified access"
        );
    }

    #[test]
    fn test_preprocess_cursor_shell_command() {
        let mut input = json!({
            "hook_event_name": "beforeShellExecution",
            "command": "rm  -rf  .cupcake",
            "cwd": "/tmp"
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::Cursor);

        assert_eq!(input["command"].as_str().unwrap(), "rm -rf .cupcake");
        // Other fields unchanged
        assert_eq!(input["cwd"].as_str().unwrap(), "/tmp");
    }

    #[test]
    fn test_preprocess_preserves_non_bash() {
        let mut input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Read",
            "tool_input": {
                "file_path": "test  file.txt"  // Spaces in filename preserved
            }
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // Bash-specific preprocessing (whitespace normalization) is not applied
        // But symlink resolution IS applied to all file operations (including Read)
        assert_eq!(
            input["tool_input"]["file_path"].as_str().unwrap(),
            "test  file.txt",
            "Original file_path should be preserved"
        );

        // Symlink resolution metadata should be attached
        assert!(
            input.get("resolved_file_path").is_some(),
            "Should have resolved_file_path from symlink resolution"
        );
        assert!(
            input.get("original_file_path").is_some(),
            "Should have original_file_path from symlink resolution"
        );
        assert!(
            input.get("is_symlink").is_some(),
            "Should have is_symlink flag from symlink resolution"
        );
    }

    #[test]
    fn test_preprocess_disabled() {
        let mut input = json!({
            "tool_name": "Bash",
            "tool_input": {
                "command": "rm  -rf  test"
            }
        });

        let original = input.clone();
        let config = PreprocessConfig {
            normalize_whitespace: false,
            ..Default::default()
        };

        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // No changes when disabled
        assert_eq!(input, original);
    }

    #[test]
    fn test_preprocess_handles_missing_fields() {
        let mut input = json!({
            "tool_name": "Bash",
            // Missing tool_input
        });

        let config = PreprocessConfig::default();
        // Should not panic
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);
    }

    #[test]
    fn test_preprocess_handles_non_string_command() {
        let mut input = json!({
            "tool_name": "Bash",
            "tool_input": {
                "command": 123  // Not a string
            }
        });

        let config = PreprocessConfig::default();
        // Should not panic
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);
    }

    #[test]
    fn test_symlink_resolution_claude_code_write() {
        #[cfg(unix)]
        use std::os::unix::fs::symlink;
        #[cfg(windows)]
        use std::os::windows::fs::symlink_file as symlink;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");
        let link_path = temp_dir.path().join("link.txt");

        // Create target file
        std::fs::write(&target_path, "test").unwrap();

        // Create symlink
        symlink(&target_path, &link_path).unwrap();

        let mut input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": {
                "file_path": link_path.to_str().unwrap(),
                "content": "data"
            },
            "cwd": temp_dir.path().to_str().unwrap()
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // Should have symlink metadata attached
        assert_eq!(input["is_symlink"], json!(true));
        assert!(input.get("resolved_file_path").is_some());
        assert!(input.get("original_file_path").is_some());
    }

    #[test]
    fn test_symlink_resolution_cursor_file_edit() {
        #[cfg(unix)]
        use std::os::unix::fs::symlink;
        #[cfg(windows)]
        use std::os::windows::fs::symlink_file as symlink;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");
        let link_path = temp_dir.path().join("link.txt");

        // Create target file
        std::fs::write(&target_path, "test").unwrap();

        // Create symlink
        symlink(&target_path, &link_path).unwrap();

        let mut input = json!({
            "hook_event_name": "beforeFileEdit",
            "file_path": link_path.to_str().unwrap(),
            "cwd": temp_dir.path().to_str().unwrap()
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::Cursor);

        // Should have symlink metadata attached
        assert_eq!(input["is_symlink"], json!(true));
        assert!(input.get("resolved_file_path").is_some());
    }

    #[test]
    fn test_symlink_resolution_disabled() {
        #[cfg(unix)]
        use std::os::unix::fs::symlink;
        #[cfg(windows)]
        use std::os::windows::fs::symlink_file as symlink;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");
        let link_path = temp_dir.path().join("link.txt");

        // Create target and symlink
        std::fs::write(&target_path, "test").unwrap();
        symlink(&target_path, &link_path).unwrap();

        let mut input = json!({
            "tool_name": "Write",
            "tool_input": {
                "file_path": link_path.to_str().unwrap()
            }
        });

        // Disable symlink resolution
        let config = PreprocessConfig {
            enable_symlink_resolution: false,
            ..Default::default()
        };
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // Should NOT have symlink metadata
        assert!(input.get("is_symlink").is_none());
        assert!(input.get("resolved_file_path").is_none());
    }

    #[test]
    fn test_symlink_resolution_non_symlink() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("regular.txt");

        // Create regular file (not a symlink)
        std::fs::write(&file_path, "test").unwrap();

        let mut input = json!({
            "tool_name": "Read",
            "tool_input": {
                "file_path": file_path.to_str().unwrap()
            }
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut input, &config, HarnessType::ClaudeCode);

        // Always-on approach: Regular files SHOULD have canonical path metadata
        assert_eq!(
            input["is_symlink"],
            json!(false),
            "Regular file should be marked as NOT a symlink"
        );
        assert!(
            input.get("resolved_file_path").is_some(),
            "Should ALWAYS have canonical path"
        );
    }
}
