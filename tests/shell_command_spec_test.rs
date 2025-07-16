//! Tests for ShellCommandSpec configuration and serialization
//! 
//! This test suite validates the configuration structures for shell-based
//! command specifications and their YAML serialization/deserialization.

use cupcake::config::actions::{CommandSpec, ShellCommandSpec, Action, OnFailureBehavior};
use cupcake::config::types::Settings;

#[cfg(test)]
mod shell_command_spec_tests {
    use super::*;

    #[test]
    fn test_shell_command_spec_creation() {
        let spec = ShellCommandSpec {
            script: "find /tmp -name '*.old' -delete".to_string(),
        };
        
        assert_eq!(spec.script, "find /tmp -name '*.old' -delete");
    }

    #[test]
    fn test_command_spec_shell_variant() {
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "set -euo pipefail\necho 'Complex shell script'".to_string(),
        });
        
        match spec {
            CommandSpec::Shell(shell_spec) => {
                assert_eq!(shell_spec.script, "set -euo pipefail\necho 'Complex shell script'");
            }
            _ => panic!("Expected Shell variant"),
        }
    }

    #[test]
    fn test_shell_command_spec_yaml_serialization() {
        let spec = CommandSpec::Shell(ShellCommandSpec {
            script: "terraform state list | grep '^module.old' | xargs -r terraform state rm".to_string(),
        });

        let yaml = serde_yaml_ng::to_string(&spec).unwrap();
        let expected = "mode: shell\nscript: terraform state list | grep '^module.old' | xargs -r terraform state rm\n";
        assert_eq!(yaml, expected);
    }

    #[test]
    fn test_shell_command_spec_yaml_deserialization() {
        let yaml = r#"
mode: shell
script: |
  set -euo pipefail
  for f in {a..z}*.tmp; do
    [ -e "$f" ] && rm "$f"
  done
"#;

        let spec: CommandSpec = serde_yaml_ng::from_str(yaml).unwrap();
        match spec {
            CommandSpec::Shell(shell_spec) => {
                assert!(shell_spec.script.contains("set -euo pipefail"));
                assert!(shell_spec.script.contains("for f in {a..z}*.tmp"));
            }
            _ => panic!("Expected Shell variant"),
        }
    }

    #[test]
    fn test_action_run_command_with_shell_spec() {
        let action = Action::RunCommand {
            spec: CommandSpec::Shell(ShellCommandSpec {
                script: "legacy-cleanup.sh --force".to_string(),
            }),
            on_failure: OnFailureBehavior::Block,
            on_failure_feedback: Some("Legacy cleanup failed".to_string()),
            background: false,
            timeout_seconds: Some(300),
        };

        match action {
            Action::RunCommand { spec, .. } => {
                match spec {
                    CommandSpec::Shell(shell_spec) => {
                        assert_eq!(shell_spec.script, "legacy-cleanup.sh --force");
                    }
                    _ => panic!("Expected Shell command spec"),
                }
            }
            _ => panic!("Expected RunCommand action"),
        }
    }

    #[test]
    fn test_multiline_shell_script() {
        let spec = ShellCommandSpec {
            script: r#"#!/bin/bash
set -euo pipefail

# Complex shell operations that require true shell
for service in api worker scheduler; do
    if systemctl is-active --quiet "$service"; then
        echo "Stopping $service"
        systemctl stop "$service"
    fi
done"#.to_string(),
        };

        assert!(spec.script.contains("#!/bin/bash"));
        assert!(spec.script.contains("systemctl"));
    }

    #[test]
    fn test_settings_allow_shell_default() {
        let settings = Settings::default();
        assert!(!settings.allow_shell); // Security: defaults to false
    }

    #[test]
    fn test_settings_allow_shell_serialization() {
        let settings = Settings {
            audit_logging: true,
            debug_mode: false,
            allow_shell: true,
            timeout_ms: 30000,
            sandbox_uid: None,
        };

        let yaml = serde_yaml_ng::to_string(&settings).unwrap();
        assert!(yaml.contains("allow_shell: true"));
        assert!(yaml.contains("audit_logging: true"));
        assert!(yaml.contains("debug_mode: false"));
    }

    #[test]
    fn test_settings_allow_shell_deserialization() {
        let yaml = r#"
audit_logging: false
debug_mode: true
allow_shell: true
"#;

        let settings: Settings = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(!settings.audit_logging);
        assert!(settings.debug_mode);
        assert!(settings.allow_shell);
    }

    #[test]
    fn test_edge_case_empty_shell_script() {
        let spec = ShellCommandSpec {
            script: "".to_string(),
        };

        let yaml = serde_yaml_ng::to_string(&CommandSpec::Shell(spec)).unwrap();
        assert!(yaml.contains("mode: shell"));
        assert!(yaml.contains("script: ''"));
    }

    #[test]
    fn test_shell_spec_with_dangerous_content() {
        // Test that we can serialize dangerous content - security is enforced at execution time
        let spec = ShellCommandSpec {
            script: "rm -rf / --no-preserve-root".to_string(),
        };

        // This should serialize fine - the security check happens during execution
        let yaml = serde_yaml_ng::to_string(&CommandSpec::Shell(spec)).unwrap();
        assert!(yaml.contains("rm -rf"));
    }

    #[test]
    fn test_complex_pipeline_yaml_serialization() {
        let yaml_action = r#"
type: run_command
spec:
  mode: shell
  script: |
    set -euo pipefail
    terraform state list | grep '^module.old' | xargs -r terraform state rm
    terraform plan -out=plan.tfplan
    terraform apply plan.tfplan
on_failure: block
on_failure_feedback: "Terraform cleanup and apply failed"
background: false
timeout_seconds: 600
"#;

        let action: Action = serde_yaml_ng::from_str(yaml_action).unwrap();
        match action {
            Action::RunCommand { spec, on_failure, timeout_seconds, .. } => {
                assert!(matches!(spec, CommandSpec::Shell(_)));
                assert_eq!(on_failure, OnFailureBehavior::Block);
                assert_eq!(timeout_seconds, Some(600));
            }
            _ => panic!("Expected RunCommand action"),
        }
    }
}