//! File system scanner for discovering .rego policy files
//! 
//! Implements policy discovery as defined in CRITICAL_GUIDING_STAR.md Step 1:
//! "Scan & Compile (On Startup/Change): Cupcake scans all .rego policies"

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Scan a directory recursively for all .rego files
pub async fn scan_policies(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Err(anyhow::anyhow!(
            "Policy directory does not exist: {:?}",
            dir
        ));
    }
    
    if !dir.is_dir() {
        return Err(anyhow::anyhow!(
            "Policy path is not a directory: {:?}",
            dir
        ));
    }
    
    info!("Scanning for .rego files in: {:?}", dir);
    
    let mut policy_files = Vec::new();
    scan_directory_recursive(dir, &mut policy_files).await?;
    
    info!("Scan complete: found {} .rego files", policy_files.len());
    
    Ok(policy_files)
}

/// Recursively scan a directory for .rego files
fn scan_directory_recursive<'a>(
    dir: &'a Path,
    files: &'a mut Vec<PathBuf>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .context("Failed to read directory")?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let file_type = entry.file_type().await?;
        
        if file_type.is_dir() {
            // Recurse into subdirectories
            debug!("Scanning subdirectory: {:?}", path);
            scan_directory_recursive(&path, files).await?;
        } else if file_type.is_file() {
            // Check if it's a .rego file
            if let Some(extension) = path.extension() {
                if extension == "rego" {
                    debug!("Found policy file: {:?}", path);
                    files.push(path);
                }
            }
        }
    }
    
    Ok(())
    })
}

/// Watch a directory for changes to .rego files (future enhancement)
/// This will enable hot-reload as described in CRITICAL_GUIDING_STAR.md:
/// "This process is automatic and repeats if any policy files change"
pub async fn watch_policies(_dir: &Path) -> Result<()> {
    // TODO: Implement file watching with notify crate
    // This is for future hot-reload functionality
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    
    #[tokio::test]
    async fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let files = scan_policies(temp_dir.path()).await.unwrap();
        assert_eq!(files.len(), 0);
    }
    
    #[tokio::test]
    async fn test_scan_with_rego_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create some .rego files
        fs::write(temp_dir.path().join("policy1.rego"), "package test1").await.unwrap();
        fs::write(temp_dir.path().join("policy2.rego"), "package test2").await.unwrap();
        
        // Create a non-.rego file (should be ignored)
        fs::write(temp_dir.path().join("readme.md"), "# README").await.unwrap();
        
        // Create a subdirectory with more policies
        let sub_dir = temp_dir.path().join("sub");
        fs::create_dir(&sub_dir).await.unwrap();
        fs::write(sub_dir.join("policy3.rego"), "package test3").await.unwrap();
        
        let files = scan_policies(temp_dir.path()).await.unwrap();
        assert_eq!(files.len(), 3);
    }
    
    #[tokio::test]
    async fn test_scan_nonexistent_directory() {
        let result = scan_policies(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());
    }
}

// Aligns with CRITICAL_GUIDING_STAR.md:
// - Scans all .rego files in the policy directory
// - Supports recursive directory scanning
// - Foundation for hot-reload capability
// - Clean separation of concerns: scanner only finds files