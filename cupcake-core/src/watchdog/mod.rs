//! Cupcake Watchdog - LLM-as-judge for AI agent tool calls
//!
//! Watchdog evaluates agent events using an LLM to provide semantic security
//! analysis that complements deterministic Rego policies.
//!
//! ## Configuration
//!
//! Enable in `rulebook.yml`:
//! ```yaml
//! watchdog: true
//! ```
//!
//! Settings in `.cupcake/watchdog/config.json`:
//! ```json
//! {"model": "google/gemini-2.5-flash", "timeout_seconds": 10}
//! ```
//!
//! Custom prompts: `system.txt` and `user.txt` (with `{{event}}` placeholder).
//!
//! ## Policy Usage
//!
//! ```rego
//! deny contains decision if {
//!     input.signals.watchdog.allow == false
//!     decision := {"rule_id": "WATCHDOG-001", "reason": input.signals.watchdog.reasoning}
//! }
//! ```

pub mod backend;
pub mod config;
pub mod prompts;
pub mod types;

#[cfg(feature = "watchdog")]
pub mod openrouter;

// Re-export main types
pub use config::{
    OpenRouterConfig, RulesContext, WatchdogConfig, WatchdogConfigInput, WatchdogDirConfig,
};
pub use prompts::WatchdogPrompts;
pub use types::{WatchdogInput, WatchdogOutput};

#[cfg(feature = "watchdog")]
pub use backend::WatchdogBackend;

use anyhow::Result;
use std::path::Path;
use tracing::{debug, info, warn};

/// Watchdog evaluator - orchestrates LLM-based event evaluation
pub struct Watchdog {
    #[allow(dead_code)] // Used when watchdog feature is enabled
    config: WatchdogConfig,
    #[allow(dead_code)]
    prompts: WatchdogPrompts,
    #[cfg(feature = "watchdog")]
    backend: Option<Box<dyn backend::WatchdogBackend>>,
}

impl Watchdog {
    /// Create a new Watchdog instance from configuration
    pub fn new(config: WatchdogConfig) -> Result<Self> {
        Self::with_prompts(config, WatchdogPrompts::default())
    }

    /// Create a new Watchdog instance with custom prompts
    pub fn with_prompts(config: WatchdogConfig, prompts: WatchdogPrompts) -> Result<Self> {
        if !config.enabled {
            debug!("Watchdog is disabled");
            return Ok(Self {
                config,
                prompts,
                #[cfg(feature = "watchdog")]
                backend: None,
            });
        }

        #[cfg(feature = "watchdog")]
        {
            let backend: Box<dyn backend::WatchdogBackend> = match config.backend.as_str() {
                "openrouter" => {
                    let or_config = config.effective_openrouter_config();
                    info!(
                        "Initializing Watchdog with OpenRouter backend (model: {}, dry_run: {})",
                        or_config.model, config.dry_run
                    );
                    Box::new(openrouter::OpenRouterBackend::with_prompts_and_dry_run(
                        or_config,
                        prompts.clone(),
                        config.dry_run,
                    )?)
                }
                other => {
                    return Err(anyhow::anyhow!("Unknown watchdog backend: {}", other));
                }
            };

            Ok(Self {
                config,
                prompts,
                backend: Some(backend),
            })
        }

        #[cfg(not(feature = "watchdog"))]
        {
            warn!("Watchdog is enabled in config but the 'watchdog' feature is not compiled in");
            Ok(Self { config, prompts })
        }
    }

    /// Create Watchdog from directory-based configuration
    ///
    /// Loads config from `.cupcake/watchdog/` with fallback to global config.
    pub fn from_directories(
        project_watchdog_dir: Option<&Path>,
        global_watchdog_dir: Option<&Path>,
    ) -> Result<Self> {
        Self::from_directories_with_dry_run(project_watchdog_dir, global_watchdog_dir, false)
    }

