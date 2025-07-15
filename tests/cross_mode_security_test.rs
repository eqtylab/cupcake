//! Cross-mode security tests
//!
//! These tests validate security boundaries between different command modes:
//! - Mode escalation prevention (array mode can't become shell mode)
//! - Configuration bypass prevention (allow_shell=false can't be circumvented)
//! - Context switching security (mixed mode policies)
//! - Consistent security behavior across modes
//!
//! SAFETY: These tests validate that security boundaries between modes are maintained.
//! They ensure that choosing a different mode doesn't bypass security controls.

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec, StringCommandSpec, ShellCommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;

/// Test that array mode cannot escalate to shell mode
#[test]
fn test_array_mode_no_shell_escalation() {
    let mut vars = HashMap::new();
    vars.insert("malicious_var".to_string(), "$(whoami)".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["{{malicious_var}}".to_string()]),
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
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Array mode keeps shell metacharacters as literal arguments
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["$(whoami)"]);
}

/// Test that string mode cannot escalate to shell mode
#[test]
fn test_string_mode_no_shell_escalation() {
    let mut vars = HashMap::new();
    vars.insert("malicious_var".to_string(), "$(whoami)".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo {{malicious_var}}".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // String mode keeps shell metacharacters as literal arguments
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["$(whoami)"]);
}

/// Test that shell mode requires explicit governance
#[test]
fn test_shell_mode_requires_governance() {
    let executor = CommandExecutor::new(HashMap::new());
    
    let spec = CommandSpec::Shell(ShellCommandSpec {
        script: "echo 'hello'".to_string(),
    });
    
    let result = executor.build_graph(&spec);
    
    // Should fail without allow_shell=true
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("allow_shell=true"));
}

/// Test that allow_shell=false cannot be bypassed through other modes
#[test]
fn test_allow_shell_false_cannot_be_bypassed() {
    let settings = Settings {
        allow_shell: false,
        ..Settings::default()
    };
    
    let executor = CommandExecutor::with_settings(HashMap::new(), settings);
    
    // Test array mode still works
    let array_spec = CommandSpec::Array(ArrayCommandSpec {
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
    
    let array_result = executor.build_graph(&array_spec);
    assert!(array_result.is_ok());
    
    // Test string mode still works  
    let string_spec = CommandSpec::String(StringCommandSpec {
        command: "echo test".to_string(),
    });
    
    let string_result = executor.build_graph(&string_spec);
    assert!(string_result.is_ok());
    
    // Test shell mode is blocked
    let shell_spec = CommandSpec::Shell(ShellCommandSpec {
        script: "echo test".to_string(),
    });
    
    let shell_result = executor.build_graph(&shell_spec);
    assert!(shell_result.is_err());
    let error = shell_result.unwrap_err();
    assert!(error.to_string().contains("allow_shell=true"));
}

/// Test consistent security behavior across modes with same malicious input
#[test]
fn test_consistent_security_across_modes() {
    let mut vars = HashMap::new();
    vars.insert("malicious_input".to_string(), "; rm -rf /tmp/test #".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    // Test array mode
    let array_spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["{{malicious_input}}".to_string()]),
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
    
    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];
    
    // Test string mode
    let string_spec = CommandSpec::String(StringCommandSpec {
        command: "echo {{malicious_input}}".to_string(),
    });
    
    let string_graph = executor.build_graph(&string_spec).unwrap();
    let string_node = &string_graph.nodes[0];
    
    // Both modes should produce the same safe literal argument
    assert_eq!(array_node.command.program, "echo");
    assert_eq!(array_node.command.args, vec!["; rm -rf /tmp/test #"]);
    
    assert_eq!(string_node.command.program, "echo");
    assert_eq!(string_node.command.args, vec!["; rm -rf /tmp/test #"]);
}

/// Test that shell mode with governance enabled handles the same input differently
#[test]
fn test_shell_mode_governance_difference() {
    let mut vars = HashMap::new();
    vars.insert("shell_input".to_string(), "echo hello".to_string());
    
    let settings = Settings {
        allow_shell: true,
        ..Settings::default()
    };
    
    let executor = CommandExecutor::with_settings(vars, settings);
    
    // Test array mode - input becomes literal
    let array_spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["sh".to_string()],
        args: Some(vec!["-c".to_string(), "{{shell_input}}".to_string()]),
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
    
    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];
    
    // Test shell mode - input is interpreted by shell
    let shell_spec = CommandSpec::Shell(ShellCommandSpec {
        script: "{{shell_input}}".to_string(),
    });
    
    let shell_graph = executor.build_graph(&shell_spec).unwrap();
    let shell_node = &shell_graph.nodes[0];
    
    // Array mode: sh command with literal arguments
    assert_eq!(array_node.command.program, "sh");
    assert_eq!(array_node.command.args, vec!["-c", "echo hello"]);
    
    // Shell mode: /bin/sh command with script as argument
    assert_eq!(shell_node.command.program, "/bin/sh");
    assert_eq!(shell_node.command.args, vec!["-c", "echo hello"]);
}

