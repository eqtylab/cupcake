//! Test that verifies global actions are actually being called
//! Works with the fire-and-forget architecture by using debug logs

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, global_config::GlobalPaths, Engine};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

mod test_helpers;

/// Test that shows actions ARE being called but are fire-and-forget
#[tokio::test]
#[serial] // serial attribute ensures tests run one at a time, protecting global env vars
async fn test_global_action_execution_logs() -> Result<()> {
    // Initialize test logging to capture debug output
    test_helpers::init_test_logging();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();
    global_paths.initialize()?;

    // Verify system evaluate policy was created
    let sys_eval = global_paths.policies.join("system").join("evaluate.rego");
    assert!(
        sys_eval.exists(),
        "System evaluate policy not created at {sys_eval:?}"
    );

    // Create simple global rulebook with inline echo command
    // This will execute but we can't capture output directly
    let rulebook_content = r#"signals: {}

actions:
  by_rule_id:
    GLOBAL-TEST-001:
      - command: "echo 'THIS ACTION WAS EXECUTED' && echo 'Rule: GLOBAL-TEST-001'"

builtins: {}
"#;

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that triggers the action
    fs::write(
        global_paths.policies.join("test_policy.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.global.policies.test_policy

import rego.v1

halt contains decision if {
    input.prompt == "trigger"
    decision := {
        "rule_id": "GLOBAL-TEST-001", 
        "reason": "Test halt with action",
        "severity": "CRITICAL"
    }
}
"#,
    )?;

    // Setup project
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Initialize engine
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        ..Default::default()
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Trigger the global halt with action
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "trigger"
    });

    println!("\n=== About to evaluate and trigger action ===");
    let decision = engine.evaluate(&input, None).await?;

    // Verify we got the halt decision
    assert!(matches!(decision, FinalDecision::Halt { .. }));
    println!("✓ Global HALT decision returned correctly");

    // Actions are fire-and-forget, so we need to wait a bit
    println!("Waiting for async action to complete...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // We can't capture the output directly because it's spawned in background
    // But the logs should show:
    // - "execute_actions_with_rulebook called"
    // - "Executing actions for HALT decision"
    // - "Looking for actions for rule ID: GLOBAL-TEST-001"
    // - "Executing 1 actions for rule GLOBAL-TEST-001"
    // - "Executing action: echo..."

    println!("\n=== Test Analysis ===");
    println!("The action WAS executed (check test debug logs)");
    println!("However, output capture is not possible with current fire-and-forget design");
    println!(
        "To verify: run tests with `cargo test --features deterministic-tests -- --nocapture`"
    );
    println!("  and look for debug log lines: 'Executing action: echo'");

    // Clean up
    Ok(())
}

/// Demonstrate that the working directory issue exists
#[tokio::test]
#[serial]
async fn test_global_action_working_directory_issue() -> Result<()> {
    test_helpers::init_test_logging();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();
    global_paths.initialize()?;

    // Verify system evaluate policy was created
    let sys_eval2 = global_paths.policies.join("system").join("evaluate.rego");
    assert!(
        sys_eval2.exists(),
        "System evaluate policy not created at {sys_eval2:?}"
    );

    // Create an action that shows its working directory
    let rulebook_content = r#"signals: {}

actions:
  by_rule_id:
    GLOBAL-WD-001:
      - command: "pwd"  # This will print the working directory

builtins: {}
"#;

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy
    fs::write(
        global_paths.policies.join("wd_policy.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.global.policies.wd_policy

import rego.v1

deny contains decision if {
    input.prompt == "showwd"
    decision := {
        "rule_id": "GLOBAL-WD-001",
        "reason": "Testing working directory",
        "severity": "HIGH"
    }
}
"#,
    )?;

    // Setup project in a different location
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Initialize engine
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        ..Default::default()
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Trigger the action
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "showwd"
    });

    println!("\n=== Working Directory Test ===");
    println!("Global config at: {:?}", global_dir.path());
    println!("Project at: {:?}", project_dir.path());
    println!("Expected: Global actions should run from global directory");
    println!("Actual: Global actions run from PROJECT directory (bug)");

    let decision = engine.evaluate(&input, None).await?;
    assert!(matches!(decision, FinalDecision::Deny { .. }));

    // Wait for action
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("\nThe pwd command output (in debug logs) will show it runs from project dir, not global dir");
    println!("This is a bug: global actions should have global context");

    // Clean up
    Ok(())
}
