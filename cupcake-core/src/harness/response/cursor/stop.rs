use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's stop hook
///
/// Cursor's stop hook supports an optional `followup_message` field:
/// ```json
/// {
///   "followup_message": "<message text>"  // optional
/// }
/// ```
///
/// When `followup_message` is provided and non-empty, Cursor will automatically
/// submit it as the next user message, enabling agent loop workflows.
/// Maximum of 5 auto follow-ups enforced by Cursor.
///
/// ## Decision Mapping
///
/// - `Block` → Returns `followup_message` with the feedback text, causing Cursor
///   to continue the agent loop with that message as the next user input.
/// - `Allow` → Returns empty `{}`, allowing the agent to stop normally.
///
/// This mirrors Claude Code's Stop hook behavior where `block` + `reason` prevents
/// the agent from stopping. The difference is mechanical: Claude Code injects the
/// reason as feedback, while Cursor submits it as a new user message.
///
/// ## Loop Prevention
///
/// Cursor enforces a maximum of 5 auto follow-ups via the `loop_count` input field.
/// Policies should check `input.loop_count < 5` before blocking to avoid hitting
/// the limit unexpectedly.
pub fn build(decision: &EngineDecision, _agent_messages: Option<Vec<String>>) -> Value {
    match decision {
        EngineDecision::Block { feedback } => {
            // Block on stop = submit followup message to continue the agent loop
            json!({
                "followup_message": feedback
            })
        }
        EngineDecision::Allow { .. }
        | EngineDecision::Ask { .. }
        | EngineDecision::Modify { .. } => {
            // Allow the agent to stop normally
            json!({})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_returns_empty() {
        let decision = EngineDecision::Allow { reason: None };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));
    }

    #[test]
    fn test_allow_with_reason_returns_empty() {
        // Even with a reason, Allow should return empty (let agent stop)
        let decision = EngineDecision::Allow {
            reason: Some("Task completed successfully".to_string()),
        };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));
    }

    #[test]
    fn test_block_returns_followup_message() {
        let decision = EngineDecision::Block {
            feedback: "Tests are still failing. Please fix them before stopping.".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(
            response,
            json!({
                "followup_message": "Tests are still failing. Please fix them before stopping."
            })
        );
    }

    #[test]
    fn test_ask_returns_empty() {
        // Ask is not supported for stop events - treat as allow
        let decision = EngineDecision::Ask {
            reason: "Should I continue?".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));
    }

    #[test]
    fn test_modify_returns_empty() {
        // Modify is not applicable for stop events - treat as allow
        let decision = EngineDecision::Modify {
            reason: "Modified".to_string(),
            updated_input: json!({}),
        };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));
    }
}
