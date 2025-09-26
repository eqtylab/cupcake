//! Test helper functions for integration tests

use anyhow::Result;
use std::fs;
use std::path::Path;
use std::sync::Once;

/// Initialize logging for tests (only once per test run)
static INIT: Once = Once::new();

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
pub fn create_test_project(project_path: &Path) -> Result<()> {
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(cupcake_dir.join("signals"))?;
    fs::create_dir_all(cupcake_dir.join("actions"))?;

    // Create minimal guidebook
    fs::write(
        cupcake_dir.join("guidebook.yml"),
        "signals: {}\nactions: {}\nbuiltins: {}",
    )?;

    // Use fixture for system policy
    let system_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), system_policy)?;

    // Add minimal policy to ensure compilation works
    let minimal_policy = include_str!("fixtures/minimal_policy.rego");
    fs::write(policies_dir.join("minimal.rego"), minimal_policy)?;

    Ok(())
}

/// Create global configuration structure for testing
pub fn create_test_global_config(global_path: &Path) -> Result<()> {
    let policies_dir = global_path.join("policies");
    let system_dir = policies_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(global_path.join("signals"))?;
    fs::create_dir_all(global_path.join("actions"))?;

    // Create minimal guidebook
    fs::write(
        global_path.join("guidebook.yml"),
        "signals: {}\nactions: {}\nbuiltins: {}",
    )?;

    // Use fixture for global system policy
    let global_system_policy = include_str!("fixtures/global_system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), global_system_policy)?;

    Ok(())
}
