//! Integration tests for audit logging functionality
//! 
//! This test suite validates that command execution audit logs are properly
//! written to files or stdout based on configuration.

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;
use tempfile::tempdir;

#[tokio::test]
async fn test_audit_logs_to_file() {
    // Create a temporary directory for audit logs
    let temp_dir = tempdir().unwrap();
    let audit_dir = temp_dir.path().join(".cupcake").join("audit");
    
    // Set HOME to temp directory so audit logs go there
    std::env::set_var("HOME", temp_dir.path());
    
    let mut vars = HashMap::new();
    vars.insert("greeting".to_string(), "hello".to_string());
    
    let settings = Settings {
        audit_logging: true,
        debug_mode: true,
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    
    // Execute a simple command
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["{{greeting}}".to_string(), "world".to_string()]),
        working_dir: None,
        env: None,
        pipe: None,
        redirect_stdout: None,
        append_stdout: None,
        redirect_stderr: None,
        merge_stderr: None,
        on_success: None,
        on_failure: None,
    });
    
    let result = executor.execute_spec(&spec).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.success);
    
    // Wait a moment for file to be written
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Check that audit log file was created
    let files: Vec<_> = std::fs::read_dir(&audit_dir)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    
    assert_eq!(files.len(), 1);
    let audit_file = &files[0];
    let file_name = audit_file.file_name();
    let file_name_str = file_name.to_string_lossy();
    
    // Verify file name format
    assert!(file_name_str.starts_with("exec-"));
    assert!(file_name_str.ends_with(".jsonl"));
    
    // Read and verify audit log content
    let content = std::fs::read_to_string(audit_file.path()).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    
    // Parse JSON and verify fields
    let audit_record: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert!(audit_record["graph"].is_string());
    assert_eq!(audit_record["mode"], "array");
    assert_eq!(audit_record["argv"], serde_json::json!(["echo", "hello", "world"]));
    assert!(audit_record["timestamp"].is_string());
    assert_eq!(audit_record["exit_code"], 0);
    assert!(audit_record["duration_ms"].is_number());
    assert_eq!(audit_record["shell_used"], false);
}

#[tokio::test]
async fn test_audit_disabled() {
    let temp_dir = tempdir().unwrap();
    let audit_dir = temp_dir.path().join(".cupcake").join("audit");
    
    // Set HOME to temp directory
    std::env::set_var("HOME", temp_dir.path());
    
    let settings = Settings {
        audit_logging: false, // Audit logging disabled
        debug_mode: true,
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Execute a simple command
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["test".to_string()]),
        working_dir: None,
        env: None,
        pipe: None,
        redirect_stdout: None,
        append_stdout: None,
        redirect_stderr: None,
        merge_stderr: None,
        on_success: None,
        on_failure: None,
    });
    
    let result = executor.execute_spec(&spec).await.unwrap();
    assert_eq!(result.exit_code, 0);
    
    // Verify no audit log was created
    assert!(!audit_dir.exists());
}

#[tokio::test]
async fn test_audit_shell_command_tracking() {
    let temp_dir = tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());
    
    let settings = Settings {
        audit_logging: true,
        debug_mode: true,
        allow_shell: true, // Enable shell
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Execute a shell command
    let spec = CommandSpec::Shell(cupcake::config::actions::ShellCommandSpec {
        script: "echo 'shell test'".to_string(),
    });
    
    let result = executor.execute_spec(&spec).await.unwrap();
    assert_eq!(result.exit_code, 0);
    
    // Wait a moment for file to be written
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Read audit log and verify shell_used flag
    let audit_dir = temp_dir.path().join(".cupcake").join("audit");
    let files: Vec<_> = std::fs::read_dir(&audit_dir)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    
    let content = std::fs::read_to_string(files[0].path()).unwrap();
    let audit_record: serde_json::Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
    
    assert_eq!(audit_record["mode"], "shell");
    assert_eq!(audit_record["shell_used"], true);
    assert_eq!(audit_record["argv"], serde_json::json!(["/bin/sh", "-c", "echo 'shell test'"]));
}