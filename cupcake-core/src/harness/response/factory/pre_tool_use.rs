use crate::harness::response::types::{
    CupcakeResponse, EngineDecision, HookSpecificOutput, PermissionDecision,
};

/// Builder for Factory AI PreToolUse hook responses
///
/// Factory AI supports updatedInput to modify tool parameters before execution
pub struct PreToolUseResponseBuilder;

impl PreToolUseResponseBuilder {
    /// Build response for PreToolUse event
    pub fn build(decision: &EngineDecision, suppress_output: bool) -> CupcakeResponse {
        Self::build_with_updated_input(decision, None, suppress_output)
    }

    /// Build response with optional updated input
    /// The updated_input allows policies to modify tool parameters before execution
    pub fn build_with_updated_input(
        decision: &EngineDecision,
        updated_input: Option<serde_json::Value>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        // PreToolUse always uses hookSpecificOutput with permissionDecision
        match decision {
            EngineDecision::Allow { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Allow,
                    permission_decision_reason: reason.clone(),
                    updated_input,
                });
            }
            EngineDecision::Block { feedback } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Deny,
                    permission_decision_reason: Some(feedback.clone()),
                    updated_input: None, // No modifications when denying
                });
            }
            EngineDecision::Ask { reason } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Ask,
                    permission_decision_reason: Some(reason.clone()),
                    updated_input: None, // No modifications when asking
                });
            }
            // Modify implies Allow with updated input
            EngineDecision::Modify {
                reason,
                updated_input: input,
            } => {
                response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                    permission_decision: PermissionDecision::Allow,
                    permission_decision_reason: Some(reason.clone()),
                    updated_input: Some(input.clone()),
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
