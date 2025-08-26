//! Unit tests for trust system components

use anyhow::Result;
use cupcake_rego::trust::{TrustManifest, TrustError};
use cupcake_rego::trust::manifest::ScriptReference;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_script_reference_parsing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test inline command parsing
    let inline_ref = ScriptReference::parse("echo hello", temp_dir.path());
    assert!(matches!(inline_ref, ScriptReference::Inline(_)));
    
    // Test file script parsing
    let test_script = temp_dir.path().join("test.sh");
    fs::write(&test_script, "#!/bin/bash\necho test")?;
    
    let file_ref = ScriptReference::parse("./test.sh", temp_dir.path());
    assert!(matches!(file_ref, ScriptReference::File(_)));
    
    // Test complex command parsing  
    let complex_ref = ScriptReference::parse("python script.py --flag", temp_dir.path());
    // This might be inline or complex depending on whether script.py exists
    assert!(matches!(complex_ref, ScriptReference::Inline(_) | ScriptReference::Complex { .. }));
    
    Ok(())
}

#[tokio::test]
async fn test_script_reference_hashing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test inline command hashing
    let inline_ref = ScriptReference::parse("echo hello", temp_dir.path());
    let hash1 = inline_ref.compute_hash().await?;
    let hash2 = inline_ref.compute_hash().await?;
    assert_eq!(hash1, hash2, "Same command should hash consistently");
    
    // Test different commands have different hashes
    let inline_ref2 = ScriptReference::parse("echo goodbye", temp_dir.path());
    let hash3 = inline_ref2.compute_hash().await?;
    assert_ne!(hash1, hash3, "Different commands should have different hashes");
    
    Ok(())
}

#[tokio::test] 
async fn test_script_reference_file_hashing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_script = temp_dir.path().join("test.sh");
    
    // Create script file
    fs::write(&test_script, "#!/bin/bash\necho test")?;
    
    let file_ref = ScriptReference::parse("./test.sh", temp_dir.path());
    let hash1 = file_ref.compute_hash().await?;
    
    // Same file should hash consistently
    let hash2 = file_ref.compute_hash().await?;
    assert_eq!(hash1, hash2);
    
    // Modified file should have different hash
    fs::write(&test_script, "#!/bin/bash\necho modified")?;
    let hash3 = file_ref.compute_hash().await?;
    assert_ne!(hash1, hash3, "Modified file should have different hash");
    
    Ok(())
}

#[tokio::test]
async fn test_trust_manifest_creation() -> Result<()> {
    let manifest = TrustManifest::new();
    
    assert_eq!(manifest.scripts().len(), 2); // signals and actions
    assert!(manifest.scripts().contains_key("signals"));
    assert!(manifest.scripts().contains_key("actions"));
    assert!(manifest.scripts().get("signals").unwrap().is_empty());
    assert!(manifest.scripts().get("actions").unwrap().is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_trust_manifest_script_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut manifest = TrustManifest::new();
    
    // Create a test script entry
    let script_entry = cupcake_rego::trust::manifest::ScriptEntry::from_command(
        "echo test",
        temp_dir.path()
    ).await?;
    
    // Add script to manifest
    manifest.add_script("signals", "test_signal", script_entry);
    
    // Verify it was added
    assert_eq!(manifest.scripts().get("signals").unwrap().len(), 1);
    assert!(manifest.get_script("signals", "test_signal").is_some());
    
    // Test script lookup
    let found_script = manifest.get_script("signals", "test_signal").unwrap();
    assert_eq!(found_script.command, "echo test");
    
    Ok(())
}

#[tokio::test]
async fn test_trust_manifest_hmac_verification() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let trust_file = temp_dir.path().join(".trust");
    
    // Create and save manifest
    let mut manifest = TrustManifest::new();
    manifest.save(&trust_file)?;
    
    // Load and verify HMAC (HMAC verification happens automatically in load)
    let loaded_manifest = TrustManifest::load(&trust_file)?;
    
    // Manually corrupt the file and verify detection
    let content = fs::read_to_string(&trust_file)?;
    let corrupted = content.replace("signals", "CORRUPTED");
    fs::write(&trust_file, corrupted)?;
    
    // Should detect tampering
    let result = TrustManifest::load(&trust_file);
    assert!(matches!(result, Err(TrustError::ManifestTampered)));
    
    Ok(())
}

#[tokio::test] 
async fn test_trust_manifest_no_hmac_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let trust_file = temp_dir.path().join(".trust");
    
    // Create file without HMAC
    let manifest_json = r#"{
  "version": 1,
  "timestamp": "2025-08-25T18:52:12.640302Z",
  "scripts": {
    "signals": {},
    "actions": {}
  }
}"#;
    fs::write(&trust_file, manifest_json)?;
    
    // Should detect missing HMAC as tampering
    let result = TrustManifest::load(&trust_file);
    assert!(matches!(result, Err(TrustError::ManifestTampered)));
    
    Ok(())
}

#[tokio::test]
async fn test_trust_manifest_round_trip_with_scripts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let trust_file = temp_dir.path().join(".trust");
    
    // Create manifest with scripts
    let mut manifest = TrustManifest::new();
    
    let script_entry = cupcake_rego::trust::manifest::ScriptEntry::from_command(
        "echo hello world",
        temp_dir.path()
    ).await?;
    
    manifest.add_script("signals", "greeting", script_entry);
    manifest.save(&trust_file)?;
    
    // Load it back
    let loaded_manifest = TrustManifest::load(&trust_file)?;
    
    // Verify script is preserved
    assert_eq!(loaded_manifest.scripts().get("signals").unwrap().len(), 1);
    let loaded_script = loaded_manifest.get_script("signals", "greeting").unwrap();
    assert_eq!(loaded_script.command, "echo hello world");
    
    // HMAC verification happens automatically in load() - if we got here, it's valid
    
    Ok(())
}

#[tokio::test]
async fn test_trust_error_types() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test NotInitialized error
    let nonexistent = temp_dir.path().join("nonexistent/.trust");
    let result = TrustManifest::load(&nonexistent);
    assert!(matches!(result, Err(TrustError::NotInitialized)));
    
    // Test ManifestTampered error (already covered above)
    
    // Test ScriptNotFound error would require TrustVerifier
    
    Ok(())
}