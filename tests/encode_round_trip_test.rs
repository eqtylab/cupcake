//! Round-trip tests for encode functionality
//! 
//! This test suite verifies that commands encoded by `cupcake encode` can be
//! successfully executed by the CommandExecutor, ensuring the encode tool
//! produces valid and secure command specifications.

use cupcake::cli::commands::encode::EncodeCommand;
use cupcake::config::actions::{ArrayCommandSpec, CommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;
use tokio;

#[tokio::test]
async fn test_encode_and_execute_simple_command() {
    // Test encoding and executing a simple echo command
    let shell_cmd = "echo 'hello world'";
    
    // Step 1: Encode the shell command
    let encode_cmd = EncodeCommand::new(
        shell_cmd.to_string(),
        "yaml".to_string(),
        false,
    );
    
    let array_spec = encode_cmd.parse_shell_to_array(shell_cmd).unwrap();
    
    // Step 2: Execute the encoded command
    let vars = HashMap::new();
    let settings = Settings {
        audit_logging: true,
        debug_mode: true, // Skip UID dropping for tests
        allow_shell: false, // This is array mode, not shell mode
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    let result = executor.execute_spec(&CommandSpec::Array(array_spec)).await.unwrap();
    
    // Step 3: Verify execution succeeded
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.is_some());
    assert_eq!(result.stdout.unwrap().trim(), "hello world");
}

#[tokio::test]
async fn test_encode_and_execute_piped_command() {
    // Test encoding and executing a command with pipes
    let shell_cmd = "echo 'apple banana cherry' | grep banana";
    
    // Step 1: Encode the shell command
    let encode_cmd = EncodeCommand::new(
        shell_cmd.to_string(),
        "yaml".to_string(),
        false,
    );
    
    let array_spec = encode_cmd.parse_shell_to_array(shell_cmd).unwrap();
    
    // Verify pipe structure
    assert_eq!(array_spec.command, vec!["echo"]);
    assert_eq!(array_spec.args, Some(vec!["apple banana cherry".to_string()]));
    assert!(array_spec.pipe.is_some());
    
    let pipes = array_spec.pipe.as_ref().unwrap();
    assert_eq!(pipes.len(), 1);
    assert_eq!(pipes[0].cmd, vec!["grep", "banana"]);
    
    // Step 2: Execute the encoded command
    let vars = HashMap::new();
    let settings = Settings {
        audit_logging: true,
        debug_mode: true, // Skip UID dropping for tests
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    let result = executor.execute_spec(&CommandSpec::Array(array_spec)).await.unwrap();
    
    // Step 3: Verify execution succeeded
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.is_some());
    assert_eq!(result.stdout.unwrap().trim(), "apple banana cherry");
}

#[tokio::test]
async fn test_encode_and_execute_redirected_command() {
    // Test encoding and executing a command with output redirection
    let temp_file = format!("/tmp/cupcake_test_{}.txt", std::process::id());
    let shell_cmd = format!("echo 'test content' > {}", temp_file);
    
    // Step 1: Encode the shell command
    let encode_cmd = EncodeCommand::new(
        shell_cmd.clone(),
        "yaml".to_string(),
        false,
    );
    
    let array_spec = encode_cmd.parse_shell_to_array(&shell_cmd).unwrap();
    
    // Verify redirect structure
    assert_eq!(array_spec.command, vec!["echo"]);
    assert_eq!(array_spec.args, Some(vec!["test content".to_string()]));
    assert_eq!(array_spec.redirect_stdout, Some(temp_file.clone()));
    
    // Step 2: Execute the encoded command
    let vars = HashMap::new();
    let settings = Settings {
        audit_logging: true,
        debug_mode: true, // Skip UID dropping for tests
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    let result = executor.execute_spec(&CommandSpec::Array(array_spec)).await.unwrap();
    
    // Step 3: Verify execution succeeded
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    
    // Step 4: Verify file was created with correct content
    let file_content = std::fs::read_to_string(&temp_file).unwrap();
    assert_eq!(file_content.trim(), "test content");
    
    // Cleanup
    let _ = std::fs::remove_file(&temp_file);
}

#[tokio::test]
async fn test_encode_and_execute_with_template_variables() {
    // Test encoding and executing with template variable substitution
    let shell_cmd = "echo 'Hello {{name}}, your session is {{session_id}}'";
    
    // Step 1: Encode the shell command
    let encode_cmd = EncodeCommand::new(
        shell_cmd.to_string(),
        "yaml".to_string(),
        false,
    );
    
    let array_spec = encode_cmd.parse_shell_to_array(shell_cmd).unwrap();
    
    // Step 2: Execute with template variables
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), "Alice".to_string());
    vars.insert("session_id".to_string(), "test-123".to_string());
    
    let settings = Settings {
        audit_logging: true,
        debug_mode: true, // Skip UID dropping for tests
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    let result = executor.execute_spec(&CommandSpec::Array(array_spec)).await.unwrap();
    
    // Step 3: Verify execution and template substitution
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.is_some());
    assert_eq!(result.stdout.unwrap().trim(), "Hello Alice, your session is test-123");
}

