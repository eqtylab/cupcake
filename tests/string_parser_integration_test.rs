//! Integration tests for string command parsing
//! 
//! This test suite validates the integration between StringCommandSpec and
//! the CommandExecutor, ensuring that string commands are properly parsed
//! and executed securely.

use cupcake::config::actions::{CommandSpec, StringCommandSpec};
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;

#[cfg(test)]
mod string_parser_integration_tests {
    use super::*;

    fn create_executor() -> CommandExecutor {
        let mut vars = HashMap::new();
        vars.insert("file_path".to_string(), "/tmp/test.txt".to_string());
        vars.insert("user".to_string(), "alice".to_string());
        vars.insert("message".to_string(), "Hello World".to_string());
        CommandExecutor::new(vars)
    }

    #[test]
    fn test_simple_string_command_parsing() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo hello world".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["hello", "world"]);
        assert!(node.operations.is_empty());
        assert!(node.conditional.is_none());
    }

    #[test]
    fn test_string_command_with_template_substitution() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "cat {{file_path}}".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "cat");
        assert_eq!(node.command.args, vec!["/tmp/test.txt"]);
    }

    #[test]
    fn test_string_command_with_multiple_templates() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo 'User: {{user}}' file: {{file_path}}".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["User: alice", "file:", "/tmp/test.txt"]);
    }

    #[test]
    fn test_string_command_with_quoted_arguments() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: r#"grep "{{message}}" /var/log/app.log"#.to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "grep");
        assert_eq!(node.command.args, vec!["Hello World", "/var/log/app.log"]);
    }

    #[test]
    fn test_string_command_security_command_substitution_blocked() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo $(whoami)".to_string(),
        });

        let result = executor.build_graph(&spec);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Command substitution"));
    }

    #[test]
    fn test_string_command_security_backticks_blocked() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo `date`".to_string(),
        });

        let result = executor.build_graph(&spec);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Backtick"));
    }

    #[test]
    fn test_string_command_pipe_operator() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo hello world | grep world".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["hello", "world"]);
        assert_eq!(node.operations.len(), 1);
        
        match &node.operations[0] {
            cupcake::engine::command_executor::Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "grep");
                assert_eq!(cmd.args, vec!["world"]);
            }
            _ => panic!("Expected Pipe operation"),
        }
    }

    #[test]
    fn test_string_command_redirect_operators() {
        let executor = create_executor();
        
        // Test > redirect
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo test content > output.txt".to_string(),
        });
        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["test", "content"]);
        assert_eq!(node.operations.len(), 1);
        assert!(matches!(&node.operations[0], cupcake::engine::command_executor::Operation::RedirectStdout(_)));
        
        // Test >> append
        let spec2 = CommandSpec::String(StringCommandSpec {
            command: "echo more content >> output.txt".to_string(),
        });
        let graph2 = executor.build_graph(&spec2).unwrap();
        let node2 = &graph2.nodes[0];
        assert_eq!(node2.operations.len(), 1);
        assert!(matches!(&node2.operations[0], cupcake::engine::command_executor::Operation::AppendStdout(_)));
    }

    #[test]
    fn test_string_command_conditional_operators() {
        let executor = create_executor();
        
        // Test && operator
        let spec = CommandSpec::String(StringCommandSpec {
            command: "test -f {{file_path}} && echo file exists".to_string(),
        });
        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "test");
        assert_eq!(node.command.args, vec!["-f", "/tmp/test.txt"]);
        assert!(node.conditional.is_some());
        let cond = node.conditional.as_ref().unwrap();
        assert_eq!(cond.on_success.len(), 1);
        assert_eq!(cond.on_success[0].command.program, "echo");
        
        // Test || operator
        let spec2 = CommandSpec::String(StringCommandSpec {
            command: "test -f missing.txt || echo file not found".to_string(),
        });
        let graph2 = executor.build_graph(&spec2).unwrap();
        let node2 = &graph2.nodes[0];
        let cond2 = node2.conditional.as_ref().unwrap();
        assert_eq!(cond2.on_failure.len(), 1);
        assert_eq!(cond2.on_failure[0].command.program, "echo");
    }

    #[test]
    fn test_string_command_complex_pipe_with_template() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "cat {{file_path}} | grep {{user}} | wc -l > count.txt".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "cat");
        assert_eq!(node.command.args, vec!["/tmp/test.txt"]);
        assert_eq!(node.operations.len(), 3);
        
        // Verify template substitution in pipe args
        match &node.operations[0] {
            cupcake::engine::command_executor::Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "grep");
                assert_eq!(cmd.args, vec!["alice"]);
            }
            _ => panic!("Expected Pipe operation"),
        }
    }

    #[test]
    fn test_string_command_empty_command_errors() {
        let executor = create_executor();

        let error_cases = vec![
            ("", "No command provided"),
            ("   ", "No command provided"),
            ("| grep test", "no command before"),
            ("> output.txt", "no command before"),
            ("&& echo test", "no command before"),
        ];

        for (command, expected_msg) in error_cases {
            let spec = CommandSpec::String(StringCommandSpec {
                command: command.to_string(),
            });

            let result = executor.build_graph(&spec);
            assert!(result.is_err(), "Command should fail: '{}'", command);
            let error_msg = result.unwrap_err().to_string();
            assert!(error_msg.to_lowercase().contains(&expected_msg.to_lowercase()), 
                "Error should mention '{}' for command '{}', got: {}", expected_msg, command, error_msg);
        }
    }

    #[test]
    fn test_string_command_trailing_operators_error() {
        let executor = create_executor();

        let error_cases = vec![
            ("echo test |", "Trailing operator"),
            ("echo test >", "Trailing operator"),
            ("echo test >>", "Trailing operator"),
            ("echo test &&", "Trailing operator"),
            ("echo test ||", "Trailing operator"),
        ];

        for (command, expected_msg) in error_cases {
            let spec = CommandSpec::String(StringCommandSpec {
                command: command.to_string(),
            });

            let result = executor.build_graph(&spec);
            assert!(result.is_err(), "Command should fail: '{}'", command);
            let error_msg = result.unwrap_err().to_string();
            assert!(error_msg.contains(expected_msg), 
                "Error should mention '{}' for command '{}', got: {}", expected_msg, command, error_msg);
        }
    }

    #[test]
    fn test_string_command_complex_quoting() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: r#"echo 'Single quotes: {{message}}' "Double quotes: {{user}}" plain_{{file_path}}"#.to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec![
            "Single quotes: Hello World",
            "Double quotes: alice", 
            "plain_/tmp/test.txt"
        ]);
    }

    #[test]
    fn test_string_command_with_escapes() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: r#"echo "Hello \"World\"" '{{user}}'"#.to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["Hello \"World\"", "alice"]);
    }

    #[tokio::test]
    async fn test_string_command_execution() {
        let executor = create_executor();
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo test_string_execution".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let result = executor.execute_graph(&graph).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.is_some());
        assert_eq!(result.stdout.unwrap().trim(), "test_string_execution");
    }
}