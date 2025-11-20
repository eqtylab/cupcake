//! Final verification that global actions work
//! This test proves the execution path is complete

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, global_config::GlobalPaths, Engine};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

mod common;

/// Use a marker file to prove action executed
#[tokio::test]
#[serial] // serial attribute ensures tests run one at a time, protecting global env vars
async fn test_global_action_creates_marker_file() -> Result<()> {
    // Initialize test logging
    common::init_test_logging();

    // Clean environment first
    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    common::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create a marker file path
    let marker_file = global_dir.path().join("action_executed.marker");

    // Convert path to Unix format for Git Bash on Windows
    let marker_path = if cfg!(windows) {
        let path_str = marker_file.to_str().unwrap();
        if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
            let drive = path_str.chars().next().unwrap().to_lowercase();
            let path_part = &path_str[2..].replace('\\', "/");
            format!("/{drive}{path_part}")
        } else {
            path_str.replace('\\', "/")
        }
    } else {
        marker_file.to_str().unwrap().to_string()
    };

    // Create global rulebook with action that creates a marker file
    let rulebook_content = format!(
        r#"signals: {{}}

actions:
  by_rule_id:
    GLOBAL-MARKER-001:
      - command: touch {marker_path}

builtins: {{}}
"#
    );

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy
    fs::write(
        global_paths.policies.join("claude/marker_policy.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.global.policies.marker_policy

import rego.v1

halt contains decision if {
    input.prompt == "create-marker"
    decision := {
        "rule_id": "GLOBAL-MARKER-001",
        "reason": "Creating marker file",
        "severity": "CRITICAL"
    }
}
"#,
    )?;

    // Setup project
    let project_dir = TempDir::new()?;
    common::create_test_project_for_harness(
        project_dir.path(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )?;

    // Initialize engine
    let config = cupcake_core::engine::EngineConfig {
        governance_bundle_path: None,
        governance_service_url: None,
        governance_rulebook_id: None,
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Verify marker doesn't exist yet
    assert!(
        !marker_file.exists(),
        "Marker file should not exist before action"
    );

    // Trigger the global halt with action
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "create-marker"
    });

    let decision = engine.evaluate(&input, None).await?;

    // Debug: print what decision we got
    eprintln!("Decision received: {decision:?}");

    // Verify halt decision
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Expected Halt but got: {decision:?}"
    );

    // Wait for async action to complete
    // Actions are fire-and-forget so we need to poll
    let mut attempts = 0;
    while !marker_file.exists() && attempts < 20 {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        attempts += 1;
    }

    // Verify marker was created
    assert!(
        marker_file.exists(),
        "Marker file was not created after {attempts} attempts - action did not execute!"
    );

    println!("✅ SUCCESS: Global action executed and created marker file!");
    println!("   Marker file: {marker_file:?}");
    println!("   Action executed within {} ms", attempts * 100);

    // Clean up
    Ok(())
}

/// Test that global DENY with on_any_denial action works
#[tokio::test]
#[serial]
async fn test_global_deny_on_any_denial_action() -> Result<()> {
    // Clean environment first
    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    common::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create a marker file path
    let marker_file = global_dir.path().join("deny_action.marker");

    // Convert path to Unix format for Git Bash on Windows
    let marker_path = if cfg!(windows) {
        let path_str = marker_file.to_str().unwrap();
        if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
            let drive = path_str.chars().next().unwrap().to_lowercase();
            let path_part = &path_str[2..].replace('\\', "/");
            format!("/{drive}{path_part}")
        } else {
            path_str.replace('\\', "/")
        }
    } else {
        marker_file.to_str().unwrap().to_string()
    };

    // Create global rulebook with on_any_denial action
    let rulebook_content = format!(
        r#"signals: {{}}

actions:
  on_any_denial:
    - command: touch {marker_path}

builtins: {{}}
"#
    );

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that denies
    fs::write(
        global_paths.policies.join("claude/deny_policy.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.global.policies.deny_policy

import rego.v1

deny contains decision if {
    input.tool_input.command == "dangerous"
    decision := {
        "rule_id": "GLOBAL-DENY-001",
        "reason": "Dangerous command blocked",
        "severity": "HIGH"
    }
}
"#,
    )?;

    // Setup project
    let project_dir = TempDir::new()?;
    common::create_test_project_for_harness(
        project_dir.path(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )?;

    // Initialize engine
    let config = cupcake_core::engine::EngineConfig {
        governance_bundle_path: None,
        governance_service_url: None,
        governance_rulebook_id: None,
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Verify marker doesn't exist
    assert!(!marker_file.exists());

    // Trigger global deny
    let input = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "dangerous"
        }
    });

    let decision = engine.evaluate(&input, None).await?;
    eprintln!("Deny test - Decision received: {decision:?}");
    eprintln!("Input was: {input:?}");
    assert!(
        matches!(decision, FinalDecision::Deny { .. }),
        "Expected Deny but got: {decision:?}"
    );

    // Wait for action
    let mut attempts = 0;
    while !marker_file.exists() && attempts < 20 {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        attempts += 1;
    }

    assert!(
        marker_file.exists(),
        "on_any_denial action did not execute!"
    );

    println!("✅ SUCCESS: Global on_any_denial action executed!");

    // Clean up
    Ok(())
}