#[tokio::test]
async fn test_encode_security_vs_shell_mode() {
    // Test that encoded array commands are secure vs dangerous shell commands
    let dangerous_shell = "echo 'test'; rm -rf /tmp/fake_important_file";
    
    // Step 1: Encode the dangerous shell command
    let encode_cmd = EncodeCommand::new(
        dangerous_shell.to_string(),
        "yaml".to_string(),
        false,
    );
    
    let array_spec = encode_cmd.parse_shell_to_array(dangerous_shell).unwrap();
    
    // The encoded version should be safe - shell-words will split it into separate args
    assert_eq!(array_spec.command, vec!["echo"]);
    assert_eq!(array_spec.args, Some(vec!["test;".to_string(), "rm".to_string(), "-rf".to_string(), "/tmp/fake_important_file".to_string()]));
    assert!(array_spec.pipe.is_none());
    
    // Step 2: Execute the encoded (safe) version
    let vars = HashMap::new();
    let settings = Settings {
        audit_logging: true,
        debug_mode: true,
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    let result = executor.execute_spec(&CommandSpec::Array(array_spec)).await.unwrap();
    
    // Step 3: Verify it safely echoed the literal arguments
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.is_some());
    assert_eq!(result.stdout.unwrap().trim(), "test; rm -rf /tmp/fake_important_file");
    
    // The dangerous command became a safe literal string - demonstrating security improvement
}

#[tokio::test]
async fn test_encode_output_formats_produce_valid_specs() {
    // Test that both YAML and JSON encode outputs can be parsed back into valid specs
    let shell_cmd = "ls -la | head -10";
    
    // Step 1: Encode to YAML
    let encode_cmd = EncodeCommand::new(
        shell_cmd.to_string(),
        "yaml".to_string(),
        false,
    );
    
    let array_spec = encode_cmd.parse_shell_to_array(shell_cmd).unwrap();
    let yaml_output = encode_cmd.serialize_array_spec(&array_spec).unwrap();
    
    // Step 2: Parse YAML back to spec
    let parsed_from_yaml: ArrayCommandSpec = serde_yaml_ng::from_str(&yaml_output).unwrap();
    
    // Step 3: Encode to JSON
    let encode_cmd_json = EncodeCommand::new(
        shell_cmd.to_string(),
        "json".to_string(),
        false,
    );
    
    let json_output = encode_cmd_json.serialize_array_spec(&array_spec).unwrap();
    
    // Step 4: Parse JSON back to spec
    let parsed_from_json: ArrayCommandSpec = serde_json::from_str(&json_output).unwrap();
    
    // Step 5: Verify both parsed specs are identical to original
    assert_eq!(parsed_from_yaml.command, array_spec.command);
    assert_eq!(parsed_from_yaml.args, array_spec.args);
    assert_eq!(parsed_from_yaml.pipe, array_spec.pipe);
    
    assert_eq!(parsed_from_json.command, array_spec.command);
    assert_eq!(parsed_from_json.args, array_spec.args);
    assert_eq!(parsed_from_json.pipe, array_spec.pipe);
    
    // Step 6: Verify both can be executed
    let settings = Settings {
        audit_logging: true,
        debug_mode: true,
        allow_shell: false,
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    let result_yaml = executor.execute_spec(&CommandSpec::Array(parsed_from_yaml)).await.unwrap();
    let result_json = executor.execute_spec(&CommandSpec::Array(parsed_from_json)).await.unwrap();
    
    // Both should succeed (ls command should work in most environments)
    assert!(result_yaml.success);
    assert!(result_json.success);
    assert_eq!(result_yaml.exit_code, 0);
    assert_eq!(result_json.exit_code, 0);
}

#[test]
fn test_encode_complex_commands() {
    // Test encoding various complex shell constructs
    let test_cases = vec![
        ("echo hello", vec!["echo".to_string()], Some(vec!["hello".to_string()])),
        ("ls -la", vec!["ls".to_string()], Some(vec!["-la".to_string()])),
        ("npm test", vec!["npm".to_string()], Some(vec!["test".to_string()])),
        ("git status --porcelain", vec!["git".to_string()], Some(vec!["status".to_string(), "--porcelain".to_string()])),
    ];
    
    for (shell_cmd, expected_cmd, expected_args) in test_cases {
        let encode_cmd = EncodeCommand::new(
            shell_cmd.to_string(),
            "yaml".to_string(),
            false,
        );
        
        let array_spec = encode_cmd.parse_shell_to_array(shell_cmd).unwrap();
        
        assert_eq!(array_spec.command, expected_cmd, "Failed for command: {}", shell_cmd);
        assert_eq!(array_spec.args, expected_args, "Failed for command: {}", shell_cmd);
    }
}

#[test]
fn test_encode_error_handling() {
    // Test that encode handles invalid commands gracefully
    let invalid_commands = vec![
        "", // Empty command
        "   ", // Whitespace only
    ];
    
    for invalid_cmd in invalid_commands {
        let encode_cmd = EncodeCommand::new(
            invalid_cmd.to_string(),
            "yaml".to_string(),
            false,
        );
        
        let result = encode_cmd.parse_shell_to_array(invalid_cmd);
        assert!(result.is_err(), "Expected error for invalid command: '{}'", invalid_cmd);
    }
}