//! Watchdog configuration types
//!
//! Configuration for the Watchdog LLM-as-judge feature.
//!
//! ## Configuration Sources (in precedence order)
//!
//! 1. `.cupcake/watchdog/config.json` - Project-level config
//! 2. `~/.config/cupcake/watchdog/config.json` - Global config
//! 3. Built-in defaults
//!
//! ## Directory Structure
//!
//! ```text
//! .cupcake/watchdog/
//! ├── config.json   # Backend, model, timeout settings
//! ├── system.txt    # Custom system prompt (optional)
//! └── user.txt      # User message template with {{event}} (optional)
//! ```
//!
//! ## Rulebook Syntax
//!
//! In rulebook.yml, use `watchdog: true` or `watchdog: false`:
//!
//! ```yaml
//! watchdog: true   # Enable with directory-based config
//! watchdog: false  # Disable
//! ```

use serde::{Deserialize, Deserializer, Serialize};
use std::path::Path;

/// Wrapper for deserializing watchdog config from either `true` or full object
///
/// Supports both:
/// - `watchdog: true` - shorthand for enabled with defaults
/// - `watchdog: { enabled: true, ... }` - full configuration
#[derive(Debug, Clone)]
pub enum WatchdogConfigInput {
    /// Shorthand: `watchdog: true` or `watchdog: false`
    Enabled(bool),
    /// Full config object
    Full(WatchdogConfig),
}

impl<'de> Deserialize<'de> for WatchdogConfigInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct WatchdogConfigInputVisitor;

        impl<'de> Visitor<'de> for WatchdogConfigInputVisitor {
            type Value = WatchdogConfigInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean or watchdog configuration object")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(WatchdogConfigInput::Enabled(value))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let config =
                    WatchdogConfig::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(WatchdogConfigInput::Full(config))
            }
        }

        deserializer.deserialize_any(WatchdogConfigInputVisitor)
    }
}

impl From<WatchdogConfigInput> for WatchdogConfig {
    fn from(input: WatchdogConfigInput) -> Self {
        match input {
            WatchdogConfigInput::Enabled(enabled) => {
                if enabled {
                    // `watchdog: true` -> enabled with defaults
                    WatchdogConfig {
                        enabled: true,
                        openrouter: Some(OpenRouterConfig::default()),
                        ..Default::default()
                    }
                } else {
                    // `watchdog: false` -> disabled
                    WatchdogConfig::default()
                }
            }
            WatchdogConfigInput::Full(config) => config,
        }
    }
}

impl Default for WatchdogConfigInput {
    fn default() -> Self {
        WatchdogConfigInput::Enabled(false)
    }
}

/// Top-level Watchdog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    /// Whether Watchdog is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Which backend to use ("openrouter" for now, "claude-code" in future)
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Timeout for LLM calls in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Behavior on error: "allow" (fail-open) or "deny" (fail-closed)
    #[serde(default = "default_on_error")]
    pub on_error: String,

    /// Dry run mode - logs resolved config but skips actual LLM calls
    /// Useful for testing configuration without API costs
    #[serde(default)]
    pub dry_run: bool,

    /// OpenRouter-specific configuration
    #[serde(default)]
    pub openrouter: Option<OpenRouterConfig>,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: default_backend(),
            timeout_seconds: default_timeout(),
            on_error: default_on_error(),
            dry_run: false,
            openrouter: None,
        }
    }
}

fn default_backend() -> String {
    "openrouter".to_string()
}

fn default_timeout() -> u64 {
    10
}

fn default_on_error() -> String {
    "allow".to_string()
}

/// OpenRouter backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    /// OpenRouter model ID (e.g., "google/gemini-2.5-flash")
    #[serde(default = "default_model")]
    pub model: String,

    /// Environment variable name containing the API key
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Timeout in seconds (inherited from parent if not set)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Custom system prompt (uses default if not set)
    #[serde(default)]
    pub system_prompt: Option<String>,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            api_key_env: default_api_key_env(),
            timeout_seconds: default_timeout(),
            system_prompt: None,
        }
    }
}

fn default_model() -> String {
    "google/gemini-2.5-flash".to_string()
}

fn default_api_key_env() -> String {
    "OPENROUTER_API_KEY".to_string()
}

fn default_rules_context_root_path() -> String {
    "../..".to_string()
}

fn default_strict() -> bool {
    true
}

/// Configuration for injecting rules context into prompts
///
/// Allows loading files (like CLAUDE.md, .cursorrules) to provide
/// context to the Watchdog LLM about project-specific rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RulesContext {
    /// Root path for resolving file paths, relative to config.json location
    /// Default: "../.." (project root, assuming config is in .cupcake/watchdog/)
    #[serde(default = "default_rules_context_root_path")]
    pub root_path: String,

    /// List of files to load, relative to root_path
    #[serde(default)]
    pub files: Vec<String>,

    /// If true (default), fail initialization when any rules file cannot be loaded.
    /// If false, log a warning and continue with available files.
    #[serde(default = "default_strict")]
    pub strict: bool,
}

