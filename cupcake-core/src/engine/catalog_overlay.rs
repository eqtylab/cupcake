//! Catalog overlay discovery and integration
//!
//! Discovers installed catalog rulebooks and prepares them for compilation
//! and evaluation. Catalog overlays are installed in `.cupcake/catalog/<name>/`
//! and provide additional policies that layer between global and project policies.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use super::PolicyUnit;
use crate::harness::types::HarnessType;

/// Represents a discovered catalog overlay
#[derive(Debug, Clone)]
pub struct CatalogOverlay {
    /// Rulebook name (e.g., "security-hardened")
    pub name: String,
    /// Path to the harness-specific policies directory (e.g., policies/opencode/)
    pub path: PathBuf,
    /// Path to the rulebook root (for accessing helpers/)
    pub rulebook_root: PathBuf,
    /// Discovered policy units for this overlay
    pub policies: Vec<PolicyUnit>,
    /// The namespace prefix for this overlay
    /// e.g., "cupcake.catalog.security_hardened"
    pub namespace: String,
}

impl CatalogOverlay {
    /// Convert rulebook name to Rego-compatible namespace
    /// e.g., "security-hardened" -> "security_hardened"
    pub fn name_to_namespace(name: &str) -> String {
        name.replace('-', "_")
    }
}

/// Discover all installed catalog overlays for a specific harness
pub async fn discover_catalog_overlays(
    cupcake_dir: &Path,
    harness: HarnessType,
) -> Result<Vec<CatalogOverlay>> {
    let catalog_dir = cupcake_dir.join("catalog");

    if !catalog_dir.exists() {
        debug!("No catalog directory found at {:?}", catalog_dir);
        return Ok(Vec::new());
    }

    let harness_subdir = match harness {
        HarnessType::ClaudeCode => "claude",
        HarnessType::Cursor => "cursor",
        HarnessType::Factory => "factory",
        HarnessType::OpenCode => "opencode",
    };

    let mut overlays = Vec::new();
    let mut entries = tokio::fs::read_dir(&catalog_dir)
        .await
        .context("Failed to read catalog directory")?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Check for manifest.yaml to verify this is a valid rulebook
        let manifest_path = path.join("manifest.yaml");
        if !manifest_path.exists() {
            debug!("Skipping {:?} - no manifest.yaml", path);
            continue;
        }

        // Check if this rulebook has policies for the current harness
        let harness_policies_dir = path.join("policies").join(harness_subdir);
        if !harness_policies_dir.exists() {
            debug!(
                "Rulebook {} doesn't have policies for {} harness",
                name, harness_subdir
            );
            continue;
        }

        info!(
            "Discovered catalog overlay: {} at {:?}",
            name, harness_policies_dir
        );

        let namespace = format!(
            "cupcake.catalog.{}",
            CatalogOverlay::name_to_namespace(&name)
        );

        overlays.push(CatalogOverlay {
            name,
            path: harness_policies_dir,
            rulebook_root: path, // Store the rulebook root for accessing helpers/
            policies: Vec::new(), // Will be populated by scan_catalog_policies
            namespace,
        });
    }

    info!("Discovered {} catalog overlays", overlays.len());
    Ok(overlays)
}

