use crate::engine::response::{CupcakeResponse, EngineDecision, HookSpecificOutput};

/// Builder for context injection hook responses
///
/// UserPromptSubmit and SessionStart events support context injection
/// via hookSpecificOutput.additionalContext field
pub struct ContextInjectionResponseBuilder;

impl ContextInjectionResponseBuilder {
    /// Build response for context injection events
    pub fn build(
        decision: &EngineDecision,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        match decision {
            EngineDecision::Block { feedback } => {
                // Blocking uses continue/stopReason format
                response.continue_execution = Some(false);
                response.stop_reason = Some(feedback.clone());
            }
            EngineDecision::Allow { .. } => {
                // Add context injection if provided
                if let Some(contexts) = context_to_inject {
                    if !contexts.is_empty() {
                        let combined_context = contexts.join("\n");
                        response.hook_specific_output =
                            Some(HookSpecificOutput::UserPromptSubmit {
                                additional_context: Some(combined_context),
                            });
                    }
                }
            }
            EngineDecision::Ask { reason } => {
                // Ask uses context injection to provide the reason
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
        let response = ContextInjectionResponseBuilder::build(&decision, Some(context), false);

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
        let response = ContextInjectionResponseBuilder::build(&decision, None, false);

        assert_eq!(response.hook_specific_output, None);
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_context_injection_block() {
        let decision = EngineDecision::Block {
            feedback: "Sensitive content detected".to_string(),
        };
        let response = ContextInjectionResponseBuilder::build(&decision, None, false);

        assert_eq!(response.continue_execution, Some(false));
        assert_eq!(
            response.stop_reason,
            Some("Sensitive content detected".to_string())
        );
        assert_eq!(response.hook_specific_output, None);
    }

    #[test]
    fn test_context_injection_ask() {
        let decision = EngineDecision::Ask {
            reason: "Please confirm this action".to_string(),
        };
        let response = ContextInjectionResponseBuilder::build(&decision, None, false);

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
        let response = ContextInjectionResponseBuilder::build(&decision, Some(context), true);

        match response.hook_specific_output {
            Some(HookSpecificOutput::UserPromptSubmit { additional_context }) => {
                assert_eq!(additional_context, Some("Silent context".to_string()));
            }
            _ => panic!("Expected UserPromptSubmit hook output"),
        }
        assert_eq!(response.suppress_output, Some(true));
    }
}
