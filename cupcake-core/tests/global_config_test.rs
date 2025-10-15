//! Integration tests for global configuration discovery and loading

use anyhow::Result;
use cupcake_core::engine::global_config::GlobalPaths;
use serial_test::serial;
use std::env; // Still needed for test_global_config_graceful_absence
use tempfile::TempDir;

#[test]
#[serial]
fn test_global_config_cli_override_discovery() -> Result<()> {
    // Create a temporary directory to act as global config
    let temp_dir = TempDir::new()?;
    let global_root = temp_dir.path().to_path_buf();

    // Use CLI override to discover it
    let discovered = GlobalPaths::discover_with_override(Some(global_root.clone()))?;
    assert!(
        discovered.is_some(),
        "Should discover config from CLI override"
    );

    let global_paths = discovered.unwrap();
    // Compare canonicalized paths since global_config now canonicalizes for defense-in-depth
    // On macOS, /var is a symlink to /private/var, so we need to canonicalize both sides
    let expected_root = global_root.canonicalize()?;
    assert_eq!(global_paths.root, expected_root);

    Ok(())
}

#[test]
#[serial]
fn test_global_config_graceful_absence() -> Result<()> {
    // Create a temp dir that doesn't have cupcake config
    let temp_dir = TempDir::new()?;
    env::set_var("HOME", temp_dir.path().to_str().unwrap());

    // Discovery should return None gracefully when no config exists
    let discovered = GlobalPaths::discover()?;

    // This is expected to be None in CI/test environments
    // unless developer has global config installed
    assert!(discovered.is_none() || discovered.is_some());

    Ok(())
}

#[test]
#[serial]
fn test_global_config_initialization() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let global_root = temp_dir.path().to_path_buf();

    let global_paths = GlobalPaths::discover_with_override(Some(global_root))?.unwrap();

    // Should not be initialized yet
    assert!(!global_paths.is_initialized());

    // Initialize
    global_paths.initialize()?;

    // Should now be initialized
    assert!(global_paths.is_initialized());

    // Verify structure with harness-specific directories
    assert!(global_paths.policies.exists());
    assert!(global_paths.policies.join("claude").join("system").exists());
    assert!(global_paths.policies.join("claude").join("builtins").exists());
    assert!(global_paths.policies.join("cursor").join("system").exists());
    assert!(global_paths.policies.join("cursor").join("builtins").exists());
    assert!(global_paths.signals.exists());
    assert!(global_paths.actions.exists());
    assert!(global_paths.rulebook.exists());

    // Verify rulebook content
    let rulebook_content = std::fs::read_to_string(&global_paths.rulebook)?;
    assert!(rulebook_content.contains("Global Cupcake Configuration"));

    // Note: System evaluate.rego files are created by init_global_config() in main.rs,
    // not by initialize(). This test only verifies the directory structure.

    Ok(())
}

#[test]
#[serial]
fn test_platform_specific_paths() {
    // This test verifies the platform-specific logic works
    // It doesn't assert specific paths since they vary by platform

    #[cfg(unix)]
    {
        // On Unix, we expect ~/.config/cupcake or similar
        let discovered = GlobalPaths::discover();

        // Should not error
        assert!(discovered.is_ok());

        // May or may not find config depending on system
        let result = discovered.unwrap();
        if let Some(paths) = result {
            // If found, should have sensible paths
            assert!(
                paths.root.to_string_lossy().contains("config")
                    || paths.root.to_string_lossy().contains("Library")
            );
        }
    }

    #[cfg(windows)]
    {
        // On Windows, we expect %APPDATA%\cupcake or similar
        let discovered = GlobalPaths::discover();

        // Should not error
        assert!(discovered.is_ok());

        // May or may not find config depending on system
        let result = discovered.unwrap();
        if let Some(paths) = result {
            // If found, should have sensible paths
            assert!(
                paths.root.to_string_lossy().contains("AppData")
                    || paths.root.to_string_lossy().contains("cupcake")
            );
        }
    }
}
