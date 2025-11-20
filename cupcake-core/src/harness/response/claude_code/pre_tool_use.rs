use crate::harness::response::types::{
    CupcakeResponse, EngineDecision, HookSpecificOutput, PermissionDecision,
};

/// Builder for PreToolUse hook responses
///
/// PreToolUse events require hookSpecificOutput with permissionDecision field
pub struct PreToolUseResponseBuilder;

impl PreToolUseResponseBuilder {
    /// Build response for PreToolUse event
    pub fn build(decision: &EngineDecision, suppress_output: bool) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        // PreToolUse always uses hookSpecificOutput with permissionDecision
        match decision {
            EngineDecision::Allow { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Allow,
                    permission_decision_reason: reason.clone(),
                    updated_input: None, // Claude Code doesn't support updated_input
                });
            }
            EngineDecision::Block { feedback } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Deny,
                    permission_decision_reason: Some(feedback.clone()),
                    updated_input: None,
                });
            }
            EngineDecision::Ask { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Ask,
                    permission_decision_reason: Some(reason.clone()),
                    updated_input: None,
                });
            }
        }

        // Apply suppress_output if requested
        if suppress_output {
            response.suppress_output = Some(true);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_tool_use_allow() {
        let decision = EngineDecision::Allow {
            reason: Some("Test reason".to_string()),
        };
        let response = PreToolUseResponseBuilder::build(&decision, false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse {
                permission_decision,
                permission_decision_reason,
                ..
            }) => {
                assert_eq!(permission_decision, PermissionDecision::Allow);
                assert_eq!(permission_decision_reason, Some("Test reason".to_string()));
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
        assert_eq!(response.suppress_output, None);
    }

    #[test]
    fn test_pre_tool_use_deny_with_suppress() {
        let decision = EngineDecision::Block {
            feedback: "Security violation".to_string(),
        };
        let response = PreToolUseResponseBuilder::build(&decision, true);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse {
                permission_decision,
                permission_decision_reason,
                ..
            }) => {
                assert_eq!(permission_decision, PermissionDecision::Deny);
                assert_eq!(
                    permission_decision_reason,
                    Some("Security violation".to_string())
                );
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
        assert_eq!(response.suppress_output, Some(true));
    }

    #[test]
    fn test_pre_tool_use_ask() {
        let decision = EngineDecision::Ask {
            reason: "Please confirm action".to_string(),
        };
        let response = PreToolUseResponseBuilder::build(&decision, false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse {
                permission_decision,
                permission_decision_reason,
                ..
            }) => {
                assert_eq!(permission_decision, PermissionDecision::Ask);
                assert_eq!(
                    permission_decision_reason,
                    Some("Please confirm action".to_string())
                );
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
    }
}
