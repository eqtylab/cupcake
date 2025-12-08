//! Catalog index parsing and management
//!
//! The index.yaml file lists all available rulebooks in a registry
//! with their versions, descriptions, and download URLs.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A catalog index (index.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogIndex {
    /// API version
    pub api_version: String,

    /// Kind (CatalogIndex)
    pub kind: String,

    /// When the index was generated
    pub generated: String,

    /// All rulebook entries, keyed by name
    pub entries: HashMap<String, Vec<IndexEntry>>,
}

/// An entry for a specific rulebook version
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    /// Rulebook name
    pub name: String,

    /// Version string
    pub version: String,

    /// Description
    pub description: String,

    /// Supported harnesses
    pub harnesses: Vec<String>,

    /// Searchable keywords
    #[serde(default)]
    pub keywords: Vec<String>,

    /// SHA-256 digest of the tarball
    #[serde(default)]
    pub digest: Option<String>,

    /// When this version was created
    #[serde(default)]
    pub created: Option<String>,

    /// Download URLs for the tarball
    #[serde(default)]
    pub urls: Vec<String>,

    /// Whether this version is deprecated
    #[serde(default)]
    pub deprecated: bool,
}

impl Default for CatalogIndex {
    fn default() -> Self {
        Self {
            api_version: "cupcake.dev/v1".to_string(),
            kind: "CatalogIndex".to_string(),
            generated: chrono::Utc::now().to_rfc3339(),
            entries: HashMap::new(),
        }
    }
}

