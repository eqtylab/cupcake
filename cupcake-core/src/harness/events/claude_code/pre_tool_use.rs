use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};

/// Payload for PreToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUsePayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Name of the tool being called
    pub tool_name: String,

    /// Input parameters for the tool
    pub tool_input: serde_json::Value,
}

impl EventPayload for PreToolUsePayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl PreToolUsePayload {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::events::claude_code::BashToolInput;
    use serde_json::json;

    #[test]
    fn test_pre_tool_use_payload() {
        let payload = PreToolUsePayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            tool_name: "Bash".to_string(),
            tool_input: json!({
                "command": "echo 'Hello'",
                "timeout": 30
            }),
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert!(payload.is_tool("Bash"));
        assert!(!payload.is_tool("Read"));
        assert_eq!(payload.get_command(), Some("echo 'Hello'".to_string()));

        // Test parsing as specific tool type
        let bash_input: BashToolInput = payload.parse_tool_input().unwrap();
        assert_eq!(bash_input.command, "echo 'Hello'");
        assert_eq!(bash_input.timeout, Some(30));
    }
}
