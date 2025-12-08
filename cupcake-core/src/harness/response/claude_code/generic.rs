use crate::harness::response::types::{CupcakeResponse, EngineDecision};

/// Builder for generic hook responses
///
/// Notification and PreCompact (when not injecting) use minimal response format
pub struct GenericResponseBuilder;

impl GenericResponseBuilder {
    /// Build response for generic events (Notification)
    pub fn build(decision: &EngineDecision, suppress_output: bool) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        // Generic events use continue/stopReason for blocking
        match decision {
            EngineDecision::Block { feedback } => {
                response.continue_execution = Some(false);
                response.stop_reason = Some(feedback.clone());
            }
            EngineDecision::Allow { .. } => {
                // Allow produces empty response for generic events
            }
            EngineDecision::Ask { reason: _ } => {
                // Per tactical advisory: Ask is a no-op for non-tool events
                // Log warning and treat as Allow
                tracing::warn!(
                    "Ask action not supported for {} - treating as Allow",
                    if suppress_output {
                        "event with suppress_output"
                    } else {
                        "generic event"
                    }
                );
                // Empty response (same as Allow)
            }
            EngineDecision::Modify { .. } => {
                // Modify is only meaningful for PreToolUse - treat as Allow for generic events
                tracing::warn!(
                    "Modify action not supported for generic events - treating as Allow"
                );
                // Empty response (same as Allow)
            }
        }

        // Apply suppress_output if requested
        if suppress_output {
            response.suppress_output = Some(true);
        }

        response
    }

    /// Build special response for PreCompact when injecting context
    /// PreCompact is unique - it outputs instructions to stdout, not JSON
    pub fn build_precompact(
        decision: &EngineDecision,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        // For PreCompact with context injection, we don't return a CupcakeResponse
        // The run/mod.rs handles this special case by printing to stdout
        // This method is here for completeness but shouldn't be called
        // when context injection is present

        if context_to_inject.is_some() && !context_to_inject.as_ref().unwrap().is_empty() {
            // This case should be handled specially in run/mod.rs
            // We return empty response as a fallback
            CupcakeResponse::empty()
        } else {
            // No context injection - use standard generic response
            Self::build(decision, suppress_output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_allow() {
        let decision = EngineDecision::Allow { reason: None };
        let response = GenericResponseBuilder::build(&decision, false);

        // Allow produces empty response
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
        assert_eq!(response.hook_specific_output, None);
        assert_eq!(response.decision, None);
        assert_eq!(response.reason, None);
    }

    #[test]
    fn test_generic_block() {
        let decision = EngineDecision::Block {
            feedback: "Notification rejected".to_string(),
        };
        let response = GenericResponseBuilder::build(&decision, false);

        assert_eq!(response.continue_execution, Some(false));
        assert_eq!(
            response.stop_reason,
            Some("Notification rejected".to_string())
        );
        assert_eq!(response.decision, None);
        assert_eq!(response.reason, None);
    }

    #[test]
    fn test_generic_with_suppress() {
        let decision = EngineDecision::Allow { reason: None };
        let response = GenericResponseBuilder::build(&decision, true);

        assert_eq!(response.suppress_output, Some(true));
    }

    #[test]
    fn test_precompact_no_injection() {
        let decision = EngineDecision::Allow { reason: None };
        let response = GenericResponseBuilder::build_precompact(&decision, None, false);

        // Should behave like generic response when no context
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_precompact_with_empty_injection() {
        let decision = EngineDecision::Allow { reason: None };
        let response = GenericResponseBuilder::build_precompact(&decision, Some(vec![]), false);

        // Empty context vector should behave like no context
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }
}
