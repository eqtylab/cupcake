use crate::harness::response::types::{
    CupcakeResponse, EngineDecision, HookSpecificOutput, PermissionRequestBehavior,
    PermissionRequestDecision,
};

/// Builder for PermissionRequest hook responses
///
/// PermissionRequest is the hook that fires when a user is shown a permission dialog.
/// It allows policies to auto-approve or auto-deny on behalf of the user.
///
/// Response format:
/// ```json
/// {
///   "hookSpecificOutput": {
///     "hookEventName": "PermissionRequest",
///     "decision": {
///       "behavior": "allow",  // or "deny"
///       "updatedInput": {...},  // optional, for allow
///       "message": "...",       // optional, for deny (shown to model)
///       "interrupt": true       // optional, for deny (stops Claude)
///     }
///   }
/// }
/// ```
///
/// Note: There is no "ask" behavior because PermissionRequest IS the ask dialog.
pub struct PermissionRequestResponseBuilder;

impl PermissionRequestResponseBuilder {
    /// Build response for PermissionRequest event
    ///
    /// Decision mapping:
    /// - Allow → behavior: "allow"
    /// - Block/Deny → behavior: "deny" with message
    /// - Halt → behavior: "deny" with interrupt: true
    /// - Ask → behavior: "allow" (let the dialog show to user as normal)
    /// - Modify → behavior: "allow" with updatedInput
    pub fn build(decision: &EngineDecision, suppress_output: bool) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        // PermissionRequest uses hookSpecificOutput with nested decision object
        match decision {
            EngineDecision::Allow { .. } => {
                response.hook_specific_output = Some(HookSpecificOutput::PermissionRequest {
                    decision: PermissionRequestDecision {
                        behavior: PermissionRequestBehavior::Allow,
                        updated_input: None,
                        message: None,
                        interrupt: None,
                    },
                });
            }
            EngineDecision::Block { feedback } => {
                // Block/Deny → deny with message (tells model why denied)
                response.hook_specific_output = Some(HookSpecificOutput::PermissionRequest {
                    decision: PermissionRequestDecision {
                        behavior: PermissionRequestBehavior::Deny,
                        updated_input: None,
                        message: Some(feedback.clone()),
                        interrupt: None,
                    },
                });
            }
            EngineDecision::Ask { .. } => {
                // Ask doesn't make sense for PermissionRequest - it IS the ask dialog
                // Treat as Allow (let the normal permission dialog show to user)
                response.hook_specific_output = Some(HookSpecificOutput::PermissionRequest {
                    decision: PermissionRequestDecision {
                        behavior: PermissionRequestBehavior::Allow,
                        updated_input: None,
                        message: None,
                        interrupt: None,
                    },
                });
            }
            // Modify implies Allow with updated input
            EngineDecision::Modify { updated_input, .. } => {
                response.hook_specific_output = Some(HookSpecificOutput::PermissionRequest {
                    decision: PermissionRequestDecision {
                        behavior: PermissionRequestBehavior::Allow,
                        updated_input: Some(updated_input.clone()),
                        message: None,
                        interrupt: None,
                    },
                });
            }
        }

        // Apply suppress_output if requested
        if suppress_output {
            response.suppress_output = Some(true);
        }

