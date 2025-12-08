//! Rulebook installation from the catalog
//!
//! Handles downloading, verifying, and extracting rulebooks from the catalog
//! into the local `.cupcake/catalog/` directory.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use super::IndexEntry;

/// Default catalog installation directory
const CATALOG_DIR: &str = ".cupcake/catalog";

/// Installer for catalog rulebooks
pub struct Installer {
    client: reqwest::Client,
    catalog_dir: PathBuf,
}

impl Installer {
    /// Create a new installer with default settings
    pub fn new() -> Result<Self> {
        Self::with_catalog_dir(PathBuf::from(CATALOG_DIR))
    }

    /// Create an installer with a custom catalog directory
    pub fn with_catalog_dir(catalog_dir: PathBuf) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("cupcake-cli")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            catalog_dir,
        })
    }

    /// Install a rulebook from the catalog
    pub async fn install(&self, entry: &IndexEntry) -> Result<PathBuf> {
        let url = entry
            .urls
            .first()
            .context("No download URL available for rulebook")?;

        tracing::info!("Downloading {} v{} from {}", entry.name, entry.version, url);

        // Download tarball
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to download rulebook")?;

        if !response.status().is_success() {
            anyhow::bail!("Download failed: HTTP {} for {}", response.status(), url);
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read download response")?;

        // Verify digest if present
        if let Some(expected_digest) = &entry.digest {
            self.verify_digest(&bytes, expected_digest)?;
            tracing::info!("Verified digest: {}", expected_digest);
        }

        // Extract to catalog directory
        let install_dir = self.extract_tarball(&entry.name, &bytes)?;

        tracing::info!(
            "Installed {} v{} to {:?}",
            entry.name,
            entry.version,
            install_dir
        );

        Ok(install_dir)
    }

    /// Install from a local tarball file
    pub fn install_local(&self, tarball_path: &Path) -> Result<(String, PathBuf)> {
        let bytes = std::fs::read(tarball_path)
            .with_context(|| format!("Failed to read tarball: {tarball_path:?}"))?;

        // Extract to temp to read manifest
        let temp_dir = tempfile::tempdir()?;
        self.extract_to_dir(&bytes, temp_dir.path())?;

        // Find the rulebook directory (first directory in archive)
        let rulebook_dir = std::fs::read_dir(temp_dir.path())?
            .filter_map(|e| e.ok())
            .find(|e| e.path().is_dir())
            .context("No directory found in tarball")?;

        // Load manifest to get rulebook name
        let manifest_path = rulebook_dir.path().join("manifest.yaml");
        let manifest = super::RulebookManifest::from_file(&manifest_path)?;
        let name = manifest.metadata.name.clone();

        // Ensure catalog directory exists
        std::fs::create_dir_all(&self.catalog_dir)?;

        // Remove existing installation if present
        let install_dir = self.catalog_dir.join(&name);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir)?;
        }

        // Move to final location
        std::fs::rename(rulebook_dir.path(), &install_dir)?;

        Ok((name, install_dir))
    }

    /// Uninstall a rulebook by name
    pub fn uninstall(&self, name: &str) -> Result<()> {
        let install_dir = self.catalog_dir.join(name);

        if !install_dir.exists() {
            anyhow::bail!("Rulebook '{}' is not installed", name);
        }

        std::fs::remove_dir_all(&install_dir)
            .with_context(|| format!("Failed to remove rulebook directory: {install_dir:?}"))?;

        tracing::info!("Uninstalled rulebook: {}", name);
        Ok(())
    }

    /// Check if a rulebook is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.catalog_dir.join(name).exists()
    }

    /// Get the installation directory for a rulebook
    pub fn install_path(&self, name: &str) -> PathBuf {
        self.catalog_dir.join(name)
    }

    /// List all installed rulebooks
    pub fn list_installed(&self) -> Result<Vec<String>> {
        if !self.catalog_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();
        for entry in std::fs::read_dir(&self.catalog_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                // Check if it has a manifest.yaml
                let manifest_path = entry.path().join("manifest.yaml");
                if manifest_path.exists() {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(name.to_string());
                    }
                }
            }
        }

        installed.sort();
        Ok(installed)
    }

    /// Verify the SHA256 digest of downloaded content
    fn verify_digest(&self, bytes: &[u8], expected: &str) -> Result<()> {
        let actual = format!("sha256:{:x}", Sha256::digest(bytes));

        if actual != *expected {
            anyhow::bail!(
                "Digest verification failed!\nExpected: {}\nActual: {}",
                expected,
                actual
            );
        }

        Ok(())
    }

    /// Extract tarball to the catalog directory
    fn extract_tarball(&self, name: &str, bytes: &[u8]) -> Result<PathBuf> {
        // Ensure catalog directory exists
        std::fs::create_dir_all(&self.catalog_dir)?;

        // Remove existing installation if present
        let install_dir = self.catalog_dir.join(name);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir)?;
        }

        // Extract tarball
        self.extract_to_dir(bytes, &self.catalog_dir)?;

        // Verify installation
        if !install_dir.exists() {
            anyhow::bail!(
                "Installation failed: expected directory '{}' not created. \
                The tarball may have a different root directory name.",
                name
            );
        }

        let manifest_path = install_dir.join("manifest.yaml");
        if !manifest_path.exists() {
            anyhow::bail!(
                "Installation failed: manifest.yaml not found in {:?}",
                install_dir
            );
        }

        Ok(install_dir)
    }

    /// Extract tarball bytes to a directory
    fn extract_to_dir(&self, bytes: &[u8], dest: &Path) -> Result<()> {
        let cursor = Cursor::new(bytes);
        let gz_decoder = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(gz_decoder);

        archive.unpack(dest).context("Failed to extract tarball")?;

        Ok(())
    }
}

