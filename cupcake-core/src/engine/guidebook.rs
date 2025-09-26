//! Guidebook parser - Simple key-value lookup for signals and actions
//!
//! The guidebook.yml is just a phonebook - no logic, just mappings

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

use super::builtins::BuiltinsConfig;

/// Signal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Command to execute for this signal
    pub command: String,

    /// Timeout in seconds (optional, default 5)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_timeout() -> u64 {
    5
}

/// Action configuration  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    /// Command to execute for this action
    pub command: String,
}

/// The simplified guidebook structure from CRITICAL_GUIDING_STAR.md
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Guidebook {
    /// Signal name -> command mappings
    #[serde(default)]
    pub signals: HashMap<String, SignalConfig>,

    /// Action configurations
    #[serde(default)]
    pub actions: ActionSection,

    /// Builtin abstractions configuration
    #[serde(default)]
    pub builtins: BuiltinsConfig,
}

/// Action section with both general and ID-specific actions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionSection {
    /// Actions that run on any denial
    #[serde(default)]
    pub on_any_denial: Vec<ActionConfig>,

    /// Violation ID -> action mappings
    #[serde(default)]
    pub by_rule_id: HashMap<String, Vec<ActionConfig>>,
}

impl Guidebook {
    /// Load guidebook from a YAML file, enhanced with convention-based discovery
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        info!("Loading guidebook from: {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read guidebook file")?;

        let guidebook: Guidebook =
            serde_yaml_ng::from_str(&content).context("Failed to parse guidebook YAML")?;

        debug!(
            "Loaded {} explicit signals and {} rule-specific actions",
            guidebook.signals.len(),
            guidebook.actions.by_rule_id.len()
        );