/// Test that mode boundaries are maintained with complex mixed scenarios
#[test]
fn test_mode_boundaries_with_complex_scenarios() {
    let mut vars = HashMap::new();
    vars.insert("complex_input".to_string(), "test && echo success || echo failure".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    // In array mode, shell operators become literal
    let array_spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["{{complex_input}}".to_string()]),
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
    
    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];
    
    // In string mode, shell operators should be parsed (but safely)
    let string_spec = CommandSpec::String(StringCommandSpec {
        command: "echo {{complex_input}}".to_string(),
    });
    
    let string_graph = executor.build_graph(&string_spec).unwrap();
    let string_node = &string_graph.nodes[0];
    
    // Array mode: all operators become literal
    assert_eq!(array_node.command.program, "echo");
    assert_eq!(array_node.command.args, vec!["test && echo success || echo failure"]);
    
    // String mode: operators also become literal in arguments
    assert_eq!(string_node.command.program, "echo");
    assert_eq!(string_node.command.args, vec!["test && echo success || echo failure"]);
}

/// Test that template variables behave consistently across modes
#[test]
fn test_template_consistency_across_modes() {
    let mut vars = HashMap::new();
    vars.insert("file_path".to_string(), "/tmp/test.txt".to_string());
    vars.insert("user_name".to_string(), "testuser".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    // Test array mode
    let array_spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["User {{user_name}} accessing {{file_path}}".to_string()]),
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
    
    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];
    
    // Test string mode
    let string_spec = CommandSpec::String(StringCommandSpec {
        command: "echo 'User {{user_name}} accessing {{file_path}}'".to_string(),
    });
    
    let string_graph = executor.build_graph(&string_spec).unwrap();
    let string_node = &string_graph.nodes[0];
    
    // Both modes should substitute templates identically
    assert_eq!(array_node.command.program, "echo");
    assert_eq!(array_node.command.args, vec!["User testuser accessing /tmp/test.txt"]);
    
    assert_eq!(string_node.command.program, "echo");
    assert_eq!(string_node.command.args, vec!["User testuser accessing /tmp/test.txt"]);
}

/// Test that security boundaries prevent privilege escalation between modes
#[test]
fn test_no_privilege_escalation_between_modes() {
    let mut vars = HashMap::new();
    vars.insert("escalation_attempt".to_string(), "sudo rm -rf /".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    // Array mode should make sudo command literal
    let array_spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["{{escalation_attempt}}".to_string()]),
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
    
    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];
    
    // String mode should also make sudo command literal
    let string_spec = CommandSpec::String(StringCommandSpec {
        command: "echo {{escalation_attempt}}".to_string(),
    });
    
    let string_graph = executor.build_graph(&string_spec).unwrap();
    let string_node = &string_graph.nodes[0];
    
    // Both modes neutralize privilege escalation attempts
    assert_eq!(array_node.command.program, "echo");
    assert_eq!(array_node.command.args, vec!["sudo rm -rf /"]);
    
    assert_eq!(string_node.command.program, "echo");
    assert_eq!(string_node.command.args, vec!["sudo rm -rf /"]);
}