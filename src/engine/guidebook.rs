//! Guidebook parser - Simple key-value lookup for signals and actions
//! 
//! The guidebook.yml is just a phonebook - no logic, just mappings

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

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
    /// Load guidebook from a YAML file
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        info!("Loading guidebook from: {:?}", path);
        
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read guidebook file")?;
            
        let guidebook: Guidebook = serde_yaml_ng::from_str(&content)
            .context("Failed to parse guidebook YAML")?;
            
        debug!("Loaded {} signals and {} rule-specific actions", 
            guidebook.signals.len(),
            guidebook.actions.by_rule_id.len()
        );
        
        Ok(guidebook)
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
    
    /// Execute a signal and return its output
    pub async fn execute_signal(&self, signal_name: &str) -> Result<String> {
        let signal = self.get_signal(signal_name)
            .with_context(|| format!("Signal '{}' not found in guidebook", signal_name))?;
            
        debug!("Executing signal '{}': {}", signal_name, signal.command);
        
        // Execute with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(signal.timeout_seconds),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&signal.command)
                .output()
        )
        .await
        .context("Signal execution timed out")?
        .context("Failed to execute signal command")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Signal '{}' failed: {}", signal_name, stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }
    
    /// Execute multiple signals concurrently
    pub async fn execute_signals(&self, signal_names: &[String]) -> Result<HashMap<String, String>> {
        use futures::future::join_all;
        
        if signal_names.is_empty() {
            return Ok(HashMap::new());
        }
        
        debug!("Executing {} signals concurrently", signal_names.len());
        
        let futures: Vec<_> = signal_names.iter()
            .map(|name| {
                let name = name.clone();
                async move {
                    let result = self.execute_signal(&name).await;
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