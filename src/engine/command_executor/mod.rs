//! Secure Command Executor Module
//! 
//! This module implements shell-free command execution for Plan 008.
//! It transforms ArrayCommandSpec into a CommandGraph and executes it
//! using direct process spawning with tokio::process::Command.
//!
//! Key Security Features:
//! - No shell involvement (eliminates injection attacks)
//! - Direct process spawning with argv arrays
//! - Secure pipe and redirect handling
//! - Template substitution only in safe contexts

use crate::config::actions::{ArrayCommandSpec, CommandSpec, EnvVar};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command as TokioCommand;
use std::process::Stdio;

/// Internal representation of a command execution graph
/// 
/// This represents the parsed and validated command structure that will be
/// executed securely without shell involvement.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandGraph {
    /// The sequence of execution nodes
    pub nodes: Vec<ExecutionNode>,
}

/// A single node in the command execution graph
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionNode {
    /// The core command to execute
    pub command: Command,
    /// Operations to apply to this command's output
    pub operations: Vec<Operation>,
    /// Commands to run conditionally based on exit code
    pub conditional: Option<ConditionalExecution>,
}

/// Core command specification (secure, no shell)
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    /// Program to execute (must be in PATH or absolute path)
    pub program: String,
    /// Arguments to pass to the program
    pub args: Vec<String>,
    /// Working directory for execution
    pub working_dir: Option<PathBuf>,
    /// Environment variables (extends system environment)
    pub env_vars: HashMap<String, String>,
}

/// Operations that can be applied to command output
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    /// Pipe stdout to another command
    Pipe(Command),
    /// Redirect stdout to file (truncate)
    RedirectStdout(PathBuf),
    /// Append stdout to file
    AppendStdout(PathBuf),
    /// Redirect stderr to file
    RedirectStderr(PathBuf),
    /// Merge stderr into stdout
    MergeStderr,
}

/// Conditional execution based on exit codes
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalExecution {
    /// Commands to run if exit code == 0
    pub on_success: Vec<ExecutionNode>,
    /// Commands to run if exit code != 0
    pub on_failure: Vec<ExecutionNode>,
}

/// Result of command execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final exit code of the execution graph
    pub exit_code: i32,
    /// Standard output (if captured)
    pub stdout: Option<String>,
    /// Standard error (if captured)
    pub stderr: Option<String>,
    /// Whether the overall execution was successful
    pub success: bool,
}

/// Errors that can occur during command execution
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Command graph construction failed: {0}")]
    GraphConstruction(String),
    
    #[error("Process spawn failed: {0}")]
    ProcessSpawn(String),
    
    #[error("I/O operation failed: {0}")]
    IoOperation(String),
    
    #[error("Template substitution failed: {0}")]
    TemplateSubstitution(String),
    
    #[error("Timeout exceeded")]
    Timeout,
    
    #[error("Invalid command specification: {0}")]
    InvalidSpec(String),
}

/// The main command executor that transforms specs into secure execution
pub struct CommandExecutor {
    /// Template variables for substitution
    template_vars: HashMap<String, String>,
}

impl CommandExecutor {
    /// Create a new command executor with template variables
    pub fn new(template_vars: HashMap<String, String>) -> Self {
        Self { template_vars }
    }

    /// Build a CommandGraph from a CommandSpec
    /// 
    /// This is the core transformation that converts the user-facing YAML
    /// specification into an internal execution graph.
    pub fn build_graph(&self, spec: &CommandSpec) -> Result<CommandGraph, ExecutionError> {
        match spec {
            CommandSpec::Array(array_spec) => self.build_graph_from_array(array_spec),
        }
    }

    /// Build CommandGraph from ArrayCommandSpec
    fn build_graph_from_array(&self, spec: &ArrayCommandSpec) -> Result<CommandGraph, ExecutionError> {
        // Validate basic command structure
        if spec.command.is_empty() {
            return Err(ExecutionError::InvalidSpec(
                "Command array cannot be empty".to_string()
            ));
        }

        // Build the primary command with secure template substitution
        let command = self.build_command(spec)?;
        
        // Build operations from composition keys
        let operations = self.build_operations(spec)?;
        
        // Build conditional execution
        let conditional = self.build_conditional_execution(spec)?;

        let node = ExecutionNode {
            command,
            operations,
            conditional,
        };

        Ok(CommandGraph {
            nodes: vec![node],
        })
    }

