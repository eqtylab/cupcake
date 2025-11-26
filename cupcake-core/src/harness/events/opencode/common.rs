use serde::{Deserialize, Serialize};

/// Common data fields present in all OpenCode events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommonOpenCodeData {
    /// Unique identifier for the session
    pub session_id: String,

    /// Current working directory where the tool is executed
    pub cwd: String,

    /// Optional agent identifier (e.g., "main", subagent name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Optional message identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

impl CommonOpenCodeData {
    /// Create new CommonOpenCodeData with required fields
    pub fn new(session_id: String, cwd: String) -> Self {
        Self {
            session_id,
            cwd,
            agent: None,
            message_id: None,
        }
    }

    /// Create new CommonOpenCodeData with all fields
    pub fn with_optional(
        session_id: String,
        cwd: String,
        agent: Option<String>,
        message_id: Option<String>,
    ) -> Self {
        Self {
            session_id,
            cwd,
            agent,
            message_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_data_new() {
        let data = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        assert_eq!(data.session_id, "session123");
        assert_eq!(data.cwd, "/home/user");
        assert_eq!(data.agent, None);
        assert_eq!(data.message_id, None);
    }

    #[test]
    fn test_common_data_with_optional() {
        let data = CommonOpenCodeData::with_optional(
            "session123".to_string(),
            "/home/user".to_string(),
            Some("main".to_string()),
            Some("msg456".to_string()),
        );
        assert_eq!(data.session_id, "session123");
        assert_eq!(data.cwd, "/home/user");
        assert_eq!(data.agent, Some("main".to_string()));
        assert_eq!(data.message_id, Some("msg456".to_string()));
    }

    #[test]
    fn test_common_data_serialization() {
        let data = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("session123"));
        assert!(json.contains("/home/user"));
        assert!(!json.contains("agent")); // Optional field should be omitted
    }

    #[test]
    fn test_common_data_deserialization() {
        let json = r#"{
            "session_id": "session123",
            "cwd": "/home/user"
        }"#;
        let data: CommonOpenCodeData = serde_json::from_str(json).unwrap();
        assert_eq!(data.session_id, "session123");
        assert_eq!(data.cwd, "/home/user");
        assert_eq!(data.agent, None);
    }
}
