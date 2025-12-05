//! File system scanner for discovering .rego policy files
//!
//! Implements policy discovery as defined in CRITICAL_GUIDING_STAR.md Step 1:
//! "Scan & Compile (On Startup/Change): Cupcake scans all .rego policies"

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Scan a directory recursively for all .rego files
pub async fn scan_policies(dir: &Path) -> Result<Vec<PathBuf>> {
    scan_policies_with_filter(dir, &[]).await
}

/// Scan a directory recursively for .rego files with builtin filtering
pub async fn scan_policies_with_filter(
    dir: &Path,
    enabled_builtins: &[String],
) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Err(anyhow::anyhow!(
            "Policy directory does not exist: {:?}",
            dir
        ));
    }

    if !dir.is_dir() {
        return Err(anyhow::anyhow!("Policy path is not a directory: {:?}", dir));
    }

    info!("Scanning for .rego files in: {:?}", dir);
    if !enabled_builtins.is_empty() {
        info!("With builtin filter for: {:?}", enabled_builtins);
    }

    let mut policy_files = Vec::new();
    scan_directory_recursive_filtered(dir, &mut policy_files, enabled_builtins).await?;

    info!("Scan complete: found {} .rego files", policy_files.len());

    Ok(policy_files)
}

/// Recursively scan a directory for .rego files with optional builtin filtering
fn scan_directory_recursive_filtered<'a>(
    dir: &'a Path,
    files: &'a mut Vec<PathBuf>,
    enabled_builtins: &'a [String],
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
                scan_directory_recursive_filtered(&path, files, enabled_builtins).await?;
            } else if file_type.is_file() {
                // Check if it's a .rego file
                if let Some(extension) = path.extension() {
                    if extension == "rego" {
                        // Check if this is a builtin policy that should be filtered
                        if should_include_policy(&path, enabled_builtins) {
                            debug!("Found policy file: {:?}", path);
                            files.push(path);
                        } else {
                            debug!("Skipping disabled builtin policy: {:?}", path);
                        }
                    }
                }
            }
        }

        Ok(())
    })
}

/// Check if a policy should be included based on builtin filtering
fn should_include_policy(path: &Path, enabled_builtins: &[String]) -> bool {
    // If no builtin filter is specified, include all policies
    // This is the case for catalog overlays which should include all their policies
    if enabled_builtins.is_empty() {
        return true;
    }

    // Check if this is a builtin policy
    if let Some(parent) = path.parent() {
        if parent.file_name() == Some(std::ffi::OsStr::new("builtins")) {
            // This is a builtin policy - only include if enabled
            if let Some(stem) = path.file_stem() {
                let policy_name = stem.to_string_lossy();
                return enabled_builtins.contains(&policy_name.to_string());
            }
            // Unknown builtin, exclude by default
            return false;
        }
    }

    // Not a builtin policy, always include
    true
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
        fs::write(temp_dir.path().join("policy1.rego"), "package test1")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("policy2.rego"), "package test2")
            .await
            .unwrap();

        // Create a non-.rego file (should be ignored)
        fs::write(temp_dir.path().join("readme.md"), "# README")
            .await
            .unwrap();

        // Create a subdirectory with more policies
        let sub_dir = temp_dir.path().join("sub");
        fs::create_dir(&sub_dir).await.unwrap();
        fs::write(sub_dir.join("policy3.rego"), "package test3")
            .await
            .unwrap();

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
