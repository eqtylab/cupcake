//! Tests for StringCommandSpec configuration and serialization
//! 
//! This test suite validates the configuration structures for string-based
//! command specifications and their YAML serialization/deserialization.

use cupcake::config::actions::{CommandSpec, StringCommandSpec, Action, OnFailureBehavior};

#[cfg(test)]
mod string_command_spec_tests {
    use super::*;

    #[test]
    fn test_string_command_spec_creation() {
        let spec = StringCommandSpec {
            command: "npm test | grep PASS".to_string(),
        };
        
        assert_eq!(spec.command, "npm test | grep PASS");
    }

    #[test]
    fn test_command_spec_string_variant() {
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo hello | grep hello".to_string(),
        });
        
        match spec {
            CommandSpec::String(string_spec) => {
                assert_eq!(string_spec.command, "echo hello | grep hello");
            }
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_string_command_spec_yaml_serialization() {
        let spec = CommandSpec::String(StringCommandSpec {
            command: "cargo build --release".to_string(),
        });

        let yaml = serde_yaml_ng::to_string(&spec).unwrap();
        let expected = "mode: string\ncommand: cargo build --release\n";
        assert_eq!(yaml, expected);
    }

    #[test]
    fn test_string_command_spec_yaml_deserialization() {
        let yaml = r#"
mode: string
command: "npm test | tee result.log"
"#;

        let spec: CommandSpec = serde_yaml_ng::from_str(yaml).unwrap();
        match spec {
            CommandSpec::String(string_spec) => {
                assert_eq!(string_spec.command, "npm test | tee result.log");
            }
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_action_run_command_with_string_spec() {
        let action = Action::RunCommand {
            spec: CommandSpec::String(StringCommandSpec {
                command: "echo test && echo success".to_string(),
            }),
            on_failure: OnFailureBehavior::Continue,
            on_failure_feedback: None,
            background: false,
            timeout_seconds: None,
        };

        match action {
            Action::RunCommand { spec, .. } => {
                match spec {
                    CommandSpec::String(string_spec) => {
                        assert_eq!(string_spec.command, "echo test && echo success");
                    }
                    _ => panic!("Expected String CommandSpec"),
                }
            }
            _ => panic!("Expected RunCommand action"),
        }
    }

    #[test]
    fn test_complex_pipeline_yaml_serialization() {
        let action = Action::RunCommand {
            spec: CommandSpec::String(StringCommandSpec {
                command: "docker ps -a | grep backend | awk '{print $1}' > containers.txt".to_string(),
            }),
            on_failure: OnFailureBehavior::Block,
            on_failure_feedback: Some("Docker command failed".to_string()),
            background: false,
            timeout_seconds: Some(30),
        };

        let yaml = serde_yaml_ng::to_string(&action).unwrap();
        let deserialized: Action = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_string_spec_with_shell_operators() {
        let test_cases = vec![
            "echo hello | grep hello",
            "npm test > test.log",
            "build.sh >> build.log",
            "test.sh && echo success",
            "validate.sh || echo failed",
            "find . -name '*.rs' | wc -l",
            "ls -la | grep -v node_modules | sort",
        ];

        for command in test_cases {
            let spec = CommandSpec::String(StringCommandSpec {
                command: command.to_string(),
            });

            // Test serialization roundtrip
            let yaml = serde_yaml_ng::to_string(&spec).unwrap();
            let deserialized: CommandSpec = serde_yaml_ng::from_str(&yaml).unwrap();
            
            assert_eq!(spec, deserialized);
        }
    }

    #[test]
    fn test_string_spec_with_quotes() {
        let spec = CommandSpec::String(StringCommandSpec {
            command: r#"grep "Hello World" file.txt | head -5"#.to_string(),
        });

        let yaml = serde_yaml_ng::to_string(&spec).unwrap();
        let deserialized: CommandSpec = serde_yaml_ng::from_str(&yaml).unwrap();
        
        assert_eq!(spec, deserialized);
    }

    #[test]
    fn test_edge_case_empty_string() {
        let spec = CommandSpec::String(StringCommandSpec {
            command: "".to_string(),
        });

        let yaml = serde_yaml_ng::to_string(&spec).unwrap();
        let deserialized: CommandSpec = serde_yaml_ng::from_str(&yaml).unwrap();
        
        assert_eq!(spec, deserialized);
    }

    #[test]
    fn test_multiline_command_string() {
        let spec = CommandSpec::String(StringCommandSpec {
            command: "echo 'line 1' && echo 'line 2'".to_string(),
        });

        let yaml = serde_yaml_ng::to_string(&spec).unwrap();
        let deserialized: CommandSpec = serde_yaml_ng::from_str(&yaml).unwrap();
        
        assert_eq!(spec, deserialized);
    }
}