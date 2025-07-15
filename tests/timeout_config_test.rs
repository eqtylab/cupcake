//! Tests for configurable timeout functionality
//! 
//! This test validates that the timeout_ms setting properly controls
//! command execution timeouts.

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::{CommandExecutor, ExecutionError};
use std::collections::HashMap;

#[tokio::test]
async fn test_custom_timeout_short() {
    let settings = Settings {
        audit_logging: false,
        debug_mode: true,
        allow_shell: false,
        timeout_ms: 100, // Very short timeout - 100ms
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Command that takes longer than 100ms
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["sleep".to_string()],
        args: Some(vec!["0.2".to_string()]), // Sleep for 200ms
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
    
    let result = executor.execute_spec(&spec).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ExecutionError::Timeout => (),
        other => panic!("Expected Timeout error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_custom_timeout_long() {
    let settings = Settings {
        audit_logging: false,
        debug_mode: true,
        allow_shell: false,
        timeout_ms: 5000, // 5 second timeout
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Command that completes quickly
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["quick test".to_string()]),
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
}

#[tokio::test]
async fn test_default_timeout() {
    // Test that default timeout is applied when not specified
    let settings = Settings::default();
    assert_eq!(settings.timeout_ms, 30000); // 30 seconds
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Quick command should complete within default timeout
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["default timeout test".to_string()]),
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
}

#[test]
fn test_timeout_serialization() {
    // Test that timeout_ms can be deserialized from YAML
    let yaml = r#"
audit_logging: true
debug_mode: false
allow_shell: true
timeout_ms: 60000
"#;
    
    let settings: Settings = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(settings.timeout_ms, 60000);
    
    // Test that default is used when not specified
    let yaml_no_timeout = r#"
audit_logging: true
debug_mode: false
"#;
    
    let settings2: Settings = serde_yaml_ng::from_str(yaml_no_timeout).unwrap();
    assert_eq!(settings2.timeout_ms, 30000); // Default
}