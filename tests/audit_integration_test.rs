//! Integration tests for audit logging functionality
//! 
//! This test suite validates that command execution audit logs are properly
//! written to files or stdout based on configuration.
//! 
//! NOTE: These tests modify the HOME environment variable and must run serially
//! to avoid race conditions. Use `cargo test -- --test-threads=1` if needed.

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::tempdir;

// Global mutex to ensure audit tests run serially (they modify HOME env var)
static AUDIT_TEST_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_audit_logs_to_file() {
    // Acquire mutex to run serially (modifies global HOME env var)
    let _guard = AUDIT_TEST_MUTEX.lock().unwrap();
    
    // Store original HOME for restoration
    let original_home = std::env::var("HOME").ok();
    
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
        timeout_ms: 30000,
        sandbox_uid: None,
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    
    // Execute a simple command
    let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
    }));
    
    let result = executor.execute_spec(&spec).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.success);
    
    // Wait a moment for file to be written (increased wait time)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Check that audit log file was created
    if !audit_dir.exists() {
        eprintln!("DEBUG: HOME env var: {:?}", std::env::var("HOME"));
        eprintln!("DEBUG: Expected audit dir: {:?}", audit_dir);
        eprintln!("DEBUG: Temp dir contents:");
        if let Ok(entries) = std::fs::read_dir(temp_dir.path()) {
            for entry in entries {
                if let Ok(entry) = entry {
                    eprintln!("  {:?}", entry.path());
                }
            }
        }
        if let Ok(cupcake_dir) = std::fs::read_dir(temp_dir.path().join(".cupcake")) {
            eprintln!("DEBUG: .cupcake dir contents:");
            for entry in cupcake_dir {
                if let Ok(entry) = entry {
                    eprintln!("  {:?}", entry.path());
                }
            }
        }
        panic!("Audit directory does not exist: {:?}", audit_dir);
    }
    
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
    
    // Debug: print the audit log content if assertion fails
    if lines.len() != 1 {
        eprintln!("Expected 1 audit log line, got {}:", lines.len());
        for (i, line) in lines.iter().enumerate() {
            eprintln!("Line {}: {}", i, line);
        }
    }
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
    
    // Restore original HOME environment variable
    match original_home {
        Some(home) => std::env::set_var("HOME", home),
        None => std::env::remove_var("HOME"),
    }
}

#[tokio::test]
async fn test_audit_disabled() {
    // Acquire mutex to run serially (modifies global HOME env var)
    let _guard = AUDIT_TEST_MUTEX.lock().unwrap();
    
    let temp_dir = tempdir().unwrap();
    let audit_dir = temp_dir.path().join(".cupcake").join("audit");
    
    // Set HOME to temp directory
    std::env::set_var("HOME", temp_dir.path());
    
    let settings = Settings {
        audit_logging: false, // Audit logging disabled
        debug_mode: true,
        allow_shell: false,
        timeout_ms: 30000,
        sandbox_uid: None,
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Execute a simple command
    let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
    }));
    
    let result = executor.execute_spec(&spec).await.unwrap();
    assert_eq!(result.exit_code, 0);
    
    // Verify no audit log was created
    assert!(!audit_dir.exists());
}

#[tokio::test]
async fn test_audit_shell_command_tracking() {
    // Acquire mutex to run serially (modifies global HOME env var)
    let _guard = AUDIT_TEST_MUTEX.lock().unwrap();
    
    let temp_dir = tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());
    
    let settings = Settings {
        audit_logging: true,
        debug_mode: true,
        allow_shell: true, // Enable shell
        timeout_ms: 30000,
        sandbox_uid: None,
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
    
    if !audit_dir.exists() {
        panic!("Audit directory does not exist: {:?}", audit_dir);
    }
    
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