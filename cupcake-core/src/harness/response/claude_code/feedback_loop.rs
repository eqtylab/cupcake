use crate::harness::events::claude_code::ClaudeCodeEvent;
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
        hook_event: &ClaudeCodeEvent,
        suppress_output: bool,
    ) -> CupcakeResponse {
        let mut response = CupcakeResponse::empty();

        // Feedback loop events use decision/reason for blocks
        match decision {
            EngineDecision::Block { feedback } => {
                response.decision = Some("block".to_string());
                response.reason = Some(feedback.clone());
            }
            EngineDecision::Allow { .. } | EngineDecision::Ask { .. } | EngineDecision::Modify { .. } => {
                // Modify is only meaningful for PreToolUse - treat as Allow for feedback events
                if matches!(decision, EngineDecision::Modify { .. }) {
                    tracing::warn!("Modify action not supported for feedback loop events - treating as Allow");
                }
                // Only PostToolUse supports context injection
                match hook_event {
                    ClaudeCodeEvent::PostToolUse(_) => {
                        if let Some(contexts) = context_to_inject {
                            if !contexts.is_empty() {
                                response.hook_specific_output =
                                    Some(HookSpecificOutput::PostToolUse {
                                        additional_context: Some(contexts.join("\n")),
                                    });
                            }
                        }
                    }
                    ClaudeCodeEvent::Stop(_) | ClaudeCodeEvent::SubagentStop(_) => {
                        // These events don't support context injection
                    }
                    _ => {
                        // This builder should only be called for feedback loop events
                        unreachable!(
                            "FeedbackLoopResponseBuilder called with non-feedback-loop event: {}",
                            hook_event.event_name()
                        )
                    }
                }
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
    use crate::harness::events::claude_code::{
        CommonEventData, PostToolUsePayload, StopPayload, SubagentStopPayload,
    };
    use serde_json::json;

    fn create_common_data() -> CommonEventData {
        CommonEventData {
            session_id: "test".to_string(),
            transcript_path: "/test".to_string(),
            cwd: "/test".to_string(),
        }
    }

    #[test]
    fn test_feedback_loop_block() {
        let decision = EngineDecision::Block {
            feedback: "Output format incorrect - please return JSON".to_string(),
        };
        let event = ClaudeCodeEvent::PostToolUse(PostToolUsePayload {
            common: create_common_data(),
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "ls"}),
            tool_response: json!({"success": true}),
        });
        let response = FeedbackLoopResponseBuilder::build(&decision, None, &event, false);

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
        let event = ClaudeCodeEvent::Stop(StopPayload {
            common: create_common_data(),
            stop_hook_active: false,
        });
        let response = FeedbackLoopResponseBuilder::build(&decision, None, &event, false);

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
        let event = ClaudeCodeEvent::PostToolUse(PostToolUsePayload {
            common: create_common_data(),
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "cat file.txt"}),
            tool_response: json!({"success": true}),
        });
        let response = FeedbackLoopResponseBuilder::build(&decision, Some(context), &event, false);

        // Test Rust struct correctness
        match &response.hook_specific_output {
            Some(HookSpecificOutput::PostToolUse { additional_context }) => {
                assert_eq!(
                    additional_context,
                    &Some("File contains TODO on line 45".to_string())
                );
            }
            _ => panic!("Expected PostToolUse hook output"),
        }

        // Test JSON wire format matches Claude Code hook contract
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json["hookSpecificOutput"]["hookEventName"], "PostToolUse",
            "hookEventName field should be 'PostToolUse'"
        );
        assert_eq!(
            json["hookSpecificOutput"]["additionalContext"], "File contains TODO on line 45",
            "additionalContext should contain the injected context"
        );
    }

    #[test]
    fn test_feedback_loop_with_suppress() {
        let decision = EngineDecision::Block {
            feedback: "Task incomplete".to_string(),
        };
        let event = ClaudeCodeEvent::SubagentStop(SubagentStopPayload {
            common: create_common_data(),
            stop_hook_active: false,
        });
        let response = FeedbackLoopResponseBuilder::build(&decision, None, &event, true);

        assert_eq!(response.decision, Some("block".to_string()));
        assert_eq!(response.reason, Some("Task incomplete".to_string()));
        assert_eq!(response.suppress_output, Some(true));
    }

    #[test]
    fn test_feedback_loop_events() {
        // Test all three feedback loop event types
        let events = vec![
            (
                "PostToolUse",
                ClaudeCodeEvent::PostToolUse(PostToolUsePayload {
                    common: create_common_data(),
                    tool_name: "Bash".to_string(),
                    tool_input: json!({"command": "ls"}),
                    tool_response: json!({"success": true}),
                }),
            ),
            (
                "Stop",
                ClaudeCodeEvent::Stop(StopPayload {
                    common: create_common_data(),
                    stop_hook_active: false,
                }),
            ),
            (
                "SubagentStop",
                ClaudeCodeEvent::SubagentStop(SubagentStopPayload {
                    common: create_common_data(),
                    stop_hook_active: false,
                }),
            ),
        ];

        for (name, event) in events {
            let decision = EngineDecision::Block {
                feedback: format!("Feedback for {name}"),
            };
            let response = FeedbackLoopResponseBuilder::build(&decision, None, &event, false);

            assert_eq!(response.decision, Some("block".to_string()));
            assert_eq!(response.reason, Some(format!("Feedback for {name}")));
        }
    }
}
