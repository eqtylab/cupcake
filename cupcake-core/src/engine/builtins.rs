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

    /// Global file lock configuration (prevents all file writes)
    #[serde(default, alias = "never_edit_files")]
    pub global_file_lock: Option<GlobalFileLockConfig>,

    /// Git pre-check configuration
    #[serde(default)]
    pub git_pre_check: Option<GitPreCheckConfig>,

    /// Post-edit check configuration
    #[serde(default)]
    pub post_edit_check: Option<PostEditCheckConfig>,

    /// Rulebook security guardrails configuration
    #[serde(default)]
    pub rulebook_security_guardrails: Option<RulebookSecurityConfig>,

    /// Protected paths configuration (user-defined read-only paths)
    #[serde(default)]
    pub protected_paths: Option<ProtectedPathsConfig>,

    /// Git block no-verify configuration (prevents bypassing commit hooks)
    #[serde(default)]
    pub git_block_no_verify: Option<GitBlockNoVerifyConfig>,

    // Global-only builtins (for machine-wide security)
    /// System protection configuration - prevents modification of OS paths
    #[serde(default)]
    pub system_protection: Option<SystemProtectionConfig>,

    /// Sensitive data protection - blocks reading credentials/secrets
    #[serde(default)]
    pub sensitive_data_protection: Option<SensitiveDataProtectionConfig>,

    /// Cupcake execution protection - prevents direct binary execution
    #[serde(default)]
    pub cupcake_exec_protection: Option<CupcakeExecProtectionConfig>,

    /// Enforce full file read - prevents partial reads of small files
    #[serde(default)]
    pub enforce_full_file_read: Option<EnforceFullFileReadConfig>,
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

/// Configuration for global_file_lock builtin (formerly never_edit_files)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFileLockConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Message to show when blocking edits
    #[serde(default = "default_global_file_lock_message")]
    pub message: String,
}

fn default_global_file_lock_message() -> String {
    "File editing is disabled globally by policy".to_string()
}

fn default_enabled() -> bool {
    true // If a builtin is configured, it's enabled by default
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

/// Configuration for rulebook_security_guardrails builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookSecurityConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Message to show when blocking operations
    #[serde(default = "default_rulebook_security_message")]
    pub message: String,

    /// Protected paths (defaults to [".cupcake/"])
    #[serde(default = "default_protected_paths")]
    pub protected_paths: Vec<String>,
}

fn default_rulebook_security_message() -> String {
    "Cupcake configuration files are protected from modification".to_string()
}

fn default_protected_paths() -> Vec<String> {
    vec![".cupcake/".to_string()]
}

/// Configuration for protected_paths builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedPathsConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Message to show when blocking modifications
    #[serde(default = "default_protected_paths_message")]
    pub message: String,

    /// List of paths to protect (supports globs)
    #[serde(default)]
    pub paths: Vec<String>,
}

fn default_protected_paths_message() -> String {
    "This path is read-only and cannot be modified".to_string()
}

/// Configuration for git_block_no_verify builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBlockNoVerifyConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Message to show when blocking operations
    #[serde(default = "default_git_block_no_verify_message")]
    pub message: String,

    /// Allow specific exceptions (e.g., for CI environments)
    #[serde(default)]
    pub exceptions: Vec<String>,
}

fn default_git_block_no_verify_message() -> String {
    "Git operations with --no-verify are not permitted. Commit hooks must run.".to_string()
}

// Global builtin configurations

/// Configuration for system protection builtin (global only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProtectionConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Additional custom paths to protect (beyond the defaults)
    #[serde(default)]
    pub additional_paths: Vec<String>,

    /// Custom message for blocked operations
    #[serde(default = "default_system_protection_message")]
    pub message: String,
}

fn default_system_protection_message() -> String {
    "Access to critical system path blocked".to_string()
}

/// Configuration for sensitive data protection builtin (global only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataProtectionConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Additional file patterns to consider sensitive
    #[serde(default)]
    pub additional_patterns: Vec<String>,

    /// Custom message for blocked operations
    #[serde(default = "default_sensitive_data_message")]
    pub message: String,
}

fn default_sensitive_data_message() -> String {
    "Access to potentially sensitive data blocked".to_string()
}

/// Configuration for cupcake execution protection builtin (global only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CupcakeExecProtectionConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Allow specific cupcake commands (e.g., ["version", "help"])
    #[serde(default)]
    pub allowed_commands: Vec<String>,

    /// Custom message for blocked operations
    #[serde(default = "default_cupcake_exec_message")]
    pub message: String,
}

fn default_cupcake_exec_message() -> String {
    "Direct execution of Cupcake binary is not permitted".to_string()
}

/// Configuration for enforce_full_file_read builtin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforceFullFileReadConfig {
    /// Whether this builtin is enabled (defaults to true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Maximum lines threshold - files under this size must be read in full
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,

    /// Message to show when blocking partial reads
    #[serde(default = "default_enforce_full_read_message")]
    pub message: String,
}

fn default_max_lines() -> usize {
    2000
}

