//! Governance bundle loading and integration
//!
//! This module handles loading OPA bundles from the governance-service,
//! extracting their contents (manifest, WASM, signals, actions), and
//! making them available for integration with the Cupcake engine.
//!
//! ## Governance Bundle Format
//!
//! Signals and actions in governance bundles are self-contained executable scripts.
//! The manifest supports multiple formats for backward compatibility:
//!
//! ### Governance Service Format (with metadata)
//! ```json
//! {
//!   "wasm": [{
//!     "annotations": [{
//!       "custom": {
//!         "available_signals": {
//!           "signals": {
//!             "signal_name": {
//!               "script": "signal_script.py",
//!               "type": "object",
//!               "cache_ttl": 300
//!             }
//!           }
//!         },
//!         "available_actions": {
//!           "actions": {
//!             "action_name": {
//!               "script": "action_script.py",
//!               "trigger_on": ["deny", "halt"],
//!               "severity_threshold": "LOW"
//!             }
//!           }
//!         }
//!       }
//!     }]
//!   }]
//! }
//! ```
//!
//! ### Simple Format (script paths only)
//! ```json
//! {
//!   "available_signals": {
//!     "signals": {
//!       "signal_name": "/path/to/script.sh"
//!     }
//!   },
//!   "available_actions": {
//!     "actions": {
//!       "RULE_ID": ["/path/to/action.sh"]
//!     }
//!   }
//! }
//! ```
//!
//! All formats are automatically converted to `SignalConfig` and `ActionConfig`
//! structs when loaded into Cupcake.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

use crate::engine::rulebook::{ActionConfig, SignalConfig};

/// Governance service signal metadata format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GovernanceSignalMeta {
    script: String,
    #[serde(default)]
    cache_ttl: Option<u64>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "type", default)]
    signal_type: Option<String>,
}

/// Governance service action metadata format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GovernanceActionMeta {
    script: String,
    #[serde(default)]
    trigger_on: Vec<String>,
    #[serde(default)]
    severity_threshold: Option<String>,
}

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
    /// Path to extracted bundle directory (for accessing policy files)
    pub extracted_path: std::path::PathBuf,
}

/// Loader for governance bundles
pub struct GovernanceBundleLoader;

impl GovernanceBundleLoader {
    /// Get the directory containing policy files in the extracted bundle
    /// Governance bundles always have policies under /policies/ subdirectory
    pub fn policies_directory(extracted_path: &Path) -> std::path::PathBuf {
        // The governance service creates bundles with a /policies/ directory
        // Check the tarball structure from your example
        extracted_path.join("governance").join("policies")
    }

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
            extracted_path: extracted,
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
                    // Try different formats for signals:

