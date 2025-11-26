use serde::{Deserialize, Serialize};

mod common;
mod post_tool_use;
mod pre_tool_use;

pub use common::CommonOpenCodeData;
pub use post_tool_use::{PostToolUsePayload, ToolResult};
pub use pre_tool_use::PreToolUsePayload;

/// All possible OpenCode hook events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "hook_event_name", rename_all = "PascalCase")]
pub enum OpenCodeEvent {
    /// Before tool execution (tool.execute.before)
    PreToolUse(PreToolUsePayload),

    /// After tool execution (tool.execute.after)
    PostToolUse(PostToolUsePayload),
}

impl OpenCodeEvent {
    /// Get the common event data
    pub fn common(&self) -> &CommonOpenCodeData {
        match self {
            OpenCodeEvent::PreToolUse(payload) => &payload.common,
            OpenCodeEvent::PostToolUse(payload) => &payload.common,
        }
    }

    /// Get the tool name for tool-related events
    pub fn tool(&self) -> &str {
        match self {
            OpenCodeEvent::PreToolUse(payload) => &payload.tool,
            OpenCodeEvent::PostToolUse(payload) => &payload.tool,
        }
    }

    /// Get the tool input/args
    pub fn args(&self) -> &serde_json::Value {
        match self {
            OpenCodeEvent::PreToolUse(payload) => &payload.args,
            OpenCodeEvent::PostToolUse(payload) => &payload.args,
        }
    }

    /// Get the event name as a string
    pub fn event_name(&self) -> &'static str {
        match self {
            OpenCodeEvent::PreToolUse(_) => "PreToolUse",
            OpenCodeEvent::PostToolUse(_) => "PostToolUse",
        }
    }

    /// Check if this is a PreToolUse event
    pub fn is_pre_tool_use(&self) -> bool {
        matches!(self, OpenCodeEvent::PreToolUse(_))
    }

    /// Check if this is a PostToolUse event
    pub fn is_post_tool_use(&self) -> bool {
        matches!(self, OpenCodeEvent::PostToolUse(_))
    }

    /// Parse tool input as specific tool type
    pub fn parse_args<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.args().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_tool_use_event() {
        let json = r#"{
            "hook_event_name": "PreToolUse",
            "session_id": "session123",
            "cwd": "/home/user",
            "tool": "bash",
            "args": {"command": "ls"}
        }"#;

        let event: OpenCodeEvent = serde_json::from_str(json).unwrap();
        assert!(event.is_pre_tool_use());
        assert_eq!(event.event_name(), "PreToolUse");
        assert_eq!(event.tool(), "bash");
        assert_eq!(event.common().session_id, "session123");
    }

    #[test]
    fn test_post_tool_use_event() {
        let json = r#"{
            "hook_event_name": "PostToolUse",
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

        let event: OpenCodeEvent = serde_json::from_str(json).unwrap();
        assert!(event.is_post_tool_use());
        assert_eq!(event.event_name(), "PostToolUse");
        assert_eq!(event.tool(), "bash");
    }

    #[test]
    fn test_parse_args() {
        #[derive(Deserialize)]
        struct BashArgs {
            command: String,
        }

        let json = r#"{
            "hook_event_name": "PreToolUse",
            "session_id": "session123",
            "cwd": "/home/user",
            "tool": "bash",
            "args": {"command": "git status"}
        }"#;

        let event: OpenCodeEvent = serde_json::from_str(json).unwrap();
        let args: BashArgs = event.parse_args().unwrap();
        assert_eq!(args.command, "git status");
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let common = CommonOpenCodeData::new("session123".to_string(), "/home/user".to_string());
        let args = serde_json::json!({"command": "pwd"});
        let payload = PreToolUsePayload::new(common, "bash".to_string(), args);
        let event = OpenCodeEvent::PreToolUse(payload);

        let json = serde_json::to_string(&event).unwrap();
        let parsed: OpenCodeEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, parsed);
    }
}
