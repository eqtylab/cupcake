//! Integration tests for dual engine architecture with global config

use anyhow::Result;
use cupcake_core::engine::{Engine, global_config::GlobalPaths};
use std::env;
use std::fs;
use tempfile::TempDir;

mod test_helpers;

/// Test that engine initializes correctly without global config
#[tokio::test]
async fn test_engine_without_global_config() -> Result<()> {
    // Make sure no global config is set
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
    
    let project_dir = TempDir::new()?;
    
    // Create project structure using helper
    test_helpers::create_test_project(project_dir.path())?;
    
    // Engine should initialize without global config
    let engine = Engine::new(project_dir.path()).await?;
    
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
async fn test_engine_with_global_config() -> Result<()> {
    // Setup global config
    let global_dir = TempDir::new()?;
    env::set_var("CUPCAKE_GLOBAL_CONFIG", global_dir.path().to_str().unwrap());
    
    let global_paths = GlobalPaths::discover()?.unwrap();
    global_paths.initialize()?;
    
    // Use helper to create global structure
    test_helpers::create_test_global_config(&global_paths.root)?;
    
    // Create a simple global policy
    fs::write(
        global_paths.policies.join("test_global.rego"),
        r#"package cupcake.global.policies.test

import rego.v1

add_context contains "Global policy active"
"#
    )?;
    
    // Create project config
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;
    
    // Engine should initialize with both configs
    let engine = Engine::new(project_dir.path()).await?;
    
    // Basic smoke test - ensure it doesn't crash
    let input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test"
    });
    
    let _decision = engine.evaluate(&input, None).await?;
    
    // Clean up
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
    
    Ok(())
}

/// Test namespace isolation - global and project policies don't interfere
#[tokio::test]
async fn test_namespace_isolation() -> Result<()> {
    // Setup global config
    let global_dir = TempDir::new()?;
    env::set_var("CUPCAKE_GLOBAL_CONFIG", global_dir.path().to_str().unwrap());
    
    let global_paths = GlobalPaths::discover()?.unwrap();
    global_paths.initialize()?;
    test_helpers::create_test_global_config(&global_paths.root)?;
    
    // Create conflicting global policy (same name but global namespace)
    fs::write(
        global_paths.policies.join("conflict.rego"),
        r#"package cupcake.global.policies.conflict

import rego.v1

test_value := "global"
"#
    )?;
    
    // Create project config
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;
    
    // Create conflicting project policy (same base name but project namespace)
    fs::write(
        project_dir.path().join(".cupcake/policies/conflict.rego"),
        r#"package cupcake.policies.conflict

import rego.v1

test_value := "project"
"#
    )?;
    
    // Engine should handle both without namespace collision
    let engine = Engine::new(project_dir.path()).await?;
    
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
    
    // Clean up
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
    
    Ok(())
}