                    // Format 1: { "signals": { "name": { "script": "path", ... } } } or { "name": "path" }
                    if let Some(signals_obj) = available_signals.get("signals") {
                        // Try to parse as HashMap<String, SignalConfig> first (Cupcake format)
                        if let Ok(parsed_signals) = serde_json::from_value::<
                            HashMap<String, SignalConfig>,
                        >(signals_obj.clone())
                        {
                            signals = parsed_signals;
                        }
                        // Try governance service metadata format with "script" field
                        else if let Ok(meta_signals) =
                            serde_json::from_value::<HashMap<String, GovernanceSignalMeta>>(
                                signals_obj.clone(),
                            )
                        {
                            debug!("Parsing signals from governance service metadata format");
                            for (signal_name, meta) in meta_signals {
                                signals.insert(
                                    signal_name,
                                    SignalConfig {
                                        command: meta.script,
                                        timeout_seconds: 5, // Default timeout
                                    },
                                );
                            }
                        }
                        // Fall back to HashMap<String, String> (simple script paths)
                        else if let Ok(script_signals) =
                            serde_json::from_value::<HashMap<String, String>>(signals_obj.clone())
                        {
                            debug!("Parsing signals as simple script paths");
                            for (signal_name, script_path) in script_signals {
                                signals.insert(
                                    signal_name,
                                    SignalConfig {
                                        command: script_path,
                                        timeout_seconds: 5, // Default timeout
                                    },
                                );
                            }
                        } else {
                            debug!("Signals object format: {:?}", signals_obj);
                            anyhow::bail!("Failed to parse signals from manifest: expected HashMap<String, String>, HashMap<String, SignalConfig>, or governance metadata format");
                        }
                    }
                    // Format 2: available_signals IS the signals object directly
                    else {
                        // Try to parse available_signals directly as HashMap<String, SignalConfig>
                        if let Ok(parsed_signals) =
                            serde_json::from_value::<HashMap<String, SignalConfig>>(
                                available_signals.clone(),
                            )
                        {
                            debug!("Parsing signals directly from available_signals (SignalConfig format)");
                            signals = parsed_signals;
                        }
                        // Try governance service metadata format
                        else if let Ok(meta_signals) =
                            serde_json::from_value::<HashMap<String, GovernanceSignalMeta>>(
                                available_signals.clone(),
                            )
                        {
                            debug!("Parsing signals directly from available_signals (governance metadata)");
                            for (signal_name, meta) in meta_signals {
                                signals.insert(
                                    signal_name,
                                    SignalConfig {
                                        command: meta.script,
                                        timeout_seconds: 5,
                                    },
                                );
                            }
                        }
                        // Try to parse available_signals directly as HashMap<String, String>
                        else if let Ok(script_signals) =
                            serde_json::from_value::<HashMap<String, String>>(
                                available_signals.clone(),
                            )
                        {
                            debug!(
                                "Parsing signals directly from available_signals (simple paths)"
                            );
                            for (signal_name, script_path) in script_signals {
                                signals.insert(
                                    signal_name,
                                    SignalConfig {
                                        command: script_path,
                                        timeout_seconds: 5, // Default timeout
                                    },
                                );
                            }
                        } else {
                            debug!("Available signals format: {:?}", available_signals);
                            anyhow::bail!("Failed to parse signals from manifest: expected HashMap<String, String>, HashMap<String, SignalConfig>, or governance metadata format");
                        }
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
                    // Try different formats for actions:

                    // Format 1: { "actions": { "rule_id": ["path"] } } or { "action_name": { "script": "path", ... } }
                    if let Some(actions_obj) = available_actions.get("actions") {
                        // Try to parse as HashMap<String, Vec<ActionConfig>> first (Cupcake format)
                        if let Ok(parsed_actions) = serde_json::from_value::<
                            HashMap<String, Vec<ActionConfig>>,
                        >(actions_obj.clone())
                        {
                            actions = parsed_actions;
                        }
                        // Try governance service metadata format: HashMap<String, GovernanceActionMeta>
                        // These are global actions keyed by action name, not rule ID
                        else if let Ok(meta_actions) =
                            serde_json::from_value::<HashMap<String, GovernanceActionMeta>>(
                                actions_obj.clone(),
                            )
                        {
                            debug!("Parsing actions from governance service metadata format (global actions)");
                            // For governance bundles, actions with trigger_on are global actions
                            // We'll store them with a special key to indicate they're global
                            // The engine can decide how to apply them based on trigger_on
                            for (action_name, meta) in meta_actions {
                                let action_config = ActionConfig {
                                    command: meta.script,
                                };
                                // Use the action name as the key
                                // TODO: In the future, we might want to use trigger_on to determine
                                // which rules this action applies to
                                actions.insert(action_name, vec![action_config]);
                            }
                        }
                        // Fall back to HashMap<String, Vec<String>> (simple script paths)
                        else if let Ok(script_actions) =
                            serde_json::from_value::<HashMap<String, Vec<String>>>(
                                actions_obj.clone(),
                            )
                        {
                            debug!("Parsing actions as simple script paths");
                            for (rule_id, scripts) in script_actions {
                                let action_configs: Vec<ActionConfig> = scripts
                                    .into_iter()
                                    .map(|script_path| ActionConfig {
                                        command: script_path,
                                    })
                                    .collect();
                                actions.insert(rule_id, action_configs);
                            }
                        } else {
                            debug!("Actions object format: {:?}", actions_obj);
                            anyhow::bail!("Failed to parse actions from manifest: expected HashMap<String, Vec<String>>, HashMap<String, Vec<ActionConfig>>, or governance metadata format");
                        }
                    }
                    // Format 2: available_actions IS the actions object directly
                    else {
                        // Try to parse available_actions directly as HashMap<String, Vec<ActionConfig>>
                        if let Ok(parsed_actions) =
                            serde_json::from_value::<HashMap<String, Vec<ActionConfig>>>(
                                available_actions.clone(),
                            )
                        {
                            debug!("Parsing actions directly from available_actions (ActionConfig format)");
                            actions = parsed_actions;
                        }
                        // Try governance service metadata format
                        else if let Ok(meta_actions) =
                            serde_json::from_value::<HashMap<String, GovernanceActionMeta>>(
                                available_actions.clone(),
                            )
                        {
                            debug!("Parsing actions directly from available_actions (governance metadata)");
                            for (action_name, meta) in meta_actions {
                                let action_config = ActionConfig {
                                    command: meta.script,
                                };
                                actions.insert(action_name, vec![action_config]);
                            }
                        }
                        // Try to parse available_actions directly as HashMap<String, Vec<String>>
                        else if let Ok(script_actions) =
                            serde_json::from_value::<HashMap<String, Vec<String>>>(
                                available_actions.clone(),
                            )
                        {
                            debug!(
                                "Parsing actions directly from available_actions (simple paths)"
                            );
                            for (rule_id, scripts) in script_actions {
                                let action_configs: Vec<ActionConfig> = scripts
                                    .into_iter()
                                    .map(|script_path| ActionConfig {
                                        command: script_path,
                                    })
                                    .collect();
                                actions.insert(rule_id, action_configs);
                            }
                        } else {
                            debug!("Available actions format: {:?}", available_actions);
                            anyhow::bail!("Failed to parse actions from manifest: expected HashMap<String, Vec<String>>, HashMap<String, Vec<ActionConfig>>, or governance metadata format");
                        }
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

    #[test]
    fn test_parse_actions_as_script_paths() {
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
                            "custom": {
                                "available_actions": {
                                    "actions": {
                                        "RULE_001": ["/bundle/actions/action1.sh"],
                                        "RULE_002": ["/bundle/actions/action2.sh", "/bundle/actions/action3.sh"]
                                    }
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let actions = GovernanceBundleLoader::parse_actions_from_manifest(&manifest).unwrap();

        assert_eq!(actions.len(), 2);
        assert_eq!(actions.get("RULE_001").unwrap().len(), 1);
        assert_eq!(
            actions.get("RULE_001").unwrap()[0].command,
            "/bundle/actions/action1.sh"
        );
        assert_eq!(actions.get("RULE_002").unwrap().len(), 2);
        assert_eq!(
            actions.get("RULE_002").unwrap()[0].command,
            "/bundle/actions/action2.sh"
        );
        assert_eq!(
            actions.get("RULE_002").unwrap()[1].command,
            "/bundle/actions/action3.sh"
        );
    }

    #[test]
    fn test_parse_signals_as_script_paths() {
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
                            "custom": {
                                "available_signals": {
                                    "signals": {
                                        "git_branch": "/bundle/signals/git_branch.sh",
                                        "file_count": "/bundle/signals/file_count.sh"
                                    }
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let signals = GovernanceBundleLoader::parse_signals_from_manifest(&manifest).unwrap();

        assert_eq!(signals.len(), 2);
        assert_eq!(
            signals.get("git_branch").unwrap().command,
            "/bundle/signals/git_branch.sh"
        );
        assert_eq!(signals.get("git_branch").unwrap().timeout_seconds, 5);
        assert_eq!(
            signals.get("file_count").unwrap().command,
            "/bundle/signals/file_count.sh"
        );
    }

    #[test]
    fn test_parse_actions_with_actionconfig_format() {
        // Test backward compatibility with ActionConfig format
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
                            "custom": {
                                "available_actions": {
                                    "actions": {
                                        "RULE_001": [
                                            {"command": "/bundle/actions/action1.sh"}
                                        ]
                                    }
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let actions = GovernanceBundleLoader::parse_actions_from_manifest(&manifest).unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions.get("RULE_001").unwrap()[0].command,
            "/bundle/actions/action1.sh"
        );
    }

    #[test]
    fn test_parse_signals_direct_format() {
        // Test format where available_signals IS the signals map directly
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
                            "custom": {
                                "available_signals": {
                                    "git_branch": "/bundle/signals/git_branch.sh",
                                    "file_count": "/bundle/signals/file_count.sh"
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let signals = GovernanceBundleLoader::parse_signals_from_manifest(&manifest).unwrap();

        assert_eq!(signals.len(), 2);
        assert_eq!(
            signals.get("git_branch").unwrap().command,
            "/bundle/signals/git_branch.sh"
        );
        assert_eq!(
            signals.get("file_count").unwrap().command,
            "/bundle/signals/file_count.sh"
        );
    }

    #[test]
    fn test_parse_signals_governance_service_format() {
        // Test actual governance service format with metadata
        let manifest_json = r#"{
            "revision": "1.2.0",
            "roots": [""],
            "wasm": [
                {
                    "entrypoint": "governance/system/evaluate",
                    "module": "policy.wasm",
                    "annotations": [
                        {
                            "scope": "document",
                            "custom": {
                                "available_signals": {
                                    "signals": {
                                        "collect_data": {
                                            "script": "collect_data.py",
                                            "type": "object",
                                            "cache_ttl": 300,
                                            "description": "Data collection signal"
                                        }
                                    }
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let signals = GovernanceBundleLoader::parse_signals_from_manifest(&manifest).unwrap();

        assert_eq!(signals.len(), 1);
        assert_eq!(
            signals.get("collect_data").unwrap().command,
            "collect_data.py"
        );
        assert_eq!(signals.get("collect_data").unwrap().timeout_seconds, 5);
    }

    #[test]
    fn test_parse_actions_governance_service_format() {
        // Test actual governance service format with metadata
        let manifest_json = r#"{
            "revision": "1.2.0",
            "roots": [""],
            "wasm": [
                {
                    "entrypoint": "governance/system/evaluate",
                    "module": "policy.wasm",
                    "annotations": [
                        {
                            "scope": "document",
                            "custom": {
                                "available_actions": {
                                    "actions": {
                                        "handle_decision": {
                                            "script": "handle_decision.py",
                                            "trigger_on": ["deny", "halt"],
                                            "severity_threshold": "LOW"
                                        }
                                    }
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let actions = GovernanceBundleLoader::parse_actions_from_manifest(&manifest).unwrap();

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions.get("handle_decision").unwrap()[0].command,
            "handle_decision.py"
        );
    }

    #[test]
    fn test_parse_actions_direct_format() {
        // Test format where available_actions IS the actions map directly
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
                            "custom": {
                                "available_actions": {
                                    "RULE_001": ["/bundle/actions/action1.sh"],
                                    "RULE_002": ["/bundle/actions/action2.sh"]
                                }
                            }
                        }
                    ]
                }
            ],
            "rego_version": 1
        }"#;

        let manifest: BundleManifest = serde_json::from_str(manifest_json).unwrap();
        let actions = GovernanceBundleLoader::parse_actions_from_manifest(&manifest).unwrap();

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions.get("RULE_001").unwrap()[0].command,
            "/bundle/actions/action1.sh"
        );
        assert_eq!(
            actions.get("RULE_002").unwrap()[0].command,
            "/bundle/actions/action2.sh"
        );
    }
}
