use serde::{Deserialize, Serialize};

mod common;
mod notification;
mod post_tool_use;
mod pre_compact;
mod pre_tool_use;
mod session_end;
mod session_start;
mod stop;
mod subagent_stop;
mod user_prompt_submit;

pub use common::{CommonFactoryData, PermissionMode};
pub use notification::NotificationPayload;
pub use post_tool_use::PostToolUsePayload;
pub use pre_compact::{CompactTrigger, PreCompactPayload};
pub use pre_tool_use::PreToolUsePayload;
pub use session_end::{SessionEndPayload, SessionEndReason};
pub use session_start::{SessionSource, SessionStartPayload};
pub use stop::StopPayload;
pub use subagent_stop::SubagentStopPayload;
pub use user_prompt_submit::UserPromptSubmitPayload;

/// All possible Factory AI Droid hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hookEventName", rename_all = "PascalCase")]
pub enum FactoryEvent {
    /// Before tool execution
    PreToolUse(PreToolUsePayload),

    /// After tool execution (success only)
    PostToolUse(PostToolUsePayload),

    /// Droid notifications
    Notification(NotificationPayload),

    /// Main agent stopping
    Stop(StopPayload),

    /// Subagent (Task tool) stopping
    SubagentStop(SubagentStopPayload),

    /// Before memory compaction
    PreCompact(PreCompactPayload),

    /// User prompt submission
    UserPromptSubmit(UserPromptSubmitPayload),

    /// Session start event
    SessionStart(SessionStartPayload),

    /// Session end event
    SessionEnd(SessionEndPayload),
}

/// Specific tool input structures for common tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolInput {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadToolInput {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteToolInput {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolInput {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToolInput {
    pub description: String,
    pub prompt: String,
}

impl FactoryEvent {
    /// Get the common event data
    pub fn common(&self) -> &CommonFactoryData {
        match self {
            FactoryEvent::PreToolUse(payload) => &payload.common,
            FactoryEvent::PostToolUse(payload) => &payload.common,
            FactoryEvent::Notification(payload) => &payload.common,
            FactoryEvent::Stop(payload) => &payload.common,
            FactoryEvent::SubagentStop(payload) => &payload.common,
            FactoryEvent::PreCompact(payload) => &payload.common,
            FactoryEvent::UserPromptSubmit(payload) => &payload.common,
            FactoryEvent::SessionStart(payload) => &payload.common,
            FactoryEvent::SessionEnd(payload) => &payload.common,
        }
    }

    /// Get the tool name for tool-related events
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            FactoryEvent::PreToolUse(payload) => Some(&payload.tool_name),
            FactoryEvent::PostToolUse(payload) => Some(&payload.tool_name),
            _ => None,
        }
    }

    /// Get the tool input for tool-related events
    pub fn tool_input(&self) -> Option<&serde_json::Value> {
        match self {
            FactoryEvent::PreToolUse(payload) => Some(&payload.tool_input),
            FactoryEvent::PostToolUse(payload) => Some(&payload.tool_input),
            _ => None,
        }
    }

    /// Get the event name as a string
    pub fn event_name(&self) -> &'static str {
        match self {
            FactoryEvent::PreToolUse(_) => "PreToolUse",
            FactoryEvent::PostToolUse(_) => "PostToolUse",
            FactoryEvent::Notification(_) => "Notification",
            FactoryEvent::Stop(_) => "Stop",
            FactoryEvent::SubagentStop(_) => "SubagentStop",
            FactoryEvent::PreCompact(_) => "PreCompact",
            FactoryEvent::UserPromptSubmit(_) => "UserPromptSubmit",
            FactoryEvent::SessionStart(_) => "SessionStart",
            FactoryEvent::SessionEnd(_) => "SessionEnd",
        }
    }

    /// Check if this is a tool-related event
    pub fn is_tool_event(&self) -> bool {
        matches!(
            self,
            FactoryEvent::PreToolUse(_) | FactoryEvent::PostToolUse(_)
        )
    }

    /// Check if this is a stop event
    pub fn is_stop_event(&self) -> bool {
        matches!(
            self,
            FactoryEvent::Stop { .. } | FactoryEvent::SubagentStop { .. }
        )
    }

    /// Parse tool input as specific tool type
    pub fn parse_tool_input<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.tool_input() {
            Some(input) => serde_json::from_value(input.clone()),
            None => Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No tool input available",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_event_common_data() {
        let event = FactoryEvent::PreToolUse(PreToolUsePayload {
            common: CommonFactoryData {
                session_id: "test-session".to_string(),
                transcript_path: "/path/to/transcript".to_string(),
                cwd: "/home/user/project".to_string(),
                permission_mode: PermissionMode::Default,
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
        });

        assert_eq!(event.common().session_id, "test-session");
        assert_eq!(event.common().cwd, "/home/user/project");
        assert_eq!(event.tool_name(), Some("Bash"));
        assert!(event.is_tool_event());
        assert!(!event.is_stop_event());
    }

    #[test]
    fn test_permission_mode_parsing() {
        let json = r#"{
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/path",
            "cwd": "/project",
            "permission_mode": "bypassPermissions",
            "tool_name": "Bash",
            "tool_input": {"command": "test"}
        }"#;

        let event: FactoryEvent = serde_json::from_str(json).unwrap();
        match event {
            FactoryEvent::PreToolUse(payload) => {
                assert_eq!(
                    payload.common.permission_mode,
                    PermissionMode::BypassPermissions
                );
            }
            _ => panic!("Wrong event type"),
        }
    }
}
