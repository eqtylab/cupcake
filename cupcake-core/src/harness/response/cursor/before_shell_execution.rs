use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's beforeShellExecution hook
///
/// Supports full permission model:
/// {
///   "permission": "allow" | "deny" | "ask",
///   "userMessage"?: string,
///   "agentMessage"?: string,
///   "question"?: string  // Only for "ask" permission
/// }
pub fn build(decision: &EngineDecision) -> Value {
    match decision {
        EngineDecision::Allow { .. } => {
            json!({ "permission": "allow" })
        }
        EngineDecision::Block { feedback } => {
            json!({
                "permission": "deny",
                "userMessage": feedback,
                "agentMessage": feedback
            })
        }
        EngineDecision::Ask { reason } => {
            json!({
                "permission": "ask",
                "question": reason,
                "userMessage": reason,
                "agentMessage": reason
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_response() {
        let decision = EngineDecision::Allow { reason: None };
        let response = build(&decision);
        assert_eq!(response["permission"], "allow");
    }

    #[test]
    fn test_block_response() {
        let decision = EngineDecision::Block {
            feedback: "Dangerous command blocked".to_string(),
        };
        let response = build(&decision);
        assert_eq!(response["permission"], "deny");
        assert_eq!(response["userMessage"], "Dangerous command blocked");
        assert_eq!(response["agentMessage"], "Dangerous command blocked");
    }

    #[test]
    fn test_ask_response() {
        let decision = EngineDecision::Ask {
            reason: "Delete production database?".to_string(),
        };
        let response = build(&decision);
        assert_eq!(response["permission"], "ask");
        assert_eq!(response["question"], "Delete production database?");
        assert_eq!(response["userMessage"], "Delete production database?");
        assert_eq!(response["agentMessage"], "Delete production database?");
    }
}
