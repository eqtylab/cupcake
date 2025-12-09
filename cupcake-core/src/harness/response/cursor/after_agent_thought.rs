use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's afterAgentThought hook
///
/// This is a fire-and-forget event for observing agent reasoning.
/// No response schema is documented - we return an empty object.
pub fn build(_decision: &EngineDecision, _agent_messages: Option<Vec<String>>) -> Value {
    json!({})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_empty() {
        let decision = EngineDecision::Allow { reason: None };
        let response = build(&decision, None);
        assert_eq!(response, json!({}));
    }
}
