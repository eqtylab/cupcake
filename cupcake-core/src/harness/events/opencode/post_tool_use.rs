use super::common::CommonOpenCodeData;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    /// Whether the tool execution was successful
    pub success: bool,

    /// Standard output from the tool (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,

    /// Standard error from the tool (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Exit code (for command-line tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// PostToolUse event payload for OpenCode
/// Fired after a tool has been executed (tool.execute.after)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostToolUsePayload {
    /// Common event data
    #[serde(flatten)]
    pub common: CommonOpenCodeData,

    /// Tool name (e.g., "bash", "edit", "read", "write")
    pub tool: String,

    /// Tool-specific arguments as a JSON value
    pub args: Value,

    /// Execution result
    pub result: ToolResult,
}

impl PostToolUsePayload {
    /// Create a new PostToolUsePayload
    pub fn new(common: CommonOpenCodeData, tool: String, args: Value, result: ToolResult) -> Self {
        Self {
            common,
            tool,
            args,
            result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult {
            success: true,
            output: Some("Hello, World!".to_string()),
            error: None,
            exit_code: Some(0),
        };

        assert!(result.success);
        assert_eq!(result.output, Some("Hello, World!".to_string()));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_tool_result_failure() {
        let result = ToolResult {
            success: false,
            output: None,
            error: Some("Command failed".to_string()),
            exit_code: Some(1),
        };

        assert!(!result.success);
        assert_eq!(result.error, Some("Command failed".to_string()));
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn test_post_tool_use_creation() {
        let common = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        let args = serde_json::json!({"command": "npm test"});
        let result = ToolResult {
            success: true,
            output: Some("All tests passed".to_string()),
            error: None,
            exit_code: Some(0),
        };

        let payload = PostToolUsePayload::new(
            common.clone(),
            "bash".to_string(),
            args.clone(),
            result.clone(),
        );

        assert_eq!(payload.common.session_id, "session123");
        assert_eq!(payload.tool, "bash");
        assert_eq!(payload.args, args);
        assert!(payload.result.success);
    }

    #[test]
    fn test_post_tool_use_serialization() {
        let common = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        let args = serde_json::json!({"command": "cargo test"});
        let result = ToolResult {
            success: false,
            output: None,
            error: Some("Test failed".to_string()),
            exit_code: Some(1),
        };

        let payload = PostToolUsePayload::new(common, "bash".to_string(), args, result);
        let json = serde_json::to_string(&payload).unwrap();

        assert!(json.contains("session123"));
        assert!(json.contains("bash"));
        assert!(json.contains("Test failed"));
        assert!(json.contains("\"success\":false"));
    }

    #[test]
    fn test_post_tool_use_deserialization() {
        let json = r#"{
            "session_id": "session123",
            "cwd": "/home/user",
            "tool": "bash",
            "args": {"command": "echo test"},
            "result": {
                "success": true,
                "output": "test",
                "exit_code": 0
            }
        }"#;

        let payload: PostToolUsePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.common.session_id, "session123");
        assert_eq!(payload.tool, "bash");
        assert!(payload.result.success);
        assert_eq!(payload.result.output, Some("test".to_string()));
        assert_eq!(payload.result.exit_code, Some(0));
    }
}