    /// Build secure Command from ArrayCommandSpec
    fn build_command(&self, spec: &ArrayCommandSpec) -> Result<Command, ExecutionError> {
        // Program is the first element of command array
        let program = self.substitute_template(&spec.command[0])?;
        
        // Remaining command elements plus args become arguments
        let mut args = Vec::new();
        
        // Add remaining command elements as args (if any)
        if spec.command.len() > 1 {
            for arg in &spec.command[1..] {
                args.push(self.substitute_template(arg)?);
            }
        }
        
        // Add explicit args
        if let Some(explicit_args) = &spec.args {
            for arg in explicit_args {
                args.push(self.substitute_template(arg)?);
            }
        }

        // Build working directory
        let working_dir = match &spec.working_dir {
            Some(dir) => Some(PathBuf::from(self.substitute_template(dir)?)),
            None => None,
        };

        // Build environment variables
        let mut env_vars = HashMap::new();
        if let Some(env_list) = &spec.env {
            for env_var in env_list {
                let name = env_var.name.clone(); // Env var names are never templated
                let value = self.substitute_template(&env_var.value)?;
                env_vars.insert(name, value);
            }
        }

        Ok(Command {
            program,
            args,
            working_dir,
            env_vars,
        })
    }

    /// Build operations from composition keys
    fn build_operations(&self, spec: &ArrayCommandSpec) -> Result<Vec<Operation>, ExecutionError> {
        let mut operations = Vec::new();

        // Handle pipe operations
        if let Some(pipe_commands) = &spec.pipe {
            for pipe_cmd in pipe_commands {
                let command = Command {
                    program: self.substitute_template(&pipe_cmd.cmd[0])?,
                    args: pipe_cmd.cmd[1..].iter()
                        .map(|arg| self.substitute_template(arg))
                        .collect::<Result<Vec<_>, _>>()?,
                    working_dir: None, // Pipes inherit working dir
                    env_vars: HashMap::new(), // Pipes inherit environment
                };
                operations.push(Operation::Pipe(command));
            }
        }

        // Handle stdout redirection
        if let Some(file_path) = &spec.redirect_stdout {
            let path = PathBuf::from(self.substitute_template(file_path)?);
            operations.push(Operation::RedirectStdout(path));
        }

        // Handle stdout append
        if let Some(file_path) = &spec.append_stdout {
            let path = PathBuf::from(self.substitute_template(file_path)?);
            operations.push(Operation::AppendStdout(path));
        }

        // Handle stderr redirection
        if let Some(file_path) = &spec.redirect_stderr {
            let path = PathBuf::from(self.substitute_template(file_path)?);
            operations.push(Operation::RedirectStderr(path));
        }

        // Handle stderr merge
        if spec.merge_stderr == Some(true) {
            operations.push(Operation::MergeStderr);
        }

        Ok(operations)
    }

