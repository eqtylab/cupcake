//! Integration tests for the trust CLI commands
//!
//! These tests verify the full CLI workflow that users experience

use anyhow::Result;
use cupcake_rego::trust::{TrustManifest, TrustCommand};
use std::fs;
use tempfile::TempDir;

/// Create a test project with basic Cupcake structure
async fn setup_cupcake_project() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    
    // Create directories
    fs::create_dir_all(cupcake_dir.join("policies/system"))?;
    fs::create_dir_all(cupcake_dir.join("signals"))?;
    fs::create_dir_all(cupcake_dir.join("actions"))?;
    
    // Create minimal system evaluate policy
    fs::write(
        cupcake_dir.join("policies/system/evaluate.rego"),
        r#"package cupcake.system
import rego.v1

evaluate := {
    "halts": [],
    "denials": [],
    "blocks": [],
    "asks": [],
    "allow_overrides": [],
    "add_context": []
}
"#,
    )?;
    
    // Create empty guidebook.yml
    fs::write(
        cupcake_dir.join("guidebook.yml"),
        r#"signals: {}
actions: {}
"#,
    )?;
    
    Ok(temp_dir)
}

#[tokio::test]
async fn test_trust_init_empty_project() -> Result<()> {
    let project = setup_cupcake_project().await?;
    
    let trust_cmd = TrustCommand::Init {
        project_dir: project.path().to_path_buf(),
        empty: true,
    };
    
    // Should succeed
    trust_cmd.execute().await?;
    
    // Should create trust file
    let trust_file = project.path().join(".cupcake/.trust");
    assert!(trust_file.exists(), "Trust file should be created");
    
    // Should be valid manifest - use same project path as the CLI command
    let manifest = TrustManifest::load(&trust_file)?;
    assert!(manifest.scripts().get("signals").unwrap().is_empty());
    assert!(manifest.scripts().get("actions").unwrap().is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_trust_init_with_existing_scripts() -> Result<()> {
    let project = setup_cupcake_project().await?;
    let signals_dir = project.path().join(".cupcake/signals");
    let actions_dir = project.path().join(".cupcake/actions");
    
    // Create some test scripts
    fs::write(signals_dir.join("test_signal.sh"), "#!/bin/bash\necho 'test'")?;
    fs::write(actions_dir.join("test_action.py"), "#!/usr/bin/env python3\nprint('test')")?;
    
    // Update guidebook.yml to reference these scripts
    fs::write(
        project.path().join(".cupcake/guidebook.yml"),
        r#"signals:
  test_signal:
    command: "./.cupcake/signals/test_signal.sh"
actions:
  test_action:
    command: "python ./.cupcake/actions/test_action.py"
"#,
    )?;
    
    let trust_cmd = TrustCommand::Init {
        project_dir: project.path().to_path_buf(), 
        empty: false,
    };
    
    trust_cmd.execute().await?;
    
    // Should create trust file with scripts
    let trust_file = project.path().join(".cupcake/.trust");
    let manifest = TrustManifest::load(&trust_file)?;
    
    assert_eq!(manifest.scripts().get("signals").unwrap().len(), 1);
    assert_eq!(manifest.scripts().get("actions").unwrap().len(), 1);
    
    assert!(manifest.scripts().get("signals").unwrap().contains_key("test_signal"));
    assert!(manifest.scripts().get("actions").unwrap().contains_key("test_action"));
    
    Ok(())
}

#[tokio::test]
async fn test_trust_list_empty_manifest() -> Result<()> {
    let project = setup_cupcake_project().await?;
    
    // Initialize empty trust
    let init_cmd = TrustCommand::Init {
        project_dir: project.path().to_path_buf(),
        empty: true,
    };
    init_cmd.execute().await?;
    
    // List should work without errors
    let list_cmd = TrustCommand::List {
        project_dir: project.path().to_path_buf(),
        modified: false,
        hashes: false,
    };
    
    // Should not panic or error
    list_cmd.execute().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_trust_list_with_scripts() -> Result<()> {
    let project = setup_cupcake_project().await?;
    let signals_dir = project.path().join(".cupcake/signals");
    
    // Create test script
    fs::write(signals_dir.join("test.sh"), "echo hello")?;
    
    // Update guidebook.yml to reference the script
    fs::write(
        project.path().join(".cupcake/guidebook.yml"),
        r#"signals:
  test_signal:
    command: "./.cupcake/signals/test.sh"
actions: {}
"#,
    )?;
    
    // Initialize trust with scripts
    let init_cmd = TrustCommand::Init {
        project_dir: project.path().to_path_buf(),
        empty: false,
    };
    init_cmd.execute().await?;
    
    // List should show the script
    let list_cmd = TrustCommand::List {
        project_dir: project.path().to_path_buf(),
        modified: false,
        hashes: true, // Test hash display
    };
    
    list_cmd.execute().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_trust_verify_valid_manifest() -> Result<()> {
    let project = setup_cupcake_project().await?;
    
    // Initialize trust
    let init_cmd = TrustCommand::Init {
        project_dir: project.path().to_path_buf(),
        empty: true,
    };
    init_cmd.execute().await?;
    
    // Verify should succeed
    let verify_cmd = TrustCommand::Verify {
        project_dir: project.path().to_path_buf(),
        verbose: true,
    };
    
    verify_cmd.execute().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_trust_verify_no_manifest() -> Result<()> {
    let project = setup_cupcake_project().await?;
    
    // Verify without trust should handle gracefully
    let verify_cmd = TrustCommand::Verify {
        project_dir: project.path().to_path_buf(),
        verbose: false,
    };
    
    // Should not error, just inform user
    verify_cmd.execute().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_hmac_integrity_round_trip() -> Result<()> {
    let project = setup_cupcake_project().await?;
    let trust_file = project.path().join(".cupcake/.trust");
    
    // Create and save manifest
    let mut manifest = TrustManifest::new();
    manifest.save(&trust_file)?;
    
    // Load it back - this tests HMAC verification
    let loaded_manifest = TrustManifest::load(&trust_file)?;
    
    // Should have same timestamp (within reason)
    assert_eq!(loaded_manifest.created_at().date_naive(), manifest.created_at().date_naive());
    
    // HMAC verification happens automatically in load() - if we got here, it's valid
    
    Ok(())
}

#[tokio::test]
async fn test_trust_init_already_exists() -> Result<()> {
    let project = setup_cupcake_project().await?;
    
    // Initialize once
    let init_cmd = TrustCommand::Init {
        project_dir: project.path().to_path_buf(),
        empty: true,
    };
    init_cmd.execute().await?;
    
    // Initialize again - should be idempotent
    let init_cmd2 = TrustCommand::Init {
        project_dir: project.path().to_path_buf(),
        empty: true,
    };
    init_cmd2.execute().await?;
    
    // Should still have valid trust file
    let trust_file = project.path().join(".cupcake/.trust");
    assert!(trust_file.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_trust_init_no_cupcake_project() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    let init_cmd = TrustCommand::Init {
        project_dir: temp_dir.path().to_path_buf(),
        empty: true,
    };
    
    // Should handle gracefully (not error, just inform user)
    init_cmd.execute().await?;
    
    // Should not create trust file
    let trust_file = temp_dir.path().join(".cupcake/.trust");
    assert!(!trust_file.exists());
    
    Ok(())
}