impl WatchdogConfig {
    /// Check if watchdog is properly configured and can be used
    pub fn is_usable(&self) -> bool {
        if !self.enabled {
            return false;
        }

        // When using directory-based config, openrouter config is auto-created
        matches!(self.backend.as_str(), "openrouter")
    }

    /// Get the effective OpenRouter config, applying defaults
    pub fn effective_openrouter_config(&self) -> OpenRouterConfig {
        let mut config = self.openrouter.clone().unwrap_or_default();
        // Inherit timeout from parent if not explicitly set
        if config.timeout_seconds == default_timeout() {
            config.timeout_seconds = self.timeout_seconds;
        }
        config
    }

    /// Check if errors should allow actions to proceed (fail-open behavior)
    pub fn allows_on_error(&self) -> bool {
        self.on_error == "allow"
    }

    /// Load configuration from watchdog directories
    ///
    /// Takes optional project and global watchdog directory paths.
    /// Tries project config first, then global, then uses defaults.
    pub fn load_from_directory(
        project_watchdog_dir: Option<&Path>,
        global_watchdog_dir: Option<&Path>,
    ) -> Self {
        // Try project config first, then global
        let dir_config = project_watchdog_dir
            .and_then(WatchdogDirConfig::load_from_dir)
            .or_else(|| global_watchdog_dir.and_then(WatchdogDirConfig::load_from_dir));

        if let Some(dir_config) = dir_config {
            dir_config.into_watchdog_config()
        } else {
            // No directory config found, return enabled default
            Self {
                enabled: true,
                openrouter: Some(OpenRouterConfig::default()),
                ..Default::default()
            }
        }
    }
}

/// Configuration loaded from `.cupcake/watchdog/config.json`
///
/// This is a flattened structure (no nested `openrouter:` key) for simplicity.
/// Note: While `WatchdogConfig` includes a `dry_run` field for programmatic use,
/// it is intentionally excluded here—`dry_run` cannot be set via config files
/// and should only be set from CLI flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogDirConfig {
    /// Which backend to use
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Model ID for the backend
    #[serde(default = "default_model")]
    pub model: String,

    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Behavior on error: "allow" or "deny"
    #[serde(default = "default_on_error")]
    pub on_error: String,

    /// Environment variable name for API key
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Rules context configuration for injecting file contents into prompts
    /// Uses camelCase in JSON: `rulesContext`
    #[serde(default, rename = "rulesContext")]
    pub rules_context: Option<RulesContext>,
}

impl Default for WatchdogDirConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            model: default_model(),
            timeout_seconds: default_timeout(),
            on_error: default_on_error(),
            api_key_env: default_api_key_env(),
            rules_context: None,
        }
    }
}

impl RulesContext {
    /// Load the contents of all configured files
    ///
    /// `config_dir` is the directory containing config.json (e.g., `.cupcake/watchdog/`)
    /// Files are resolved as: config_dir / root_path / file
    ///
    /// Returns an error if `strict` is true and any file fails to load.
    /// If `strict` is false, logs warnings for missing files and continues.
    pub fn load_files(&self, config_dir: &Path) -> Result<String, std::io::Error> {
        if self.files.is_empty() {
            return Ok(String::new());
        }

        let root = config_dir.join(&self.root_path);
        let mut contents = Vec::new();

        for file in &self.files {
            let file_path = root.join(file);
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    tracing::debug!("Loaded rules context file: {}", file_path.display());
                    contents.push(format!("=== {file} ===\n{content}"));
                }
                Err(e) => {
                    if self.strict {
                        return Err(std::io::Error::new(
                            e.kind(),
                            format!("Failed to load rules context file '{file}': {e}"),
                        ));
                    }
                    tracing::warn!(
                        "Failed to load rules context file {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }

        Ok(contents.join("\n\n"))
    }
}

impl WatchdogDirConfig {
    /// Load config from a directory's config.json
    pub fn load_from_dir(dir: &Path) -> Option<Self> {
        let config_path = dir.join("config.json");
        if !config_path.exists() {
            // No config.json, but directory exists - use defaults
            if dir.exists() {
                return Some(Self::default());
            }
            return None;
        }

        match std::fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => Some(config),
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", config_path.display(), e);
                    Some(Self::default())
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read {}: {}", config_path.display(), e);
                None
            }
        }
    }

