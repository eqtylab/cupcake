//! Trust verification logic - the runtime enforcement of script integrity
//!
//! The verifier checks scripts against the trust manifest before execution.
//! It's designed to be lightweight with minimal overhead when enabled.

use crate::trust::error::TrustError;
use crate::trust::manifest::{ScriptReference, TrustManifest};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Trust verifier - holds the manifest and performs verification
#[derive(Clone)]
pub struct TrustVerifier {
    /// The loaded trust manifest
    manifest: Arc<RwLock<TrustManifest>>,

    /// Project root path for resolving relative paths
    project_root: PathBuf,

    /// Path to the manifest file
    manifest_path: PathBuf,
}

impl TrustVerifier {
    /// Create a new trust verifier by loading the manifest
    pub async fn new(project_root: &Path) -> Result<Self, TrustError> {
        let manifest_path = project_root.join(".cupcake").join(".trust");

        if !manifest_path.exists() {
            return Err(TrustError::NotInitialized);
        }

        info!("Loading trust manifest from: {}", manifest_path.display());
        let manifest = TrustManifest::load(&manifest_path)?;

        debug!(
            "Trust manifest loaded: {} signals, {} actions",
            manifest.scripts.get("signals").map_or(0, |s| s.len()),
            manifest.scripts.get("actions").map_or(0, |a| a.len())
        );

        Ok(TrustVerifier {
            manifest: Arc::new(RwLock::new(manifest)),
            project_root: project_root.to_path_buf(),
            manifest_path,
        })
    }

    /// Convenience alias for new
    pub async fn load(project_root: &Path) -> Result<Self, TrustError> {
        Self::new(project_root).await
    }

    /// Create a verifier with a pre-loaded manifest (for testing)
    #[cfg(test)]
    pub fn with_manifest(manifest: TrustManifest, project_root: &Path) -> Self {
        TrustVerifier {
            manifest: Arc::new(RwLock::new(manifest)),
            project_root: project_root.to_path_buf(),
            manifest_path: project_root.join(".cupcake").join(".trust"),
        }
    }

    /// Verify a script command before execution
    pub async fn verify_script(&self, command: &str) -> Result<(), TrustError> {
        let script_ref = ScriptReference::parse(command, &self.project_root);

        debug!("Verifying script: {} -> {:?}", command, script_ref);

        // Compute current hash
        let current_hash = script_ref.compute_hash().await.map_err(|e| {
            if let Some(path) = script_ref.as_path() {
                TrustError::ScriptNotFound {
                    path: path.to_path_buf(),
                    source: std::io::Error::other(e.to_string()),
                }
            } else {
                // For inline scripts, this shouldn't happen
                TrustError::ScriptNotTrusted {
                    path: PathBuf::from("<inline>"),
                }
            }
        })?;

        // Find script in manifest
        let manifest = self.manifest.read().await;
        let script_entry = manifest.find_script_by_command(command);

        match script_entry {
            Some((category, name, entry)) => {
                debug!(
                    "Found script in manifest: category={}, name={}, expected_hash={}",
                    category, name, entry.hash
                );

                // Compare hashes
                if current_hash != entry.hash {
                    let path = script_ref
                        .as_path()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from(format!("<inline: {command}>")));

                    return Err(TrustError::ScriptModified {
                        path,
                        expected: entry.hash.clone(),
                        actual: current_hash,
                    });
                }

                debug!("Script verification successful: {}", command);
                Ok(())
            }
            None => {
                // Script not in manifest
                let path = script_ref
                    .as_path()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(format!("<inline: {command}>")));

                Err(TrustError::ScriptNotTrusted { path })
            }
        }
    }

    /// Verify a script synchronously (for non-async contexts)
    pub fn verify_script_sync(&self, command: &str) -> Result<(), TrustError> {
        // Use tokio's block_in_place if we're in a runtime, otherwise block_on
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(self.verify_script(command))
            })
        } else {
            // Create a temporary runtime for sync context
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(self.verify_script(command))
        }
    }

    /// Reload the manifest from disk (useful after trust update)
    pub async fn reload(&self) -> Result<(), TrustError> {
        info!("Reloading trust manifest");
        let new_manifest = TrustManifest::load(&self.manifest_path)?;

        let mut manifest = self.manifest.write().await;
        *manifest = new_manifest;

        debug!("Trust manifest reloaded successfully");
        Ok(())
    }

    /// Get a copy of the current manifest
    pub async fn get_manifest(&self) -> TrustManifest {
        self.manifest.read().await.clone()
    }

    /// Check if a specific command is trusted (without full verification)
    pub async fn is_trusted(&self, command: &str) -> bool {
        let manifest = self.manifest.read().await;
        manifest.find_script_by_command(command).is_some()
    }
}