fn default_enforce_full_read_message() -> String {
    "Please read the entire file first (files under 2000 lines must be read completely)".to_string()
}

impl BuiltinsConfig {
    /// Validate configuration and return errors if invalid
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate always_inject_on_prompt
        if let Some(config) = &self.always_inject_on_prompt {
            if config.enabled && config.context.is_empty() {
                errors
                    .push("always_inject_on_prompt: enabled but no context configured".to_string());
            }

            for (idx, source) in config.context.iter().enumerate() {
                if let ContextSource::Dynamic { file, command } = source {
                    if file.is_none() && command.is_none() {
                        errors.push(format!(
                            "always_inject_on_prompt.context[{idx}]: dynamic source must have either 'file' or 'command'"
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
                        "git_pre_check.checks[{idx}]: command cannot be empty"
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
                        "post_edit_check.by_extension.{ext}: command cannot be empty"
                    ));
                }

                // Warn about common extension mistakes
                if ext.contains('.') {
                    errors.push(format!(
                        "post_edit_check.by_extension.{ext}: extension should not include dot (use 'rs' not '.rs')"
                    ));
                }
            }
        }

        // Validate rulebook_security_guardrails
        if let Some(config) = &self.rulebook_security_guardrails {
            if config.enabled && config.protected_paths.is_empty() {
                errors.push(
                    "rulebook_security_guardrails: enabled but no protected paths configured"
                        .to_string(),
                );
            }

            for (idx, path) in config.protected_paths.iter().enumerate() {
                if path.trim().is_empty() {
                    errors.push(format!(
                        "rulebook_security_guardrails.protected_paths[{idx}]: path cannot be empty"
                    ));
                }
            }
        }

        // Validate protected_paths
        if let Some(config) = &self.protected_paths {
            if config.enabled && config.paths.is_empty() {
                errors.push("protected_paths: enabled but no paths configured".to_string());
            }

            for (idx, path) in config.paths.iter().enumerate() {
                if path.trim().is_empty() {
                    errors.push(format!(
                        "protected_paths.paths[{idx}]: path cannot be empty"
                    ));
                }
            }
        }

        // Validate git_block_no_verify (no specific validation needed - it's simple)

        // Validate enforce_full_file_read
        if let Some(config) = &self.enforce_full_file_read {
            if config.enabled && config.max_lines == 0 {
                errors.push("enforce_full_file_read: max_lines cannot be 0".to_string());
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
        self.always_inject_on_prompt
            .as_ref()
            .is_some_and(|c| c.enabled)
            || self.global_file_lock.as_ref().is_some_and(|c| c.enabled)
            || self.git_pre_check.as_ref().is_some_and(|c| c.enabled)
            || self.post_edit_check.as_ref().is_some_and(|c| c.enabled)
            || self
                .rulebook_security_guardrails
                .as_ref()
                .is_some_and(|c| c.enabled)
            || self.protected_paths.as_ref().is_some_and(|c| c.enabled)
            || self
                .git_block_no_verify
                .as_ref()
                .is_some_and(|c| c.enabled)
            || self.system_protection.as_ref().is_some_and(|c| c.enabled)
            || self
                .sensitive_data_protection
                .as_ref()
                .is_some_and(|c| c.enabled)
            || self
                .cupcake_exec_protection
                .as_ref()
                .is_some_and(|c| c.enabled)
    }

    /// Get list of enabled builtin names
    pub fn enabled_builtins(&self) -> Vec<String> {
        let mut enabled = Vec::new();

        if self
            .always_inject_on_prompt
            .as_ref()
            .is_some_and(|c| c.enabled)
        {
            enabled.push("always_inject_on_prompt".to_string());
        }
        if self.global_file_lock.as_ref().is_some_and(|c| c.enabled) {
            enabled.push("global_file_lock".to_string());
        }
        if self.git_pre_check.as_ref().is_some_and(|c| c.enabled) {
            enabled.push("git_pre_check".to_string());
        }
        if self.post_edit_check.as_ref().is_some_and(|c| c.enabled) {
            enabled.push("post_edit_check".to_string());
        }
        if self
            .rulebook_security_guardrails
            .as_ref()
            .is_some_and(|c| c.enabled)
        {
            enabled.push("rulebook_security_guardrails".to_string());
        }
        if self.protected_paths.as_ref().is_some_and(|c| c.enabled) {
            enabled.push("protected_paths".to_string());
        }
        if self
            .git_block_no_verify
            .as_ref()
            .is_some_and(|c| c.enabled)
        {
            enabled.push("git_block_no_verify".to_string());
        }
        if self.system_protection.as_ref().is_some_and(|c| c.enabled) {
            enabled.push("system_protection".to_string());
        }
        if self
            .sensitive_data_protection
            .as_ref()
            .is_some_and(|c| c.enabled)
        {
            enabled.push("sensitive_data_protection".to_string());
        }
        if self
            .cupcake_exec_protection
            .as_ref()
            .is_some_and(|c| c.enabled)
        {
            enabled.push("cupcake_exec_protection".to_string());
        }
        if self
            .enforce_full_file_read
            .as_ref()
            .is_some_and(|c| c.enabled)
        {
            enabled.push("enforce_full_file_read".to_string());
        }

        enabled
    }

    /// Generate signals required by enabled builtins
    pub fn generate_signals(&self) -> HashMap<String, SignalConfig> {
        let mut signals = HashMap::new();

        // Generate signals for always_inject_on_prompt (only for dynamic sources)
        if let Some(config) = &self.always_inject_on_prompt {
            if config.enabled {
                for (idx, source) in config.context.iter().enumerate() {
                    // Only generate signals for dynamic sources (commands and files)
                    // Static strings are now injected directly via builtin_config
                    if matches!(source, ContextSource::Dynamic { .. }) {
                        let signal_name = format!("__builtin_prompt_context_{idx}");

                        if let Some(signal) = context_source_to_signal(source) {
                            signals.insert(signal_name, signal);
                        }
                    }
                }
            }
        }

        // Generate signals for git_pre_check
        if let Some(config) = &self.git_pre_check {
            if config.enabled {
                for (idx, check) in config.checks.iter().enumerate() {
                    let signal_name = format!("__builtin_git_check_{idx}");
                    signals.insert(
                        signal_name,
                        SignalConfig {
                            command: check.command.clone(),
                            timeout_seconds: 30, // Reasonable timeout for tests/linting
                        },
                    );
                }
            }
        }

        // Generate signals for post_edit_check
        if let Some(config) = &self.post_edit_check {
            if config.enabled {
                for (ext, check) in &config.by_extension {
                    let signal_name = format!("__builtin_post_edit_{ext}");
                    signals.insert(
                        signal_name,
                        SignalConfig {
                            command: check.command.clone(),
                            timeout_seconds: 10, // Quick feedback for edit checks
                        },
                    );
                }
            }
        }

        // rulebook_security_guardrails: No signals needed - static config injected directly

        // protected_paths: No signals needed - static config injected directly

        // Generate signals for git_block_no_verify (if needed)
        if let Some(config) = &self.git_block_no_verify {
            if config.enabled {
                // No signals needed for this simple builtin - it just blocks patterns
                // But we could add a signal for the message if desired
            }
        }

        // system_protection: No signals needed - static config injected directly

        // sensitive_data_protection: No signals needed - static config injected directly

        // cupcake_exec_protection: No signals needed - static config injected directly

        // enforce_full_file_read: No signals needed - static config injected directly

        if !signals.is_empty() {
            info!("Generated {} signals for enabled builtins", signals.len());
            for name in signals.keys() {
                debug!("  Generated signal: {}", name);
            }
        }

        signals
    }
}

/// Convert a ContextSource to a SignalConfig (only for dynamic sources)
fn context_source_to_signal(source: &ContextSource) -> Option<SignalConfig> {
    match source {
        ContextSource::String(_) => {
            // Static strings are no longer converted to signals
            // They're injected directly via builtin_config
            None
        }
        ContextSource::Dynamic { file, command } => {
            if let Some(cmd) = command {
                Some(SignalConfig {
                    command: cmd.clone(),
                    timeout_seconds: 5,
                })
            } else { file.as_ref().map(|path| SignalConfig {
                    command: format!("cat '{}'", path.replace('\'', "\\'")),
                    timeout_seconds: 2,
                }) }
        }
    }
}

/// Check if a builtin policy should be loaded
pub fn should_load_builtin_policy(policy_path: &Path, enabled_builtins: &[String]) -> bool {
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

global_file_lock:
  enabled: false
  message: "Read-only mode"
"#;

        let config: BuiltinsConfig = serde_yaml_ng::from_str(yaml).unwrap();

        assert!(config.always_inject_on_prompt.as_ref().unwrap().enabled);
        assert!(!config.global_file_lock.as_ref().unwrap().enabled);
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
        assert!(errors
            .iter()
            .any(|e| e.contains("must have either 'file' or 'command'")));

        // Test extension with dot fails
        let mut by_ext = HashMap::new();
        by_ext.insert(
            ".rs".to_string(),
            CheckConfig {
                command: "cargo check".to_string(),
                message: "Check Rust".to_string(),
            },
        );
        config.post_edit_check = Some(PostEditCheckConfig {
            enabled: true,
            by_extension: by_ext,
        });
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("should not include dot")));

        // Test valid configuration
        let valid_config = BuiltinsConfig {
            git_pre_check: Some(GitPreCheckConfig {
                enabled: true,
                checks: vec![CheckConfig {
                    command: "cargo test".to_string(),
                    message: "Run tests".to_string(),
                }],
            }),
            ..Default::default()
        };
        assert!(valid_config.validate().is_ok());
    }

    #[test]
    fn test_signal_generation() {
        let mut config = BuiltinsConfig {
            global_file_lock: Some(GlobalFileLockConfig {
                enabled: true,
                message: "No edits".to_string(),
            }),
            ..Default::default()
        };

        // global_file_lock doesn't generate signals
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