        response
    }

    /// Build a deny response with interrupt flag (for Halt decisions)
    ///
    /// This is used when a Halt decision is synthesized - it denies the permission
    /// AND interrupts/stops Claude entirely.
    pub fn build_with_interrupt(feedback: &str, suppress_output: bool) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        response.hook_specific_output = Some(HookSpecificOutput::PermissionRequest {
            decision: PermissionRequestDecision {
                behavior: PermissionRequestBehavior::Deny,
                updated_input: None,
                message: Some(feedback.to_string()),
                interrupt: Some(true),
            },
        });

        if suppress_output {
            response.suppress_output = Some(true);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_permission_request_allow() {
        let decision = EngineDecision::Allow {
            reason: Some("Test reason".to_string()),
        };
        let response = PermissionRequestResponseBuilder::build(&decision, false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PermissionRequest { decision }) => {
                assert_eq!(decision.behavior, PermissionRequestBehavior::Allow);
                assert!(decision.message.is_none()); // message is for deny
                assert!(decision.updated_input.is_none());
                assert!(decision.interrupt.is_none());
            }
            _ => panic!("Expected PermissionRequest hook output"),
        }
        assert_eq!(response.suppress_output, None);
    }

    #[test]
    fn test_permission_request_deny() {
        let decision = EngineDecision::Block {
            feedback: "Security violation".to_string(),
        };
        let response = PermissionRequestResponseBuilder::build(&decision, true);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PermissionRequest { decision }) => {
                assert_eq!(decision.behavior, PermissionRequestBehavior::Deny);
                assert_eq!(decision.message, Some("Security violation".to_string()));
                assert!(decision.updated_input.is_none());
                assert!(decision.interrupt.is_none()); // no interrupt for regular deny
            }
            _ => panic!("Expected PermissionRequest hook output"),
        }
        assert_eq!(response.suppress_output, Some(true));
    }

    #[test]
    fn test_permission_request_ask_becomes_allow() {
        // Ask doesn't make sense for PermissionRequest (it IS the ask dialog)
        // So Ask is treated as Allow (let the dialog show normally)
        let decision = EngineDecision::Ask {
            reason: "Please confirm action".to_string(),
        };
        let response = PermissionRequestResponseBuilder::build(&decision, false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PermissionRequest { decision }) => {
                assert_eq!(decision.behavior, PermissionRequestBehavior::Allow);
                assert!(decision.message.is_none());
                assert!(decision.updated_input.is_none());
            }
            _ => panic!("Expected PermissionRequest hook output"),
        }
    }

    #[test]
    fn test_permission_request_modify() {
        let updated = json!({"command": "safe command"});
        let decision = EngineDecision::Modify {
            reason: "Sanitized input".to_string(),
            updated_input: updated.clone(),
        };
        let response = PermissionRequestResponseBuilder::build(&decision, false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PermissionRequest { decision }) => {
                assert_eq!(decision.behavior, PermissionRequestBehavior::Allow);
                assert_eq!(decision.updated_input, Some(updated));
                assert!(decision.message.is_none()); // message is for deny
            }
            _ => panic!("Expected PermissionRequest hook output"),
        }
    }

    #[test]
    fn test_permission_request_with_interrupt() {
        // Halt decisions should deny with interrupt flag
        let response =
            PermissionRequestResponseBuilder::build_with_interrupt("Emergency stop", false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PermissionRequest { decision }) => {
                assert_eq!(decision.behavior, PermissionRequestBehavior::Deny);
                assert_eq!(decision.message, Some("Emergency stop".to_string()));
                assert_eq!(decision.interrupt, Some(true));
                assert!(decision.updated_input.is_none());
            }
            _ => panic!("Expected PermissionRequest hook output"),
        }
    }

    #[test]
    fn test_permission_request_json_format() {
        let decision = EngineDecision::Allow {
            reason: Some("Allowed".to_string()),
        };
        let response = PermissionRequestResponseBuilder::build(&decision, false);

        // Serialize to JSON to verify format
        let json = serde_json::to_value(&response).unwrap();

        // Check nested structure
        assert!(json["hookSpecificOutput"]["hookEventName"]
            .as_str()
            .unwrap()
            .eq("PermissionRequest"));
        assert_eq!(json["hookSpecificOutput"]["decision"]["behavior"], "allow");
        // message should not be present for allow
        assert!(json["hookSpecificOutput"]["decision"]["message"].is_null());
    }

    #[test]
    fn test_permission_request_deny_json_format() {
        let decision = EngineDecision::Block {
            feedback: "Blocked for security".to_string(),
        };
        let response = PermissionRequestResponseBuilder::build(&decision, false);

        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(
            json["hookSpecificOutput"]["decision"]["message"],
            "Blocked for security"
        );
    }
}