    /// Create Watchdog from directory-based configuration with explicit dry_run flag
    ///
    /// Loads config from `.cupcake/watchdog/` with fallback to global config.
    /// dry_run is passed explicitly (from CLI flag) rather than loaded from config files.
    pub fn from_directories_with_dry_run(
        project_watchdog_dir: Option<&Path>,
        global_watchdog_dir: Option<&Path>,
        dry_run: bool,
    ) -> Result<Self> {
        // Load dir config first to get rules_context before converting to WatchdogConfig
        let dir_config = project_watchdog_dir
            .and_then(WatchdogDirConfig::load_from_dir)
            .or_else(|| global_watchdog_dir.and_then(WatchdogDirConfig::load_from_dir));

        let rules_context = dir_config.as_ref().and_then(|dc| dc.rules_context.as_ref());

        // Load prompts with rules context
        let prompts = WatchdogPrompts::load_with_rules_context(
            project_watchdog_dir,
            global_watchdog_dir,
            rules_context,
        )?;

        // Convert dir config to full config
        let mut config = dir_config
            .map(WatchdogDirConfig::into_watchdog_config)
            .unwrap_or_else(|| WatchdogConfig {
                enabled: true,
                openrouter: Some(OpenRouterConfig::default()),
                ..Default::default()
            });

        // Apply CLI-provided dry_run flag (overrides any file config)
        config.dry_run = dry_run;

        // Log resolved configuration - useful for debugging and dry_run testing
        let or_config = config.openrouter.as_ref();

        // Use info! for dry_run mode so it's always visible
        if config.dry_run {
            info!(
                "Watchdog resolved config (dry_run=true): backend={}, model={}, timeout={}s, on_error={}, api_key_env={}",
                config.backend,
                or_config.map(|o| o.model.as_str()).unwrap_or("default"),
                config.timeout_seconds,
                config.on_error,
                or_config.map(|o| o.api_key_env.as_str()).unwrap_or("OPENROUTER_API_KEY")
            );
            info!(
                "Watchdog resolved prompts: system_prompt_len={} chars, user_template_len={} chars, rules_context_len={} chars",
                prompts.system_prompt.len(),
                prompts.user_template.len(),
                prompts.rules_context.len()
            );
            // Log first 100 chars of system prompt for verification
            let system_preview: String = prompts.system_prompt.chars().take(100).collect();
            info!("Watchdog system_prompt preview: {}...", system_preview);
            let user_preview: String = prompts.user_template.chars().take(100).collect();
            info!("Watchdog user_template preview: {}...", user_preview);
            if !prompts.rules_context.is_empty() {
                let rules_preview: String = prompts.rules_context.chars().take(200).collect();
                info!("Watchdog rules_context preview: {}...", rules_preview);
            }
        } else {
            debug!(
                "Watchdog config: enabled={}, backend={}, model={}, dry_run={}, rules_context_len={}",
                config.enabled,
                config.backend,
                or_config.map(|o| o.model.as_str()).unwrap_or("default"),
                config.dry_run,
                prompts.rules_context.len()
            );
        }

        Self::with_prompts(config, prompts)
    }

    /// Check if Watchdog is enabled and ready
    pub fn is_enabled(&self) -> bool {
        #[cfg(feature = "watchdog")]
        {
            self.config.enabled && self.backend.is_some()
        }
        #[cfg(not(feature = "watchdog"))]
        {
            false
        }
    }

    /// Override the model at runtime (useful for CLI --model flag)
    pub fn override_model(&mut self, model: String) {
        #[cfg(feature = "watchdog")]
        {
            if let Some(ref mut backend) = self.backend {
                backend.override_model(model.clone());
            }
            // Also update config for consistency
            if let Some(ref mut or_config) = self.config.openrouter {
                or_config.model = model;
            } else {
                self.config.openrouter = Some(OpenRouterConfig {
                    model,
                    ..Default::default()
                });
            }
        }
        #[cfg(not(feature = "watchdog"))]
        {
            let _ = model;
        }
    }

    /// Evaluate an event using the LLM backend
    ///
    /// Returns a WatchdogOutput which can be serialized to JSON for policy consumption.
    /// On error, returns fail-open or fail-closed output based on configuration.
    pub async fn evaluate(&self, input: WatchdogInput) -> WatchdogOutput {
        #[cfg(feature = "watchdog")]
        {
            let Some(backend) = &self.backend else {
                return WatchdogOutput::fail_open("Watchdog backend not initialized");
            };

            match backend.evaluate(input).await {
                Ok(output) => output,
                Err(e) => {
                    warn!("Watchdog evaluation error: {}", e);
                    if self.config.allows_on_error() {
                        WatchdogOutput::fail_open(&e.to_string())
                    } else {
                        WatchdogOutput::fail_closed(&e.to_string())
                    }
                }
            }
        }

        #[cfg(not(feature = "watchdog"))]
        {
            let _ = input; // Suppress unused warning
            WatchdogOutput::fail_open("Watchdog feature not compiled in")
        }
    }

    /// Create WatchdogInput from an event JSON value
    pub fn input_from_event(event: &serde_json::Value) -> WatchdogInput {
        let event_type = event
            .get("hook_event_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let tool_name = event
            .get("tool_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        WatchdogInput {
            event_type,
            tool_name,
            event_payload: event.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_watchdog() {
        let config = WatchdogConfig::default();
        let watchdog = Watchdog::new(config).unwrap();
        assert!(!watchdog.is_enabled());
    }

    #[test]
    fn test_input_from_event() {
        let event = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {
                "command": "ls -la"
            }
        });

        let input = Watchdog::input_from_event(&event);
        assert_eq!(input.event_type, "PreToolUse");
        assert_eq!(input.tool_name, Some("Bash".to_string()));
    }

    #[tokio::test]
    async fn test_evaluate_when_disabled() {
        let config = WatchdogConfig::default();
        let watchdog = Watchdog::new(config).unwrap();

        let input = WatchdogInput {
            event_type: "PreToolUse".to_string(),
            tool_name: Some("Bash".to_string()),
            event_payload: serde_json::json!({}),
        };

        let output = watchdog.evaluate(input).await;
        // Should fail-open when disabled
        assert!(output.allow);
    }
}
