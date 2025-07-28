use serde::{Deserialize, Serialize};
use std::process;

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
            EngineDecision::Allow { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Allow,
                    permission_decision_reason: reason.clone(),
                });
            }
            EngineDecision::Block { feedback } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Deny,
                    permission_decision_reason: Some(feedback.clone()),
                });
            }
            EngineDecision::Ask { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Ask,
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

    /// Create a response for PostToolUse, Stop, SubagentStop events
    /// These use the decision/reason format per Claude Code July 20 spec
    pub fn from_decision_block(decision: &EngineDecision) -> Self {
        let mut response = Self::empty();
        
        match decision {
            EngineDecision::Block { feedback } => {
                // For these events, blocking uses continue: false with stopReason
                response.continue_execution = Some(false);
                response.stop_reason = Some(feedback.clone());
            }
            EngineDecision::Allow { .. } | EngineDecision::Ask { .. } => {
                // Allow and Ask don't set any special fields for these events
                // The response remains empty (which means allow by default)
            }
        }
        
        response
    }

    /// Create a response for UserPromptSubmit events with optional context
    /// Combines decision/reason format with optional additionalContext
    pub fn from_user_prompt_decision(decision: &EngineDecision, context: Option<String>) -> Self {
        let mut response = Self::from_decision_block(decision);
        
        // Add context injection if provided
        if let Some(ctx) = context {
            response.hook_specific_output = Some(HookSpecificOutput::UserPromptSubmit {
                additional_context: Some(ctx),
            });
        }
        
        response
    }

    /// Create a response for generic events (Notification, PreCompact)
    /// These events use minimal response format without hook-specific output
    pub fn from_generic_decision(decision: &EngineDecision) -> Self {
        // These events only use the common fields, no hook-specific output
        Self::from_decision_block(decision)
    }
}

/// Response handler for communicating with Claude Code
pub struct ResponseHandler {
    debug: bool,
}

impl ResponseHandler {
    pub fn new(debug: bool) -> Self {
        Self {
            debug,
        }
    }

    /// Send response to Claude Code as JSON and terminate the hook process.
    ///
    /// This method NEVER returns (`-> !`) because Claude Code expects the hook
    /// process to terminate after sending its response. The process exit is
    /// part of the Claude Code July 20 hook protocol specification.
    pub fn send_response(&self, decision: EngineDecision) -> ! {
        // Always use JSON response protocol
        if self.debug {
            match &decision {
                EngineDecision::Allow { .. } => eprintln!("Debug: Allowing operation"),
                EngineDecision::Block { .. } => eprintln!("Debug: Blocking operation with feedback"),
                EngineDecision::Ask { .. } => eprintln!("Debug: Asking for confirmation"),
            }
        }
        
        // Always create JSON response for all decision types
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        self.send_json_response(response);
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
        self.send_response(EngineDecision::Allow { reason: None })
    }

    /// Send ask response for user confirmation
    pub fn ask(&self, reason: String) -> ! {
        self.send_response(EngineDecision::Ask { reason })
    }

    /// Send response based on hook event context - correctly formats JSON per Claude Code spec
    ///
    /// This method ensures each hook event type gets the appropriate JSON format:
    /// - PreToolUse: hookSpecificOutput with permissionDecision
    /// - PostToolUse/Stop/SubagentStop: decision/reason fields
    /// - Notification/PreCompact: minimal response
    /// - UserPromptSubmit: handled separately via send_response_with_context
    pub fn send_response_for_hook(&self, decision: EngineDecision, hook_event: &str) -> ! {
        if self.debug {
            eprintln!("Debug: Sending {} response for {} event", 
                match &decision {
                    EngineDecision::Allow { .. } => "Allow",
                    EngineDecision::Block { .. } => "Block",
                    EngineDecision::Ask { .. } => "Ask",
                },
                hook_event
            );
        }

        let response = match hook_event {
            "PreToolUse" => CupcakeResponse::from_pre_tool_use_decision(&decision),
            "PostToolUse" | "Stop" | "SubagentStop" => CupcakeResponse::from_decision_block(&decision),
            "Notification" | "PreCompact" => CupcakeResponse::from_generic_decision(&decision),
            "UserPromptSubmit" => {
                // UserPromptSubmit should use send_response_with_context in run.rs
                // This is a fallback for direct calls
                CupcakeResponse::from_user_prompt_decision(&decision, None)
            }
            _ => {
                // Unknown hook types get generic handling
                if self.debug {
                    eprintln!("Debug: Unknown hook event type '{}', using generic response", hook_event);
                }
                CupcakeResponse::from_generic_decision(&decision)
            }
        };

        self.send_json_response(response);
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
        let decision = EngineDecision::Allow { reason: None };
        let response = CupcakeResponse::from_pre_tool_use_decision(&decision);
        
        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse { permission_decision, permission_decision_reason }) => {
                assert_eq!(permission_decision, PermissionDecision::Allow);
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
                assert_eq!(permission_decision, PermissionDecision::Deny);
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
                assert_eq!(permission_decision, PermissionDecision::Ask);
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
