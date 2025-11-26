use super::common::CommonOpenCodeData;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// PreToolUse event payload for OpenCode
/// Fired before a tool is executed (tool.execute.before)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreToolUsePayload {
    /// Common event data
    #[serde(flatten)]
    pub common: CommonOpenCodeData,

    /// Tool name (e.g., "bash", "edit", "read", "write")
    pub tool: String,

    /// Tool-specific arguments as a JSON value
    pub args: Value,
}

impl PreToolUsePayload {
    /// Create a new PreToolUsePayload
    pub fn new(common: CommonOpenCodeData, tool: String, args: Value) -> Self {
        Self { common, tool, args }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_tool_use_creation() {
        let common = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        let args = serde_json::json!({"command": "ls -la"});
        let payload = PreToolUsePayload::new(common.clone(), "bash".to_string(), args.clone());

        assert_eq!(payload.common.session_id, "session123");
        assert_eq!(payload.tool, "bash");
        assert_eq!(payload.args, args);
    }

    #[test]
    fn test_pre_tool_use_serialization() {
        let common = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        let args = serde_json::json!({"command": "git status"});
        let payload = PreToolUsePayload::new(common, "bash".to_string(), args);

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("session123"));
        assert!(json.contains("bash"));
        assert!(json.contains("git status"));
    }

    #[test]
    fn test_pre_tool_use_deserialization() {
        let json = r#"{
            "session_id": "session123",
            "cwd": "/home/user",
            "tool": "bash",
            "args": {"command": "ls"}
        }"#;

        let payload: PreToolUsePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.common.session_id, "session123");
        assert_eq!(payload.common.cwd, "/home/user");
        assert_eq!(payload.tool, "bash");
        assert_eq!(payload.args["command"], "ls");
    }

    #[test]
    fn test_pre_tool_use_with_optional_fields() {
        let json = r#"{
            "session_id": "session123",
            "cwd": "/home/user",
            "agent": "main",
            "message_id": "msg456",
            "tool": "edit",
            "args": {
                "filePath": "/path/to/file.ts",
                "oldString": "old",
                "newString": "new"
            }
        }"#;

        let payload: PreToolUsePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.common.agent, Some("main".to_string()));
        assert_eq!(payload.common.message_id, Some("msg456".to_string()));
        assert_eq!(payload.tool, "edit");
        assert_eq!(payload.args["filePath"], "/path/to/file.ts");
    }
}