        Ok(guidebook)
    }

    /// Create a guidebook with convention-based signal and action discovery
    /// Discovers scripts in signals/ and actions/ directories automatically
    pub async fn load_with_conventions(
        guidebook_path: impl AsRef<Path>,
        signals_dir: impl AsRef<Path>,
        actions_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let mut guidebook = if guidebook_path.as_ref().exists() {
            Self::load(guidebook_path).await?
        } else {
            info!("No guidebook.yml found, using pure convention-based approach");
            Self::default()
        };

        // Discover signals from directory (if exists)
        if signals_dir.as_ref().exists() {
            Self::discover_signals(&mut guidebook, signals_dir).await?;
        }

        // Discover actions from directory (if exists)
        if actions_dir.as_ref().exists() {
            Self::discover_actions(&mut guidebook, actions_dir).await?;
        }

        // Generate signals for enabled builtins
        if guidebook.builtins.any_enabled() {
            info!(
                "Generating signals for enabled builtins: {:?}",
                guidebook.builtins.enabled_builtins()
            );

            let builtin_signals = guidebook.builtins.generate_signals();

            // Merge builtin-generated signals (don't override user-defined)
            for (name, signal) in builtin_signals {
                use std::collections::hash_map::Entry;
                match guidebook.signals.entry(name) {
                    Entry::Vacant(e) => {
                        debug!("Adding builtin-generated signal: {}", e.key());
                        e.insert(signal);
                    }
                    Entry::Occupied(e) => {
                        debug!("Keeping user-defined signal: {} (skipping builtin)", e.key());
                    }
                }
            }
        }

        info!(
            "Final guidebook: {} signals, {} action rules, {} enabled builtins",
            guidebook.signals.len(),
            guidebook.actions.by_rule_id.len(),
            guidebook.builtins.enabled_builtins().len()
        );

        // Debug: show loaded actions
        for (rule_id, actions) in &guidebook.actions.by_rule_id {
            debug!("Rule {}: {} actions", rule_id, actions.len());
        }

        // Validate builtin configuration
        if let Err(errors) = guidebook.builtins.validate() {
            use anyhow::bail;
            bail!("Builtin configuration errors:\n{}", errors.join("\n"));
        }

        Ok(guidebook)
    }

    /// Discover signal scripts from a directory
    async fn discover_signals(
        guidebook: &mut Guidebook,
        signals_dir: impl AsRef<Path>,
    ) -> Result<()> {
        let signals_dir = signals_dir.as_ref();
        debug!("Discovering signals in: {:?}", signals_dir);

        let mut entries = tokio::fs::read_dir(signals_dir)
            .await
            .context("Failed to read signals directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden files and non-executable extensions
                if file_name.starts_with('.') {
                    continue;
                }

                // Extract signal name (filename without extension)
                let signal_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_name);

                // Don't override explicit guidebook signals
                if !guidebook.signals.contains_key(signal_name) {
                    let signal_config = SignalConfig {
                        command: path.to_string_lossy().to_string(),
                        timeout_seconds: default_timeout(),
                    };

                    guidebook
                        .signals
                        .insert(signal_name.to_string(), signal_config);
                    debug!("Discovered signal: {} -> {}", signal_name, path.display());
                }
            }
        }

        Ok(())
    }

    /// Discover action scripts from a directory  
    async fn discover_actions(
        guidebook: &mut Guidebook,
        actions_dir: impl AsRef<Path>,
    ) -> Result<()> {
        let actions_dir = actions_dir.as_ref();
        debug!("Discovering actions in: {:?}", actions_dir);

        let mut entries = tokio::fs::read_dir(actions_dir)
            .await
            .context("Failed to read actions directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden files
                if file_name.starts_with('.') {
                    continue;
                }

                // Extract action name (filename without extension)
                let action_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_name);

                // Add as rule-specific action (convention: action name = rule ID)
                let action_config = ActionConfig {
                    command: path.to_string_lossy().to_string(),
                };

                guidebook
                    .actions
                    .by_rule_id
                    .entry(action_name.to_string())
                    .or_default()
                    .push(action_config);

                info!("Discovered action: {} -> {}", action_name, path.display());
            }
        }

        Ok(())
    }

    /// Get signal command by name
    pub fn get_signal(&self, name: &str) -> Option<&SignalConfig> {
        self.signals.get(name)
    }

    /// Get actions for a specific violation ID
    pub fn get_actions_for_violation(&self, violation_id: &str) -> Vec<&ActionConfig> {
        let mut actions = Vec::new();

        // Add any "on_any_denial" actions
        for action in &self.actions.on_any_denial {
            actions.push(action);
        }

        // Add specific actions for this violation ID
        if let Some(specific_actions) = self.actions.by_rule_id.get(violation_id) {
            for action in specific_actions {
                actions.push(action);
            }
        }

        actions
    }

    /// Execute a signal and return its output as JSON Value (no event data)
    pub async fn execute_signal(&self, signal_name: &str) -> Result<serde_json::Value> {
        // Call the version with input, passing empty object for backward compatibility
        self.execute_signal_with_input(signal_name, &serde_json::json!({}))
            .await
    }

    /// Execute a signal with event data passed via stdin
    pub async fn execute_signal_with_input(
        &self,
        signal_name: &str,
        event_data: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let signal = self
            .get_signal(signal_name)
            .with_context(|| format!("Signal '{signal_name}' not found in guidebook"))?;

        debug!("Executing signal '{}': {}", signal_name, signal.command);

        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        // Spawn the command with stdin piped
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&signal.command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn signal command")?;

        // Always write event data to stdin (signals that don't need it will ignore it)
        if let Some(mut stdin) = child.stdin.take() {
            let event_json = serde_json::to_string(event_data)?;
            debug!(
                "Writing {} bytes of event data to signal stdin",
                event_json.len()
            );
            let _ = stdin.write_all(event_json.as_bytes()).await; // Ignore write errors - signal may not read stdin
            let _ = stdin.flush().await;
            // Drop stdin to close it
        }

        // Wait for the command with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(signal.timeout_seconds),
            child.wait_with_output(),
        )
        .await
        .context("Signal execution timed out")?
        .context("Failed to wait for signal output")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed_output = stdout.trim();

        // Always include exit code information for validation signals
        // This allows policies to check if validation failed
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            debug!(
                "Signal '{}' failed with exit code {}",
                signal_name, exit_code
            );

            // Return structured error info for failed commands
            return Ok(serde_json::json!({
                "exit_code": exit_code,
                "output": trimmed_output,
                "error": stderr.trim(),
                "success": false
            }));
        }

        // Try to parse as JSON first, fall back to string if it fails
        match serde_json::from_str::<serde_json::Value>(trimmed_output) {
            Ok(json_value) => {
                debug!(
                    "Signal '{}' output parsed as JSON successfully",
                    signal_name
                );
                Ok(json_value)
            }
            Err(_) => {
                debug!(
                    "Signal '{}' output is not valid JSON, storing as string",
                    signal_name
                );
                Ok(serde_json::Value::String(trimmed_output.to_string()))
            }
        }
    }

    /// Execute multiple signals concurrently (without event data - for backward compatibility)
    pub async fn execute_signals(
        &self,
        signal_names: &[String],
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execute_signals_with_input(signal_names, &serde_json::json!({}))
            .await
    }

    /// Execute multiple signals concurrently with event data
    pub async fn execute_signals_with_input(
        &self,
        signal_names: &[String],
        event_data: &serde_json::Value,
    ) -> Result<HashMap<String, serde_json::Value>> {
        use futures::future::join_all;

        if signal_names.is_empty() {
            return Ok(HashMap::new());
        }

        debug!(
            "Executing {} signals concurrently with event data",
            signal_names.len()
        );

        let futures: Vec<_> = signal_names
            .iter()
            .map(|name| {
                let name = name.clone();
                let event_data = event_data.clone();
                async move {
                    let result = self.execute_signal_with_input(&name, &event_data).await;
                    (name, result)
                }
            })
            .collect();

        let results = join_all(futures).await;

        let mut signal_data = HashMap::new();
        for (name, result) in results {
            match result {
                Ok(value) => {
                    debug!("Signal '{}' returned: {}", name, value);
                    signal_data.insert(name, value);
                }
                Err(e) => {
                    // Log error but don't fail the whole evaluation
                    tracing::error!("Signal '{}' failed: {}", name, e);
                }
            }
        }

        Ok(signal_data)
    }
}

// Example guidebook.yml structure from CRITICAL_GUIDING_STAR.md:
// ```yaml
// signals:
//   git.current_branch:
//     command: "git rev-parse --abbrev-ref HEAD"
//     timeout_seconds: 2
//
// actions:
//   on_any_denial:
//     - command: "logger 'Cupcake policy violation occurred.'"
//
//   by_rule_id:
//     BASH001:
//       - command: "notify-slack --channel dev-guidance --message 'grep usage detected'"
// ```

// Aligns with CRITICAL_GUIDING_STAR.md:
// - Guidebook is just a phonebook - simple lookup tables
// - No complex orchestration logic
// - Signals map names to commands
// - Actions map violation IDs to commands
// - Concurrent signal execution for performance
