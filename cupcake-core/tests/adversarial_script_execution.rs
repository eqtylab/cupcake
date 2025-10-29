//! Adversarial test suite for TOB-EQTY-LAB-CUPCAKE-2 Script Execution
//! Tests defense against cross-tool bypass via script creation and execution
//!
//! TOB-2 Attack Pattern:
//! 1. Write tool creates a script with dangerous commands
//! 2. Bash tool executes the script
//! 3. Policies that only check Bash commands miss the dangerous content

#[cfg(test)]
mod tests {
    use cupcake_core::harness::types::HarnessType;
    use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
    use cupcake_core::preprocessing::script_inspector::ScriptInspector;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Test that script inspection detects various script execution patterns
    #[test]
    fn test_script_detection_patterns() {
        // Direct execution
        assert!(ScriptInspector::detect_script_execution("./deploy.sh").is_some());
        assert!(ScriptInspector::detect_script_execution("./scripts/build.sh --prod").is_some());
        assert!(ScriptInspector::detect_script_execution("/usr/local/bin/script.sh").is_some());

        // Interpreter execution
        assert!(ScriptInspector::detect_script_execution("bash deploy.sh").is_some());
        assert!(ScriptInspector::detect_script_execution("sh -x deploy.sh").is_some());
        assert!(ScriptInspector::detect_script_execution("python3 script.py").is_some());
        assert!(ScriptInspector::detect_script_execution("node server.js").is_some());
        assert!(ScriptInspector::detect_script_execution("ruby script.rb").is_some());

        // With flags
        assert!(ScriptInspector::detect_script_execution("bash -e deploy.sh").is_some());
        assert!(ScriptInspector::detect_script_execution("python -u script.py").is_some());

        // Should NOT detect non-script commands
        assert!(ScriptInspector::detect_script_execution("rm -rf test").is_none());
        assert!(ScriptInspector::detect_script_execution("ls -la").is_none());
        assert!(ScriptInspector::detect_script_execution("git commit -m 'test'").is_none());
        assert!(ScriptInspector::detect_script_execution("echo hello").is_none());

        // Should NOT detect bash -c (command string, not file)
        assert!(ScriptInspector::detect_script_execution("bash -c 'echo hello'").is_none());
    }

    /// Test that script content is loaded and attached to events
    #[test]
    fn test_script_content_loading() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("dangerous.sh");

        // Write a test script with dangerous content
        let script_content = r#"#!/bin/bash
# Innocent looking script
echo "Starting..."

# Hidden dangerous commands
rm -rf .cupcake
rm -rf /important

echo "Done!"
"#;
        fs::write(&script_path, script_content).unwrap();

