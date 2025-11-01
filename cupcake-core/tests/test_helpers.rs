//! Test helper functions for integration tests

use anyhow::Result;
use cupcake_core::harness::types::HarnessType;
use std::fs;
use std::path::Path;
use std::sync::Once;

/// Initialize logging for tests (only once per test run)
#[allow(dead_code)]
static INIT: Once = Once::new();

#[allow(dead_code)]
pub fn init_test_logging() {
    INIT.call_once(|| {
        // Use tracing subscriber for tests since the engine uses tracing
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

        let _ = tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_test_writer()
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(false)
                    .with_thread_names(false),
            )
            .with(tracing_subscriber::filter::EnvFilter::from_default_env())
            .try_init();
    });
}

/// Create a complete cupcake project structure for testing
///
/// IMPORTANT: This creates BOTH claude/ and cursor/ directories which can cause
/// duplicate package errors during compilation. Use `create_test_project_for_harness`
/// instead when testing a specific harness.
pub fn create_test_project(project_path: &Path) -> Result<()> {
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");

    // Create harness-specific directory structures for both Claude and Cursor
    let claude_dir = policies_dir.join("claude");
    let claude_system_dir = claude_dir.join("system");
    let cursor_dir = policies_dir.join("cursor");
    let cursor_system_dir = cursor_dir.join("system");

    fs::create_dir_all(&claude_system_dir)?;
    fs::create_dir_all(&cursor_system_dir)?;
    fs::create_dir_all(cupcake_dir.join("signals"))?;
    fs::create_dir_all(cupcake_dir.join("actions"))?;

    // Create minimal rulebook
    fs::write(
        cupcake_dir.join("rulebook.yml"),
        "signals: {}\nactions: {}\nbuiltins: {}",
    )?;

    // Use fixture for system policy (same for both harnesses)
    let system_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(claude_system_dir.join("evaluate.rego"), system_policy)?;
    fs::write(cursor_system_dir.join("evaluate.rego"), system_policy)?;

    // Add minimal policy to ensure compilation works (for both harnesses)
    let minimal_policy = include_str!("fixtures/minimal_policy.rego");
    fs::write(claude_dir.join("minimal.rego"), minimal_policy)?;
    fs::write(cursor_dir.join("minimal.rego"), minimal_policy)?;

    Ok(())
}

/// Create a cupcake project structure for a specific harness
///
/// This only creates the directory for the specified harness, avoiding
/// duplicate package errors during compilation.
pub fn create_test_project_for_harness(project_path: &Path, harness: HarnessType) -> Result<()> {
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");

    // Create harness-specific directory structure
    let harness_name = match harness {
        HarnessType::ClaudeCode => "claude",
        HarnessType::Cursor => "cursor",
    };
    let harness_dir = policies_dir.join(harness_name);
    let system_dir = harness_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(cupcake_dir.join("signals"))?;
    fs::create_dir_all(cupcake_dir.join("actions"))?;

    // Create minimal rulebook
    fs::write(
        cupcake_dir.join("rulebook.yml"),
        "signals: {}\nactions: {}\nbuiltins: {}",
    )?;

    // Use fixture for system policy
    let system_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), system_policy)?;

    // Add minimal policy to ensure compilation works
    let minimal_policy = include_str!("fixtures/minimal_policy.rego");
    fs::write(harness_dir.join("minimal.rego"), minimal_policy)?;

    Ok(())
}

/// Create global configuration structure for testing
#[allow(dead_code)]
pub fn create_test_global_config(global_path: &Path) -> Result<()> {
    let policies_dir = global_path.join("policies");
    // Use Claude harness-specific directory structure
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(global_path.join("signals"))?;
    fs::create_dir_all(global_path.join("actions"))?;

    // Create minimal rulebook
    fs::write(
        global_path.join("rulebook.yml"),
        "signals: {}\nactions: {}\nbuiltins: {}",
    )?;

    // Use fixture for global system policy
    let global_system_policy = include_str!("fixtures/global_system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), global_system_policy)?;

    Ok(())
}
