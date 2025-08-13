use crate::harness::response::types::{
    CupcakeResponse, EngineDecision, HookSpecificOutput,
};

/// Builder for context injection hook responses
///
/// UserPromptSubmit and SessionStart events support context injection
/// via hookSpecificOutput.additionalContext field
pub struct ContextInjectionResponseBuilder;

impl ContextInjectionResponseBuilder {
    /// Build response for context injection events
    ///
    /// is_user_prompt_submit: true for UserPromptSubmit (uses special block format),
    ///                        false for SessionStart (uses standard continue: false format)
    pub fn build(
        decision: &EngineDecision,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
        is_user_prompt_submit: bool,
    ) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        match decision {
            EngineDecision::Block { feedback } => {
                if is_user_prompt_submit {
                    // UserPromptSubmit Block uses decision: "block" format
                    response.decision = Some("block".to_string());
                    response.reason = Some(feedback.clone());
                } else {
                    // SessionStart uses standard continue: false format
                    response.continue_execution = Some(false);
                    response.stop_reason = Some(feedback.clone());
                }
            }
            EngineDecision::Allow { .. } => {
                // Add context injection if provided
                if let Some(contexts) = context_to_inject {
                    if !contexts.is_empty() {
                        let combined_context = contexts.join("\n");

                        if is_user_prompt_submit {
                            response.hook_specific_output =
                                Some(HookSpecificOutput::UserPromptSubmit {
                                    additional_context: Some(combined_context),
                                });
                        } else {
                            response.hook_specific_output =
                                Some(HookSpecificOutput::SessionStart {
                                    additional_context: Some(combined_context),
                                });
                        }
                    }
                }
            }
            EngineDecision::Ask { reason } => {
                // Per tactical advisory: Ask is a no-op for non-tool events
                // Log warning and treat as Allow with context
                tracing::warn!("Ask action not supported for UserPromptSubmit - treating as Allow with context");
                response.hook_specific_output = Some(HookSpecificOutput::UserPromptSubmit {
                    additional_context: Some(reason.clone()),
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
    fn test_context_injection_allow_with_context() {
        let decision = EngineDecision::Allow { reason: None };
        let context = vec!["Context line 1".to_string(), "Context line 2".to_string()];
        let response =
            ContextInjectionResponseBuilder::build(&decision, Some(context), false, true);

        match response.hook_specific_output {
            Some(HookSpecificOutput::UserPromptSubmit { additional_context }) => {
                assert_eq!(
                    additional_context,
                    Some("Context line 1\nContext line 2".to_string())
                );
            }
            _ => panic!("Expected UserPromptSubmit hook output"),
        }
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_context_injection_allow_no_context() {
        let decision = EngineDecision::Allow { reason: None };
        let response = ContextInjectionResponseBuilder::build(&decision, None, false, true);

        assert_eq!(response.hook_specific_output, None);
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_context_injection_block() {
        let decision = EngineDecision::Block {
            feedback: "Sensitive content detected".to_string(),
        };
        let response = ContextInjectionResponseBuilder::build(&decision, None, false, true);

        // UserPromptSubmit Block uses top-level decision/reason fields
        assert_eq!(response.decision, Some("block".to_string()));
        assert_eq!(
            response.reason,
            Some("Sensitive content detected".to_string())
        );
        assert_eq!(response.hook_specific_output, None);
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_context_injection_ask() {
        let decision = EngineDecision::Ask {
            reason: "Please confirm this action".to_string(),
        };
        let response = ContextInjectionResponseBuilder::build(&decision, None, false, true);

        // Ask is treated as Allow with context for UserPromptSubmit
        match response.hook_specific_output {
            Some(HookSpecificOutput::UserPromptSubmit { additional_context }) => {
                assert_eq!(
                    additional_context,
                    Some("Please confirm this action".to_string())
                );
            }
            _ => panic!("Expected UserPromptSubmit hook output"),
        }
    }

    #[test]
    fn test_context_injection_with_suppress() {
        let decision = EngineDecision::Allow { reason: None };
        let context = vec!["Silent context".to_string()];
        let response = ContextInjectionResponseBuilder::build(&decision, Some(context), true, true);

        match response.hook_specific_output {
            Some(HookSpecificOutput::UserPromptSubmit { additional_context }) => {
                assert_eq!(additional_context, Some("Silent context".to_string()));
            }
            _ => panic!("Expected UserPromptSubmit hook output"),
        }
        assert_eq!(response.suppress_output, Some(true));
    }

    #[test]
    fn test_session_start_block() {
        let decision = EngineDecision::Block {
            feedback: "Session blocked".to_string(),
        };
        // is_user_prompt_submit = false for SessionStart
        let response = ContextInjectionResponseBuilder::build(&decision, None, false, false);

        // SessionStart uses standard continue: false format
        assert_eq!(response.continue_execution, Some(false));
        assert_eq!(response.stop_reason, Some("Session blocked".to_string()));
        assert_eq!(response.hook_specific_output, None);
    }
}
