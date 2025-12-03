//! OpenRouter backend for Watchdog
//!
//! Implements the WatchdogBackend trait using OpenRouter's API.
//! Simple HTTP POST to their chat completions endpoint.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::backend::WatchdogBackend;
use super::config::OpenRouterConfig;
use super::prompts::WatchdogPrompts;
use super::types::{WatchdogInput, WatchdogOutput};

/// OpenRouter API backend
pub struct OpenRouterBackend {
    client: reqwest::Client,
    config: OpenRouterConfig,
    prompts: WatchdogPrompts,
    api_key: String,
    /// Dry run mode - skips actual API calls
    dry_run: bool,
}

impl OpenRouterBackend {
    /// Create a new OpenRouter backend with default prompts
    ///
    /// Reads the API key from the environment variable specified in config.
    pub fn new(config: OpenRouterConfig) -> Result<Self> {
        Self::with_prompts_and_dry_run(config, WatchdogPrompts::default(), false)
    }

    /// Create a new OpenRouter backend with custom prompts
    pub fn with_prompts(config: OpenRouterConfig, prompts: WatchdogPrompts) -> Result<Self> {
        Self::with_prompts_and_dry_run(config, prompts, false)
    }

    /// Create a new OpenRouter backend with custom prompts and dry_run mode
    pub fn with_prompts_and_dry_run(
        config: OpenRouterConfig,
        prompts: WatchdogPrompts,
        dry_run: bool,
    ) -> Result<Self> {
        // In dry_run mode, we don't require the API key
        let api_key = if dry_run {
            std::env::var(&config.api_key_env).unwrap_or_else(|_| "DRY_RUN_NO_KEY".to_string())
        } else {
            std::env::var(&config.api_key_env).with_context(|| {
                format!(
                    "Watchdog requires {} environment variable to be set",
                    config.api_key_env
                )
            })?
        };

        // Clamp timeout to a minimum of 5 seconds - LLM API calls need time
        const MIN_TIMEOUT_SECONDS: u64 = 5;
        let timeout_seconds = if config.timeout_seconds < MIN_TIMEOUT_SECONDS {
            warn!(
                "Configured timeout_seconds={} is too low; using minimum of {} seconds",
                config.timeout_seconds, MIN_TIMEOUT_SECONDS
            );
            MIN_TIMEOUT_SECONDS
        } else {
            config.timeout_seconds
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            prompts,
            api_key,
            dry_run,
        })
    }

    /// Build the user message using the template
    fn build_user_message(&self, input: &WatchdogInput) -> String {
        self.prompts.render_user_message(&input.event_payload)
    }

    /// Get the system prompt
    fn system_prompt(&self) -> &str {
        // Config system_prompt takes precedence over loaded prompts
        self.config
            .system_prompt
            .as_deref()
            .unwrap_or(&self.prompts.system_prompt)
    }
}

/// OpenRouter API request structure
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// OpenRouter API response structure
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[async_trait]
impl WatchdogBackend for OpenRouterBackend {
    async fn evaluate(&self, input: WatchdogInput) -> Result<WatchdogOutput> {
        debug!(
            "Watchdog evaluating {} event via OpenRouter ({})",
            input.event_type, self.config.model
        );

        // Dry run mode: log what would be sent but skip the API call
        if self.dry_run {
            let user_message = self.build_user_message(&input);
            info!(
                "Watchdog dry_run: would send {} event to OpenRouter model {}",
                input.event_type, self.config.model
            );
            info!(
                "Watchdog dry_run: user_message ({} chars): {}...",
                user_message.len(),
                user_message.chars().take(500).collect::<String>()
            );
            return Ok(WatchdogOutput::dry_run(&input.event_type));
        }

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: self.system_prompt().to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: self.build_user_message(&input),
                },
            ],
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/eqtylab/cupcake")
            .header("X-Title", "Cupcake Watchdog")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("OpenRouter API error: {} - {}", status, body);
            return Err(anyhow::anyhow!(
                "OpenRouter API error: {} - {}",
                status,
                body
            ));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .unwrap_or("");

        debug!("Watchdog raw response: {}", content);

        // Parse the LLM's JSON response
        // Strip any markdown code fences if present
        let cleaned = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let output: WatchdogOutput = serde_json::from_str(cleaned)
            .with_context(|| format!("Failed to parse watchdog response as JSON: {cleaned}"))?;

        debug!(
            "Watchdog decision: allow={}, confidence={}, reasoning={}",
            output.allow, output.confidence, output.reasoning
        );

        Ok(output)
    }

    fn name(&self) -> &'static str {
        "openrouter"
    }

    fn override_model(&mut self, model: String) {
        self.config.model = model;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watchdog::prompts::DEFAULT_SYSTEM_PROMPT;

    #[test]
    fn test_default_system_prompt_exists() {
        // Check prompt is non-empty by verifying it contains expected content
        assert!(DEFAULT_SYSTEM_PROMPT.contains("security"));
        assert!(DEFAULT_SYSTEM_PROMPT.contains("reviewer"));
    }

    #[test]
    fn test_build_user_message() {
        // Test that WatchdogPrompts correctly renders event payload
        let prompts = WatchdogPrompts::default();

        let input = WatchdogInput {
            event_type: "PreToolUse".to_string(),
            tool_name: Some("Bash".to_string()),
            event_payload: serde_json::json!({"command": "ls -la"}),
        };

        let message = prompts.render_user_message(&input.event_payload);
        assert!(message.contains("command"));
        assert!(message.contains("ls -la"));
    }
}
