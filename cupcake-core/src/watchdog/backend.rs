//! Watchdog backend trait - Abstraction over LLM providers
//!
//! This trait allows swapping between different LLM backends:
//! - OpenRouter (HTTP API)
//! - Claude Code SDK (local daemon, future)
//! - Mock (testing)

use anyhow::Result;
use async_trait::async_trait;

use super::types::{WatchdogInput, WatchdogOutput};

/// Trait for watchdog LLM backends
///
/// Implementations handle the specifics of calling different LLM providers
/// while presenting a unified interface to the engine.
#[async_trait]
pub trait WatchdogBackend: Send + Sync {
    /// Evaluate an event using the LLM
    ///
    /// Takes the event context and returns a structured judgment.
    /// Implementations should handle their own timeout and error handling.
    async fn evaluate(&self, input: WatchdogInput) -> Result<WatchdogOutput>;

    /// Backend identifier for logging/debugging
    fn name(&self) -> &'static str;

    /// Override the model at runtime (e.g., from CLI --model flag)
    fn override_model(&mut self, model: String);
}

/// Mock backend for testing
#[cfg(test)]
pub struct MockBackend {
    pub response: WatchdogOutput,
}

#[cfg(test)]
#[async_trait]
impl WatchdogBackend for MockBackend {
    async fn evaluate(&self, _input: WatchdogInput) -> Result<WatchdogOutput> {
        Ok(self.response.clone())
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn override_model(&mut self, _model: String) {
        // Mock doesn't use models
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend() {
        let backend = MockBackend {
            response: WatchdogOutput {
                allow: true,
                confidence: 0.99,
                reasoning: "Test response".to_string(),
                concerns: vec![],
                suggestions: vec![],
            },
        };

        let input = WatchdogInput {
            event_type: "PreToolUse".to_string(),
            tool_name: Some("Bash".to_string()),
            event_payload: serde_json::json!({"command": "ls"}),
        };

        let result = backend.evaluate(input).await.unwrap();
        assert!(result.allow);
        assert_eq!(result.confidence, 0.99);
    }
}
