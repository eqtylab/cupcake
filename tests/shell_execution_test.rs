//! Tests for shell command execution with security controls
//! 
//! This test suite validates the shell execution functionality including
//! security controls, audit logging, and proper error handling.

use cupcake::config::actions::{CommandSpec, ShellCommandSpec};
use cupcake::config::types::Settings;
use cupcake::engine::command_executor::CommandExecutor;
use std::collections::HashMap;

#[cfg(test)]
mod shell_execution_tests {
    use super::*;

    fn create_executor_with_settings(allow_shell: bool) -> CommandExecutor {
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "testuser".to_string());
        vars.insert("file_path".to_string(), "/tmp/test.txt".to_string());
        
        let settings = Settings {
            audit_logging: true,
            debug_mode: true, // Enable debug mode to skip UID dropping in tests
            allow_shell,
            timeout_ms: 30000,
            sandbox_uid: None,
        };
        
        CommandExecutor::with_settings(vars, settings)
    }

    #[test]
    fn test_shell_execution_disabled_by_default() {
        let executor = CommandExecutor::new(HashMap::new());
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'test'".to_string(),
        });

        let result = executor.build_graph(&spec);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Shell command execution is disabled"));
        assert!(error_msg.contains("allow_shell=true"));
    }

    #[test]
    fn test_shell_execution_blocked_when_disabled() {
        let executor = create_executor_with_settings(false);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'test'".to_string(),
        });

        let result = executor.build_graph(&spec);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Shell command execution is disabled"));
    }

    #[test]
    fn test_shell_execution_allowed_when_enabled() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'Hello World'".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "/bin/sh");
        assert_eq!(node.command.args.len(), 2);
        assert_eq!(node.command.args[0], "-c");
        assert_eq!(node.command.args[1], "echo 'Hello World'");
    }

    #[test]
    fn test_shell_template_substitution() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'User: {{user}}' > {{file_path}}".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.args[1], "echo 'User: testuser' > /tmp/test.txt");
    }

    #[test]
    fn test_shell_complex_script() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: r#"
set -euo pipefail
for f in {a..z}*.tmp; do
    [ -e "$f" ] && rm "$f"
done
echo "Cleanup complete"
"#.to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "/bin/sh");
        assert!(node.command.args[1].contains("set -euo pipefail"));
        assert!(node.command.args[1].contains("for f in {a..z}*.tmp"));
    }

    #[test]
    fn test_shell_dangerous_content_allowed() {
        // Shell mode explicitly allows dangerous content - security is in the governance
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "rm -rf /tmp/* && echo 'Dangerous cleanup'".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert!(node.command.args[1].contains("rm -rf"));
    }

    #[test]
    fn test_shell_empty_script() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        assert_eq!(node.command.args[1], "");
    }

    #[tokio::test]
    async fn test_shell_execution_with_real_command() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'Shell execution test'".to_string(),
        });

        let result = executor.execute_spec(&spec).await.unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.is_some());
        assert_eq!(result.stdout.unwrap().trim(), "Shell execution test");
    }

    #[tokio::test]
    async fn test_shell_execution_with_failure() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "exit 42".to_string(),
        });

        let result = executor.execute_spec(&spec).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, 42);
    }

    #[tokio::test]
    async fn test_shell_execution_with_variables() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'Current user: {{user}}'".to_string(),
        });

        let result = executor.execute_spec(&spec).await.unwrap();
        assert!(result.success);
        assert!(result.stdout.unwrap().contains("testuser"));
    }

    #[test]
    fn test_shell_audit_mode_detection() {
        let executor = create_executor_with_settings(true);
        
        // Test shell mode detection
        let shell_spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo test".to_string(),
        });
        let graph = executor.build_graph(&shell_spec).unwrap();
        assert!(graph.nodes[0].command.program == "/bin/sh");
        
        // Compare with array mode
        let array_spec = CommandSpec::Array(cupcake::config::actions::ArrayCommandSpec {
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
        let array_graph = executor.build_graph(&array_spec).unwrap();
        assert!(array_graph.nodes[0].command.program == "echo");
    }

    #[test]
    fn test_settings_security_defaults() {
        let settings = Settings::default();
        assert!(!settings.allow_shell); // Critical: must default to false
        
        // Ensure CommandExecutor respects defaults
        let executor = CommandExecutor::new(HashMap::new());
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo test".to_string(),
        });
        assert!(executor.build_graph(&spec).is_err());
    }

    #[tokio::test]
    async fn test_shell_execution_with_pipes() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "echo 'line1\nline2\nline3' | grep line2".to_string(),
        });

        let result = executor.execute_spec(&spec).await.unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.unwrap().trim(), "line2");
    }

    #[tokio::test]
    async fn test_shell_execution_with_redirect() {
        let executor = create_executor_with_settings(true);
        let script = format!(
            "echo 'test content' > /tmp/cupcake_test_{}.txt && cat /tmp/cupcake_test_{}.txt",
            std::process::id(), std::process::id()
        );
        
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script,
        });

        let result = executor.execute_spec(&spec).await.unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.unwrap().trim(), "test content");
    }

    #[test]
    fn test_shell_command_graph_structure() {
        let executor = create_executor_with_settings(true);
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "complex script here".to_string(),
        });

        let graph = executor.build_graph(&spec).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        
        let node = &graph.nodes[0];
        assert_eq!(node.command.program, "/bin/sh");
        assert_eq!(node.command.args.len(), 2);
        assert_eq!(node.command.args[0], "-c");
        assert!(node.operations.is_empty());
        assert!(node.conditional.is_none());
        assert!(node.command.working_dir.is_none());
        assert!(node.command.env_vars.is_empty());
    }
}