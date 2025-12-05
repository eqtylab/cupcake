//! Catalog lock file management (.cupcake/catalog.lock)
//!
//! Tracks which catalog rulebooks are installed and their versions,
//! enabling reproducible builds and version management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::IndexEntry;

/// Default lock file location
const LOCK_FILE: &str = ".cupcake/catalog.lock";

/// The catalog lock file tracks installed rulebooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogLock {
    /// API version for schema compatibility
    pub api_version: String,
    /// When this lock file was last updated
    pub generated: String,
    /// List of installed rulebooks
    pub installed: Vec<InstalledRulebook>,
}

/// An installed rulebook entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledRulebook {
    /// Rulebook name
    pub name: String,
    /// Installed version
    pub version: String,
    /// Supported harnesses
    pub harnesses: Vec<String>,
    /// Source repository name
    pub repository: String,
    /// Content digest for verification
    pub digest: Option<String>,
    /// When this rulebook was installed
    pub installed_at: String,
}

impl Default for CatalogLock {
    fn default() -> Self {
        Self {
            api_version: "cupcake.dev/v1".to_string(),
            generated: chrono::Utc::now().to_rfc3339(),
            installed: Vec::new(),
        }
    }
}

impl CatalogLock {
    /// Load lock file or return default if not found
    pub fn load_or_default() -> Result<Self> {
        Self::load_from_path(Path::new(LOCK_FILE))
    }

    /// Load lock file from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path).context("Failed to read catalog.lock")?;

        serde_yaml_ng::from_str(&content).context("Failed to parse catalog.lock")
    }

    /// Save lock file to the default location
    pub fn save(&self) -> Result<()> {
        self.save_to_path(Path::new(LOCK_FILE))
    }

    /// Save lock file to a specific path
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut lock = self.clone();
        lock.generated = chrono::Utc::now().to_rfc3339();

        let content = serde_yaml_ng::to_string(&lock)?;
        std::fs::write(path, content)?;

        Ok(())
    }

    /// Add or update an installed rulebook from an index entry
    pub fn add_installed(&mut self, entry: &IndexEntry, repository: &str) {
        // Remove existing entry if present
        self.installed.retain(|e| e.name != entry.name);

        self.installed.push(InstalledRulebook {
            name: entry.name.clone(),
            version: entry.version.clone(),
            harnesses: entry.harnesses.clone(),
            repository: repository.to_string(),
            digest: entry.digest.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Update an existing installed rulebook
    pub fn update_installed(&mut self, entry: &IndexEntry, repository: &str) {
        self.add_installed(entry, repository);
    }

    /// Remove an installed rulebook by name
    pub fn remove_installed(&mut self, name: &str) {
        self.installed.retain(|e| e.name != name);
    }

    /// Check if a rulebook is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.iter().any(|e| e.name == name)
    }

    /// Get the installed version of a rulebook
    pub fn get_installed(&self, name: &str) -> Option<&InstalledRulebook> {
        self.installed.iter().find(|e| e.name == name)
    }

    /// Get all installed rulebooks
    pub fn list_installed(&self) -> &[InstalledRulebook] {
        &self.installed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_entry(name: &str, version: &str) -> IndexEntry {
        IndexEntry {
            name: name.to_string(),
            version: version.to_string(),
            description: "Test rulebook".to_string(),
            harnesses: vec!["claude".to_string()],
            keywords: vec![],
            digest: Some("sha256:abc123".to_string()),
            created: None,
            urls: vec![],
            deprecated: false,
        }
    }

    #[test]
    fn test_default_lock() {
        let lock = CatalogLock::default();
        assert_eq!(lock.api_version, "cupcake.dev/v1");
        assert!(lock.installed.is_empty());
    }

    #[test]
    fn test_add_installed() {
        let mut lock = CatalogLock::default();
        let entry = make_test_entry("security-hardened", "1.0.0");

        lock.add_installed(&entry, "official");

        assert!(lock.is_installed("security-hardened"));
        let installed = lock.get_installed("security-hardened").unwrap();
        assert_eq!(installed.version, "1.0.0");
        assert_eq!(installed.repository, "official");
    }

    #[test]
    fn test_update_replaces_existing() {
        let mut lock = CatalogLock::default();
        let entry_v1 = make_test_entry("security-hardened", "1.0.0");
        let entry_v2 = make_test_entry("security-hardened", "2.0.0");

        lock.add_installed(&entry_v1, "official");
        lock.update_installed(&entry_v2, "official");

        // Should only have one entry
        assert_eq!(lock.installed.len(), 1);
        let installed = lock.get_installed("security-hardened").unwrap();
        assert_eq!(installed.version, "2.0.0");
    }

    #[test]
    fn test_remove_installed() {
        let mut lock = CatalogLock::default();
        let entry = make_test_entry("security-hardened", "1.0.0");

        lock.add_installed(&entry, "official");
        assert!(lock.is_installed("security-hardened"));

        lock.remove_installed("security-hardened");
        assert!(!lock.is_installed("security-hardened"));
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("catalog.lock");

        let mut lock = CatalogLock::default();
        let entry = make_test_entry("test-rulebook", "1.0.0");
        lock.add_installed(&entry, "official");
        lock.save_to_path(&lock_path).unwrap();

        let loaded = CatalogLock::load_from_path(&lock_path).unwrap();
        assert!(loaded.is_installed("test-rulebook"));
    }
}
