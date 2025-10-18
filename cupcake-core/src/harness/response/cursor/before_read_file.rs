use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's beforeReadFile hook
///
/// Cursor's beforeReadFile has a minimal schema:
/// { "permission": "allow" | "deny" }
///
/// No userMessage or agentMessage fields are supported.
pub fn build(decision: &EngineDecision, _agent_messages: Option<Vec<String>>) -> Value {
    match decision {
        EngineDecision::Allow { .. } => {
            json!({ "permission": "allow" })
        }
        EngineDecision::Block { .. } | EngineDecision::Ask { .. } => {
            // Both Block and Ask are treated as deny for file reads
            json!({ "permission": "deny" })
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
        assert_eq!(response["permission"], "allow");
    }

    #[test]
    fn test_block_response() {
        let decision = EngineDecision::Block {
            feedback: "Sensitive file".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response["permission"], "deny");
    }

    #[test]
    fn test_ask_converts_to_deny() {
        let decision = EngineDecision::Ask {
            reason: "Allow read?".to_string(),
        };
        let response = build(&decision, None);
        // Ask is treated as deny for file reads
        assert_eq!(response["permission"], "deny");
    }
}