    /// Convert to full WatchdogConfig
    ///
    /// Note: `dry_run` is set to false here - it should be set from CLI flag.
    pub fn into_watchdog_config(self) -> WatchdogConfig {
        WatchdogConfig {
            enabled: true,
            backend: self.backend,
            timeout_seconds: self.timeout_seconds,
            on_error: self.on_error,
            dry_run: false, // Set from CLI flag, not file config
            openrouter: Some(OpenRouterConfig {
                model: self.model,
                api_key_env: self.api_key_env,
                timeout_seconds: self.timeout_seconds,
                system_prompt: None, // Loaded separately from system.txt
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WatchdogConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.backend, "openrouter");
        assert_eq!(config.timeout_seconds, 10);
        assert!(config.allows_on_error());
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
enabled: true
backend: openrouter
timeout_seconds: 15
on_error: deny
openrouter:
  model: "google/gemini-2.5-flash"
  api_key_env: "MY_API_KEY"
"#;

        let config: WatchdogConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.timeout_seconds, 15);
        assert!(!config.allows_on_error());

        let or = config.openrouter.unwrap();
        assert_eq!(or.model, "google/gemini-2.5-flash");
        assert_eq!(or.api_key_env, "MY_API_KEY");
    }

    #[test]
    fn test_is_usable() {
        let mut config = WatchdogConfig::default();
        assert!(!config.is_usable()); // Not enabled

        config.enabled = true;
        assert!(config.is_usable()); // Now usable (openrouter is default backend)

        config.backend = "unknown".to_string();
        assert!(!config.is_usable()); // Unknown backend
    }

    #[test]
    fn test_dir_config_json_parsing() {
        let json = r#"{
            "backend": "openrouter",
            "model": "google/gemini-2.5-flash",
            "timeout_seconds": 15,
            "on_error": "deny",
            "api_key_env": "MY_KEY"
        }"#;

        let dir_config: WatchdogDirConfig = serde_json::from_str(json).unwrap();
        assert_eq!(dir_config.backend, "openrouter");
        assert_eq!(dir_config.model, "google/gemini-2.5-flash");
        assert_eq!(dir_config.timeout_seconds, 15);
        assert_eq!(dir_config.on_error, "deny");
        assert_eq!(dir_config.api_key_env, "MY_KEY");

        let config = dir_config.into_watchdog_config();
        assert!(config.enabled);
        assert!(!config.allows_on_error());
    }

    #[test]
    fn test_dir_config_defaults() {
        let json = "{}";
        let dir_config: WatchdogDirConfig = serde_json::from_str(json).unwrap();

        assert_eq!(dir_config.backend, "openrouter");
        assert_eq!(dir_config.model, "google/gemini-2.5-flash");
        assert_eq!(dir_config.timeout_seconds, 10);
        assert_eq!(dir_config.on_error, "allow");
        assert_eq!(dir_config.api_key_env, "OPENROUTER_API_KEY");
    }

    #[test]
    fn test_effective_openrouter_config() {
        let config = WatchdogConfig {
            enabled: true,
            timeout_seconds: 20,
            openrouter: Some(OpenRouterConfig {
                model: "test-model".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let effective = config.effective_openrouter_config();
        assert_eq!(effective.timeout_seconds, 20); // Inherited
        assert_eq!(effective.model, "test-model");
    }

    #[test]
    fn test_shorthand_true() {
        // Simulate `watchdog: true` in YAML
        let input: WatchdogConfigInput = serde_yaml_ng::from_str("true").unwrap();
        let config: WatchdogConfig = input.into();

        assert!(config.enabled);
        assert_eq!(config.backend, "openrouter");
        assert!(config.openrouter.is_some());
    }

    #[test]
    fn test_shorthand_false() {
        // Simulate `watchdog: false` in YAML
        let input: WatchdogConfigInput = serde_yaml_ng::from_str("false").unwrap();
        let config: WatchdogConfig = input.into();

        assert!(!config.enabled);
    }

    #[test]
    fn test_shorthand_full_object() {
        // Simulate full config object in YAML
        let yaml = r#"
enabled: true
backend: openrouter
timeout_seconds: 15
on_error: deny
"#;
        let input: WatchdogConfigInput = serde_yaml_ng::from_str(yaml).unwrap();
        let config: WatchdogConfig = input.into();

        assert!(config.enabled);
        assert_eq!(config.timeout_seconds, 15);
        assert!(!config.allows_on_error());
    }

    #[test]
    fn test_rulebook_shorthand_parsing() {
        // Test that rulebook can parse `watchdog: true`
        // We test the WatchdogConfigInput directly since it's what the
        // custom deserializer in rulebook.rs uses

        // Shorthand true
        let yaml = "true";
        let input: WatchdogConfigInput = serde_yaml_ng::from_str(yaml).unwrap();
        let config: WatchdogConfig = input.into();
        assert!(config.enabled);
        assert!(config.openrouter.is_some());

        // Full object
        let yaml = "enabled: true\nbackend: openrouter";
        let input: WatchdogConfigInput = serde_yaml_ng::from_str(yaml).unwrap();
        let config: WatchdogConfig = input.into();
        assert!(config.enabled);
        assert_eq!(config.backend, "openrouter");
    }
}