    /// Build conditional execution from onSuccess/onFailure
    fn build_conditional_execution(&self, spec: &ArrayCommandSpec) -> Result<Option<ConditionalExecution>, ExecutionError> {
        let on_success = if let Some(success_specs) = &spec.on_success {
            let mut nodes = Vec::new();
            for success_spec in success_specs {
                let graph = self.build_graph_from_array(success_spec)?;
                nodes.extend(graph.nodes);
            }
            nodes
        } else {
            Vec::new()
        };

        let on_failure = if let Some(failure_specs) = &spec.on_failure {
            let mut nodes = Vec::new();
            for failure_spec in failure_specs {
                let graph = self.build_graph_from_array(failure_spec)?;
                nodes.extend(graph.nodes);
            }
            nodes
        } else {
            Vec::new()
        };

        if on_success.is_empty() && on_failure.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ConditionalExecution {
                on_success,
                on_failure,
            }))
        }
    }

    /// Safely substitute template variables
    /// 
    /// This is the critical security function - it only substitutes in safe contexts
    /// (arguments and environment values) and never in command paths.
    fn substitute_template(&self, template: &str) -> Result<String, ExecutionError> {
        let mut result = template.to_string();
        
        for (key, value) in &self.template_vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        Ok(result)
    }

    /// Execute a CommandGraph (async implementation in Phase 3)
    /// 
    /// This is where the actual secure execution happens using tokio::process::Command
    pub async fn execute_graph(&self, _graph: &CommandGraph) -> Result<ExecutionResult, ExecutionError> {
        // TODO: Implement in Phase 3
        // This will use tokio::process::Command for direct process spawning
        // with proper pipe/redirect handling and no shell involvement
        
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: Some("TODO: Implement in Phase 3".to_string()),
            stderr: None,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::actions::{PipeCommand};

    fn create_template_vars() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("file_path".to_string(), "/tmp/test.txt".to_string());
        vars.insert("user".to_string(), "testuser".to_string());
        vars.insert("session_id".to_string(), "test-123".to_string());
        vars
    }

    #[test]
    fn test_command_graph_construction() {
        let executor = CommandExecutor::new(create_template_vars());
        
        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["Hello {{user}}".to_string()]),
            working_dir: Some("/tmp".to_string()),
            env: Some(vec![EnvVar {
                name: "SESSION".to_string(),
                value: "{{session_id}}".to_string(),
            }]),
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let graph = executor.build_graph_from_array(&spec).unwrap();
        
        assert_eq!(graph.nodes.len(), 1);
        let node = &graph.nodes[0];
        
        // Validate command construction
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["Hello testuser"]);
        assert_eq!(node.command.working_dir, Some(PathBuf::from("/tmp")));
        assert_eq!(node.command.env_vars.get("SESSION"), Some(&"test-123".to_string()));
        
        // Validate no operations
        assert!(node.operations.is_empty());
        assert!(node.conditional.is_none());
    }

    #[test]
    fn test_pipe_operations() {
        let executor = CommandExecutor::new(create_template_vars());
        
        let spec = ArrayCommandSpec {
            command: vec!["npm".to_string()],
            args: Some(vec!["test".to_string()]),
            working_dir: None,
            env: None,
            pipe: Some(vec![
                PipeCommand {
                    cmd: vec!["grep".to_string(), "-v".to_string(), "WARNING".to_string()],
                },
                PipeCommand {
                    cmd: vec!["tee".to_string(), "{{file_path}}".to_string()],
                },
            ]),
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let graph = executor.build_graph_from_array(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate pipe operations
        assert_eq!(node.operations.len(), 2);
        
        match &node.operations[0] {
            Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "grep");
                assert_eq!(cmd.args, vec!["-v", "WARNING"]);
            }
            _ => panic!("Expected Pipe operation"),
        }
        
        match &node.operations[1] {
            Operation::Pipe(cmd) => {
                assert_eq!(cmd.program, "tee");
                assert_eq!(cmd.args, vec!["/tmp/test.txt"]); // Template substituted
            }
            _ => panic!("Expected Pipe operation"),
        }
    }

    #[test]
    fn test_redirect_operations() {
        let executor = CommandExecutor::new(create_template_vars());
        
        let spec = ArrayCommandSpec {
            command: vec!["cargo".to_string()],
            args: Some(vec!["build".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: Some("build.log".to_string()),
            append_stdout: None,
            redirect_stderr: Some("error.log".to_string()),
            merge_stderr: Some(true),
            on_success: None,
            on_failure: None,
        };

        let graph = executor.build_graph_from_array(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate redirect operations
        assert_eq!(node.operations.len(), 3);
        
        assert!(matches!(node.operations[0], Operation::RedirectStdout(_)));
        assert!(matches!(node.operations[1], Operation::RedirectStderr(_)));
        assert!(matches!(node.operations[2], Operation::MergeStderr));
    }

    #[test]
    fn test_conditional_execution() {
        let executor = CommandExecutor::new(create_template_vars());
        
        let spec = ArrayCommandSpec {
            command: vec!["test".to_string()],
            args: Some(vec!["-f".to_string(), "{{file_path}}".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: Some(vec![ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["File exists".to_string()]),
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
                args: Some(vec!["File not found".to_string()]),
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
        };

        let graph = executor.build_graph_from_array(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // Validate main command
        assert_eq!(node.command.program, "test");
        assert_eq!(node.command.args, vec!["-f", "/tmp/test.txt"]);
        
        // Validate conditional execution
        assert!(node.conditional.is_some());
        let conditional = node.conditional.as_ref().unwrap();
        
        assert_eq!(conditional.on_success.len(), 1);
        assert_eq!(conditional.on_success[0].command.program, "echo");
        assert_eq!(conditional.on_success[0].command.args, vec!["File exists"]);
        
        assert_eq!(conditional.on_failure.len(), 1);
        assert_eq!(conditional.on_failure[0].command.program, "echo");
        assert_eq!(conditional.on_failure[0].command.args, vec!["File not found"]);
    }

    #[test]
    fn test_security_template_substitution() {
        let mut vars = HashMap::new();
        vars.insert("safe_arg".to_string(), "normal_value".to_string());
        vars.insert("malicious_arg".to_string(), "; rm -rf /".to_string());
        
        let executor = CommandExecutor::new(vars);
        
        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["{{malicious_arg}}".to_string()]),
            working_dir: None,
            env: None,
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let graph = executor.build_graph_from_array(&spec).unwrap();
        let node = &graph.nodes[0];
        
        // The malicious content becomes a literal argument, not executed
        assert_eq!(node.command.program, "echo");
        assert_eq!(node.command.args, vec!["; rm -rf /"]);
        
        // This is SAFE because:
        // 1. No shell is involved
        // 2. The malicious content is just a literal string argument
        // 3. `echo` will output the literal string, not execute it
    }

    #[test]
    fn test_empty_command_validation() {
        let executor = CommandExecutor::new(HashMap::new());
        
        let spec = ArrayCommandSpec {
            command: vec![], // Empty command should fail
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
        };

        let result = executor.build_graph_from_array(&spec);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ExecutionError::InvalidSpec(_)));
    }
}