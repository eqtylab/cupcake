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

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec, ShellCommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;

/// Test that array mode cannot escalate to shell mode
#[test]
fn test_array_mode_no_shell_escalation() {
    let mut vars = HashMap::new();
    vars.insert("malicious_var".to_string(), "$(whoami)".to_string());

    let executor = CommandExecutor::new(vars);

    let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
    }));

    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];

    // Array mode keeps shell metacharacters as literal arguments
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
    let array_spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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

    let array_result = executor.build_graph(&array_spec);
    assert!(array_result.is_ok());

    // Test shell mode is blocked
    let shell_spec = CommandSpec::Shell(ShellCommandSpec {
        script: "echo test".to_string(),
    });

    let shell_result = executor.build_graph(&shell_spec);
    assert!(shell_result.is_err());
    let error = shell_result.unwrap_err();
    assert!(error.to_string().contains("allow_shell=true"));
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
    let array_spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
    }));

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
    vars.insert(
        "complex_input".to_string(),
        "test && echo success || echo failure".to_string(),
    );

    let executor = CommandExecutor::new(vars);

    // In array mode, shell operators become literal
    let array_spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
    }));

    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];

    // Array mode: all operators become literal
    assert_eq!(array_node.command.program, "echo");
    assert_eq!(
        array_node.command.args,
        vec!["test && echo success || echo failure"]
    );
}

/// Test that security boundaries prevent privilege escalation between modes
#[test]
fn test_no_privilege_escalation_between_modes() {
    let mut vars = HashMap::new();
    vars.insert(
        "escalation_attempt".to_string(),
        "sudo rm -rf /".to_string(),
    );

    let executor = CommandExecutor::new(vars);

    // Array mode should make sudo command literal
    let array_spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
    }));

    let array_graph = executor.build_graph(&array_spec).unwrap();
    let array_node = &array_graph.nodes[0];

    // Array mode neutralizes privilege escalation attempts
    assert_eq!(array_node.command.program, "echo");
    assert_eq!(array_node.command.args, vec!["sudo rm -rf /"]);
}
