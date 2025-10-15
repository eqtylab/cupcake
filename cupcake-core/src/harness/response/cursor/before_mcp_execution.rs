use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's beforeMCPExecution hook
///
/// Uses the same schema as beforeShellExecution:
/// {
///   "permission": "allow" | "deny" | "ask",
///   "userMessage"?: string,
///   "agentMessage"?: string,
///   "question"?: string
/// }
pub fn build(decision: &EngineDecision, agent_messages: Option<Vec<String>>) -> Value {
    match decision {
        EngineDecision::Allow { .. } => {
            json!({ "permission": "allow" })
        }
        EngineDecision::Block { feedback } => {
            let agent_message = agent_messages
                .as_ref()
                .filter(|msgs| !msgs.is_empty())
                .map(|msgs| msgs.join("; "))
                .unwrap_or_else(|| feedback.clone());

            json!({
                "permission": "deny",
                "userMessage": feedback,
                "agentMessage": agent_message
            })
        }
        EngineDecision::Ask { reason } => {
            let agent_message = agent_messages
                .as_ref()
                .filter(|msgs| !msgs.is_empty())
                .map(|msgs| msgs.join("; "))
                .unwrap_or_else(|| reason.clone());

            json!({
                "permission": "ask",
                "question": reason,
                "userMessage": reason,
                "agentMessage": agent_message
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
        let response = build(&decision, None);
        assert_eq!(response["permission"], "allow");
    }

    #[test]
    fn test_block_response() {
        let decision = EngineDecision::Block {
            feedback: "MCP tool blocked".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response["permission"], "deny");
        assert_eq!(response["userMessage"], "MCP tool blocked");
    }

    #[test]
    fn test_ask_response() {
        let decision = EngineDecision::Ask {
            reason: "Allow MCP tool?".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response["permission"], "ask");
        assert_eq!(response["question"], "Allow MCP tool?");
    }
}
