//! Simplified test to verify global action execution path
//!
//! This test focuses on verifying the fix works without complex action scripts

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, global_config::GlobalPaths, Engine};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

mod test_helpers;

/// Test that global HALT with actions doesn't crash and returns correct decision
#[tokio::test]
#[serial] // serial attribute ensures tests run one at a time, protecting global env vars
async fn test_global_halt_with_actions_simple() -> Result<()> {
    // Initialize test logging
    test_helpers::init_test_logging();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    test_helpers::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create simple global rulebook with inline action (no script file)
    let rulebook_content = r#"signals: {}

actions:
  by_rule_id:
    GLOBAL-HALT-001:
      - command: "echo 'Global halt action would execute here'"

builtins: {}
"#;

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that halts
    fs::write(
        global_paths.policies.join("claude/halt_policy.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.global.policies.halt_policy

import rego.v1

halt contains decision if {
    input.prompt == "stop"
    decision := {
        "rule_id": "GLOBAL-HALT-001",
        "reason": "Global halt triggered",
        "severity": "CRITICAL"
    }
}
"#,
    )?;

    // Setup project
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Initialize engine with global config
    eprintln!(
        "Initializing engine with global config at: {:?}",
        global_dir.path()
    );
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Test: Trigger global HALT
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "stop"
    });

    eprintln!("Evaluating input to trigger global halt...");
    let decision = engine.evaluate(&input, None).await?;

    // Verify HALT decision
    eprintln!("Decision received: {decision:?}");
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Expected Halt decision but got: {decision:?}"
    );

    // The key test: If we got here without crashing, the action execution path worked
    // (even if the action itself didn't create a file)
    eprintln!("SUCCESS: Global halt with actions executed without errors");

    Ok(())
}

/// Test that global BLOCK terminates early (verifies our Block fix)
#[tokio::test]
#[serial]
async fn test_global_block_terminates_early() -> Result<()> {
    // Serialize access to global config
    // Initialize test logging
    test_helpers::init_test_logging();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    test_helpers::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create global rulebook
    let rulebook_content = r#"signals: {}
actions: {}
builtins: {}
"#;

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that blocks
    fs::write(
        global_paths.policies.join("claude/block_policy.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.global.policies.block_policy

import rego.v1

block contains decision if {
    input.source == "Test"
    decision := {
        "rule_id": "GLOBAL-BLOCK-001",
        "reason": "Global block triggered",
        "severity": "HIGH"
    }
}
"#,
    )?;

    // Setup project with conflicting allow
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create project policy that would allow (should not execute due to early termination)
    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/allow_all.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.policies.allow_all

import rego.v1

allow_override contains decision if {
    decision := {
        "rule_id": "PROJECT-ALLOW-001",
        "reason": "Project would allow",
        "severity": "LOW"
    }
}
"#,
    )?;

    // Initialize engine with global config
    eprintln!("Initializing engine for block test...");
    eprintln!("Global config at: {:?}", global_dir.path());
    eprintln!("Project config at: {:?}", project_dir.path());
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Test: Trigger global BLOCK
    let input = serde_json::json!({
        "hook_event_name": "SessionStart",
        "source": "Test"
    });

    eprintln!("Evaluating input: {input:?}");
    let decision = engine.evaluate(&input, None).await?;

    // Verify BLOCK decision (not Allow from project)
    eprintln!("Block test decision: {decision:?}");
    assert!(
        matches!(decision, FinalDecision::Block { .. }),
        "Expected Block decision but got: {decision:?}"
    );

    eprintln!("SUCCESS: Global block terminates early as expected");

    Ok(())
}
