use serde::{Deserialize, Serialize};

/// Response from Cupcake to OpenCode plugin
///
/// Unlike Claude Code's complex response format, OpenCode requires a simple JSON response.
/// The plugin will interpret this and either throw an error (block) or return (allow).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenCodeResponse {
    /// Decision type: "allow", "deny", "block", or "ask"
    pub decision: String,

    /// Human-readable explanation of the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Optional context strings for potential injection
    /// (Phase 2 feature - may not be supported initially)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>,
}

impl OpenCodeResponse {
    /// Create an "allow" response
    pub fn allow() -> Self {
        Self {
            decision: "allow".to_string(),
            reason: None,
            context: None,
        }
    }

    /// Create an "allow" response with context
    pub fn allow_with_context(context: Vec<String>) -> Self {
        Self {
            decision: "allow".to_string(),
            reason: None,
            context: Some(context),
        }
    }

    /// Create a "deny" response with reason
    pub fn deny(reason: String) -> Self {
        Self {
            decision: "deny".to_string(),
            reason: Some(reason),
            context: None,
        }
    }

    /// Create a "block" response with reason
    pub fn block(reason: String) -> Self {
        Self {
            decision: "block".to_string(),
            reason: Some(reason),
            context: None,
        }
    }

    /// Create an "ask" response with reason
    /// Note: OpenCode plugin will convert this to deny with approval message
    pub fn ask(reason: String) -> Self {
        Self {
            decision: "ask".to_string(),
            reason: Some(reason),
            context: None,
        }
    }

    /// Convert to JSON value for stdout output
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("OpenCodeResponse should always serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_response() {
        let response = OpenCodeResponse::allow();
        assert_eq!(response.decision, "allow");
        assert_eq!(response.reason, None);
        assert_eq!(response.context, None);
    }

    #[test]
    fn test_allow_with_context() {
        let context = vec!["Context line 1".to_string(), "Context line 2".to_string()];
        let response = OpenCodeResponse::allow_with_context(context.clone());
        assert_eq!(response.decision, "allow");
        assert_eq!(response.context, Some(context));
    }

    #[test]
    fn test_deny_response() {
        let response = OpenCodeResponse::deny("Permission denied".to_string());
        assert_eq!(response.decision, "deny");
        assert_eq!(response.reason, Some("Permission denied".to_string()));
    }

    #[test]
    fn test_block_response() {
        let response = OpenCodeResponse::block("Policy violation".to_string());
        assert_eq!(response.decision, "block");
        assert_eq!(response.reason, Some("Policy violation".to_string()));
    }

    #[test]
    fn test_ask_response() {
        let response = OpenCodeResponse::ask("Approval required".to_string());
        assert_eq!(response.decision, "ask");
        assert_eq!(response.reason, Some("Approval required".to_string()));
    }

    #[test]
    fn test_serialization() {
        let response = OpenCodeResponse::deny("Test reason".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"decision\":\"deny\""));
        assert!(json.contains("Test reason"));
    }

    #[test]
    fn test_deserialization() {
        let json = r#"{
            "decision": "allow",
            "context": ["line1", "line2"]
        }"#;
        let response: OpenCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.decision, "allow");
        assert_eq!(
            response.context,
            Some(vec!["line1".to_string(), "line2".to_string()])
        );
    }

    #[test]
    fn test_to_json_value() {
        let response = OpenCodeResponse::deny("Test".to_string());
        let value = response.to_json_value();
        assert_eq!(value["decision"], "deny");
        assert_eq!(value["reason"], "Test");
    }

    #[test]
    fn test_optional_fields_not_serialized() {
        let response = OpenCodeResponse::allow();
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("reason"));
        assert!(!json.contains("context"));
    }
}
