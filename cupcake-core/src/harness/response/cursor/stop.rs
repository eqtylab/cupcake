use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's stop hook
///
/// The stop event is fire-and-forget. Cursor's documentation
/// doesn't specify a response schema for this event.
///
/// We return an empty JSON object.
pub fn build(_decision: &EngineDecision, _agent_messages: Option<Vec<String>>) -> Value {
    // stop event is fire-and-forget
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
    }
}
