use serde::{Deserialize, Serialize};
use std::process;

pub mod claude_code;

/// Internal policy evaluation result - renamed from PolicyDecision for clarity
#[derive(Debug, Clone, PartialEq)]
pub enum EngineDecision {
    /// Allow the operation to proceed (with optional reason)
    Allow { reason: Option<String> },
    /// Block the operation with feedback
    Block { feedback: String },
    /// Ask the user for confirmation (new in July 20)
    Ask { reason: String },
}

/// Permission decision for PreToolUse events - type-safe enum that serializes to correct JSON strings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Hook-specific output for different event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "hookEventName")]
pub enum HookSpecificOutput {
    #[serde(rename = "PreToolUse")]
    PreToolUse {
        #[serde(rename = "permissionDecision")]
        permission_decision: PermissionDecision,
        #[serde(
            rename = "permissionDecisionReason",
            skip_serializing_if = "Option::is_none"
        )]
        permission_decision_reason: Option<String>,
    },
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit {
        #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
        additional_context: Option<String>,
    },
}

/// Response to Claude Code - fully aligned with July 20 JSON hook contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CupcakeResponse {
    /// Whether Claude should continue after hook execution
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_execution: Option<bool>,

    /// Message shown when continue is false
    #[serde(rename = "stopReason", skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Hide stdout from transcript mode
    #[serde(rename = "suppressOutput", skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,

    /// Hook-specific output for advanced control
    #[serde(rename = "hookSpecificOutput", skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,

    /// Decision for feedback loop (PostToolUse, Stop, SubagentStop)
    #[serde(rename = "decision", skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Reason for the decision - fed back to Claude for self-correction
    #[serde(rename = "reason", skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CupcakeResponse {
    /// Create an empty response (allows by default)
    pub fn empty() -> Self {
        Self {
            continue_execution: None,
            stop_reason: None,
            suppress_output: None,
            hook_specific_output: None,
            decision: None,
            reason: None,
        }
    }

    /// Set the suppress_output flag on this response
    pub fn with_suppress_output(mut self, suppress: bool) -> Self {
        self.suppress_output = Some(suppress);
        self
    }
}

/// Response handler for communicating with Claude Code
pub struct ResponseHandler {
    debug: bool,
}

impl ResponseHandler {
    pub fn new(debug: bool) -> Self {
        Self { debug }
    }

    /// Send JSON response to Claude Code (for advanced control)
    pub fn send_json_response(&self, response: CupcakeResponse) -> ! {
        if self.debug {
            // Determine the type of response for debugging
            let response_type = if let Some(ref hook_output) = response.hook_specific_output {
                match hook_output {
                    HookSpecificOutput::PreToolUse {
                        permission_decision,
                        ..
                    } => match permission_decision {
                        PermissionDecision::Allow => "Allow response for PreToolUse event",
                        PermissionDecision::Deny => "Deny response for PreToolUse event",
                        PermissionDecision::Ask => "Ask response for PreToolUse event",
                    },
                    HookSpecificOutput::UserPromptSubmit { .. } => "UserPromptSubmit response",
                }
            } else if response.decision.is_some() {
                "Feedback loop response"
            } else if response.continue_execution == Some(false) {
                "Block response"
            } else {
                "Allow response"
            };

            eprintln!("Debug: Sending {response_type}");
            eprintln!("Debug: JSON response: {response:?}");
        }

        match serde_json::to_string(&response) {
            Ok(json) => {
                println!("{json}");
                process::exit(0);
            }
            Err(e) => {
                eprintln!("Error serializing response: {e}");
                process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_cupcake_response_empty() {
        let response = CupcakeResponse::empty();
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
        assert_eq!(response.suppress_output, None);
        assert_eq!(response.hook_specific_output, None);
    }

    #[test]
    fn test_cupcake_response_with_suppress_output() {
        let response = CupcakeResponse::empty().with_suppress_output(true);
        assert_eq!(response.suppress_output, Some(true));

        let response2 = CupcakeResponse::empty().with_suppress_output(false);
        assert_eq!(response2.suppress_output, Some(false));
    }

    #[test]
    fn test_engine_decision_equality() {
        let decision1 = EngineDecision::Allow { reason: None };
        let decision2 = EngineDecision::Allow { reason: None };
        assert_eq!(decision1, decision2);

        let decision3 = EngineDecision::Block {
            feedback: "test".to_string(),
        };
        let decision4 = EngineDecision::Block {
            feedback: "test".to_string(),
        };
        assert_eq!(decision3, decision4);

        let decision5 = EngineDecision::Ask {
            reason: "confirm".to_string(),
        };
        let decision6 = EngineDecision::Ask {
            reason: "confirm".to_string(),
        };
        assert_eq!(decision5, decision6);
    }

    #[test]
    fn test_response_handler_creation() {
        let handler = ResponseHandler::new(true);
        assert!(handler.debug);

        let handler = ResponseHandler::new(false);
        assert!(!handler.debug);
    }

    #[test]
    fn test_permission_decision_serialization() {
        // Test that PermissionDecision enum serializes to correct JSON strings
        let allow_json = serde_json::to_string(&PermissionDecision::Allow).unwrap();
        assert_eq!(allow_json, "\"allow\"");

        let deny_json = serde_json::to_string(&PermissionDecision::Deny).unwrap();
        assert_eq!(deny_json, "\"deny\"");

        let ask_json = serde_json::to_string(&PermissionDecision::Ask).unwrap();
        assert_eq!(ask_json, "\"ask\"");
    }

    #[test]
    fn test_permission_decision_deserialization() {
        // Test that JSON strings deserialize to correct PermissionDecision enum values
        let allow: PermissionDecision = serde_json::from_str("\"allow\"").unwrap();
        assert_eq!(allow, PermissionDecision::Allow);

        let deny: PermissionDecision = serde_json::from_str("\"deny\"").unwrap();
        assert_eq!(deny, PermissionDecision::Deny);

        let ask: PermissionDecision = serde_json::from_str("\"ask\"").unwrap();
        assert_eq!(ask, PermissionDecision::Ask);
    }
}
