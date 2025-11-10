//! Governance bundle loading and integration
//!
//! This module handles loading OPA bundles from the governance-service,
//! extracting their contents (manifest, WASM, signals, actions), and
//! making them available for integration with the Cupcake engine.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

use crate::engine::rulebook::{ActionConfig, SignalConfig};

/// Governance bundle manifest (OPA format)
/// This follows the OPA bundle .manifest format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub revision: String,
    pub roots: Vec<String>,
    pub wasm: Vec<WasmModule>,
    pub rego_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub entrypoint: String,
    pub module: String,

    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub scope: String,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub authors: Option<Vec<Author>>,

    #[serde(default)]
    pub custom: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
}

/// A loaded governance bundle with all components
#[derive(Debug, Clone)]
pub struct GovernanceBundle {
    pub manifest: BundleManifest,
    pub wasm: Vec<u8>,
    pub signals: HashMap<String, SignalConfig>,
    pub actions: HashMap<String, Vec<ActionConfig>>,
}

/// Loader for governance bundles
pub struct GovernanceBundleLoader;

impl GovernanceBundleLoader {
    /// Load bundle from local tarball
    pub async fn load_from_file(bundle_path: impl AsRef<Path>) -> Result<GovernanceBundle> {
        info!("Loading governance bundle from: {:?}", bundle_path.as_ref());

        // Extract tarball
        let extracted = Self::extract_tarball(bundle_path.as_ref()).await?;

        // Load manifest
        let manifest_path = extracted.join(".manifest");
        let manifest_content = tokio::fs::read_to_string(&manifest_path)
            .await
            .context("Failed to read manifest")?;
        let manifest: BundleManifest =
            serde_json::from_str(&manifest_content).context("Failed to parse manifest")?;

        // Load WASM
        let wasm_path = extracted.join("policy.wasm");
        let wasm = tokio::fs::read(&wasm_path)
            .await
            .context("Failed to read WASM module")?;

        // Parse signals from manifest or metadata
        let signals = Self::parse_signals_from_manifest(&manifest)?;

        // Parse actions from manifest or metadata
        let actions = Self::parse_actions_from_manifest(&manifest)?;

        debug!(
            "Loaded governance bundle: {} signals, {} actions, {} bytes WASM",
            signals.len(),
            actions.len(),
            wasm.len()
        );

        Ok(GovernanceBundle {
            manifest,
            wasm,
            signals,
            actions,
        })
    }

    async fn extract_tarball(path: &Path) -> Result<std::path::PathBuf> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        // Extract to temp directory
        let temp_dir =
            std::env::temp_dir().join(format!("cupcake-bundle-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        // Extract synchronously (tar crate doesn't support async)
        let temp_dir_clone = temp_dir.clone();
        let path_clone = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let bundle_bytes = std::fs::read(&path_clone)?;
            let decoder = GzDecoder::new(bundle_bytes.as_slice());
            let mut archive = Archive::new(decoder);
            archive.unpack(&temp_dir_clone)?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(temp_dir)
    }

    fn parse_signals_from_manifest(
        manifest: &BundleManifest,
    ) -> Result<HashMap<String, SignalConfig>> {
        let mut signals = HashMap::new();

        if let Some(annotation) = manifest.wasm.first().and_then(|w| w.annotations.first()) {
            if let Some(custom) = &annotation.custom {
                if let Some(available_signals) = custom.get("available_signals") {
                    if let Some(signals_obj) = available_signals.get("signals") {
                        signals = serde_json::from_value(signals_obj.clone())?;
                    }
                }
            }
        }

        Ok(signals)
    }

    fn parse_actions_from_manifest(
        manifest: &BundleManifest,
    ) -> Result<HashMap<String, Vec<ActionConfig>>> {
        let mut actions = HashMap::new();

        if let Some(annotation) = manifest.wasm.first().and_then(|w| w.annotations.first()) {
            if let Some(custom) = &annotation.custom {
                if let Some(available_actions) = custom.get("available_actions") {
                    if let Some(actions_obj) = available_actions.get("actions") {
                        actions = serde_json::from_value(actions_obj.clone())?;
                    }
                }
            }
        }

        Ok(actions)
    }
}

impl BundleManifest {
    /// Get the entrypoint from manifest
    pub fn entrypoint(&self) -> Option<&str> {
        self.wasm.first().map(|w| w.entrypoint.as_str())
    }

    /// Extract rulebook metadata
    pub fn rulebook_id(&self) -> Option<String> {
        self.wasm
            .first()?
            .annotations
            .first()?
            .custom
            .as_ref()?
            .get("rulebook_id")?
            .as_str()
            .map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parsing() {
        let manifest_json = r#"{
            "revision": "test-rev",
            "roots": ["governance"],
            "wasm": [
                {
                    "entrypoint": "governance/system/evaluate",
                    "module": "/policy.wasm",
                    "annotations": [
                        {
                            "scope": "package",
                            "title": "Test Rulebook",
                            "custom": {
                                "rulebook_id": "test-123"
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        assert_eq!(manifest.revision, "test-rev");
        assert_eq!(manifest.entrypoint(), Some("governance/system/evaluate"));
        assert_eq!(manifest.rulebook_id(), Some("test-123".to_string()));
    }
}
