// Import only the specific helpers we need
mod common;
use common::{create_test_project_for_harness, init_test_logging};

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use cupcake_core::engine::Engine;
    use serde_json::json;
    use serial_test::serial;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Helper to create a test global configuration with test policies
    fn setup_test_global_config(global_dir: &Path) -> Result<()> {
        let policies_dir = global_dir.join("policies");
        // Use Claude harness-specific directory
        let claude_dir = policies_dir.join("claude");
        let system_dir = claude_dir.join("system");

        fs::create_dir_all(&system_dir)?;

        // Use the same fixture that works in other global tests
        fs::write(
            system_dir.join("evaluate.rego"),
            include_str!("fixtures/global_system_evaluate.rego"),
        )?;

        // Create test policies (not in builtins directory to avoid filtering)
        fs::write(
            claude_dir.join("test_system_protection.rego"),
            r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit"]
package cupcake.global.policies.test_system_protection
import rego.v1

halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Edit"
    input.tool_input.file_path == "/etc/hosts"
    decision := {
        "rule_id": "TEST-SYSTEM-PROTECTION",
        "reason": "Test: System file blocked",
        "severity": "CRITICAL"
    }
}"#,
        )?;

        fs::write(
            claude_dir.join("test_sensitive_data.rego"),
            r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Read"]
package cupcake.global.policies.test_sensitive_data
import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Read"
    input.tool_input.file_path == "/home/user/.env"
    decision := {
        "rule_id": "TEST-SENSITIVE-DATA",
        "reason": "Test: Sensitive file blocked",
        "severity": "HIGH"
    }
}"#,
        )?;

        fs::write(
            claude_dir.join("test_cupcake_exec.rego"),
            r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.global.policies.test_cupcake_exec
import rego.v1

halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "cupcake init")
    decision := {
        "rule_id": "TEST-CUPCAKE-EXEC",
        "reason": "Test: Cupcake execution blocked",
        "severity": "CRITICAL"
    }
}"#,
        )?;

        // Create rulebook with builtins enabled
        // Note: The global builtins aren't in "builtins" subdirectory in tests,
        // so we need to ensure they're not filtered out
        fs::write(
            global_dir.join("rulebook.yml"),
            r#"signals: {}
actions: {}
builtins: {}
"#,
        )?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[cfg(feature = "deterministic-tests")]
    async fn test_global_system_protection_builtin() -> Result<()> {
        crate::init_test_logging();

        let global_temp = TempDir::new()?;
        let project_temp = TempDir::new()?;

        // Setup global config
        setup_test_global_config(global_temp.path())?;

        // Setup project using test helper
        crate::create_test_project_for_harness(
            project_temp.path(),
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        )?;

        // Set global config env var
        // Create engine with global config (pass project root, not policies directory)
        let config = cupcake_core::engine::EngineConfig {
            governance_bundle_path: None,
            governance_service_url: None,
            governance_rulebook_id: None,
            global_config: Some(global_temp.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let engine = Engine::new_with_config(project_temp.path(), config).await?;

        // Test system protection blocks /etc/hosts edit
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/etc/hosts",
                "old_string": "localhost",
                "new_string": "malicious"
            }
        });

        let decision = engine.evaluate(&event, None).await?;

        // Debug what decision we got
        eprintln!("Decision for /etc/hosts edit: {decision:?}");
        eprintln!("Decision reason: {:?}", decision.reason());

        // Should be HALT decision from global builtin
        assert!(
            decision.is_halt(),
            "Expected HALT decision for /etc/hosts edit, got: {decision:?}"
        );
        assert_eq!(decision.reason(), Some("Test: System file blocked"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[cfg(feature = "deterministic-tests")]
    async fn test_global_sensitive_data_builtin() -> Result<()> {
        crate::init_test_logging();

        let global_temp = TempDir::new()?;
        let project_temp = TempDir::new()?;

        // Setup global config
        setup_test_global_config(global_temp.path())?;

        // Setup project using test helper
        crate::create_test_project_for_harness(
            project_temp.path(),
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        )?;

        // Set global config env var
        // Create engine with global config (pass project root, not policies directory)
        let config = cupcake_core::engine::EngineConfig {
            governance_bundle_path: None,
            governance_service_url: None,
            governance_rulebook_id: None,
            global_config: Some(global_temp.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let engine = Engine::new_with_config(project_temp.path(), config).await?;

        // Test sensitive data blocks .env read
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Read",
            "tool_input": {
                "file_path": "/home/user/.env"
            }
        });

        let decision = engine.evaluate(&event, None).await?;

        // Should be DENY decision from global builtin
        assert!(
            decision.is_blocking(),
            "Expected blocking decision for .env read"
        );
        assert_eq!(decision.reason(), Some("Test: Sensitive file blocked"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[cfg(feature = "deterministic-tests")]
    async fn test_global_cupcake_exec_builtin() -> Result<()> {
        crate::init_test_logging();

        let global_temp = TempDir::new()?;
        let project_temp = TempDir::new()?;

        // Setup global config
        setup_test_global_config(global_temp.path())?;

        // Setup project using test helper
        crate::create_test_project_for_harness(
            project_temp.path(),
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        )?;

        // Set global config env var
        // Create engine with global config (pass project root, not policies directory)
        let config = cupcake_core::engine::EngineConfig {
            governance_bundle_path: None,
            governance_service_url: None,
            governance_rulebook_id: None,
            global_config: Some(global_temp.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let engine = Engine::new_with_config(project_temp.path(), config).await?;

        // Test cupcake execution blocks
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": "cupcake init --global"
            }
        });

        let decision = engine.evaluate(&event, None).await?;

        // Should be HALT decision from global builtin
        assert!(
            decision.is_halt(),
            "Expected HALT decision for cupcake execution"
        );
        assert_eq!(decision.reason(), Some("Test: Cupcake execution blocked"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[cfg(feature = "deterministic-tests")]
    async fn test_global_builtins_disabled() -> Result<()> {
        crate::init_test_logging();

        let global_temp = TempDir::new()?;
        let project_temp = TempDir::new()?;

        // Setup global config with builtins disabled
        let policies_dir = global_temp.path().join("policies");
        // Use Claude harness-specific directory
        let claude_dir = policies_dir.join("claude");
        let builtins_dir = claude_dir.join("builtins");
        let system_dir = claude_dir.join("system");

        fs::create_dir_all(&builtins_dir)?;
        fs::create_dir_all(&system_dir)?;

        // Create global system evaluate policy
        fs::write(
            system_dir.join("evaluate.rego"),
            r#"package cupcake.global.system
import rego.v1

halts := collect_verbs("halt")
denials := collect_verbs("deny")
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

evaluate := {
    "halts": halts,
    "denials": denials,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
}

default collect_verbs(_) := []

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.global.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}"#,
        )?;

        // Create builtin policies (they exist but are disabled)
        fs::write(
            builtins_dir.join("system_protection.rego"),
            r#"package cupcake.global.policies.builtins.system_protection
import rego.v1

halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Edit"
    input.tool_input.file_path == "/etc/hosts"
    decision := {
        "rule_id": "TEST-SYSTEM-PROTECTION-DISABLED",
        "reason": "Should not fire when disabled",
        "severity": "CRITICAL"
    }
}"#,
        )?;

        // Create rulebook with builtins DISABLED
        fs::write(
            global_temp.path().join("rulebook.yml"),
            r#"signals: {}
actions: {}
builtins:
  system_protection:
    enabled: false
  sensitive_data_protection:
    enabled: false
  cupcake_exec_protection:
    enabled: false
"#,
        )?;

        // Setup project using test helper
        crate::create_test_project_for_harness(
            project_temp.path(),
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        )?;

        // Set global config env var
        // Create engine with global config (pass project root, not policies directory)
        let config = cupcake_core::engine::EngineConfig {
            governance_bundle_path: None,
            governance_service_url: None,
            governance_rulebook_id: None,
            global_config: Some(global_temp.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let engine = Engine::new_with_config(project_temp.path(), config).await?;

        // Test that disabled builtin does NOT fire
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/etc/hosts",
                "old_string": "localhost",
                "new_string": "malicious"
            }
        });

        let decision = engine.evaluate(&event, None).await?;

        // Should be ALLOW since builtin is disabled
        assert!(
            !decision.is_halt(),
            "Expected no HALT when builtin is disabled"
        );
        assert_ne!(decision.reason(), Some("Should not fire when disabled"));

        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[cfg(feature = "deterministic-tests")]
    async fn test_global_builtin_signals() -> Result<()> {
        crate::init_test_logging();

        let global_temp = TempDir::new()?;
        let project_temp = TempDir::new()?;

        // Setup global config
        let policies_dir = global_temp.path().join("policies");
        // Use Claude harness-specific directory
        let claude_dir = policies_dir.join("claude");
        let builtins_dir = claude_dir.join("builtins");
        let system_dir = claude_dir.join("system");

        fs::create_dir_all(&builtins_dir)?;
        fs::create_dir_all(&system_dir)?;

        // Create global system evaluate policy
        fs::write(
            system_dir.join("evaluate.rego"),
            r#"package cupcake.global.system
import rego.v1

halts := collect_verbs("halt")
denials := collect_verbs("deny")
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

evaluate := {
    "halts": halts,
    "denials": denials,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
}

default collect_verbs(_) := []

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.global.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}"#,
        )?;

        // Create builtin that uses signals - signals come through input.signals, not data!
        fs::write(
            builtins_dir.join("system_protection.rego"),
            r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit"]
package cupcake.global.policies.builtins.system_protection
import rego.v1

halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Edit"
    
    # Signals are passed in input.signals, not data!
    additional_paths := input.signals.__builtin_system_protection_paths
    message := input.signals.__builtin_system_protection_message
    
    # Check if the file path is in the additional protected paths
    some path in additional_paths
    input.tool_input.file_path == path
    
    decision := {
        "rule_id": "TEST-SYSTEM-PROTECTION-SIGNAL",
        "reason": message,
        "severity": "CRITICAL"
    }
}"#,
        )?;

        // Create rulebook with additional paths AND the generated signals
        // The signals must be present for the builtin to work!
        fs::write(
            global_temp.path().join("rulebook.yml"),
            r#"signals:
  __builtin_system_protection_paths:
    command: "echo '[\"/custom/protected/path\"]'"
    timeout_seconds: 1
  __builtin_system_protection_message:
    command: "echo 'Custom block message from signal'"
    timeout_seconds: 1
actions: {}
builtins:
  system_protection:
    enabled: true
    additional_paths: 
      - "/custom/protected/path"
    message: "Custom block message from signal"
"#,
        )?;

        // Setup project using test helper
        crate::create_test_project_for_harness(
            project_temp.path(),
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        )?;

        // Set global config env var
        // Create engine with global config (pass project root, not policies directory)
        let config = cupcake_core::engine::EngineConfig {
            governance_bundle_path: None,
            governance_service_url: None,
            governance_rulebook_id: None,
            global_config: Some(global_temp.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let engine = Engine::new_with_config(project_temp.path(), config).await?;

        // Test that builtin uses signal for additional path
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/custom/protected/path",
                "old_string": "test",
                "new_string": "malicious"
            }
        });

        let decision = engine.evaluate(&event, None).await?;

        // Debug what we got
        eprintln!("Decision for custom path: {decision:?}");
        eprintln!("Is halt? {}", decision.is_halt());
        eprintln!("Reason: {:?}", decision.reason());

        // Should be HALT with custom message
        assert!(
            decision.is_halt(),
            "Expected HALT for custom protected path, got: {decision:?}"
        );
        assert_eq!(decision.reason(), Some("Custom block message from signal"));

        Ok(())
    }
}
