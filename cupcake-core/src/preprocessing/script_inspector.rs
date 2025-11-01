//! Script inspection module for detecting and loading executed scripts
//!
//! This module implements TOB-2 defense by detecting when a shell command
//! executes a script file and loading its content for policy evaluation.

use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Script inspector for detecting and loading script execution
pub struct ScriptInspector;

impl ScriptInspector {
    /// Detect if a command executes a script and return its path
    ///
    /// Detects patterns like:
    /// - `./script.sh` - Direct execution
    /// - `bash script.sh` - Explicit interpreter
    /// - `python script.py` - Language interpreter
    /// - `sh -c script.sh` - Shell with script
    /// - `/path/to/script.sh` - Absolute path
    pub fn detect_script_execution(command: &str) -> Option<PathBuf> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        // Check for direct script execution (starts with ./ or /)
        if let Some(first) = parts.first() {
            if first.starts_with("./") || first.starts_with("/") {
                // Check if it looks like a script file
                if is_likely_script(first) {
                    return Some(PathBuf::from(first));
                }
            }
        }

        // Check for interpreter patterns
        if parts.len() >= 2 {
            let interpreter = parts[0];
            let mut script_path = None;

            match interpreter {
                // Shell interpreters
                "bash" | "sh" | "zsh" | "fish" | "ksh" | "dash" => {
                    // Handle flags like: bash -c script.sh or bash -x script.sh
                    for (i, part) in parts.iter().enumerate().skip(1) {
                        if !part.starts_with('-') {
                            script_path = Some(PathBuf::from(part));
                            break;
                        } else if *part == "-c" && i + 1 < parts.len() {
                            // -c means the next arg is a command string, not a file
                            return None;
                        }
                    }
                }
                // Scripting language interpreters
                "python" | "python3" | "python2" | "node" | "ruby" | "perl" | "php" => {
                    // Find the first non-flag argument
                    for part in parts.iter().skip(1) {
                        if !part.starts_with('-') {
                            script_path = Some(PathBuf::from(part));
                            break;
                        }
                    }
                }
                _ => {}
            }

            if let Some(path) = script_path {
                if is_likely_script(&path.to_string_lossy()) {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Load the content of a script file
    ///
    /// Returns the script content if the file exists and is readable,
    /// or None if the file cannot be read.
    pub fn load_script_content(script_path: &Path, cwd: Option<&Path>) -> Option<String> {
        // Resolve the path relative to the working directory if provided
        let resolved_path = if script_path.is_absolute() {
            script_path.to_path_buf()
        } else if let Some(cwd) = cwd {
            cwd.join(script_path)
        } else {
            script_path.to_path_buf()
        };

        debug!("Attempting to load script from: {:?}", resolved_path);

        match fs::read_to_string(&resolved_path) {
            Ok(content) => {
                debug!(
                    "Successfully loaded script: {:?} ({} bytes)",
                    resolved_path,
                    content.len()
                );
                Some(content)
            }
            Err(e) => {
                // Don't warn for missing files - the script might not exist yet
                // or might be created dynamically
                debug!("Could not load script {:?}: {}", resolved_path, e);
                None
            }
        }
    }

    /// Attach script content to the event JSON
    ///
    /// Adds the script content and metadata to the event for policy evaluation
    pub fn attach_script_to_event(
        event: &mut serde_json::Value,
        script_path: &Path,
        script_content: &str,
    ) {
        if let Some(obj) = event.as_object_mut() {
            // Add the script content
            obj.insert(
                "executed_script_content".to_string(),
                serde_json::Value::String(script_content.to_string()),
            );

            // Add the script path for reference
            obj.insert(
                "executed_script_path".to_string(),
                serde_json::Value::String(script_path.to_string_lossy().to_string()),
            );

            // Add a flag indicating script inspection occurred
            obj.insert(
                "script_inspection_performed".to_string(),
                serde_json::Value::Bool(true),
            );

            debug!("Attached script content to event: {:?}", script_path);
        }
    }
}

/// Check if a path is likely a script file based on extension or pattern
fn is_likely_script(path: &str) -> bool {
    // Common script extensions
    let script_extensions = [
        ".sh", ".bash", ".zsh", ".fish", ".ksh", // Shell scripts
        ".py", ".pyw", // Python
        ".js", ".mjs", ".cjs", // JavaScript
        ".rb",  // Ruby
        ".pl", ".pm",  // Perl
        ".php", // PHP
        ".lua", // Lua
        ".r", ".R",   // R
        ".jl",  // Julia
        ".tcl", // Tcl
        ".awk", // AWK
        ".sed", // Sed
    ];

    // Check extensions
    for ext in &script_extensions {
        if path.ends_with(ext) {
            return true;
        }
    }

    // Check for common script patterns without extensions
    let base_name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);

    // Common scriptnames without extensions
    matches!(
        base_name,
        "configure"
            | "install"
            | "build"
            | "deploy"
            | "setup"
            | "bootstrap"
            | "run"
            | "test"
            | "clean"
            | "make"
            | "gradlew"
            | "mvnw"
            | "manage" // Build tool wrappers
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_direct_script_execution() {
        // Direct execution with ./
        assert_eq!(
            ScriptInspector::detect_script_execution("./deploy.sh --production"),
            Some(PathBuf::from("./deploy.sh"))
        );

        // Absolute path
        assert_eq!(
            ScriptInspector::detect_script_execution("/usr/local/bin/script.sh"),
            Some(PathBuf::from("/usr/local/bin/script.sh"))
        );

        // Not a script
        assert_eq!(
            ScriptInspector::detect_script_execution("./binary_executable"),
            None
        );
    }

    #[test]
    fn test_detect_interpreter_execution() {
        // Bash
        assert_eq!(
            ScriptInspector::detect_script_execution("bash deploy.sh"),
            Some(PathBuf::from("deploy.sh"))
        );

        // Bash with flags
        assert_eq!(
            ScriptInspector::detect_script_execution("bash -x deploy.sh --prod"),
            Some(PathBuf::from("deploy.sh"))
        );

        // Bash -c (command string, not file)
        assert_eq!(
            ScriptInspector::detect_script_execution("bash -c 'echo hello'"),
            None
        );

        // Bash with flags before -c (should still return None)
        assert_eq!(
            ScriptInspector::detect_script_execution("bash -x -c script.sh"),
            None
        );

        // Python
        assert_eq!(
            ScriptInspector::detect_script_execution("python3 script.py --verbose"),
            Some(PathBuf::from("script.py"))
        );

        // Node
        assert_eq!(
            ScriptInspector::detect_script_execution("node server.js"),
            Some(PathBuf::from("server.js"))
        );
    }

    #[test]
    fn test_detect_no_script() {
        // Regular commands
        assert_eq!(ScriptInspector::detect_script_execution("ls -la"), None);
        assert_eq!(ScriptInspector::detect_script_execution("git status"), None);
        assert_eq!(
            ScriptInspector::detect_script_execution("rm -rf test"),
            None
        );
        assert_eq!(ScriptInspector::detect_script_execution("echo hello"), None);
    }

    #[test]
    fn test_is_likely_script() {
        // Shell scripts
        assert!(is_likely_script("deploy.sh"));
        assert!(is_likely_script("script.bash"));
        assert!(is_likely_script("/path/to/script.zsh"));

        // Other languages
        assert!(is_likely_script("main.py"));
        assert!(is_likely_script("app.js"));
        assert!(is_likely_script("script.rb"));

        // Common script names without extensions
        assert!(is_likely_script("configure"));
        assert!(is_likely_script("./bootstrap"));
        assert!(is_likely_script("/usr/local/bin/deploy"));

        // Not scripts
        assert!(!is_likely_script("document.txt"));
        assert!(!is_likely_script("image.png"));
        assert!(!is_likely_script("binary"));
    }

    #[test]
    fn test_attach_script_to_event() {
        let mut event = serde_json::json!({
            "tool_name": "Bash",
            "command": "./deploy.sh"
        });

        let script_path = Path::new("./deploy.sh");
        let script_content = "#!/bin/bash\nrm -rf /important";

        ScriptInspector::attach_script_to_event(&mut event, script_path, script_content);

        assert_eq!(
            event["executed_script_content"].as_str().unwrap(),
            script_content
        );
        assert_eq!(
            event["executed_script_path"].as_str().unwrap(),
            "./deploy.sh"
        );
        assert!(event["script_inspection_performed"].as_bool().unwrap());
    }
}
