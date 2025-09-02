//! Builtin abstractions - Higher-level policy patterns
//! 
//! Provides configuration structures and activation logic for the 5 builtin
//! abstractions that simplify common security patterns without writing Rego.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

use super::guidebook::SignalConfig;

/// Configuration for all builtin abstractions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuiltinsConfig {
    /// Always inject on prompt configuration
    #[serde(default)]
    pub always_inject_on_prompt: Option<AlwaysInjectConfig>,
    
    /// Never edit files configuration
    #[serde(default)]
    pub never_edit_files: Option<NeverEditConfig>,
    
    /// Git pre-check configuration
    #[serde(default)]
    pub git_pre_check: Option<GitPreCheckConfig>,
    
    /// Post-edit check configuration
    #[serde(default)]
    pub post_edit_check: Option<PostEditCheckConfig>,
}


/// Configuration for always_inject_on_prompt builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlwaysInjectConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Context sources to inject
    #[serde(default)]
    pub context: Vec<ContextSource>,
}

/// Configuration for never_edit_files builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeverEditConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Message to show when blocking edits
    #[serde(default = "default_never_edit_message")]
    pub message: String,
}

fn default_never_edit_message() -> String {
    "File editing is disabled by policy".to_string()
}

fn default_enabled() -> bool {
    true  // If a builtin is configured, it's enabled by default
}

/// Configuration for git_pre_check builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPreCheckConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Checks to run before git operations
    #[serde(default)]
    pub checks: Vec<CheckConfig>,
}

/// Configuration for post_edit_check builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostEditCheckConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Checks by file extension
    #[serde(default)]
    pub by_extension: HashMap<String, CheckConfig>,
}

/// A check command with message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Command to execute
    pub command: String,
    
    /// Message to display if check fails
    pub message: String,
}

/// Source of context data (string, file, or command)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextSource {
    /// Static string context
    String(String),
    
    /// Context from file or command
    Dynamic {
        /// File path to read
        #[serde(default)]
        file: Option<String>,
        
        /// Command to execute
        #[serde(default)]
        command: Option<String>,
    },
}

impl BuiltinsConfig {
    /// Validate configuration and return errors if invalid
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Validate always_inject_on_prompt
        if let Some(config) = &self.always_inject_on_prompt {
            if config.enabled && config.context.is_empty() {
                errors.push("always_inject_on_prompt: enabled but no context configured".to_string());
            }
            
            for (idx, source) in config.context.iter().enumerate() {
                if let ContextSource::Dynamic { file, command } = source {
                    if file.is_none() && command.is_none() {
                        errors.push(format!(
                            "always_inject_on_prompt.context[{}]: dynamic source must have either 'file' or 'command'",
                            idx
                        ));
                    }
                }
            }
        }
        
        // Validate git_pre_check
        if let Some(config) = &self.git_pre_check {
            if config.enabled && config.checks.is_empty() {
                errors.push("git_pre_check: enabled but no checks configured".to_string());
            }
            
            for (idx, check) in config.checks.iter().enumerate() {
                if check.command.trim().is_empty() {
                    errors.push(format!(
                        "git_pre_check.checks[{}]: command cannot be empty",
                        idx
                    ));
                }
            }
        }
        