impl Default for Installer {
    fn default() -> Self {
        Self::new().expect("Failed to create default installer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;
    use tempfile::TempDir;

    /// Create a test tarball with a manifest
    fn create_test_tarball(name: &str, version: &str) -> Vec<u8> {
        let temp_dir = TempDir::new().unwrap();
        let rulebook_dir = temp_dir.path().join(name);
        std::fs::create_dir_all(&rulebook_dir).unwrap();

        // Create manifest.yaml
        let manifest = format!(
            r#"apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: {name}
  version: {version}
  description: Test rulebook
  harnesses:
    - claude
"#
        );
        std::fs::write(rulebook_dir.join("manifest.yaml"), manifest).unwrap();

        // Create tarball
        let mut bytes = Vec::new();
        {
            let encoder = GzEncoder::new(&mut bytes, Compression::default());
            let mut builder = Builder::new(encoder);
            builder.append_dir_all(name, &rulebook_dir).unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }

        bytes
    }

    #[test]
    fn test_extract_tarball() {
        let temp_dir = TempDir::new().unwrap();
        let installer = Installer::with_catalog_dir(temp_dir.path().to_path_buf()).unwrap();

        let tarball = create_test_tarball("test-rulebook", "1.0.0");
        let install_dir = installer
            .extract_tarball("test-rulebook", &tarball)
            .unwrap();

        assert!(install_dir.exists());
        assert!(install_dir.join("manifest.yaml").exists());
    }

    #[test]
    fn test_verify_digest() {
        let installer = Installer::new().unwrap();
        let data = b"test data";
        let expected = format!("sha256:{:x}", Sha256::digest(data));

        // Should succeed with correct digest
        installer.verify_digest(data, &expected).unwrap();

        // Should fail with wrong digest
        let result = installer.verify_digest(data, "sha256:wrong");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_installed() {
        let temp_dir = TempDir::new().unwrap();
        let installer = Installer::with_catalog_dir(temp_dir.path().to_path_buf()).unwrap();

        // Initially empty
        assert!(installer.list_installed().unwrap().is_empty());

        // Install a rulebook
        let tarball = create_test_tarball("test-rulebook", "1.0.0");
        installer
            .extract_tarball("test-rulebook", &tarball)
            .unwrap();

        // Should now have one installed
        let installed = installer.list_installed().unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0], "test-rulebook");
    }

    #[test]
    fn test_uninstall() {
        let temp_dir = TempDir::new().unwrap();
        let installer = Installer::with_catalog_dir(temp_dir.path().to_path_buf()).unwrap();

        // Install a rulebook
        let tarball = create_test_tarball("test-rulebook", "1.0.0");
        installer
            .extract_tarball("test-rulebook", &tarball)
            .unwrap();
        assert!(installer.is_installed("test-rulebook"));

        // Uninstall
        installer.uninstall("test-rulebook").unwrap();
        assert!(!installer.is_installed("test-rulebook"));
    }
}
