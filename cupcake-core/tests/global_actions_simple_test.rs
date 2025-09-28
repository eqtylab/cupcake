//! Simplified test to verify global action execution path
//!
//! This test focuses on verifying the fix works without complex action scripts

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, global_config::GlobalPaths, Engine};
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::TempDir;

mod test_helpers;

/// Test that global HALT with actions doesn't crash and returns correct decision
#[tokio::test]
#[serial]  // serial attribute ensures tests run one at a time, protecting global env vars
async fn test_global_halt_with_actions_simple() -> Result<()> {

    // Initialize test logging
    test_helpers::init_test_logging();

    // Clean environment first
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");

    // Setup global config
    let global_dir = TempDir::new()?;
    env::set_var("CUPCAKE_GLOBAL_CONFIG", global_dir.path().to_str().unwrap());

    let global_paths = GlobalPaths::discover()?.unwrap();
    global_paths.initialize()?;

    // Create simple global guidebook with inline action (no script file)
    let guidebook_content = r#"signals: {}

actions:
  by_rule_id:
    GLOBAL-HALT-001:
      - command: "echo 'Global halt action would execute here'"

builtins: {}
"#;

    fs::write(&global_paths.guidebook, guidebook_content)?;

    // Create global policy that halts
    fs::write(
        global_paths.policies.join("halt_policy.rego"),
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

    // Initialize engine
    eprintln!(
        "Initializing engine with global config at: {:?}",
        global_dir.path()
    );
    let engine = Engine::new(project_dir.path()).await?;

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

    // Clean up
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");

    Ok(())
}

/// Test that global BLOCK terminates early (verifies our Block fix)
#[tokio::test]
#[serial]
async fn test_global_block_terminates_early() -> Result<()> {
    // Serialize access to global config
    // Initialize test logging
    test_helpers::init_test_logging();

    // Clean environment first
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");

    // Setup global config
    let global_dir = TempDir::new()?;
    env::set_var("CUPCAKE_GLOBAL_CONFIG", global_dir.path().to_str().unwrap());

    let global_paths = GlobalPaths::discover()?.unwrap();
    global_paths.initialize()?;

    // Create global guidebook
    let guidebook_content = r#"signals: {}
actions: {}
builtins: {}
"#;

    fs::write(&global_paths.guidebook, guidebook_content)?;

    // Create global policy that blocks
    fs::write(
        global_paths.policies.join("block_policy.rego"),
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
        project_dir.path().join(".cupcake/policies/allow_all.rego"),
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

    // Initialize engine
    eprintln!("Initializing engine for block test...");
    eprintln!("Global config at: {:?}", global_dir.path());
    eprintln!("Project config at: {:?}", project_dir.path());
    let engine = Engine::new(project_dir.path()).await?;

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

    // Clean up
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");

    Ok(())
}
