use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};

/// Payload for PermissionRequest hook events
///
/// PermissionRequest is the newer, cleaner hook for tool permission decisions.
/// It provides the same functionality as PreToolUse but with:
/// - A `tool_use_id` field for tracking specific tool invocations
/// - A cleaner response format with nested `decision` object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Name of the tool being called
    pub tool_name: String,

    /// Input parameters for the tool
    pub tool_input: serde_json::Value,

    /// Unique identifier for this tool invocation
    pub tool_use_id: String,
}

impl EventPayload for PermissionRequestPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl PermissionRequestPayload {
    /// Extract tool input as a specific type
    pub fn parse_tool_input<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.tool_input.clone())
    }

    /// Check if this is a specific tool
    pub fn is_tool(&self, name: &str) -> bool {
        self.tool_name == name
    }

    /// Get tool input as a string if it's a simple command
    pub fn get_command(&self) -> Option<String> {
        self.tool_input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get file path from tool input if present
    pub fn get_file_path(&self) -> Option<String> {
        self.tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get the tool use ID
    pub fn tool_use_id(&self) -> &str {
        &self.tool_use_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::events::claude_code::BashToolInput;
    use serde_json::json;

    #[test]
    fn test_permission_request_payload() {
        let payload = PermissionRequestPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: json!({
                "command": "echo 'Hello'",
                "timeout": 30
            }),
            tool_use_id: "toolu_abc123".to_string(),
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert!(payload.is_tool("Bash"));
        assert!(!payload.is_tool("Read"));
        assert_eq!(payload.get_command(), Some("echo 'Hello'".to_string()));
        assert_eq!(payload.tool_use_id(), "toolu_abc123");

        // Test parsing as specific tool type
        let bash_input: BashToolInput = payload.parse_tool_input().unwrap();
        assert_eq!(bash_input.command, "echo 'Hello'");
        assert_eq!(bash_input.timeout, Some(30));
    }

    #[test]
    fn test_permission_request_deserialization() {
        let json = r#"
        {
            "hook_event_name": "PermissionRequest",
            "session_id": "test-session",
            "transcript_path": "/path/to/transcript",
            "cwd": "/home/user/project",
            "tool_name": "Bash",
            "tool_input": {"command": "rm -rf /tmp"},
            "tool_use_id": "toolu_123"
        }
        "#;

        let payload: PermissionRequestPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.common.session_id, "test-session");
        assert_eq!(payload.tool_name, "Bash");
        assert_eq!(payload.tool_use_id, "toolu_123");
        assert_eq!(payload.tool_input["command"], "rm -rf /tmp");
    }
}
