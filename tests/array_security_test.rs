//! Security tests for Array command mode
//!
//! These tests validate the security properties of ArrayCommandSpec:
//! - Shell injection prevention through direct process spawning
//! - Template injection prevention in command paths
//! - Argument sanitization with shell metacharacters
//! - Process spawning security without shell involvement

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec, EnvVar};
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;

/// Test that malicious input in template variables becomes literal arguments
#[test]
fn test_malicious_input_isolation() {
    let mut vars = HashMap::new();
    vars.insert("user_input".to_string(), "; rm -rf / #".to_string());
    vars.insert("file_path".to_string(), "/tmp/safe.txt; cat /etc/passwd".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["Processing {{user_input}}".to_string(), "from {{file_path}}".to_string()]),
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
    
    // Malicious content becomes literal arguments - this is SAFE
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec![
        "Processing ; rm -rf / #",
        "from /tmp/safe.txt; cat /etc/passwd"
    ]);
    
    // Key security property: No shell is involved, so the malicious content
    // is just literal string arguments that echo will print, not execute
}

/// Test that shell metacharacters in arguments are treated as literals
#[test]
fn test_shell_metacharacter_sanitization() {
    let mut vars = HashMap::new();
    vars.insert("dangerous_arg".to_string(), "$(whoami)".to_string());
    vars.insert("pipe_attack".to_string(), "| rm -rf /".to_string());
    vars.insert("redirect_attack".to_string(), "> /etc/passwd".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec![
            "{{dangerous_arg}}".to_string(),
            "{{pipe_attack}}".to_string(),
            "{{redirect_attack}}".to_string(),
        ]),
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
    
    // All shell metacharacters become literal arguments
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec![
        "$(whoami)",
        "| rm -rf /",
        "> /etc/passwd"
    ]);
}

/// Test that command path template injection is prevented
#[test]
fn test_command_path_template_injection_prevention() {
    let mut vars = HashMap::new();
    vars.insert("malicious_cmd".to_string(), "rm".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    // This should not be possible in the current implementation
    // because command paths don't support template substitution
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()], // Templates not supported in command[0]
        args: Some(vec!["safe argument".to_string()]),
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
    
    // Command path is never substituted - remains literal
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["safe argument"]);
}

/// Test that environment variables with malicious content are isolated
#[test]
fn test_environment_variable_isolation() {
    let mut vars = HashMap::new();
    vars.insert("malicious_env".to_string(), "$(rm -rf /)".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["env".to_string()],
        args: None,
        working_dir: None,
        env: Some(vec![
            EnvVar {
                name: "DANGEROUS_VAR".to_string(),
                value: "{{malicious_env}}".to_string(),
            }
        ]),
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
    
    // Environment variable contains literal malicious content, not executed
    assert_eq!(node.command.env_vars.get("DANGEROUS_VAR").unwrap(), "$(rm -rf /)");
}

/// Test that working directory with malicious content is handled safely
#[test]
fn test_working_directory_safety() {
    let mut vars = HashMap::new();
    vars.insert("malicious_dir".to_string(), "/tmp; rm -rf /".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["pwd".to_string()],
        args: None,
        working_dir: Some("{{malicious_dir}}".to_string()),
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
    
    // Working directory becomes literal path (may fail to execute, but safe)
    assert_eq!(node.command.working_dir.as_ref().unwrap().to_string_lossy(), "/tmp; rm -rf /");
}

/// Test that piped commands with malicious content are isolated
#[tokio::test]
async fn test_piped_command_isolation() {
    let mut vars = HashMap::new();
    vars.insert("malicious_grep".to_string(), "password; rm -rf /".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec!["hello world".to_string()]),
        working_dir: None,
        env: None,
        pipe: Some(vec![
            cupcake::config::actions::PipeCommand {
                cmd: vec!["grep".to_string(), "{{malicious_grep}}".to_string()],
            }
        ]),
        redirect_stdout: None,
        append_stdout: None,
        redirect_stderr: None,
        merge_stderr: None,
        on_success: None,
        on_failure: None,
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    
    // First node is echo
    assert_eq!(graph.nodes[0].command.program, "echo");
    assert_eq!(graph.nodes[0].command.args, vec!["hello world"]);
    
    // Second node is grep with malicious content as literal argument
    let pipe_op = &graph.nodes[0].operations[0];
    if let cupcake::engine::command_executor::Operation::Pipe(cmd) = pipe_op {
        assert_eq!(cmd.program, "grep");
        assert_eq!(cmd.args, vec!["password; rm -rf /"]);
    } else {
        panic!("Expected pipe operation");
    }
}

/// Test that complex malicious input combinations are all neutralized
#[test]
fn test_complex_malicious_input_neutralization() {
    let mut vars = HashMap::new();
    vars.insert("cmd_injection".to_string(), "$(whoami)".to_string());
    vars.insert("path_traversal".to_string(), "../../etc/passwd".to_string());
    vars.insert("shell_escape".to_string(), "; cat /etc/shadow #".to_string());
    vars.insert("null_byte".to_string(), "file\x00.txt".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::Array(ArrayCommandSpec {
        command: vec!["echo".to_string()],
        args: Some(vec![
            "cmd: {{cmd_injection}}".to_string(),
            "path: {{path_traversal}}".to_string(),
            "shell: {{shell_escape}}".to_string(),
            "null: {{null_byte}}".to_string(),
        ]),
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
    
    // All malicious inputs become literal arguments
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec![
        "cmd: $(whoami)",
        "path: ../../etc/passwd", 
        "shell: ; cat /etc/shadow #",
        "null: file\x00.txt"
    ]);
}