        // Test loading with absolute path
        let loaded = ScriptInspector::load_script_content(&script_path, None);
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), script_content);

        // Test loading with relative path and cwd
        let loaded = ScriptInspector::load_script_content(
            &PathBuf::from("dangerous.sh"),
            Some(temp_dir.path()),
        );
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), script_content);
    }

    /// Test preprocessing with script inspection for Claude Code
    #[test]
    fn test_claude_code_script_inspection() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("deploy.sh");

        // Write a test script
        fs::write(&script_path, "#!/bin/bash\nrm -rf /important").unwrap();

        let mut event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": "./deploy.sh --production"
            },
            "cwd": temp_dir.path().to_string_lossy()
        });

        // Preprocess with script inspection enabled
        let config = PreprocessConfig::with_script_inspection();
        preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

        // Verify script content was attached
        assert!(event.get("executed_script_content").is_some());
        assert!(event.get("executed_script_path").is_some());
        assert_eq!(event["script_inspection_performed"], true);

        let content = event["executed_script_content"].as_str().unwrap();
        assert!(content.contains("rm -rf /important"));
    }

    /// Test preprocessing with script inspection for Cursor
    #[test]
    fn test_cursor_script_inspection() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("build.sh");

        // Write a test script
        fs::write(&script_path, "#!/bin/bash\nrm -rf .cupcake").unwrap();

        let mut event = json!({
            "hook_event_name": "beforeShellExecution",
            "command": "bash build.sh",
            "cwd": temp_dir.path().to_string_lossy()
        });

        // Preprocess with script inspection enabled
        let config = PreprocessConfig::with_script_inspection();
        preprocess_input(&mut event, &config, HarnessType::Cursor);

        // Verify script content was attached
        assert!(event.get("executed_script_content").is_some());
        assert!(event.get("executed_script_path").is_some());

        let content = event["executed_script_content"].as_str().unwrap();
        assert!(content.contains("rm -rf .cupcake"));
    }

    /// Test that script inspection is disabled by default
    #[test]
    fn test_script_inspection_disabled_by_default() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("script.sh");
        fs::write(&script_path, "#!/bin/bash\necho test").unwrap();

        let mut event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": "./script.sh"
            },
            "cwd": temp_dir.path().to_string_lossy()
        });

        // Use default config (script inspection disabled)
        let config = PreprocessConfig::default();
        preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

        // Verify NO script content was attached
        assert!(event.get("executed_script_content").is_none());
        assert!(event.get("executed_script_path").is_none());
    }

    /// Test various script execution patterns with different interpreters
    #[test]
    fn test_multiple_interpreter_patterns() {
        let temp_dir = TempDir::new().unwrap();

        // Create different script files
        let scripts = vec![
            ("deploy.sh", "#!/bin/bash\necho bash script"),
            ("script.py", "#!/usr/bin/env python3\nprint('python script')"),
            ("app.js", "console.log('node script');"),
            ("script.rb", "puts 'ruby script'"),
        ];

        for (filename, content) in &scripts {
            fs::write(temp_dir.path().join(filename), content).unwrap();
        }

        // Test patterns that should be detected
        let test_cases = vec![
            ("./deploy.sh", "deploy.sh"),
            ("bash deploy.sh", "deploy.sh"),
            ("sh deploy.sh", "deploy.sh"),
            ("python3 script.py", "script.py"),
            ("python script.py --verbose", "script.py"),
            ("node app.js", "app.js"),
            ("ruby script.rb", "script.rb"),
        ];

        for (command, expected_script) in test_cases {
            let mut event = json!({
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": {
                    "command": command
                },
                "cwd": temp_dir.path().to_string_lossy()
            });

            let config = PreprocessConfig::with_script_inspection();
            preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

            assert!(
                event.get("executed_script_content").is_some(),
                "Command '{}' should attach script content", command
            );

            let attached_path = event["executed_script_path"].as_str().unwrap();
            assert!(
                attached_path.contains(expected_script),
                "Command '{}' should detect script '{}', got '{}'",
                command, expected_script, attached_path
            );
        }
    }

    /// Test that missing scripts don't cause errors
    #[test]
    fn test_missing_script_handling() {
        let temp_dir = TempDir::new().unwrap();

        let mut event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": "./nonexistent.sh"
            },
            "cwd": temp_dir.path().to_string_lossy()
        });

        // Should not panic even if script doesn't exist
        let config = PreprocessConfig::with_script_inspection();
        preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

        // No script content should be attached
        assert!(event.get("executed_script_content").is_none());
    }

    /// Test edge cases in script detection
    #[test]
    fn test_script_detection_edge_cases() {
        // Empty command
        assert!(ScriptInspector::detect_script_execution("").is_none());

        // Just interpreter name
        assert!(ScriptInspector::detect_script_execution("bash").is_none());
        assert!(ScriptInspector::detect_script_execution("python").is_none());

        // Common script names without extensions
        assert!(ScriptInspector::detect_script_execution("./configure").is_some());
        assert!(ScriptInspector::detect_script_execution("./bootstrap").is_some());
        assert!(ScriptInspector::detect_script_execution("./gradlew").is_some());

        // Scripts with arguments
        assert!(ScriptInspector::detect_script_execution("./deploy.sh --prod --verbose").is_some());
        assert!(ScriptInspector::detect_script_execution("python script.py arg1 arg2").is_some());

        // Complex paths
        assert!(ScriptInspector::detect_script_execution("/usr/local/bin/custom.sh").is_some());
        assert!(ScriptInspector::detect_script_execution("./scripts/nested/deep/build.sh").is_some());
    }
}