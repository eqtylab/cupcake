//! Tests for rulebook_security_guardrails with user-configured protected paths (Cursor format)
//!
//! Verifies that custom paths (not just .cupcake) get total lockdown protection in Cursor

#![allow(unused_imports)]

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that rulebook_security_guardrails protects user-configured custom paths in Cursor format
/// with total lockdown (blocks both reads AND writes, unlike protected_paths builtin)
#[tokio::test]
async fn test_rulebook_security_protects_custom_paths_cursor() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let cursor_dir = policies_dir.join("cursor");
    let system_dir = cursor_dir.join("system");
    let builtins_dir = cursor_dir.join("builtins");
    let shared_system_dir = cupcake_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&shared_system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    let helpers_commands = include_str!("../../fixtures/system/commands.rego");
    fs::write(shared_system_dir.join("commands.rego"), helpers_commands)?;

    let rulebook_policy =
        include_str!("../../fixtures/cursor/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    // Configure with CUSTOM protected paths (not just .cupcake)
    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
    message: "Critical files are locked down"
    protected_paths:
      - ".cupcake/"
      - "secrets/"
      - ".env.production"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::Cursor,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // TEST 1: Block afterFileEdit operations to custom paths
    let edit_secrets = json!({
        "hook_event_name": "afterFileEdit",
        "workspace_roots": [temp_dir.path().to_string_lossy()],
        "conversation_id": "test",
        "generation_id": "test",
        "file_path": "secrets/api-key.txt",
        "edits": [{
            "old_string": "old-key",
            "new_string": "sk-1234567890"
        }]
    });

    let decision = engine.evaluate(&edit_secrets, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert!(
                reason.contains("not permitted") || reason.contains("protected"),
                "Should block afterFileEdit on secrets/: {reason}"
            );
        }
        _ => panic!("Expected Deny for afterFileEdit on secrets/, got: {decision:?}"),
    }

    // TEST 2: Block beforeReadFile operations to custom paths
    let read_secrets = json!({
        "hook_event_name": "beforeReadFile",
        "workspace_roots": [temp_dir.path().to_string_lossy()],
        "conversation_id": "test",
        "generation_id": "test",
        "file_path": "secrets/api-key.txt",
        "content": "sk-1234567890"
    });

    let decision = engine.evaluate(&read_secrets, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("prohibited") || reason.contains("protected"),
                "Should block beforeReadFile on secrets/: {reason}"
            );
        }
        _ => panic!("Expected Halt for beforeReadFile on secrets/, got: {decision:?}"),
    }

    // TEST 3: Block beforeShellExecution commands mentioning custom paths
    let shell_cat_secrets = json!({
        "hook_event_name": "beforeShellExecution",
        "workspace_roots": [temp_dir.path().to_string_lossy()],
        "conversation_id": "test",
        "generation_id": "test",
        "command": "cat secrets/api-key.txt",
        "cwd": temp_dir.path().to_string_lossy()
    });

    eprintln!(
        "DEBUG: Testing shell command: {}",
        serde_json::to_string_pretty(&shell_cat_secrets)?
    );

    let decision = engine.evaluate(&shell_cat_secrets, None).await?;

    eprintln!("DEBUG: Decision for shell command: {decision:?}");

    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert!(
                reason.contains("not permitted") || reason.contains("protected"),
                "Should block shell command on secrets/: {reason}"
            );
        }
        _ => panic!("Expected Deny for shell command on secrets/, got: {decision:?}"),
    }

    // TEST 4: Block symlink creation via shell
    let shell_ln_secrets = json!({
        "hook_event_name": "beforeShellExecution",
        "workspace_roots": [temp_dir.path().to_string_lossy()],
        "conversation_id": "test",
        "generation_id": "test",
        "command": "ln -s /tmp/exposed secrets/link",
        "cwd": temp_dir.path().to_string_lossy()
    });

    let decision = engine.evaluate(&shell_ln_secrets, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert!(
                reason.contains("symlink") || reason.contains("not permitted"),
                "Should block symlink creation involving secrets/: {reason}"
            );
        }
        _ => panic!("Expected Deny for symlink to secrets/, got: {decision:?}"),
    }

    // TEST 5: Allow operations on non-protected paths
    let edit_allowed = json!({
        "hook_event_name": "afterFileEdit",
        "workspace_roots": [temp_dir.path().to_string_lossy()],
        "conversation_id": "test",
        "generation_id": "test",
        "file_path": "src/main.rs",
        "edits": [{
            "old_string": "fn main()",
            "new_string": "fn main() {"
        }]
    });

    let decision = engine.evaluate(&edit_allowed, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Expected
        }
        _ => panic!("Expected Allow for non-protected path, got: {decision:?}"),
    }

    Ok(())
}