/// Extension trait for Option<TrustVerifier> to simplify integration
pub trait TrustVerifierExt {
    /// Verify a script if trust is enabled, otherwise no-op
    fn verify_if_enabled(
        &self,
        command: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl TrustVerifierExt for Option<TrustVerifier> {
    async fn verify_if_enabled(&self, command: &str) -> Result<()> {
        match self {
            Some(verifier) => {
                verifier.verify_script(command).await?;
                Ok(())
            }
            None => {
                // Trust not enabled, allow execution
                debug!("Trust verification skipped (not enabled)");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::manifest::ScriptEntry;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_verify_trusted_inline_script() {
        let temp_dir = TempDir::new().unwrap();

        // Create manifest with trusted script
        let mut manifest = TrustManifest::new();
        manifest.add_script(
            "signals",
            "test",
            ScriptEntry {
                script_type: "inline".to_string(),
                command: "npm test".to_string(),
                hash: crate::trust::hasher::hash_string("npm test"),
                absolute_path: None,
                size: None,
                modified: None,
                interpreter: None,
                args: None,
            },
        );

        let verifier = TrustVerifier::with_manifest(manifest, temp_dir.path());

        // Should succeed for trusted script
        assert!(verifier.verify_script("npm test").await.is_ok());
    }

    #[tokio::test]
    async fn test_verify_untrusted_script() {
        let temp_dir = TempDir::new().unwrap();

        // Create empty manifest
        let manifest = TrustManifest::new();
        let verifier = TrustVerifier::with_manifest(manifest, temp_dir.path());

        // Should fail for untrusted script
        let result = verifier.verify_script("rm -rf /").await;
        assert!(matches!(result, Err(TrustError::ScriptNotTrusted { .. })));
    }

    #[tokio::test]
    async fn test_verify_modified_script() {
        let temp_dir = TempDir::new().unwrap();

        // Create manifest with wrong hash
        let mut manifest = TrustManifest::new();
        manifest.add_script(
            "signals",
            "test",
            ScriptEntry {
                script_type: "inline".to_string(),
                command: "npm test".to_string(),
                hash: "sha256:wrong_hash".to_string(),
                absolute_path: None,
                size: None,
                modified: None,
                interpreter: None,
                args: None,
            },
        );

        let verifier = TrustVerifier::with_manifest(manifest, temp_dir.path());

        // Should fail with modification error
        let result = verifier.verify_script("npm test").await;
        assert!(matches!(result, Err(TrustError::ScriptModified { .. })));
    }

    #[tokio::test]
    async fn test_verify_if_enabled_with_none() {
        let verifier: Option<TrustVerifier> = None;

        // Should succeed (no-op)
        assert!(verifier.verify_if_enabled("any command").await.is_ok());
    }

    #[tokio::test]
    async fn test_verify_if_enabled_with_some() {
        let temp_dir = TempDir::new().unwrap();

        let mut manifest = TrustManifest::new();
        manifest.add_script(
            "signals",
            "test",
            ScriptEntry {
                script_type: "inline".to_string(),
                command: "npm test".to_string(),
                hash: crate::trust::hasher::hash_string("npm test"),
                absolute_path: None,
                size: None,
                modified: None,
                interpreter: None,
                args: None,
            },
        );

        let verifier = Some(TrustVerifier::with_manifest(manifest, temp_dir.path()));

        // Should succeed for trusted
        assert!(verifier.verify_if_enabled("npm test").await.is_ok());

        // Should fail for untrusted
        assert!(verifier.verify_if_enabled("rm -rf /").await.is_err());
    }
}
