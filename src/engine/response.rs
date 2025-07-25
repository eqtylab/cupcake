use serde::{Deserialize, Serialize};
use std::process;

/// Internal policy evaluation result - renamed from PolicyDecision for clarity
#[derive(Debug, Clone, PartialEq)]
pub enum EngineDecision {
    /// Allow the operation to proceed
    Allow,
    /// Block the operation with feedback
    Block { feedback: String },
    /// Approve the operation (bypass permission system)
    Approve { reason: Option<String> },
    /// Ask the user for confirmation (new in July 20)
    Ask { reason: String },
}

/// Hook-specific output for different event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hookEventName")]
pub enum HookSpecificOutput {
    #[serde(rename = "PreToolUse")]
    PreToolUse {
        #[serde(rename = "permissionDecision")]
        permission_decision: String, // "allow" | "deny" | "ask"
        #[serde(rename = "permissionDecisionReason", skip_serializing_if = "Option::is_none")]
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

    /// Legacy decision field (deprecated but still supported)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Legacy reason field (deprecated but still supported)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CupcakeResponse {
    /// Create a response that allows the operation
    pub fn allow() -> Self {
        Self {
            continue_execution: None,
            stop_reason: None,
            suppress_output: None,
            decision: None,
            reason: None,
        }
    }

    /// Create a response that blocks the operation
    pub fn block(reason: String) -> Self {
        Self {
            continue_execution: None,
            stop_reason: None,
            suppress_output: None,
            decision: Some("block".to_string()),
            reason: Some(reason),
        }
    }

    /// Create a response that approves the operation
    pub fn approve(reason: Option<String>) -> Self {
        Self {
            continue_execution: None,
            stop_reason: None,
            suppress_output: None,
            decision: Some("approve".to_string()),
            reason,
        }
    }
}

/// Response handler for communicating with Claude Code
pub struct ResponseHandler {
    debug: bool,
    test_mode: bool,
}

impl ResponseHandler {
    pub fn new(debug: bool) -> Self {
        Self {
            debug,
            test_mode: false,
        }
    }

    pub fn new_test_mode(debug: bool) -> Self {
        Self {
            debug,
            test_mode: true,
        }
    }

    /// Send response to Claude Code and exit with appropriate code
    pub fn send_response(&self, decision: PolicyDecision) -> ! {
        if self.test_mode {
            // In test mode, just print debug info and exit with status 0
            match decision {
                PolicyDecision::Allow => {
                    if self.debug {
                        eprintln!("Debug: Test mode - would allow operation (exit code 0)");
                    }
                    process::exit(0);
                }
                PolicyDecision::Block { feedback } => {
                    if self.debug {
                        eprintln!(
                            "Debug: Test mode - would block operation with feedback (exit code 2)"
                        );
                        eprintln!("Debug: Feedback: {}", feedback);
                    }
                    process::exit(0);
                }
                PolicyDecision::Approve { reason } => {
                    if self.debug {
                        eprintln!("Debug: Test mode - would approve operation (exit code 0)");
                        if let Some(reason) = reason {
                            eprintln!("Debug: Reason: {}", reason);
                        }
                    }
                    process::exit(0);
                }
            }
        } else {
            // Production mode - actual exit codes
            match decision {
                PolicyDecision::Allow => {
                    if self.debug {
                        eprintln!("Debug: Allowing operation (exit code 0)");
                    }
                    process::exit(0);
                }
                PolicyDecision::Block { feedback } => {
                    if self.debug {
                        eprintln!("Debug: Blocking operation with feedback (exit code 2)");
                    }
                    eprintln!("{}", feedback);
                    process::exit(2);
                }
                PolicyDecision::Approve { reason } => {
                    if self.debug {
                        eprintln!("Debug: Approving operation (exit code 0)");
                    }

                    // Send JSON response for approval
                    let response = CupcakeResponse::approve(reason);
                    if let Ok(json) = serde_json::to_string(&response) {
                        println!("{}", json);
                    }
                    process::exit(0);
                }
            }
        }
    }

    /// Send JSON response to Claude Code (for advanced control)
    pub fn send_json_response(&self, response: CupcakeResponse) -> ! {
        if self.debug {
            eprintln!("Debug: Sending JSON response: {:?}", response);
        }

        match serde_json::to_string(&response) {
            Ok(json) => {
                println!("{}", json);
                process::exit(0);
            }
            Err(e) => {
                eprintln!("Error serializing response: {}", e);
                process::exit(1);
            }
        }
    }

    /// Send simple blocking response (for most common case)
    pub fn block_with_feedback(&self, feedback: String) -> ! {
        self.send_response(PolicyDecision::Block { feedback })
    }

    /// Send simple allow response (for most common case)
    pub fn allow(&self) -> ! {
        self.send_response(PolicyDecision::Allow)
    }

    /// Send approval response with optional reason
    pub fn approve(&self, reason: Option<String>) -> ! {
        self.send_response(PolicyDecision::Approve { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_cupcake_response_allow() {
        let response = CupcakeResponse::allow();
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.decision, None);
        assert_eq!(response.reason, None);
    }

    #[test]
    fn test_cupcake_response_block() {
        let response = CupcakeResponse::block("Test block reason".to_string());
        assert_eq!(response.decision, Some("block".to_string()));
        assert_eq!(response.reason, Some("Test block reason".to_string()));
    }

    #[test]
    fn test_cupcake_response_approve() {
        let response = CupcakeResponse::approve(Some("Test approve reason".to_string()));
        assert_eq!(response.decision, Some("approve".to_string()));
        assert_eq!(response.reason, Some("Test approve reason".to_string()));
    }

    #[test]
    fn test_cupcake_response_json_serialization() {
        let response = CupcakeResponse::block("Test feedback".to_string());
        let json = serde_json::to_string(&response).unwrap();

        // Should serialize to JSON without null fields
        assert!(json.contains("\"decision\":\"block\""));
        assert!(json.contains("\"reason\":\"Test feedback\""));
        assert!(!json.contains("\"continue_execution\""));
    }

    #[test]
    fn test_policy_decision_equality() {
        let decision1 = PolicyDecision::Allow;
        let decision2 = PolicyDecision::Allow;
        assert_eq!(decision1, decision2);

        let decision3 = PolicyDecision::Block {
            feedback: "test".to_string(),
        };
        let decision4 = PolicyDecision::Block {
            feedback: "test".to_string(),
        };
        assert_eq!(decision3, decision4);
    }

    #[test]
    fn test_response_handler_creation() {
        let handler = ResponseHandler::new(true);
        assert!(handler.debug);

        let handler = ResponseHandler::new(false);
        assert!(!handler.debug);
    }
}
