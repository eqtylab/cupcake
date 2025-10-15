use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};
use tracing::debug;

/// Build response for Cursor's beforeSubmitPrompt hook
///
/// CRITICAL: Cursor's beforeSubmitPrompt does NOT support context injection.
/// It only accepts: { "continue": true | false }
///
/// Unlike Claude Code's UserPromptSubmit which supports `additionalContext`,
/// Cursor's implementation is limited to a boolean continue flag.
pub fn build(decision: &EngineDecision, _agent_messages: Option<Vec<String>>) -> Value {
    // Log if context would have been injected (for debugging)
    if let EngineDecision::Allow { reason } = decision {
        if let Some(ref msg) = reason {
            if !msg.is_empty() {
                debug!(
                    "Context injection not supported by Cursor's beforeSubmitPrompt; dropping context: {}",
                    msg
                );
            }
        }
    }

    match decision {
        EngineDecision::Allow { .. } => {
            // Allow prompt to continue
            json!({ "continue": true })
        }
        EngineDecision::Block { .. } | EngineDecision::Ask { .. } => {
            // Block prompt submission
            // Note: Ask is treated as block since we can't prompt user at this stage
            if matches!(decision, EngineDecision::Ask { .. }) {
                debug!("Ask decision on beforeSubmitPrompt not supported by Cursor; blocking instead");
            }
            json!({ "continue": false })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_response() {
        let decision = EngineDecision::Allow { reason: None };
        let response = build(&decision, None);
        assert_eq!(response["continue"], true);
    }

    #[test]
    fn test_allow_with_context_drops_it() {
        let decision = EngineDecision::Allow {
            reason: Some("Some context to inject".to_string()),
        };
        let response = build(&decision, None);
        // Context is dropped - only continue field is present
        assert_eq!(response["continue"], true);
        assert!(response.get("additionalContext").is_none());
    }

    #[test]
    fn test_block_response() {
        let decision = EngineDecision::Block {
            feedback: "Blocked".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response["continue"], false);
    }

    #[test]
    fn test_ask_converts_to_block() {
        let decision = EngineDecision::Ask {
            reason: "Question?".to_string(),
        };
        let response = build(&decision, None);
        // Ask is treated as block for prompt events
        assert_eq!(response["continue"], false);
    }
}
