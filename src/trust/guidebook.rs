//! Guidebook parsing for the trust system
//!
//! Parses .cupcake/guidebook.yml to extract signal and action script references
//! for trust manifest creation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A single signal configuration from guidebook.yml
#[derive(Debug, Deserialize, Serialize)]
pub struct SignalConfig {
    /// The command to execute for this signal
    pub command: String,
    
    /// Optional timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    
    /// Optional working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
}

/// A single action configuration from guidebook.yml
#[derive(Debug, Deserialize, Serialize)]
pub struct ActionConfig {
    /// The command to execute for this action
    pub command: String,
    
    /// Optional timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    
    /// Optional working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
}

/// The complete guidebook structure
#[derive(Debug, Deserialize, Serialize)]
pub struct Guidebook {
    /// Signal configurations
    #[serde(default)]
    pub signals: HashMap<String, SignalConfig>,
    
    /// Action configurations
    #[serde(default)]
    pub actions: HashMap<String, ActionConfig>,
}

impl Guidebook {
    /// Load a guidebook from the standard location
    pub fn load(project_dir: &Path) -> Result<Self> {
        let guidebook_path = project_dir.join(".cupcake/guidebook.yml");
        Self::load_from(&guidebook_path)
    }
    
    /// Load a guidebook from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            // Return empty guidebook if file doesn't exist
            return Ok(Guidebook {
                signals: HashMap::new(),
                actions: HashMap::new(),
            });
        }
        
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read guidebook: {}", path.display()))?;
        
        serde_yaml_ng::from_str(&content)
            .with_context(|| format!("Failed to parse guidebook YAML: {}", path.display()))
    }
    
    /// Get all script commands from signals and actions
    pub fn get_all_scripts(&self) -> Vec<(String, String, String)> {
        let mut scripts = Vec::new();
        
        // Add all signals
        for (name, signal) in &self.signals {
            scripts.push(("signals".to_string(), name.clone(), signal.command.clone()));
        }
        
        // Add all actions  
        for (name, action) in &self.actions {
            scripts.push(("actions".to_string(), name.clone(), action.command.clone()));
        }
        
        scripts
    }
    
    /// Get the working directory for script execution
    pub fn get_working_dir(&self, project_dir: &Path) -> PathBuf {
        project_dir.to_path_buf()
    }
}