//! Tests for Cursor-specific preprocessing functionality
//!
//! Cursor uses different event structures than Claude Code:
//! - Event name: "beforeShellExecution" (not "PreToolUse")
//! - Command location: input.command (not input.tool_input.command)

use cupcake_core::harness::types::HarnessType;
use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
use serde_json::json;

#[test]
fn test_cursor_before_shell_execution_basic_normalization() {
    let mut event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "rm  -rf  test",  // Double spaces
        "cwd": "/tmp"
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::Cursor);

    assert_eq!(
        event["command"].as_str().unwrap(),
        "rm -rf test",
        "Command should be normalized"
    );
}

#[test]
fn test_cursor_multiple_spaces_patterns() {
    let test_cases = vec![
        ("ls  -la", "ls -la"),                                  // Double space
        ("git   commit   -m   'test'", "git commit -m 'test'"), // Triple spaces
        ("  npm install  ", "npm install"),                     // Leading/trailing
        ("docker\trun\tnginx", "docker run nginx"),             // Tabs
        ("echo\n\nhello", "echo hello"),                        // Newlines
    ];

    for (input_cmd, expected) in test_cases {
        let mut event = json!({
            "hook_event_name": "beforeShellExecution",
            "command": input_cmd,
            "cwd": "/workspace"
        });

        let config = PreprocessConfig::default();
        preprocess_input(&mut event, &config, HarnessType::Cursor);

        assert_eq!(
            event["command"].as_str().unwrap(),
            expected,
            "Failed for input: '{input_cmd}'"
        );
    }
}

#[test]
fn test_cursor_quote_preservation() {
    let mut event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "echo  'spaces  inside  quotes'  should  preserve",
        "cwd": "/workspace"
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::Cursor);

    assert_eq!(
        event["command"].as_str().unwrap(),
        "echo 'spaces  inside  quotes' should preserve",
        "Spaces inside quotes should be preserved"
    );
}

#[test]
fn test_cursor_after_file_edit_not_normalized() {
    // Whitespace normalization only applies to shell commands
    // But symlink resolution applies to ALL file operations (including afterFileEdit)
    let mut event = json!({
        "hook_event_name": "afterFileEdit",
        "file_path": "file  with  spaces.txt",
        "content": "content  with  spaces"
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::Cursor);

    // Original file_path and content should be preserved (no whitespace normalization)
    assert_eq!(
        event["file_path"].as_str().unwrap(),
        "file  with  spaces.txt",
        "File path should preserve spaces"
    );
    assert_eq!(
        event["content"].as_str().unwrap(),
        "content  with  spaces",
        "Content should preserve spaces"
    );

    // But symlink resolution metadata should be added
    assert!(
        event.get("resolved_file_path").is_some(),
        "Should have resolved_file_path from symlink resolution"
    );
    assert!(
        event.get("original_file_path").is_some(),
        "Should have original_file_path from symlink resolution"
    );
    assert!(
        event.get("is_symlink").is_some(),
        "Should have is_symlink flag from symlink resolution"
    );
}

#[test]
fn test_cursor_disabled_preprocessing() {
    let mut event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "rm  -rf  test",
        "cwd": "/tmp"
    });

    let original = event.clone();
    let config = PreprocessConfig::disabled();
    preprocess_input(&mut event, &config, HarnessType::Cursor);

    assert_eq!(
        event, original,
        "Disabled preprocessing should not modify event"
    );
}

#[test]
fn test_cursor_complex_command_with_pipes() {
    let mut event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "ps  aux  |  grep  node  |  wc  -l",
        "cwd": "/tmp"
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::Cursor);

    assert_eq!(
        event["command"].as_str().unwrap(),
        "ps aux | grep node | wc -l",
        "Piped commands should be normalized"
    );
}

#[test]
fn test_cursor_script_execution_detection() {
    // This test will check if script content is loaded when executing a script
    // Will be implemented after script inspection feature is added

    let mut event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "./deploy.sh --production",
        "cwd": "/workspace"
    });

    let config = PreprocessConfig {
        normalize_whitespace: true,
        audit_transformations: false,
        enable_script_inspection: false, // Not testing script inspection in this test
        enable_symlink_resolution: false, // Not testing symlink resolution in this test
    };

    // For now, just test that command is normalized
    preprocess_input(&mut event, &config, HarnessType::Cursor);

    assert_eq!(
        event["command"].as_str().unwrap(),
        "./deploy.sh --production"
    );

    // TODO: After script inspection is implemented:
    // assert!(event.get("executed_script_content").is_some());
    // assert_eq!(event["executed_script_path"], "./deploy.sh");
}

#[test]
fn test_claude_code_not_affected_by_cursor_logic() {
    // Ensure Claude Code events still work with their structure
    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm  -rf  test"
        }
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

    assert_eq!(
        event["tool_input"]["command"].as_str().unwrap(),
        "rm -rf test",
        "Claude Code events should still work"
    );
}
