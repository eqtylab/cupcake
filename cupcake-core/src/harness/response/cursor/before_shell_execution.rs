use crate::harness::response::types::EngineDecision;
use serde_json::{json, Value};

/// Build response for Cursor's beforeShellExecution hook
///
/// Supports full permission model:
/// {
///   "permission": "allow" | "deny" | "ask",
///   "user_message"?: string,
///   "agent_message"?: string,
///   "question"?: string  // Only for "ask" permission
/// }
///
/// agent_messages: Optional technical details for the agent (separate from user message)
///                If provided, these are joined with "; " and used for agent_message field
///                If not provided, defaults to using the same message for both user and agent
pub fn build(decision: &EngineDecision, agent_messages: Option<Vec<String>>) -> Value {
    match decision {
        EngineDecision::Allow { .. } => {
            json!({ "permission": "allow" })
        }
        EngineDecision::Block { feedback } => {
            // Use agent_messages if provided, otherwise duplicate user_message
            let agent_message = agent_messages
                .as_ref()
                .filter(|msgs| !msgs.is_empty())
                .map(|msgs| msgs.join("; "))
                .unwrap_or_else(|| feedback.clone());

            json!({
                "permission": "deny",
                "user_message": feedback,
                "agent_message": agent_message
            })
        }
        EngineDecision::Ask { reason } => {
            // Use agent_messages if provided, otherwise duplicate user_message
            let agent_message = agent_messages
                .as_ref()
                .filter(|msgs| !msgs.is_empty())
                .map(|msgs| msgs.join("; "))
                .unwrap_or_else(|| reason.clone());

            json!({
                "permission": "ask",
                "question": reason,
                "user_message": reason,
                "agent_message": agent_message
            })
        }
        EngineDecision::Modify { .. } => {
            // Cursor doesn't support updatedInput - treat Modify as Allow
            json!({ "permission": "allow" })
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
            feedback: "Dangerous command blocked".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response["permission"], "deny");
        assert_eq!(response["user_message"], "Dangerous command blocked");
        assert_eq!(response["agent_message"], "Dangerous command blocked");
    }

    #[test]
    fn test_block_response_with_agent_messages() {
        let decision = EngineDecision::Block {
            feedback: "Command blocked".to_string(),
        };
        let agent_messages = Some(vec![
            "rm -rf / detected on line 42".to_string(),
            "Use 'trash' command instead".to_string(),
        ]);
        let response = build(&decision, agent_messages);
        assert_eq!(response["permission"], "deny");
        assert_eq!(response["user_message"], "Command blocked");
        assert_eq!(
            response["agent_message"],
            "rm -rf / detected on line 42; Use 'trash' command instead"
        );
    }

    #[test]
    fn test_ask_response() {
        let decision = EngineDecision::Ask {
            reason: "Delete production database?".to_string(),
        };
        let response = build(&decision, None);
        assert_eq!(response["permission"], "ask");
        assert_eq!(response["question"], "Delete production database?");
        assert_eq!(response["user_message"], "Delete production database?");
        assert_eq!(response["agent_message"], "Delete production database?");
    }

    #[test]
    fn test_ask_response_with_agent_messages() {
        let decision = EngineDecision::Ask {
            reason: "Allow dangerous operation?".to_string(),
        };
        let agent_messages = Some(vec![
            "This will delete all data in /tmp".to_string(),
            "See policy DANGER-001 for details".to_string(),
        ]);
        let response = build(&decision, agent_messages);
        assert_eq!(response["permission"], "ask");
        assert_eq!(response["question"], "Allow dangerous operation?");
        assert_eq!(response["user_message"], "Allow dangerous operation?");
        assert_eq!(
            response["agent_message"],
            "This will delete all data in /tmp; See policy DANGER-001 for details"
        );
    }
}
