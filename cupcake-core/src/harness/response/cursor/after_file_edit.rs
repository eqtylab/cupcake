use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's afterFileEdit hook
///
/// Note: This is a post-execution event, so no output is typically expected.
/// Cursor's documentation doesn't specify a response schema for this event.
///
/// We return an empty JSON object for now.
pub fn build(_decision: &EngineDecision, _agent_messages: Option<Vec<String>>) -> Value {
    // afterFileEdit is fire-and-forget in Cursor
    // No response schema documented
    json!({})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_decision_returns_empty() {
        let decision = EngineDecision::Allow { reason: None };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));

        let decision = EngineDecision::Block {
            feedback: "Test".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));
    }
}
