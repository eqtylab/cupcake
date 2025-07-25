use serde::{Deserialize, Serialize};
use std::process;

/// Internal policy evaluation result - renamed from PolicyDecision for clarity
#[derive(Debug, Clone, PartialEq)]
pub enum EngineDecision {
    /// Allow the operation to proceed (default when no policies match)
    Allow,
    /// Block the operation with feedback
    Block { feedback: String },
    /// Allow the operation with an explanation (from Action::Allow)
    Approve { reason: Option<String> },
    /// Ask the user for confirmation (new in July 20)
    Ask { reason: String },
}

/// Hook-specific output for different event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

impl CupcakeResponse {
    /// Create an empty response (allows by default)
    pub fn empty() -> Self {
        Self {
            continue_execution: None,
            stop_reason: None,
            suppress_output: None,
            hook_specific_output: None,
        }
    }

    /// Create a response from an EngineDecision for PreToolUse events
    pub fn from_pre_tool_use_decision(decision: &EngineDecision) -> Self {
        let mut response = Self::empty();
        
        match decision {
            EngineDecision::Allow => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: "allow".to_string(),
                    permission_decision_reason: None,
                });
            }
            EngineDecision::Block { feedback } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: "deny".to_string(),
                    permission_decision_reason: Some(feedback.clone()),
                });
            }
            EngineDecision::Approve { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: "allow".to_string(),
                    permission_decision_reason: reason.clone(),
                });
            }
            EngineDecision::Ask { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: "ask".to_string(),
                    permission_decision_reason: Some(reason.clone()),
                });
            }
        }
        
        response
    }

    /// Create a response with context injection for UserPromptSubmit
    pub fn with_context_injection(context: String) -> Self {
        Self {
            continue_execution: None,
            stop_reason: None,
            suppress_output: None,
            hook_specific_output: Some(HookSpecificOutput::UserPromptSubmit {
                additional_context: Some(context),
            }),
        }
    }

    /// Stop execution with a reason
    pub fn stop(reason: String) -> Self {
        Self {
            continue_execution: Some(false),
            stop_reason: Some(reason),
            suppress_output: None,
            hook_specific_output: None,
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
    pub fn send_response(&self, decision: EngineDecision) -> ! {
        if self.test_mode {
            // In test mode, just print debug info and exit with status 0
            match decision {
                EngineDecision::Allow => {
                    if self.debug {
                        eprintln!("Debug: Test mode - would allow operation");
                    }
                    process::exit(0);
                }
                EngineDecision::Block { feedback } => {
                    if self.debug {
                        eprintln!("Debug: Test mode - would block operation");
                        eprintln!("Debug: Feedback: {}", feedback);
                    }
                    process::exit(0);
                }
                EngineDecision::Approve { reason } => {
                    if self.debug {
                        eprintln!("Debug: Test mode - would approve operation");
                        if let Some(reason) = reason {
                            eprintln!("Debug: Reason: {}", reason);
                        }
                    }
                    process::exit(0);
                }
                EngineDecision::Ask { reason } => {
                    if self.debug {
                        eprintln!("Debug: Test mode - would ask for confirmation");
                        eprintln!("Debug: Reason: {}", reason);
                    }
                    process::exit(0);
                }
            }
        } else {
            // Production mode - send JSON response
            // Note: This is deprecated - use send_json_response instead
            match &decision {
                EngineDecision::Allow => {
                    if self.debug {
                        eprintln!("Debug: Allowing operation");
                    }
                    process::exit(0);
                }
                EngineDecision::Block { feedback } => {
                    if self.debug {
                        eprintln!("Debug: Blocking operation with feedback");
                    }
                    // For backward compatibility, still use exit code 2 with stderr
                    eprintln!("{}", feedback);
                    process::exit(2);
                }
                _ => {
                    // For Approve and Ask, we need JSON output
                    let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
                    self.send_json_response(response);
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
        self.send_response(EngineDecision::Block { feedback })
    }

    /// Send simple allow response (for most common case)
    pub fn allow(&self) -> ! {
        self.send_response(EngineDecision::Allow)
    }

    /// Send approval response with optional reason
    pub fn approve(&self, reason: Option<String>) -> ! {
        self.send_response(EngineDecision::Approve { reason })
    }

    /// Send ask response for user confirmation
    pub fn ask(&self, reason: String) -> ! {
        self.send_response(EngineDecision::Ask { reason })
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
    fn test_cupcake_response_pre_tool_use_allow() {
        let decision = EngineDecision::Allow;
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        
        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse { permission_decision, permission_decision_reason }) => {
                assert_eq!(permission_decision, "allow");
                assert_eq!(permission_decision_reason, None);
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
    }

    #[test]
    fn test_cupcake_response_pre_tool_use_deny() {
        let decision = EngineDecision::Block { feedback: "Test block reason".to_string() };
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        
        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse { permission_decision, permission_decision_reason }) => {
                assert_eq!(permission_decision, "deny");
                assert_eq!(permission_decision_reason, Some("Test block reason".to_string()));
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
    }

    #[test]
    fn test_cupcake_response_pre_tool_use_ask() {
        let decision = EngineDecision::Ask { reason: "Please confirm".to_string() };
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        
        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse { permission_decision, permission_decision_reason }) => {
                assert_eq!(permission_decision, "ask");
                assert_eq!(permission_decision_reason, Some("Please confirm".to_string()));
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
    }

    #[test]
    fn test_cupcake_response_context_injection() {
        let response = CupcakeResponse::with_context_injection("Test context".to_string());
        
        match response.hook_specific_output {
            Some(HookSpecificOutput::UserPromptSubmit { additional_context }) => {
                assert_eq!(additional_context, Some("Test context".to_string()));
            }
            _ => panic!("Expected UserPromptSubmit hook output"),
        }
    }

    #[test]
    fn test_cupcake_response_json_serialization() {
        let decision = EngineDecision::Block { feedback: "Test feedback".to_string() };
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        let json = serde_json::to_string(&response).unwrap();

        // Should serialize to JSON with proper hook contract format
        assert!(json.contains("\"hookSpecificOutput\""));
        assert!(json.contains("\"hookEventName\":\"PreToolUse\""));
        assert!(json.contains("\"permissionDecision\":\"deny\""));
        assert!(json.contains("\"permissionDecisionReason\":\"Test feedback\""));
        assert!(!json.contains("\"continue\""));  // None fields should be omitted
    }

    #[test]
    fn test_engine_decision_equality() {
        let decision1 = EngineDecision::Allow;
        let decision2 = EngineDecision::Allow;
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
}
