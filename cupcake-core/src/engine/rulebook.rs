//! Rulebook parser - Simple key-value lookup for signals and actions
//!
//! The rulebook.yml is just a phonebook - no logic, just mappings

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use super::builtins::BuiltinsConfig;
use crate::watchdog::{WatchdogConfig, WatchdogConfigInput};

/// Telemetry output format
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryFormat {
    #[default]
    Json,
    Text,
}

/// Telemetry configuration for SOC/SIEM integration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetryConfig {
    /// Whether telemetry export is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Output format (json or text)
    #[serde(default)]
    pub format: TelemetryFormat,

    /// Output directory (defaults to .cupcake/telemetry)
    pub destination: Option<PathBuf>,
}

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

/// The simplified rulebook structure from CRITICAL_GUIDING_STAR.md
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Rulebook {
    /// Signal name -> command mappings
    #[serde(default)]
    pub signals: HashMap<String, SignalConfig>,

    /// Action configurations
    #[serde(default)]
    pub actions: ActionSection,

    /// Builtin abstractions configuration
    #[serde(default)]
    pub builtins: BuiltinsConfig,

    /// Watchdog LLM-as-judge configuration (top-level, not under builtins)
    ///
    /// Supports both shorthand (`watchdog: true`) and full config:
    /// ```yaml
    /// # Shorthand - enables with defaults
    /// watchdog: true
    ///
    /// # Full config
    /// watchdog:
    ///   enabled: true
    ///   backend: openrouter
    ///   # ...
    /// ```
    #[serde(default, deserialize_with = "deserialize_watchdog_config")]
    pub watchdog: WatchdogConfig,

    /// Telemetry configuration for SOC/SIEM integration
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

/// Custom deserializer for watchdog config that handles both `true` and full object
fn deserialize_watchdog_config<'de, D>(deserializer: D) -> Result<WatchdogConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let input = WatchdogConfigInput::deserialize(deserializer)?;
    Ok(input.into())
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

impl Rulebook {
    /// Load rulebook from a YAML file, enhanced with convention-based discovery
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        info!("Loading rulebook from: {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read rulebook file")?;

        let rulebook: Rulebook =
            serde_yaml_ng::from_str(&content).context("Failed to parse rulebook YAML")?;

        debug!(
            "Loaded {} explicit signals and {} rule-specific actions",
            rulebook.signals.len(),
            rulebook.actions.by_rule_id.len()
        );

        Ok(rulebook)
    }

    /// Create a rulebook with convention-based signal and action discovery
    /// Discovers scripts in signals/ and actions/ directories automatically
    pub async fn load_with_conventions(
        rulebook_path: impl AsRef<Path>,
        signals_dir: impl AsRef<Path>,
        actions_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let mut rulebook = if rulebook_path.as_ref().exists() {
            Self::load(rulebook_path).await?
        } else {
            info!("No rulebook.yml found, using pure convention-based approach");
            Self::default()
        };

        // Discover signals from directory (if exists)
        if signals_dir.as_ref().exists() {
            Self::discover_signals(&mut rulebook, signals_dir).await?;
        }

        // Discover actions from directory (if exists)
        if actions_dir.as_ref().exists() {
            Self::discover_actions(&mut rulebook, actions_dir).await?;
        }

        // Generate signals for enabled builtins
        if rulebook.builtins.any_enabled() {
            info!(
                "Generating signals for enabled builtins: {:?}",
                rulebook.builtins.enabled_builtins()
            );

            let builtin_signals = rulebook.builtins.generate_signals();

            // Merge builtin-generated signals (don't override user-defined)
            for (name, signal) in builtin_signals {
                use std::collections::hash_map::Entry;
                match rulebook.signals.entry(name) {
                    Entry::Vacant(e) => {
                        debug!("Adding builtin-generated signal: {}", e.key());
                        e.insert(signal);
                    }
                    Entry::Occupied(e) => {
                        debug!(
                            "Keeping user-defined signal: {} (skipping builtin)",
                            e.key()
                        );
                    }
                }
            }
        }

        info!(
            "Final rulebook: {} signals, {} action rules, {} enabled builtins, watchdog={}",
            rulebook.signals.len(),
            rulebook.actions.by_rule_id.len(),
            rulebook.builtins.enabled_builtins().len(),
            if rulebook.watchdog.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );

        // Debug: show loaded actions
        for (rule_id, actions) in &rulebook.actions.by_rule_id {
            debug!("Rule {}: {} actions", rule_id, actions.len());
        }

        // Validate builtin configuration
        if let Err(errors) = rulebook.builtins.validate() {
            use anyhow::bail;
            bail!("Builtin configuration errors:\n{}", errors.join("\n"));
        }

        Ok(rulebook)
    }

    /// Discover signal scripts from a directory
    async fn discover_signals(
        rulebook: &mut Rulebook,
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

                // Don't override explicit rulebook signals
                if !rulebook.signals.contains_key(signal_name) {
                    let signal_config = SignalConfig {
                        command: path.to_string_lossy().to_string(),
                        timeout_seconds: default_timeout(),
                    };

                    rulebook
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
        rulebook: &mut Rulebook,
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

                rulebook
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
            .with_context(|| format!("Signal '{signal_name}' not found in rulebook"))?;

        debug!("Executing signal '{}': {}", signal_name, signal.command);

        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        // Use platform-appropriate shell (bash on Windows, sh on Unix)
        let shell = *super::SHELL_COMMAND;

        // On Windows, convert .sh file paths to Git Bash format
        // C:\Users\foo\script.sh -> /c/Users/foo/script.sh
        #[cfg(windows)]
        let command_arg = if signal.command.ends_with(".sh")
            && signal.command.len() >= 3
            && signal.command.chars().nth(1) == Some(':')
        {
            let drive = signal.command.chars().next().unwrap().to_lowercase();
            let path_part = &signal.command[2..].replace('\\', "/");
            format!("/{}{}", drive, path_part)
        } else {
            signal.command.clone()
        };

        #[cfg(not(windows))]
        let command_arg = &signal.command;

        // Spawn the command with stdin piped
        let mut child = Command::new(shell)
            .arg("-c")
            .arg(command_arg)
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

// Example rulebook.yml structure from CRITICAL_GUIDING_STAR.md:
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
// - Rulebook is just a phonebook - simple lookup tables
// - No complex orchestration logic
// - Signals map names to commands
// - Actions map violation IDs to commands
// - Concurrent signal execution for performance
