//! Tests for the secure CommandExecutor implementation
//! 
//! This test suite validates Phase 2 of Plan 008 - the CommandGraph
//! construction and secure execution design.

use cupcake::config::actions::{ArrayCommandSpec, CommandSpec, EnvVar, PipeCommand};
use cupcake::engine::command_executor::{
    CommandExecutor, Operation, ExecutionError
};
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(test)]
mod command_executor_tests {
    use super::*;

    fn create_test_template_vars() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("file_path".to_string(), "/tmp/test.txt".to_string());
        vars.insert("user_name".to_string(), "alice".to_string());
        vars.insert("session_id".to_string(), "sess-12345".to_string());
        vars.insert("env_var".to_string(), "test_value".to_string());
        vars
    }

    #[test]
    fn test_simple_command_graph_construction() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["Hello World".to_string()]),
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
        
        assert_eq!(graph.nodes.len(), 1);
        let node = &graph.nodes[0];
        
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["Hello World"]);
        assert!(node.command.working_dir.is_none());
        assert!(node.command.env_vars.is_empty());
        assert!(node.operations.is_empty());
        assert!(node.conditional.is_none());
    }

    #[test]
    fn test_template_substitution_in_command() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["cat".to_string()],
            args: Some(vec!["{{file_path}}".to_string()]),
            working_dir: Some("/home/{{user_name}}".to_string()),
            env: Some(vec![EnvVar {
                name: "SESSION_ID".to_string(),
                value: "{{session_id}}".to_string(),
            }]),
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
        
        // Validate template substitution
        assert_eq!(node.command.program, "cat");
        assert_eq!(node.command.args, vec!["/tmp/test.txt"]);
        assert_eq!(node.command.working_dir, Some(PathBuf::from("/home/alice")));
        assert_eq!(node.command.env_vars.get("SESSION_ID"), Some(&"sess-12345".to_string()));
    }

    #[test]
    fn test_pipe_chain_construction() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["find".to_string()],
            args: Some(vec!["/tmp".to_string(), "-name".to_string(), "*.log".to_string()]),
            working_dir: None,
            env: None,
            pipe: Some(vec![
                PipeCommand {
                    cmd: vec!["grep".to_string(), "-v".to_string(), "DEBUG".to_string()],
                },
                PipeCommand {
                    cmd: vec!["sort".to_string()],
                },
                PipeCommand {
                    cmd: vec!["head".to_string(), "-n".to_string(), "10".to_string()],
                },
            ]),
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        }));

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate main command
        assert_eq!(node.command.program, "find");
        assert_eq!(node.command.args, vec!["/tmp", "-name", "*.log"]);
        
        // Validate pipe chain
        assert_eq!(node.operations.len(), 3);
        
        if let Operation::Pipe(cmd) = &node.operations[0] {
            assert_eq!(cmd.program, "grep");
            assert_eq!(cmd.args, vec!["-v", "DEBUG"]);
        } else {
            panic!("Expected Pipe operation");
        }
        
        if let Operation::Pipe(cmd) = &node.operations[1] {
            assert_eq!(cmd.program, "sort");
            assert!(cmd.args.is_empty());
        } else {
            panic!("Expected Pipe operation");
        }
        
        if let Operation::Pipe(cmd) = &node.operations[2] {
            assert_eq!(cmd.program, "head");
            assert_eq!(cmd.args, vec!["-n", "10"]);
        } else {
            panic!("Expected Pipe operation");
        }
    }

    #[test]
    fn test_redirect_operations_construction() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["cargo".to_string()],
            args: Some(vec!["build".to_string(), "--release".to_string()]),
            working_dir: Some("project".to_string()),
            env: Some(vec![
                EnvVar {
                    name: "RUSTFLAGS".to_string(),
                    value: "-C target-cpu=native".to_string(),
                },
            ]),
            pipe: None,
            redirect_stdout: Some("build.log".to_string()),
            append_stdout: None,
            redirect_stderr: Some("error.log".to_string()),
            merge_stderr: Some(true),
            on_success: None,
            on_failure: None,
        }));

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate main command
        assert_eq!(node.command.program, "cargo");
        assert_eq!(node.command.args, vec!["build", "--release"]);
        assert_eq!(node.command.working_dir, Some(PathBuf::from("project")));
        assert_eq!(node.command.env_vars.get("RUSTFLAGS"), Some(&"-C target-cpu=native".to_string()));
        
        // Validate redirect operations
        assert_eq!(node.operations.len(), 3);
        
        assert!(matches!(node.operations[0], Operation::RedirectStdout(_)));
        if let Operation::RedirectStdout(path) = &node.operations[0] {
            assert_eq!(path, &PathBuf::from("build.log"));
        }
        
        assert!(matches!(node.operations[1], Operation::RedirectStderr(_)));
        if let Operation::RedirectStderr(path) = &node.operations[1] {
            assert_eq!(path, &PathBuf::from("error.log"));
        }
        
        assert!(matches!(node.operations[2], Operation::MergeStderr));
    }

    #[test]
    fn test_conditional_execution_construction() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["test".to_string()],
            args: Some(vec!["-f".to_string(), "{{file_path}}".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: Some(vec![
                ArrayCommandSpec {
                    command: vec!["echo".to_string()],
                    args: Some(vec!["File {{file_path}} exists".to_string()]),
                    working_dir: None,
                    env: None,
                    pipe: None,
                    redirect_stdout: None,
                    append_stdout: None,
                    redirect_stderr: None,
                    merge_stderr: None,
                    on_success: None,
                    on_failure: None,
                },
                ArrayCommandSpec {
                    command: vec!["wc".to_string()],
                    args: Some(vec!["-l".to_string(), "{{file_path}}".to_string()]),
                    working_dir: None,
                    env: None,
                    pipe: None,
                    redirect_stdout: None,
                    append_stdout: None,
                    redirect_stderr: None,
                    merge_stderr: None,
                    on_success: None,
                    on_failure: None,
                },
            ]),
            on_failure: Some(vec![
                ArrayCommandSpec {
                    command: vec!["echo".to_string()],
                    args: Some(vec!["File {{file_path}} not found".to_string()]),
                    working_dir: None,
                    env: None,
                    pipe: None,
                    redirect_stdout: None,
                    append_stdout: None,
                    redirect_stderr: None,
                    merge_stderr: None,
                    on_success: None,
                    on_failure: None,
                },
            ]),
        }));

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate main command
        assert_eq!(node.command.program, "test");
        assert_eq!(node.command.args, vec!["-f", "/tmp/test.txt"]);
        
        // Validate conditional execution
        assert!(node.conditional.is_some());
        let conditional = node.conditional.as_ref().unwrap();
        
        // Validate success branch
        assert_eq!(conditional.on_success.len(), 2);
        assert_eq!(conditional.on_success[0].command.program, "echo");
        assert_eq!(conditional.on_success[0].command.args, vec!["File /tmp/test.txt exists"]);
        assert_eq!(conditional.on_success[1].command.program, "wc");
        assert_eq!(conditional.on_success[1].command.args, vec!["-l", "/tmp/test.txt"]);
        
        // Validate failure branch
        assert_eq!(conditional.on_failure.len(), 1);
        assert_eq!(conditional.on_failure[0].command.program, "echo");
        assert_eq!(conditional.on_failure[0].command.args, vec!["File /tmp/test.txt not found"]);
    }

    #[test]
    fn test_complex_composition() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["docker".to_string()],
            args: Some(vec!["ps".to_string(), "-a".to_string()]),
            working_dir: None,
            env: Some(vec![EnvVar {
                name: "DOCKER_CLI_EXPERIMENTAL".to_string(),
                value: "enabled".to_string(),
            }]),
            pipe: Some(vec![
                PipeCommand {
                    cmd: vec!["grep".to_string(), "{{user_name}}".to_string()],
                },
                PipeCommand {
                    cmd: vec!["awk".to_string(), "{print $1}".to_string()],
                },
            ]),
            redirect_stdout: Some("containers.txt".to_string()),
            append_stdout: None,
            redirect_stderr: Some("docker_errors.log".to_string()),
            merge_stderr: None,
            on_success: Some(vec![ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["Container list saved successfully".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }]),
            on_failure: Some(vec![ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["Failed to get container list".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }]),
        }));

        let graph = executor.build_graph(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate this is a complex, realistic command composition
        assert_eq!(node.command.program, "docker");
        assert_eq!(node.operations.len(), 4); // 2 pipes + 2 redirects
        assert!(node.conditional.is_some());
        
        // Validate template substitution in pipe
        if let Operation::Pipe(cmd) = &node.operations[0] {
            assert_eq!(cmd.args, vec!["alice"]); // Template substituted
        }
    }

    #[test]
    fn test_security_malicious_input_isolation() {
        let mut vars = HashMap::new();
        vars.insert("user_input".to_string(), "; rm -rf / #".to_string());
        vars.insert("file_path".to_string(), "/tmp/safe.txt; cat /etc/passwd".to_string());
        
        let executor = CommandExecutor::new(vars);
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
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
        }));

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

    #[test]
    fn test_error_handling_empty_command() {
        let executor = CommandExecutor::new(HashMap::new());
        
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec![], // Invalid: empty command
            args: None,
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

        let result = executor.build_graph(&spec);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ExecutionError::InvalidSpec(msg) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidSpec error"),
        }
    }

    #[test]
    fn test_nested_conditional_execution() {
        let executor = CommandExecutor::new(create_test_template_vars());
        
        // Test deeply nested conditional execution
        let spec = CommandSpec::Array(Box::new(ArrayCommandSpec {
            command: vec!["git".to_string()],
            args: Some(vec!["status".to_string(), "--porcelain".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: Some(vec![ArrayCommandSpec {
                command: vec!["test".to_string()],
                args: Some(vec!["-z".to_string(), "$(git status --porcelain)".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: Some(vec![ArrayCommandSpec {
                    command: vec!["echo".to_string()],
                    args: Some(vec!["Repository is clean".to_string()]),
                    working_dir: None,
                    env: None,
                    pipe: None,
                    redirect_stdout: None,
                    append_stdout: None,
                    redirect_stderr: None,
                    merge_stderr: None,
                    on_success: None,
                    on_failure: None,
                }]),
                on_failure: Some(vec![ArrayCommandSpec {
                    command: vec!["echo".to_string()],
                    args: Some(vec!["Repository has uncommitted changes".to_string()]),
                    working_dir: None,
                    env: None,
                    pipe: None,
                    redirect_stdout: None,
                    append_stdout: None,
                    redirect_stderr: None,
                    merge_stderr: None,
                    on_success: None,
                    on_failure: None,
                }]),
            }]),
            on_failure: Some(vec![ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["Not a git repository".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }]),
        }));

        let graph = executor.build_graph(&spec).unwrap();
        
        // This should successfully construct a complex nested graph
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.nodes[0].conditional.is_some());
        
        // The nested command should also have conditionals
        let success_commands = &graph.nodes[0].conditional.as_ref().unwrap().on_success;
        assert_eq!(success_commands.len(), 1);
        assert!(success_commands[0].conditional.is_some());
    }

    #[tokio::test]
    async fn test_execute_graph_placeholder() {
        let executor = CommandExecutor::new(HashMap::new());
        
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

        let graph = executor.build_graph(&spec).unwrap();
        let result = executor.execute_graph(&graph).await.unwrap();
        
        // Phase 3 - actual execution working!
        assert!(result.success);
        assert!(result.stdout.is_some());
        assert_eq!(result.stdout.unwrap().trim(), "test");
    }
}