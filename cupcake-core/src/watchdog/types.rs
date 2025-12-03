//! Watchdog types - Input and output structures for LLM evaluation
//!
//! These types define the contract between the engine and watchdog backends.

use serde::{Deserialize, Serialize};

/// Input to the watchdog LLM evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogInput {
    /// The event type being evaluated (e.g., "PreToolUse", "UserPromptSubmit")
    pub event_type: String,

    /// Tool name if applicable (e.g., "Bash", "Edit")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    /// The full event payload for context
    pub event_payload: serde_json::Value,
}

/// Output from the watchdog LLM evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogOutput {
    /// Whether the action should be allowed
    pub allow: bool,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,

    /// Human-readable explanation of the decision
    pub reasoning: String,

    /// List of specific concerns identified
    #[serde(default)]
    pub concerns: Vec<String>,

    /// Suggestions for the agent or user
    #[serde(default)]
    pub suggestions: Vec<String>,
}

impl WatchdogOutput {
    /// Create a default "allow" output for error cases (fail-open)
    pub fn fail_open(error_message: &str) -> Self {
        Self {
            allow: true,
            confidence: 0.0,
            reasoning: format!("Watchdog evaluation failed: {error_message}. Defaulting to allow."),
            concerns: vec!["watchdog_error".to_string()],
            suggestions: vec![],
        }
    }

    /// Create a default "deny" output for error cases (fail-closed)
    pub fn fail_closed(error_message: &str) -> Self {
        Self {
            allow: false,
            confidence: 0.0,
            reasoning: format!("Watchdog evaluation failed: {error_message}. Defaulting to deny."),
            concerns: vec!["watchdog_error".to_string()],
            suggestions: vec![],
        }
    }

    /// Create a dry_run response - allows the action but indicates it was a test
    pub fn dry_run(event_type: &str) -> Self {
        Self {
            allow: true,
            confidence: 1.0,
            reasoning: format!(
                "Watchdog dry_run mode: {event_type} event would be evaluated but API call was skipped."
            ),
            concerns: vec![],
            suggestions: vec!["dry_run_mode".to_string()],
        }
    }
}

impl From<WatchdogInput> for serde_json::Value {
    fn from(input: WatchdogInput) -> Self {
        serde_json::to_value(input).unwrap_or_default()
    }
}

impl From<WatchdogOutput> for serde_json::Value {
    fn from(output: WatchdogOutput) -> Self {
        serde_json::to_value(output).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_output_serialization() {
        let output = WatchdogOutput {
            allow: false,
            confidence: 0.85,
            reasoning: "Command attempts to read SSH keys".to_string(),
            concerns: vec!["sensitive_file_access".to_string()],
            suggestions: vec!["Use a deploy key instead".to_string()],
        };

        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["allow"], false);
        assert_eq!(json["confidence"], 0.85);
    }

    #[test]
    fn test_fail_open() {
        let output = WatchdogOutput::fail_open("API timeout");
        assert!(output.allow);
        assert_eq!(output.confidence, 0.0);
        assert!(output.reasoning.contains("timeout"));
    }

    #[test]
    fn test_fail_closed() {
        let output = WatchdogOutput::fail_closed("API timeout");
        assert!(!output.allow);
        assert_eq!(output.confidence, 0.0);
    }
}
