//! Integration tests for dual engine architecture with global config

use anyhow::Result;
use cupcake_core::engine::{global_config::GlobalPaths, Engine};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

mod common;

/// Test that engine initializes correctly without global config
#[tokio::test]
#[serial]
async fn test_engine_without_global_config() -> Result<()> {
    // Initialize test logging
    common::init_test_logging();

    let project_dir = TempDir::new()?;

    // Create project structure using helper
    common::create_test_project_for_harness(
        project_dir.path(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )?;

    // Engine should initialize without global config
    let engine = Engine::new(
        project_dir.path(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )
    .await?;

    // Verify basic evaluation works
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test"
    });

    let decision = engine.evaluate(&input, None).await?;

    // Should allow by default
    assert!(matches!(
        decision,
        cupcake_core::engine::decision::FinalDecision::Allow { .. }
    ));

    Ok(())
}

/// Test that engine initializes with both global and project config
#[tokio::test]
#[serial]
async fn test_engine_with_global_config() -> Result<()> {
    // Initialize test logging
    common::init_test_logging();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    common::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();

    // Use helper to create global structure
    common::create_test_global_config(&global_paths.root)?;

    // Create a simple global policy
    fs::write(
        global_paths.policies.join("claude/test_global.rego"),
        r#"package cupcake.global.policies.test

import rego.v1

add_context contains "Global policy active"
"#,
    )?;

    // Create project config
    let project_dir = TempDir::new()?;
    common::create_test_project_for_harness(
        project_dir.path(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )?;

    // Engine should initialize with both configs
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Basic smoke test - ensure it doesn't crash
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test"
    });

    let _decision = engine.evaluate(&input, None).await?;

    Ok(())
}

/// Test namespace isolation - global and project policies don't interfere
#[tokio::test]
#[serial]
async fn test_namespace_isolation() -> Result<()> {
    // Initialize test logging
    common::init_test_logging();

    // Setup global config
    let global_dir = TempDir::new()?;
    let global_root = global_dir.path().to_path_buf();

    // Create global config structure with evaluate.rego
    common::create_test_global_config(global_dir.path())?;
    let global_paths = GlobalPaths::discover_with_override(Some(global_root.clone()))?.unwrap();
    common::create_test_global_config(&global_paths.root)?;

    // Create conflicting global policy (same name but global namespace)
    fs::write(
        global_paths.policies.join("claude/conflict.rego"),
        r#"package cupcake.global.policies.conflict

import rego.v1

test_value := "global"
"#,
    )?;

    // Create project config
    let project_dir = TempDir::new()?;
    common::create_test_project_for_harness(
        project_dir.path(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )?;

    // Create conflicting project policy (same base name but project namespace)
    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/conflict.rego"),
        r#"package cupcake.policies.conflict

import rego.v1

test_value := "project"
"#,
    )?;

    // Engine should handle both without namespace collision
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(global_root),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Policies are isolated - no runtime errors expected
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test"
    });

    let decision = engine.evaluate(&input, None).await?;
    assert!(matches!(
        decision,
        cupcake_core::engine::decision::FinalDecision::Allow { .. }
    ));

    Ok(())
}