        // Validate post_edit_check
        if let Some(config) = &self.post_edit_check {
            if config.enabled && config.by_extension.is_empty() {
                errors.push("post_edit_check: enabled but no extensions configured".to_string());
            }
            
            for (ext, check) in &config.by_extension {
                if check.command.trim().is_empty() {
                    errors.push(format!(
                        "post_edit_check.by_extension.{}: command cannot be empty",
                        ext
                    ));
                }
                
                // Warn about common extension mistakes
                if ext.contains('.') {
                    errors.push(format!(
                        "post_edit_check.by_extension.{}: extension should not include dot (use 'rs' not '.rs')",
                        ext
                    ));
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Check if any builtin is enabled
    pub fn any_enabled(&self) -> bool {
        self.always_inject_on_prompt.as_ref().map_or(false, |c| c.enabled)
            || self.never_edit_files.as_ref().map_or(false, |c| c.enabled)
            || self.git_pre_check.as_ref().map_or(false, |c| c.enabled)
            || self.post_edit_check.as_ref().map_or(false, |c| c.enabled)
    }
    
    /// Get list of enabled builtin names
    pub fn enabled_builtins(&self) -> Vec<String> {
        let mut enabled = Vec::new();
        
        if self.always_inject_on_prompt.as_ref().map_or(false, |c| c.enabled) {
            enabled.push("always_inject_on_prompt".to_string());
        }
        if self.never_edit_files.as_ref().map_or(false, |c| c.enabled) {
            enabled.push("never_edit_files".to_string());
        }
        if self.git_pre_check.as_ref().map_or(false, |c| c.enabled) {
            enabled.push("git_pre_check".to_string());
        }
        if self.post_edit_check.as_ref().map_or(false, |c| c.enabled) {
            enabled.push("post_edit_check".to_string());
        }
        
        enabled
    }
    
    /// Generate signals required by enabled builtins
    pub fn generate_signals(&self) -> HashMap<String, SignalConfig> {
        let mut signals = HashMap::new();
        
        // Generate signals for always_inject_on_prompt
        if let Some(config) = &self.always_inject_on_prompt {
            if config.enabled {
                for (idx, source) in config.context.iter().enumerate() {
                    let signal_name = format!("__builtin_prompt_context_{}", idx);
                    
                    if let Some(signal) = context_source_to_signal(source) {
                        signals.insert(signal_name, signal);
                    }
                }
            }
        }
        
        // Generate signals for git_pre_check
        if let Some(config) = &self.git_pre_check {
            if config.enabled {
                for (idx, check) in config.checks.iter().enumerate() {
                    let signal_name = format!("__builtin_git_check_{}", idx);
                    signals.insert(signal_name, SignalConfig {
                        command: check.command.clone(),
                        timeout_seconds: 30, // Reasonable timeout for tests/linting
                    });
                }
            }
        }
        
        // Generate signals for post_edit_check
        if let Some(config) = &self.post_edit_check {
            if config.enabled {
                for (ext, check) in &config.by_extension {
                    let signal_name = format!("__builtin_post_edit_{}", ext);
                    signals.insert(signal_name, SignalConfig {
                        command: check.command.clone(),
                        timeout_seconds: 10, // Quick feedback for edit checks
                    });
                }
            }
        }
        
        if !signals.is_empty() {
            info!("Generated {} signals for enabled builtins", signals.len());
            for name in signals.keys() {
                debug!("  Generated signal: {}", name);
            }
        }
        
        signals
    }
}

/// Convert a ContextSource to a SignalConfig
fn context_source_to_signal(source: &ContextSource) -> Option<SignalConfig> {
    match source {
        ContextSource::String(s) => {
            // Static strings become echo commands that output JSON strings
            Some(SignalConfig {
                command: format!("echo '\"{}\"'", s.replace('\'', "\\'")),
                timeout_seconds: 1,
            })
        }
        ContextSource::Dynamic { file, command } => {
            if let Some(cmd) = command {
                Some(SignalConfig {
                    command: cmd.clone(),
                    timeout_seconds: 5,
                })
            } else if let Some(path) = file {
                // File reads become cat commands
                Some(SignalConfig {
                    command: format!("cat '{}'", path.replace('\'', "\\'")),
                    timeout_seconds: 2,
                })
            } else {
                None
            }
        }
    }
}

/// Check if a builtin policy should be loaded
pub fn should_load_builtin_policy(
    policy_path: &Path,
    enabled_builtins: &[String],
) -> bool {
    // Check if this is a builtin policy
    if let Some(parent) = policy_path.parent() {
        if parent.file_name() == Some(std::ffi::OsStr::new("builtins")) {
            // Extract policy name from filename
            if let Some(stem) = policy_path.file_stem() {
                let policy_name = stem.to_string_lossy();
                let should_load = enabled_builtins.contains(&policy_name.to_string());
                
                if should_load {
                    debug!("Loading enabled builtin policy: {}", policy_name);
                } else {
                    debug!("Skipping disabled builtin policy: {}", policy_name);
                }
                
                return should_load;
            }
        }
    }
    
    // Not a builtin policy, always load
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_builtins_config_parsing() {
        // Test with explicit enabled values
        let yaml = r#"
always_inject_on_prompt:
  enabled: true
  context:
    - "Test context"

never_edit_files:
  enabled: false
  message: "Read-only mode"
"#;
        
        let config: BuiltinsConfig = serde_yaml_ng::from_str(yaml).unwrap();
        
        assert!(config.always_inject_on_prompt.as_ref().unwrap().enabled);
        assert!(!config.never_edit_files.as_ref().unwrap().enabled);
        assert_eq!(config.enabled_builtins(), vec!["always_inject_on_prompt"]);
    }
    
    #[test]
    fn test_default_enabled() {
        // Test that builtins default to enabled when field is omitted
        let yaml = r#"
always_inject_on_prompt:
  context:
    - "Test context"

git_pre_check:
  checks:
    - command: "cargo test"
      message: "Tests must pass"
"#;
        
        let config: BuiltinsConfig = serde_yaml_ng::from_str(yaml).unwrap();
        
        // Both should default to enabled=true
        assert!(config.always_inject_on_prompt.as_ref().unwrap().enabled);
        assert!(config.git_pre_check.as_ref().unwrap().enabled);
        
        let enabled = config.enabled_builtins();
        assert!(enabled.contains(&"always_inject_on_prompt".to_string()));
        assert!(enabled.contains(&"git_pre_check".to_string()));
    }
    
    #[test]
    fn test_validation() {
        // Test valid configuration passes
        let mut config = BuiltinsConfig::default();
        assert!(config.validate().is_ok());
        
        // Test empty enabled builtin fails
        config.always_inject_on_prompt = Some(AlwaysInjectConfig {
            enabled: true,
            context: vec![],
        });
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].contains("no context configured"));
        
        // Test invalid dynamic source
        config.always_inject_on_prompt = Some(AlwaysInjectConfig {
            enabled: true,
            context: vec![ContextSource::Dynamic {
                file: None,
                command: None,
            }],
        });
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("must have either 'file' or 'command'")));
        