impl CatalogIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse index from YAML string
    pub fn from_yaml(content: &str) -> Result<Self> {
        serde_yaml_ng::from_str(content).context("Failed to parse catalog index YAML")
    }

    /// Serialize to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml_ng::to_string(self).context("Failed to serialize catalog index")
    }

    /// Merge another index into this one
    ///
    /// Entries from the other index are added. If both indexes have
    /// the same rulebook, versions are merged and sorted newest-first.
    pub fn merge(&mut self, other: CatalogIndex) {
        for (name, versions) in other.entries {
            self.entries.entry(name).or_default().extend(versions);
        }

        // Sort versions (newest first) and deduplicate
        for versions in self.entries.values_mut() {
            versions.sort_by(|a, b| {
                // Try to parse as semver for proper comparison
                match (
                    semver::Version::parse(&a.version),
                    semver::Version::parse(&b.version),
                ) {
                    (Ok(va), Ok(vb)) => vb.cmp(&va), // Newest first
                    _ => b.version.cmp(&a.version),  // Fallback to string comparison
                }
            });

            // Remove duplicates (keep first occurrence = newest)
            versions.dedup_by(|a, b| a.version == b.version);
        }
    }

    /// Get all versions of a rulebook
    pub fn get_versions(&self, name: &str) -> Option<&Vec<IndexEntry>> {
        self.entries.get(name)
    }

    /// Get the latest version of a rulebook
    pub fn get_latest(&self, name: &str) -> Option<&IndexEntry> {
        self.entries.get(name).and_then(|v| v.first())
    }

    /// Get a specific version of a rulebook
    pub fn get_version(&self, name: &str, version: &str) -> Option<&IndexEntry> {
        self.entries
            .get(name)
            .and_then(|versions| versions.iter().find(|e| e.version == version))
    }

    /// Search entries by query string
    ///
    /// Matches against name, description, and keywords (case-insensitive).
    /// Returns the latest version of each matching rulebook.
    pub fn search(&self, query: &str) -> Vec<&IndexEntry> {
        let query_lower = query.to_lowercase();

        self.entries
            .values()
            .filter_map(|versions| {
                let latest = versions.first()?;

                let matches_name = latest.name.to_lowercase().contains(&query_lower);
                let matches_desc = latest.description.to_lowercase().contains(&query_lower);
                let matches_keyword = latest
                    .keywords
                    .iter()
                    .any(|k| k.to_lowercase().contains(&query_lower));

                if matches_name || matches_desc || matches_keyword {
                    Some(latest)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Filter entries by harness
    ///
    /// Returns the latest version of each rulebook that supports the harness.
    pub fn filter_by_harness(&self, harness: &str) -> Vec<&IndexEntry> {
        self.entries
            .values()
            .filter_map(|versions| {
                let latest = versions.first()?;
                if latest.harnesses.contains(&harness.to_string()) {
                    Some(latest)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all rulebooks (latest version of each)
    pub fn list_all(&self) -> Vec<&IndexEntry> {
        self.entries
            .values()
            .filter_map(|versions| versions.first())
            .collect()
    }

    /// Get total number of unique rulebooks
    pub fn rulebook_count(&self) -> usize {
        self.entries.len()
    }

    /// Get total number of versions across all rulebooks
    pub fn version_count(&self) -> usize {
        self.entries.values().map(|v| v.len()).sum()
    }

    /// Resolve a version specifier to a concrete version
    ///
    /// Supports:
    /// - Exact version: "1.2.0"
    /// - Caret (compatible): "^1.2" means >=1.2.0 <2.0.0, "^0.2" means >=0.2.0 <0.3.0
    /// - Tilde (patch-level): "~1.2" means >=1.2.0 <1.3.0
    /// - Latest: "" or "latest"
    pub fn resolve_version(&self, name: &str, specifier: &str) -> Option<&IndexEntry> {
        let versions = self.entries.get(name)?;

        if specifier.is_empty() || specifier.eq_ignore_ascii_case("latest") {
            return versions.first();
        }

        // Check for exact version first
        if let Some(entry) = versions.iter().find(|e| e.version == specifier) {
            return Some(entry);
        }

        // Parse version specifier
        if let Some(range) = specifier.strip_prefix('^') {
            // Caret: ^1.2 means >=1.2.0 <2.0.0
            return self.resolve_caret_range(versions, range);
        } else if let Some(range) = specifier.strip_prefix('~') {
            // Tilde: ~1.2 means >=1.2.0 <1.3.0
            return self.resolve_tilde_range(versions, range);
        }

        // Fallback: try exact match
        versions.iter().find(|e| e.version == specifier)
    }

    /// Resolve caret range (^1.2 means >=1.2.0 <2.0.0)
    fn resolve_caret_range<'a>(
        &self,
        versions: &'a [IndexEntry],
        range: &str,
    ) -> Option<&'a IndexEntry> {
        let parts: Vec<&str> = range.split('.').collect();
        let (major, minor, patch) = match parts.len() {
            1 => (parts[0].parse::<u64>().ok()?, 0, 0),
            2 => (
                parts[0].parse::<u64>().ok()?,
                parts[1].parse::<u64>().ok()?,
                0,
            ),
            _ => (
                parts[0].parse::<u64>().ok()?,
                parts[1].parse::<u64>().ok()?,
                parts[2].parse::<u64>().ok()?,
            ),
        };

        let min_version = semver::Version::new(major, minor, patch);

        // For caret, max is next major (or next minor if major is 0)
        let max_version = if major == 0 {
            semver::Version::new(0, minor + 1, 0)
        } else {
            semver::Version::new(major + 1, 0, 0)
        };

        // Find best matching version (newest that satisfies range)
        versions.iter().find(|entry| {
            if let Ok(v) = semver::Version::parse(&entry.version) {
                v >= min_version && v < max_version
            } else {
                false
            }
        })
    }

    /// Resolve tilde range (~1.2 means >=1.2.0 <1.3.0)
    fn resolve_tilde_range<'a>(
        &self,
        versions: &'a [IndexEntry],
        range: &str,
    ) -> Option<&'a IndexEntry> {
        let parts: Vec<&str> = range.split('.').collect();
        let (major, minor, patch) = match parts.len() {
            1 => (parts[0].parse::<u64>().ok()?, 0, 0),
            2 => (
                parts[0].parse::<u64>().ok()?,
                parts[1].parse::<u64>().ok()?,
                0,
            ),
            _ => (
                parts[0].parse::<u64>().ok()?,
                parts[1].parse::<u64>().ok()?,
                parts[2].parse::<u64>().ok()?,
            ),
        };

        let min_version = semver::Version::new(major, minor, patch);
        let max_version = semver::Version::new(major, minor + 1, 0);

        // Find best matching version (newest that satisfies range)
        versions.iter().find(|entry| {
            if let Ok(v) = semver::Version::parse(&entry.version) {
                v >= min_version && v < max_version
            } else {
                false
            }
        })
    }
}

impl IndexEntry {
    /// Get the first download URL if available
    pub fn download_url(&self) -> Option<&str> {
        self.urls.first().map(|s| s.as_str())
    }

    /// Check if this entry has a valid download URL
    pub fn has_download(&self) -> bool {
        !self.urls.is_empty()
    }

    /// Get harnesses as a comma-separated string
    pub fn harnesses_display(&self) -> String {
        self.harnesses.join(", ")
    }

    /// Truncate description to first line
    pub fn short_description(&self) -> &str {
        self.description
            .lines()
            .next()
            .unwrap_or(&self.description)
            .trim()
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;

    fn sample_index_yaml() -> &'static str {
        r#"
apiVersion: cupcake.dev/v1
kind: CatalogIndex
generated: "2025-12-04T00:00:00Z"
entries:
  security-hardened:
    - name: security-hardened
      version: "1.2.0"
      description: Production security policies
      harnesses: [claude, cursor]
      keywords: [security, production]
      deprecated: false
    - name: security-hardened
      version: "1.1.0"
      description: Production security policies
      harnesses: [claude]
      keywords: [security]
      deprecated: false
  git-workflow:
    - name: git-workflow
      version: "0.5.0"
      description: Git best practices
      harnesses: [claude, cursor, opencode, factory]
      keywords: [git, workflow]
      deprecated: false
"#
    }

    #[test]
    fn test_parse_index() {
        let index = CatalogIndex::from_yaml(sample_index_yaml()).unwrap();
        assert_eq!(index.rulebook_count(), 2);
        assert_eq!(index.version_count(), 3);
    }

    #[test]
    fn test_get_latest() {
        let index = CatalogIndex::from_yaml(sample_index_yaml()).unwrap();

        let latest = index.get_latest("security-hardened").unwrap();
        assert_eq!(latest.version, "1.2.0");

        let latest = index.get_latest("git-workflow").unwrap();
        assert_eq!(latest.version, "0.5.0");

        assert!(index.get_latest("nonexistent").is_none());
    }

    #[test]
    fn test_get_version() {
        let index = CatalogIndex::from_yaml(sample_index_yaml()).unwrap();

        let entry = index.get_version("security-hardened", "1.1.0").unwrap();
        assert_eq!(entry.version, "1.1.0");
        assert_eq!(entry.harnesses, vec!["claude"]);

        assert!(index.get_version("security-hardened", "2.0.0").is_none());
    }

    #[test]
    fn test_search() {
        let index = CatalogIndex::from_yaml(sample_index_yaml()).unwrap();

        let results = index.search("security");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "security-hardened");

        let results = index.search("git");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "git-workflow");

        let results = index.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_by_harness() {
        let index = CatalogIndex::from_yaml(sample_index_yaml()).unwrap();

        let results = index.filter_by_harness("claude");
        assert_eq!(results.len(), 2); // Both rulebooks support claude

        let results = index.filter_by_harness("factory");
        assert_eq!(results.len(), 1); // Only git-workflow
        assert_eq!(results[0].name, "git-workflow");
    }

    #[test]
    fn test_merge_indexes() {
        let mut index1 = CatalogIndex::from_yaml(sample_index_yaml()).unwrap();

        let index2_yaml = r#"
apiVersion: cupcake.dev/v1
kind: CatalogIndex
generated: "2025-12-04T01:00:00Z"
entries:
  security-hardened:
    - name: security-hardened
      version: "1.3.0"
      description: New version
      harnesses: [claude, cursor, opencode]
      keywords: [security]
      deprecated: false
  python-dev:
    - name: python-dev
      version: "0.1.0"
      description: Python development
      harnesses: [claude]
      keywords: [python]
      deprecated: false
"#;
        let index2 = CatalogIndex::from_yaml(index2_yaml).unwrap();

        index1.merge(index2);

        // Should now have 3 rulebooks
        assert_eq!(index1.rulebook_count(), 3);

        // security-hardened should have 3 versions, newest first
        let versions = index1.get_versions("security-hardened").unwrap();
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version, "1.3.0");
        assert_eq!(versions[1].version, "1.2.0");
        assert_eq!(versions[2].version, "1.1.0");

        // New rulebook should be present
        assert!(index1.get_latest("python-dev").is_some());
    }

    fn sample_versioned_index() -> CatalogIndex {
        let yaml = r#"
apiVersion: cupcake.dev/v1
kind: CatalogIndex
generated: "2025-12-04T00:00:00Z"
entries:
  test-rulebook:
    - name: test-rulebook
      version: "2.0.0"
      description: Major version
      harnesses: [claude]
      deprecated: false
    - name: test-rulebook
      version: "1.5.0"
      description: Minor version
      harnesses: [claude]
      deprecated: false
    - name: test-rulebook
      version: "1.4.2"
      description: Patch version
      harnesses: [claude]
      deprecated: false
    - name: test-rulebook
      version: "1.4.0"
      description: Another minor
      harnesses: [claude]
      deprecated: false
    - name: test-rulebook
      version: "0.9.0"
      description: Pre-1.0
      harnesses: [claude]
      deprecated: false
"#;
        CatalogIndex::from_yaml(yaml).unwrap()
    }

    #[test]
    fn test_resolve_exact_version() {
        let index = sample_versioned_index();

        let entry = index.resolve_version("test-rulebook", "1.4.2").unwrap();
        assert_eq!(entry.version, "1.4.2");

        let entry = index.resolve_version("test-rulebook", "2.0.0").unwrap();
        assert_eq!(entry.version, "2.0.0");

        // Non-existent version
        assert!(index.resolve_version("test-rulebook", "3.0.0").is_none());
    }

    #[test]
    fn test_resolve_latest() {
        let index = sample_versioned_index();

        let entry = index.resolve_version("test-rulebook", "").unwrap();
        assert_eq!(entry.version, "2.0.0");

        let entry = index.resolve_version("test-rulebook", "latest").unwrap();
        assert_eq!(entry.version, "2.0.0");
    }

    #[test]
    fn test_resolve_caret_range() {
        let index = sample_versioned_index();

        // ^1.4 should match 1.5.0 (newest in >=1.4.0 <2.0.0)
        let entry = index.resolve_version("test-rulebook", "^1.4").unwrap();
        assert_eq!(entry.version, "1.5.0");

        // ^1.4.0 should also match 1.5.0
        let entry = index.resolve_version("test-rulebook", "^1.4.0").unwrap();
        assert_eq!(entry.version, "1.5.0");

        // ^2 should match 2.0.0
        let entry = index.resolve_version("test-rulebook", "^2").unwrap();
        assert_eq!(entry.version, "2.0.0");

        // ^0.9 should match 0.9.0 (for 0.x, caret is stricter)
        let entry = index.resolve_version("test-rulebook", "^0.9").unwrap();
        assert_eq!(entry.version, "0.9.0");
    }

    #[test]
    fn test_resolve_tilde_range() {
        let index = sample_versioned_index();

        // ~1.4 should match 1.4.2 (newest in >=1.4.0 <1.5.0)
        let entry = index.resolve_version("test-rulebook", "~1.4").unwrap();
        assert_eq!(entry.version, "1.4.2");

        // ~1.5 should match 1.5.0
        let entry = index.resolve_version("test-rulebook", "~1.5").unwrap();
        assert_eq!(entry.version, "1.5.0");

        // ~2.0 should match 2.0.0
        let entry = index.resolve_version("test-rulebook", "~2.0").unwrap();
        assert_eq!(entry.version, "2.0.0");
    }
}
