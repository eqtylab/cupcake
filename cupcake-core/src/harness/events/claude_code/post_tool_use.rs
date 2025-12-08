//! PostToolUse event payload

use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Payload for PostToolUse events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUsePayload {
    /// Common event data
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Name of the tool that was executed
    pub tool_name: String,

    /// Input parameters that were provided to the tool
    pub tool_input: Value,

    /// Response from the tool execution
    pub tool_response: Value,

    /// Unique identifier for this tool invocation
    #[serde(default)]
    pub tool_use_id: Option<String>,
}

impl EventPayload for PostToolUsePayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl PostToolUsePayload {
    /// Check if the tool execution was successful
    pub fn was_successful(&self) -> Option<bool> {
        self.tool_response.get("success").and_then(|v| v.as_bool())
    }

    /// Get the tool output if available
    pub fn get_output(&self) -> Option<&str> {
        self.tool_response.get("output").and_then(|v| v.as_str())
    }

    /// Get error message if the tool failed
    pub fn get_error(&self) -> Option<&str> {
        self.tool_response.get("error").and_then(|v| v.as_str())
    }

    /// Get the unique tool use ID if present
    pub fn tool_use_id(&self) -> Option<&str> {
        self.tool_use_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_post_tool_use_payload() {
        let payload = PostToolUsePayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "echo hello"}),
            tool_response: json!({"success": true, "output": "hello\n"}),
            tool_use_id: Some("toolu_xyz789".to_string()),
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert_eq!(payload.tool_name, "Bash");
        assert_eq!(payload.was_successful(), Some(true));
        assert_eq!(payload.get_output(), Some("hello\n"));
        assert_eq!(payload.get_error(), None);
        assert_eq!(payload.tool_use_id(), Some("toolu_xyz789"));
    }

    #[test]
    fn test_post_tool_use_failed() {
        let payload = PostToolUsePayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "exit 1"}),
            tool_response: json!({"success": false, "error": "Command failed"}),
            tool_use_id: None,
        };

        assert_eq!(payload.was_successful(), Some(false));
        assert_eq!(payload.get_error(), Some("Command failed"));
        assert_eq!(payload.tool_use_id(), None);
    }
}