        // Test extension with dot fails
        let mut by_ext = HashMap::new();
        by_ext.insert(".rs".to_string(), CheckConfig {
            command: "cargo check".to_string(),
            message: "Check Rust".to_string(),
        });
        config.post_edit_check = Some(PostEditCheckConfig {
            enabled: true,
            by_extension: by_ext,
        });
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("should not include dot")));
        
        // Test valid configuration
        let mut valid_config = BuiltinsConfig::default();
        valid_config.git_pre_check = Some(GitPreCheckConfig {
            enabled: true,
            checks: vec![CheckConfig {
                command: "cargo test".to_string(),
                message: "Run tests".to_string(),
            }],
        });
        assert!(valid_config.validate().is_ok());
    }
    
    #[test]
    fn test_signal_generation() {
        let mut config = BuiltinsConfig::default();
        config.never_edit_files = Some(NeverEditConfig {
            enabled: true,
            message: "No edits".to_string(),
        });
        
        // never_edit_files doesn't generate signals
        let signals = config.generate_signals();
        assert_eq!(signals.len(), 0);
        
        // Add git_pre_check
        config.git_pre_check = Some(GitPreCheckConfig {
            enabled: true,
            checks: vec![CheckConfig {
                command: "cargo test".to_string(),
                message: "Tests must pass".to_string(),
            }],
        });
        
        let signals = config.generate_signals();
        assert_eq!(signals.len(), 1);
        assert!(signals.contains_key("__builtin_git_check_0"));
    }
    
}