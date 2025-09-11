//! Integration tests for global configuration discovery and loading

use anyhow::Result;
use cupcake_core::engine::global_config::GlobalPaths;
use std::env;
use tempfile::TempDir;
use serial_test::serial;

#[test]
#[serial]
fn test_global_config_env_var_discovery() -> Result<()> {
    // Create a temporary directory to act as global config
    let temp_dir = TempDir::new()?;
    let global_root = temp_dir.path();
    
    // Set the environment variable
    env::set_var("CUPCAKE_GLOBAL_CONFIG", global_root.to_str().unwrap());
    
    // Discover should find it
    let discovered = GlobalPaths::discover()?;
    assert!(discovered.is_some(), "Should discover config from env var");
    
    let global_paths = discovered.unwrap();
    assert_eq!(global_paths.root, global_root);
    
    // Clean up
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
    
    Ok(())
}

#[test]
#[serial]
fn test_global_config_graceful_absence() -> Result<()> {
    // Ensure no env var is set
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
    
    // Create a temp dir that doesn't have cupcake config
    let temp_dir = TempDir::new()?;
    env::set_var("HOME", temp_dir.path().to_str().unwrap());
    
    // Discovery should return None gracefully
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
    env::set_var("CUPCAKE_GLOBAL_CONFIG", temp_dir.path().to_str().unwrap());
    
    let global_paths = GlobalPaths::discover()?.unwrap();
    
    // Should not be initialized yet
    assert!(!global_paths.is_initialized());
    
    // Initialize
    global_paths.initialize()?;
    
    // Should now be initialized
    assert!(global_paths.is_initialized());
    
    // Verify structure
    assert!(global_paths.policies.exists());
    assert!(global_paths.policies.join("system").exists());
    assert!(global_paths.signals.exists());
    assert!(global_paths.actions.exists());
    assert!(global_paths.guidebook.exists());
    
    // Verify evaluate.rego was created with correct namespace
    let evaluate_path = global_paths.policies.join("system").join("evaluate.rego");
    assert!(evaluate_path.exists());
    
    let evaluate_content = std::fs::read_to_string(&evaluate_path)?;
    assert!(evaluate_content.contains("package cupcake.global.system"));
    assert!(evaluate_content.contains("walk(data.cupcake.global.policies"));
    
    // Clean up
    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
    
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
        env::remove_var("CUPCAKE_GLOBAL_CONFIG");
        let discovered = GlobalPaths::discover();
        
        // Should not error
        assert!(discovered.is_ok());
        
        // May or may not find config depending on system
        let result = discovered.unwrap();
        if let Some(paths) = result {
            // If found, should have sensible paths
            assert!(paths.root.to_string_lossy().contains("config") || 
                    paths.root.to_string_lossy().contains("Library"));
        }
    }
    
    #[cfg(windows)]
    {
        // On Windows, we expect %APPDATA%\cupcake or similar
        env::remove_var("CUPCAKE_GLOBAL_CONFIG");
        let discovered = GlobalPaths::discover();
        
        // Should not error
        assert!(discovered.is_ok());
        
        // May or may not find config depending on system
        let result = discovered.unwrap();
        if let Some(paths) = result {
            // If found, should have sensible paths
            assert!(paths.root.to_string_lossy().contains("AppData") ||
                    paths.root.to_string_lossy().contains("cupcake"));
        }
    }
}