//! Comprehensive tests for CommandSpec configuration structures
//! 
//! These tests validate the new secure command specification system
//! introduced in Plan 008 to replace vulnerable shell-based execution.

use cupcake::config::actions::{
    Action, ArrayCommandSpec, CommandSpec, EnvVar, OnFailureBehavior, PipeCommand,
};
use cupcake::config::conditions::Condition;
use serde_yaml_ng;
use std::collections::HashMap;

#[cfg(test)]
mod command_spec_tests {
    use super::*;

    /// Test basic ArrayCommandSpec structure and serialization
    #[test]
    fn test_array_command_spec_basic() {
        let spec = ArrayCommandSpec {
            command: vec!["git".to_string()],
            args: Some(vec!["status".to_string(), "-s".to_string()]),
            working_dir: Some("repo".to_string()),
            env: Some(vec![EnvVar {
                name: "GIT_TRACE".to_string(),
                value: "1".to_string(),
            }]),
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        // Validate structure
        assert_eq!(spec.command, vec!["git"]);
        assert_eq!(spec.args.as_ref().unwrap(), &vec!["status", "-s"]);
        assert_eq!(spec.working_dir.as_ref().unwrap(), "repo");
        assert_eq!(spec.env.as_ref().unwrap().len(), 1);
        assert_eq!(spec.env.as_ref().unwrap()[0].name, "GIT_TRACE");
    }

    /// Test CommandSpec enum with Array variant
    #[test]
    fn test_command_spec_array_variant() {
        let command_spec = CommandSpec::Array(ArrayCommandSpec {
            command: vec!["npm".to_string()],
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

        match command_spec {
            CommandSpec::Array(spec) => {
                assert_eq!(spec.command, vec!["npm"]);
                assert_eq!(spec.args.as_ref().unwrap(), &vec!["test"]);
            }
        }
    }

    /// Test YAML serialization/deserialization of basic ArrayCommandSpec
    #[test]
    fn test_array_command_spec_yaml_serialization() {
        let spec = CommandSpec::Array(ArrayCommandSpec {
            command: vec!["cargo".to_string()],
            args: Some(vec!["build".to_string(), "--release".to_string()]),
            working_dir: Some("project".to_string()),
            env: Some(vec![
                EnvVar {
                    name: "RUSTFLAGS".to_string(),
                    value: "-C target-cpu=native".to_string(),
                },
                EnvVar {
                    name: "CARGO_TERM_COLOR".to_string(),
                    value: "always".to_string(),
                },
            ]),
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        });

        // Serialize to YAML
        let yaml = serde_yaml_ng::to_string(&spec).expect("Failed to serialize to YAML");
        
        // Verify YAML contains expected structure
        assert!(yaml.contains("mode: array"));
        assert!(yaml.contains("command:"));
        assert!(yaml.contains("- cargo"));
        assert!(yaml.contains("workingDir: project"));
        assert!(yaml.contains("RUSTFLAGS"));

        // Deserialize back
        let deserialized: CommandSpec = serde_yaml_ng::from_str(&yaml)
            .expect("Failed to deserialize from YAML");

        // Verify round-trip correctness
        match deserialized {
            CommandSpec::Array(deserialized_spec) => {
                assert_eq!(deserialized_spec.command, vec!["cargo"]);
                assert_eq!(
                    deserialized_spec.args.as_ref().unwrap(),
                    &vec!["build", "--release"]
                );
                assert_eq!(deserialized_spec.working_dir.as_ref().unwrap(), "project");
                assert_eq!(deserialized_spec.env.as_ref().unwrap().len(), 2);
            }
        }
    }

    /// Test composition operators in ArrayCommandSpec
    #[test]
    fn test_array_command_spec_with_operators() {
        let spec = ArrayCommandSpec {
            command: vec!["npm".to_string()],
            args: Some(vec!["test".to_string()]),
            working_dir: None,
            env: None,
            pipe: Some(vec![PipeCommand {
                cmd: vec!["grep".to_string(), "-v".to_string(), "WARNING".to_string()],
            }]),
            redirect_stdout: Some("test.log".to_string()),
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: Some(true),
            on_success: Some(vec![ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["Tests passed!".to_string()]),
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
            on_failure: None,
        };

        // Validate pipe configuration
        assert!(spec.pipe.is_some());
        assert_eq!(spec.pipe.as_ref().unwrap().len(), 1);
        assert_eq!(
            spec.pipe.as_ref().unwrap()[0].cmd,
            vec!["grep", "-v", "WARNING"]
        );

        // Validate redirect configuration
        assert_eq!(spec.redirect_stdout.as_ref().unwrap(), "test.log");
        assert_eq!(spec.merge_stderr, Some(true));

        // Validate conditional execution
        assert!(spec.on_success.is_some());
        assert_eq!(spec.on_success.as_ref().unwrap().len(), 1);
        assert_eq!(
            spec.on_success.as_ref().unwrap()[0].command,
            vec!["echo"]
        );
    }

    /// Test Action::RunCommand with new CommandSpec
    #[test]
    fn test_action_run_command_with_spec() {
        let action = Action::RunCommand {
            spec: CommandSpec::Array(ArrayCommandSpec {
                command: vec!["docker".to_string()],
                args: Some(vec![
                    "build".to_string(),
                    "-t".to_string(),
                    "myimage:latest".to_string(),
                    ".".to_string(),
                ]),
                working_dir: Some("backend".to_string()),
                env: Some(vec![EnvVar {
                    name: "DOCKER_BUILDKIT".to_string(),
                    value: "1".to_string(),
                }]),
                pipe: None,
                redirect_stdout: Some("build.log".to_string()),
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }),
            on_failure: OnFailureBehavior::Block,
            on_failure_feedback: Some("Docker build failed".to_string()),
            background: false,
            timeout_seconds: Some(300),
        };

        // Validate action properties
        assert!(action.requires_execution());
        assert!(action.is_hard_action()); // Block on failure makes it hard

        // Test YAML serialization
        let yaml = serde_yaml_ng::to_string(&action).expect("Failed to serialize action");
        assert!(yaml.contains("type: run_command"));
        assert!(yaml.contains("mode: array"));
        assert!(yaml.contains("docker"));
    }

    /// Test Condition::Check with new CommandSpec
    #[test]
    fn test_condition_check_with_spec() {
        let condition = Condition::Check {
            spec: CommandSpec::Array(ArrayCommandSpec {
                command: vec!["test".to_string()],
                args: Some(vec!["-f".to_string(), "{{file_path}}".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }),
            expect_success: true,
        };

        // Test YAML serialization
        let yaml = serde_yaml_ng::to_string(&condition).expect("Failed to serialize condition");
        assert!(yaml.contains("type: check"));
        assert!(yaml.contains("mode: array"));
        assert!(yaml.contains("test"));
        assert!(yaml.contains("{{file_path}}"));
    }

    /// Test security: verify template variables are preserved for safe substitution
    #[test]
    fn test_template_variable_preservation() {
        let spec = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec![
                "Processing {{tool_input.file_path}}".to_string(),
                "by {{env.USER}}".to_string(),
            ]),
            working_dir: Some("{{tool_input.working_dir}}".to_string()),
            env: Some(vec![EnvVar {
                name: "CUSTOM_VAR".to_string(),
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

        // Serialize and verify template variables are preserved exactly
        let yaml = serde_yaml_ng::to_string(&CommandSpec::Array(spec))
            .expect("Failed to serialize");
        
        assert!(yaml.contains("{{tool_input.file_path}}"));
        assert!(yaml.contains("{{env.USER}}"));
        assert!(yaml.contains("{{tool_input.working_dir}}"));
        assert!(yaml.contains("{{session_id}}"));
    }

    /// Test complex composition with multiple operators
    #[test]
    fn test_complex_command_composition() {
        let yaml_input = r#"
mode: array
command: [npm]
args: [test]
workingDir: backend/
env:
  - name: NODE_ENV
    value: test
  - name: CI
    value: "true"
pipe:
  - cmd: [grep, -v, WARNING]
  - cmd: [tee, result.log]
redirectStdout: final.log
mergeStderr: true
onSuccess:
  - command: [echo]
    args: ["Tests passed successfully!"]
  - command: [notify-send]
    args: ["Build Status", "All tests passed"]
onFailure:
  - command: [echo]
    args: ["Tests failed!"]
    redirectStderr: error.log
"#;

        let spec: CommandSpec = serde_yaml_ng::from_str(yaml_input)
            .expect("Failed to parse complex YAML");

        match spec {
            CommandSpec::Array(array_spec) => {
                // Validate basic command
                assert_eq!(array_spec.command, vec!["npm"]);
                assert_eq!(array_spec.args.as_ref().unwrap(), &vec!["test"]);
                assert_eq!(array_spec.working_dir.as_ref().unwrap(), "backend/");

                // Validate environment
                assert_eq!(array_spec.env.as_ref().unwrap().len(), 2);
                assert_eq!(array_spec.env.as_ref().unwrap()[0].name, "NODE_ENV");
                assert_eq!(array_spec.env.as_ref().unwrap()[1].value, "true");

                // Validate pipe chain
                assert_eq!(array_spec.pipe.as_ref().unwrap().len(), 2);
                assert_eq!(
                    array_spec.pipe.as_ref().unwrap()[0].cmd,
                    vec!["grep", "-v", "WARNING"]
                );
                assert_eq!(
                    array_spec.pipe.as_ref().unwrap()[1].cmd,
                    vec!["tee", "result.log"]
                );

                // Validate redirects
                assert_eq!(array_spec.redirect_stdout.as_ref().unwrap(), "final.log");
                assert_eq!(array_spec.merge_stderr, Some(true));

                // Validate conditional execution
                assert_eq!(array_spec.on_success.as_ref().unwrap().len(), 2);
                assert_eq!(array_spec.on_failure.as_ref().unwrap().len(), 1);
            }
        }
    }

    /// Test edge cases and validation
    #[test]
    fn test_edge_cases() {
        // Minimal valid spec
        let minimal = ArrayCommandSpec {
            command: vec!["true".to_string()],
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

        let yaml = serde_yaml_ng::to_string(&CommandSpec::Array(minimal))
            .expect("Failed to serialize minimal spec");
        let _: CommandSpec = serde_yaml_ng::from_str(&yaml)
            .expect("Failed to deserialize minimal spec");

        // Empty environment list should work
        let with_empty_env = ArrayCommandSpec {
            command: vec!["echo".to_string()],
            args: Some(vec!["test".to_string()]),
            working_dir: None,
            env: Some(vec![]), // Empty but present
            pipe: None,
            redirect_stdout: None,
            append_stdout: None,
            redirect_stderr: None,
            merge_stderr: None,
            on_success: None,
            on_failure: None,
        };

        let yaml = serde_yaml_ng::to_string(&CommandSpec::Array(with_empty_env))
            .expect("Failed to serialize with empty env");
        let _: CommandSpec = serde_yaml_ng::from_str(&yaml)
            .expect("Failed to deserialize with empty env");
    }
}