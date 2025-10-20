//! Integration tests for global action execution
//!
//! Verifies that global actions are properly executed when global policies
//! trigger Halt, Deny, or Block decisions.

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, global_config::GlobalPaths, Engine};
use serial_test::serial;
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

mod test_helpers;

/// Shared state to track action execution
static ACTION_LOG: once_cell::sync::Lazy<Arc<Mutex<Vec<String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Test that global HALT executes global actions
#[tokio::test]
#[serial] // serial attribute ensures tests run one at a time, protecting global env vars
#[cfg(not(windows))] // Uses /tmp path hardcoded in bash script
async fn test_global_halt_executes_actions() -> Result<()> {
    // Clear action log
    ACTION_LOG.lock().unwrap().clear();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    test_helpers::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create global rulebook with action
    let action_script = global_paths.actions.join("log_halt.sh");
    fs::write(
        &action_script,
        r#"#!/bin/bash
echo "GLOBAL_HALT_ACTION_EXECUTED" >> /tmp/cupcake_test_actions.log
"#,
    )?;

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&action_script, fs::Permissions::from_mode(0o755))?;
    }

    // Convert path to Unix format for Git Bash on Windows
    let script_path = if cfg!(windows) {
        let path_str = action_script.to_str().unwrap();
        if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
            let drive = path_str.chars().next().unwrap().to_lowercase();
            let path_part = &path_str[2..].replace('\\', "/");
            format!("/{drive}{path_part}")
        } else {
            path_str.replace('\\', "/")
        }
    } else {
        action_script.to_str().unwrap().to_string()
    };

    // Update global rulebook with action
    let rulebook_content = format!(
        r#"signals: {{}}

actions:
  by_rule_id:
    GLOBAL-HALT-001:
      - command: "{script_path}"

builtins: {{}}
"#
    );

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that halts
    fs::write(
        global_paths.policies.join("claude/halt_test.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.global.policies.halt_test

import rego.v1

halt contains decision if {
    input.prompt == "dangerous"
    decision := {
        "rule_id": "GLOBAL-HALT-001",
        "reason": "Global policy halted execution",
        "severity": "CRITICAL"
    }
}
"#,
    )?;

    // Setup project
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Initialize engine with global config
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
        "prompt": "dangerous"
    });

    let decision = engine.evaluate(&input, None).await?;

    // Debug: Print what decision we actually got
    eprintln!("Global HALT test - Decision received: {decision:?}");

    // Verify HALT decision
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Expected Halt decision but got: {decision:?}"
    );

    // Wait longer for async action execution to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify action was executed
    let log_file = std::path::Path::new("/tmp/cupcake_test_actions.log");
    if log_file.exists() {
        let log_content = fs::read_to_string(log_file)?;
        assert!(
            log_content.contains("GLOBAL_HALT_ACTION_EXECUTED"),
            "Global action was not executed! Log content: {log_content}"
        );
        // Clean up log file
        fs::remove_file(log_file)?;
    } else {
        panic!("Action log file was not created - global action did not execute!");
    }

    Ok(())
}

