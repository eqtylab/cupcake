use crate::harness::response::types::{CupcakeResponse, EngineDecision, HookSpecificOutput};

/// Builder for feedback loop hook responses
///
/// PostToolUse, Stop, and SubagentStop events use decision/reason fields
/// for Claude's self-correction feedback loops
pub struct FeedbackLoopResponseBuilder;

impl FeedbackLoopResponseBuilder {
    /// Build response for feedback loop events
    pub fn build(
        decision: &EngineDecision,
        context_to_inject: Option<Vec<String>>,
        hook_event: &str,
        suppress_output: bool,
    ) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        // Feedback loop events use decision/reason for blocks
        match decision {
            EngineDecision::Block { feedback } => {
                response.decision = Some("block".to_string());
                response.reason = Some(feedback.clone());
            }
            EngineDecision::Allow { .. } | EngineDecision::Ask { .. } => {
                // For PostToolUse, allow context injection
                if hook_event == "PostToolUse" {
                    if let Some(contexts) = context_to_inject {
                        if !contexts.is_empty() {
                            response.hook_specific_output = Some(HookSpecificOutput::PostToolUse {
                                additional_context: Some(contexts.join("\n")),
                            });
                        }
                    }
                }
                // Stop and SubagentStop don't support context injection
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
    fn test_feedback_loop_block() {
        let decision = EngineDecision::Block {
            feedback: "Output format incorrect - please return JSON".to_string(),
        };
        let response = FeedbackLoopResponseBuilder::build(&decision, None, "PostToolUse", false);

        assert_eq!(response.decision, Some("block".to_string()));
        assert_eq!(
            response.reason,
            Some("Output format incorrect - please return JSON".to_string())
        );
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_feedback_loop_allow() {
        let decision = EngineDecision::Allow { reason: None };
        let response = FeedbackLoopResponseBuilder::build(&decision, None, "Stop", false);

        // Allow produces empty response
        assert_eq!(response.decision, None);
        assert_eq!(response.reason, None);
        assert_eq!(response.continue_execution, None);
        assert_eq!(response.stop_reason, None);
    }

    #[test]
    fn test_post_tool_use_with_context() {
        let decision = EngineDecision::Allow { reason: None };
        let context = vec!["File contains TODO on line 45".to_string()];
        let response = FeedbackLoopResponseBuilder::build(&decision, Some(context), "PostToolUse", false);

        match response.hook_specific_output {
            Some(HookSpecificOutput::PostToolUse { additional_context }) => {
                assert_eq!(additional_context, Some("File contains TODO on line 45".to_string()));
            }
            _ => panic!("Expected PostToolUse hook output"),
        }
    }

    #[test]
    fn test_feedback_loop_with_suppress() {
        let decision = EngineDecision::Block {
            feedback: "Task incomplete".to_string(),
        };
        let response = FeedbackLoopResponseBuilder::build(&decision, None, "SubagentStop", true);

        assert_eq!(response.decision, Some("block".to_string()));
        assert_eq!(response.reason, Some("Task incomplete".to_string()));
        assert_eq!(response.suppress_output, Some(true));
    }

    #[test]
    fn test_feedback_loop_events() {
        // Test all three feedback loop event types
        for event in &["PostToolUse", "Stop", "SubagentStop"] {
            let decision = EngineDecision::Block {
                feedback: format!("Feedback for {event}"),
            };
            let response = FeedbackLoopResponseBuilder::build(&decision, None, event, false);

            assert_eq!(response.decision, Some("block".to_string()));
            assert_eq!(response.reason, Some(format!("Feedback for {event}")));
        }
    }
}