/// Scan and parse policies from all catalog overlays
pub async fn scan_catalog_policies(
    overlays: &mut [CatalogOverlay],
    _opa_path: Option<PathBuf>,
) -> Result<()> {
    use super::scanner;

    for overlay in overlays.iter_mut() {
        info!("Scanning catalog overlay: {}", overlay.name);

        // Scan for policy files in the harness-specific directory
        // (catalog policies define their own namespace)
        let policy_files = match scanner::scan_policies(&overlay.path).await {
            Ok(files) => files,
            Err(e) => {
                warn!("Failed to scan catalog overlay {}: {}", overlay.name, e);
                continue;
            }
        };

        info!(
            "Found {} policy files in catalog overlay {}",
            policy_files.len(),
            overlay.name
        );

        // Also scan for helper files in the helpers/ directory at rulebook root
        let helpers_dir = overlay.rulebook_root.join("helpers");
        let helper_files = if helpers_dir.exists() {
            match scanner::scan_policies(&helpers_dir).await {
                Ok(files) => {
                    info!(
                        "Found {} helper files in catalog overlay {}",
                        files.len(),
                        overlay.name
                    );
                    files
                }
                Err(e) => {
                    warn!(
                        "Failed to scan helpers for catalog overlay {}: {}",
                        overlay.name, e
                    );
                    Vec::new()
                }
            }
        } else {
            debug!("No helpers directory for catalog overlay {}", overlay.name);
            Vec::new()
        };

        // Parse each policy file
        for path in policy_files {
            match parse_catalog_policy(&path, &overlay.namespace).await {
                Ok(unit) => {
                    debug!(
                        "Parsed catalog policy: {} from {:?}",
                        unit.package_name, path
                    );
                    overlay.policies.push(unit);
                }
                Err(e) => {
                    warn!("Failed to parse catalog policy {:?}: {}", path, e);
                }
            }
        }

        // Parse each helper file (helpers use a different namespace pattern)
        for path in helper_files {
            match parse_catalog_helper(&path, &overlay.namespace).await {
                Ok(unit) => {
                    debug!(
                        "Parsed catalog helper: {} from {:?}",
                        unit.package_name, path
                    );
                    overlay.policies.push(unit);
                }
                Err(e) => {
                    warn!("Failed to parse catalog helper {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(())
}

/// Parse a single catalog policy file
async fn parse_catalog_policy(path: &Path, expected_namespace: &str) -> Result<PolicyUnit> {
    use super::metadata::{self, RoutingDirective};

    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read policy file")?;

    // Extract package name
    let package_name =
        metadata::extract_package_name(&content).context("Failed to extract package name")?;

    // Validate namespace - catalog policies MUST use the correct namespace
    let expected_prefix = format!("{}.policies.", expected_namespace);
    let expected_system = format!("{}.system", expected_namespace);

    if !package_name.starts_with(&expected_prefix) && package_name != expected_system {
        return Err(anyhow::anyhow!(
            "Catalog policy {} has invalid namespace. Expected prefix '{}' or '{}'",
            package_name,
            expected_prefix,
            expected_system
        ));
    }

    // Parse OPA metadata
    let policy_metadata =
        metadata::parse_metadata(&content).context("Failed to parse OPA metadata")?;

    // Extract routing directive
    let routing = if let Some(ref meta) = policy_metadata {
        if let Some(ref routing_directive) = meta.custom.routing {
            routing_directive.clone()
        } else if package_name.ends_with(".system") {
            // System policies don't need routing
            RoutingDirective::default()
        } else {
            warn!("Catalog policy {} has no routing directive", package_name);
            return Err(anyhow::anyhow!("Policy missing routing directive"));
        }
    } else if package_name.ends_with(".system") {
        RoutingDirective::default()
    } else {
        warn!("Catalog policy {} has no metadata block", package_name);
        return Err(anyhow::anyhow!("Policy missing metadata"));
    };

    Ok(PolicyUnit {
        path: path.to_path_buf(),
        package_name,
        routing,
        metadata: policy_metadata,
    })
}

/// Parse a single catalog helper file
/// Helpers have a different namespace pattern: cupcake.catalog.<name>.helpers.*
async fn parse_catalog_helper(path: &Path, expected_namespace: &str) -> Result<PolicyUnit> {
    use super::metadata::{self, RoutingDirective};

    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read helper file")?;

    // Extract package name
    let package_name =
        metadata::extract_package_name(&content).context("Failed to extract package name")?;

    // Validate namespace - helpers MUST use the helpers namespace
    let expected_prefix = format!("{}.helpers.", expected_namespace);

    if !package_name.starts_with(&expected_prefix) {
        return Err(anyhow::anyhow!(
            "Catalog helper {} has invalid namespace. Expected prefix '{}'",
            package_name,
            expected_prefix
        ));
    }

    // Parse OPA metadata (optional for helpers)
    let policy_metadata = metadata::parse_metadata(&content).ok().flatten();

    // Helpers don't need routing - they're just utility functions
    let routing = RoutingDirective::default();

    Ok(PolicyUnit {
        path: path.to_path_buf(),
        package_name,
        routing,
        metadata: policy_metadata,
    })
}

/// Compile catalog overlay policies to WASM
pub async fn compile_catalog_overlay(
    overlay: &CatalogOverlay,
    opa_path: Option<PathBuf>,
) -> Result<Vec<u8>> {
    use super::compiler;

    if overlay.policies.is_empty() {
        return Err(anyhow::anyhow!(
            "No policies to compile in catalog overlay {}",
            overlay.name
        ));
    }

    // Check if we have non-system policies (helpers count as non-system)
    let non_system_count = overlay
        .policies
        .iter()
        .filter(|p| !p.package_name.ends_with(".system"))
        .count();

    if non_system_count == 0 {
        return Err(anyhow::anyhow!(
            "Catalog overlay {} has only system policies - nothing to compile",
            overlay.name
        ));
    }

    info!(
        "Compiling catalog overlay {} ({} policies, {} non-system)",
        overlay.name,
        overlay.policies.len(),
        non_system_count
    );

    // Use the specialized catalog overlay compilation function that understands
    // the rulebook root structure (policies/ and helpers/ at same level)
    let system_namespace = format!("{}.system", overlay.namespace);
    let wasm_bytes = compiler::compile_catalog_overlay_policies(
        &overlay.policies,
        &system_namespace,
        &overlay.rulebook_root,
        opa_path,
    )
    .await?;

    info!(
        "Compiled catalog overlay {} to {} bytes",
        overlay.name,
        wasm_bytes.len()
    );

    Ok(wasm_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[test]
    fn test_name_to_namespace() {
        assert_eq!(
            CatalogOverlay::name_to_namespace("security-hardened"),
            "security_hardened"
        );
        assert_eq!(
            CatalogOverlay::name_to_namespace("git-workflow"),
            "git_workflow"
        );
        assert_eq!(CatalogOverlay::name_to_namespace("simple"), "simple");
    }

    #[tokio::test]
    async fn test_discover_no_catalog_dir() {
        let temp_dir = TempDir::new().unwrap();
        let overlays = discover_catalog_overlays(temp_dir.path(), HarnessType::ClaudeCode)
            .await
            .unwrap();
        assert!(overlays.is_empty());
    }

    #[tokio::test]
    async fn test_discover_empty_catalog_dir() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("catalog"))
            .await
            .unwrap();

        let overlays = discover_catalog_overlays(temp_dir.path(), HarnessType::ClaudeCode)
            .await
            .unwrap();
        assert!(overlays.is_empty());
    }

    #[tokio::test]
    async fn test_discover_valid_overlay() {
        let temp_dir = TempDir::new().unwrap();
        let catalog_dir = temp_dir.path().join("catalog");
        let rulebook_dir = catalog_dir.join("test-rulebook");
        let policies_dir = rulebook_dir.join("policies").join("claude");

        fs::create_dir_all(&policies_dir).await.unwrap();

        // Create manifest
        fs::write(
            rulebook_dir.join("manifest.yaml"),
            r#"apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: test-rulebook
  version: "1.0.0"
  description: Test
  harnesses:
    - claude
"#,
        )
        .await
        .unwrap();

        let overlays = discover_catalog_overlays(temp_dir.path(), HarnessType::ClaudeCode)
            .await
            .unwrap();

        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].name, "test-rulebook");
        assert_eq!(overlays[0].namespace, "cupcake.catalog.test_rulebook");
        assert_eq!(overlays[0].rulebook_root, rulebook_dir);
        assert_eq!(overlays[0].path, policies_dir);
    }

    #[tokio::test]
    async fn test_discover_wrong_harness() {
        let temp_dir = TempDir::new().unwrap();
        let catalog_dir = temp_dir.path().join("catalog");
        let rulebook_dir = catalog_dir.join("test-rulebook");
        let policies_dir = rulebook_dir.join("policies").join("claude");

        fs::create_dir_all(&policies_dir).await.unwrap();

        fs::write(
            rulebook_dir.join("manifest.yaml"),
            r#"apiVersion: cupcake.dev/v1
kind: Rulebook
metadata:
  name: test-rulebook
  version: "1.0.0"
  description: Test
  harnesses:
    - claude
"#,
        )
        .await
        .unwrap();

        // Query for Cursor harness - should find nothing
        let overlays = discover_catalog_overlays(temp_dir.path(), HarnessType::Cursor)
            .await
            .unwrap();

        assert!(overlays.is_empty());
    }
}