/// Test that global DENY executes global actions
#[tokio::test]
#[serial]
#[cfg(not(windows))] // Uses /tmp path hardcoded in bash script
async fn test_global_deny_executes_actions() -> Result<()> {
    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    test_helpers::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create global rulebook with on_any_denial action
    let action_script = global_paths.actions.join("log_deny.sh");
    fs::write(
        &action_script,
        r#"#!/bin/bash
echo "GLOBAL_DENY_ACTION_EXECUTED" >> /tmp/cupcake_test_deny.log
"#,
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&action_script, fs::Permissions::from_mode(0o755))?;
    }

    // Convert path to Unix format for Git Bash on Windows
    let script_path = if cfg!(windows) {
        let path_str = action_script.to_str().unwrap();
        if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
            let drive = path_str.chars().next().unwrap().to_lowercase();
            let path_part = &path_str[2..].replace('\\', "/");
            format!("/{drive}{path_part}")
        } else {
            path_str.replace('\\', "/")
        }
    } else {
        action_script.to_str().unwrap().to_string()
    };

    let rulebook_content = format!(
        r#"signals: {{}}

actions:
  on_any_denial:
    - command: "{script_path}"

builtins: {{}}
"#
    );

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that denies
    fs::write(
        global_paths.policies.join("claude/deny_test.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.global.policies.deny_test

import rego.v1

deny contains decision if {
    input.tool_input.command == "rm -rf /"
    decision := {
        "rule_id": "GLOBAL-DENY-001",
        "reason": "Global policy denied dangerous command",
        "severity": "CRITICAL"
    }
}
"#,
    )?;

    // Setup project
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Initialize engine with global config
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Test: Trigger global DENY
    let input = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        }
    });

    let decision = engine.evaluate(&input, None).await?;

    // Verify DENY decision
    assert!(matches!(decision, FinalDecision::Deny { .. }));

    // Wait for async action
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify action was executed
    let log_file = std::path::Path::new("/tmp/cupcake_test_deny.log");
    if log_file.exists() {
        let log_content = fs::read_to_string(log_file)?;
        assert!(
            log_content.contains("GLOBAL_DENY_ACTION_EXECUTED"),
            "Global deny action was not executed! Log content: {log_content}"
        );
        fs::remove_file(log_file)?;
    } else {
        panic!("Deny action log file was not created - global action did not execute!");
    }

    Ok(())
}

/// Test that global BLOCK executes global actions (and terminates early)
#[tokio::test]
#[serial]
#[cfg(not(windows))] // Uses /tmp path hardcoded in bash script
async fn test_global_block_executes_actions() -> Result<()> {
    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    test_helpers::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Create global rulebook with block action
    let action_script = global_paths.actions.join("log_block.sh");
    fs::write(
        &action_script,
        r#"#!/bin/bash
echo "GLOBAL_BLOCK_ACTION_EXECUTED" >> /tmp/cupcake_test_block.log
"#,
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&action_script, fs::Permissions::from_mode(0o755))?;
    }

    // Convert path to Unix format for Git Bash on Windows
    let script_path = if cfg!(windows) {
        let path_str = action_script.to_str().unwrap();
        if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
            let drive = path_str.chars().next().unwrap().to_lowercase();
            let path_part = &path_str[2..].replace('\\', "/");
            format!("/{drive}{path_part}")
        } else {
            path_str.replace('\\', "/")
        }
    } else {
        action_script.to_str().unwrap().to_string()
    };

    let rulebook_content = format!(
        r#"signals: {{}}

actions:
  by_rule_id:
    GLOBAL-BLOCK-001:
      - command: "{script_path}"

builtins: {{}}
"#
    );

    fs::write(&global_paths.rulebook, rulebook_content)?;

    // Create global policy that blocks
    fs::write(
        global_paths.policies.join("claude/block_test.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.global.policies.block_test

import rego.v1

block contains decision if {
    input.source == "startup"
    decision := {
        "rule_id": "GLOBAL-BLOCK-001",
        "reason": "Global policy blocked session start",
        "severity": "HIGH"
    }
}
"#,
    )?;

    // Setup project with conflicting allow policy
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create project policy that would allow (should never run due to global block)
    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/allow_session.rego"),
        r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.policies.allow_session

import rego.v1

allow_override contains decision if {
    decision := {
        "rule_id": "PROJECT-ALLOW-001",
        "reason": "Project allows session",
        "severity": "LOW"
    }
}
"#,
    )?;

    // Initialize engine with global config
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
        "source": "startup"
    });

    let decision = engine.evaluate(&input, None).await?;

    // Verify BLOCK decision (and that it terminated early, not allowing project policy)
    assert!(matches!(decision, FinalDecision::Block { .. }));

    // Wait for async action
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify action was executed
    let log_file = std::path::Path::new("/tmp/cupcake_test_block.log");
    if log_file.exists() {
        let log_content = fs::read_to_string(log_file)?;
        assert!(
            log_content.contains("GLOBAL_BLOCK_ACTION_EXECUTED"),
            "Global block action was not executed! Log content: {log_content}"
        );
        fs::remove_file(log_file)?;
    } else {
        panic!("Block action log file was not created - global action did not execute!");
    }

    Ok(())
}
