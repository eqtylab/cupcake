//! Rulebook manifest parsing (manifest.yaml)
//!
//! The manifest defines metadata for a catalog rulebook including
//! name, version, description, supported harnesses, and more.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Valid harness types for rulebooks
pub const VALID_HARNESSES: &[&str] = &["claude", "cursor", "opencode", "factory"];

/// A rulebook manifest (manifest.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulebookManifest {
    /// API version (must be "cupcake.dev/v1")
    pub api_version: String,

    /// Kind (must be "Rulebook")
    pub kind: String,

    /// Rulebook metadata
    pub metadata: ManifestMetadata,

    /// Optional spec fields
    #[serde(default)]
    pub spec: ManifestSpec,
}

/// Rulebook metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestMetadata {
    /// Unique rulebook name (lowercase alphanumeric with hyphens)
    pub name: String,

    /// Semantic version (e.g., "1.2.3")
    pub version: String,

    /// Description of the rulebook
    pub description: String,

    /// Supported harnesses
    pub harnesses: Vec<String>,

    /// Searchable keywords
    #[serde(default)]
    pub keywords: Vec<String>,

    /// SPDX license identifier
    #[serde(default)]
    pub license: Option<String>,

    /// Maintainer information
    #[serde(default)]
    pub maintainers: Vec<Maintainer>,

    /// Homepage URL
    #[serde(default)]
    pub homepage: Option<String>,

    /// Source repository URL
    #[serde(default)]
    pub repository: Option<String>,
}

/// Maintainer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maintainer {
    /// Maintainer name
    pub name: String,

    /// Email address
    #[serde(default)]
    pub email: Option<String>,

    /// Website URL
    #[serde(default)]
    pub url: Option<String>,
}

/// Optional specification fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSpec {
    /// Minimum Cupcake version required (semver range)
    #[serde(default)]
    pub cupcake_version: Option<String>,

    /// Whether this rulebook is deprecated
    #[serde(default)]
    pub deprecated: bool,

    /// Deprecation warning message
    #[serde(default)]
    pub deprecation_warning: Option<String>,
}

impl RulebookManifest {
    /// Load manifest from a file path
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest: {}", path.display()))?;

        Self::from_yaml(&content)
            .with_context(|| format!("Failed to parse manifest: {}", path.display()))
    }

    /// Load manifest from a rulebook directory
    pub fn from_dir(dir: &Path) -> Result<Self> {
        Self::from_file(&dir.join("manifest.yaml"))
    }

    /// Parse manifest from YAML string
    pub fn from_yaml(content: &str) -> Result<Self> {
        serde_yaml_ng::from_str(content).context("Invalid manifest YAML")
    }

    /// Validate the manifest contents
    pub fn validate(&self) -> Result<()> {
        // Check API version
        if self.api_version != "cupcake.dev/v1" {
            anyhow::bail!(
                "Unsupported apiVersion '{}'. Expected 'cupcake.dev/v1'",
                self.api_version
            );
        }

        // Check kind
        if self.kind != "Rulebook" {
            anyhow::bail!("Invalid kind '{}'. Expected 'Rulebook'", self.kind);
        }

        // Check name
        if self.metadata.name.is_empty() {
            anyhow::bail!("Rulebook name is required");
        }

        if !self
            .metadata
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
        {
            anyhow::bail!(
                "Rulebook name '{}' must be lowercase alphanumeric with hyphens",
                self.metadata.name
            );
        }

        if self.metadata.name.starts_with('-') || self.metadata.name.ends_with('-') {
            anyhow::bail!(
                "Rulebook name '{}' cannot start or end with a hyphen",
                self.metadata.name
            );
        }

        // Check version
        if self.metadata.version.is_empty() {
            anyhow::bail!("Version is required");
        }

        // Basic semver check (major.minor.patch)
        let version_parts: Vec<&str> = self.metadata.version.split('.').collect();
        if version_parts.len() < 3 {
            anyhow::bail!(
                "Version '{}' must be semantic versioning (e.g., 1.0.0)",
                self.metadata.version
            );
        }

        // Check description
        if self.metadata.description.trim().len() < 10 {
            anyhow::bail!("Description must be at least 10 characters");
        }

        // Check harnesses
        if self.metadata.harnesses.is_empty() {
            anyhow::bail!("At least one harness must be specified");
        }

        for harness in &self.metadata.harnesses {
            if !VALID_HARNESSES.contains(&harness.as_str()) {
                anyhow::bail!(
                    "Invalid harness '{}'. Valid harnesses: {:?}",
                    harness,
                    VALID_HARNESSES
                );
            }
        }

        Ok(())
    }

    /// Convert rulebook name to Rego-compatible format (hyphens to underscores)
    pub fn rego_name(&self) -> String {
        self.metadata.name.replace('-', "_")
    }

    /// Get the expected Rego namespace prefix for this rulebook
    pub fn namespace_prefix(&self) -> String {
        format!("cupcake.catalog.{}", self.rego_name())
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let yaml = r#"
apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: test-rulebook
  version: 1.0.0
  description: A test rulebook for validation
  harnesses:
    - claude
    - cursor
"#;

        let manifest = RulebookManifest::from_yaml(yaml).unwrap();
        assert_eq!(manifest.metadata.name, "test-rulebook");
        assert_eq!(manifest.metadata.version, "1.0.0");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_invalid_api_version() {
        let yaml = r#"
apiVersion: cupcake.dev/v2
kind: Rulebook
metadata:
  name: test
  version: 1.0.0
  description: Test description here
  harnesses: [claude]
"#;

        let manifest = RulebookManifest::from_yaml(yaml).unwrap();
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("apiVersion"));
    }

    #[test]
    fn test_invalid_harness() {
        let yaml = r#"
apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: test
  version: 1.0.0
  description: Test description here
  harnesses: [invalid]
"#;

        let manifest = RulebookManifest::from_yaml(yaml).unwrap();
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid harness"));
    }

    #[test]
    fn test_invalid_name_format() {
        let yaml = r#"
apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: Test_Rulebook
  version: 1.0.0
  description: Test description here
  harnesses: [claude]
"#;

        let manifest = RulebookManifest::from_yaml(yaml).unwrap();
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("lowercase"));
    }

    #[test]
    fn test_rego_name_conversion() {
        let yaml = r#"
apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: security-hardened
  version: 1.0.0
  description: Test description here
  harnesses: [claude]
"#;

        let manifest = RulebookManifest::from_yaml(yaml).unwrap();
        assert_eq!(manifest.rego_name(), "security_hardened");
        assert_eq!(
            manifest.namespace_prefix(),
            "cupcake.catalog.security_hardened"
        );
    }
}
