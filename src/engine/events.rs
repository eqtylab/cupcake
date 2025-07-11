use serde::{Deserialize, Serialize};

/// Common fields present in all Claude Code hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonEventData {
    /// Unique identifier for the Claude Code session
    pub session_id: String,

    /// Path to the session transcript file
    pub transcript_path: String,
}

/// All possible Claude Code hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name", rename_all = "PascalCase")]
pub enum HookEvent {
    /// Before tool execution
    PreToolUse {
        #[serde(flatten)]
        common: CommonEventData,

        /// Name of the tool being called
        tool_name: String,

        /// Input parameters for the tool
        tool_input: serde_json::Value,
    },

    /// After tool execution (success only)
    PostToolUse {
        #[serde(flatten)]
        common: CommonEventData,

        /// Name of the tool that was called
        tool_name: String,

        /// Input parameters that were used
        tool_input: serde_json::Value,

        /// Response from the tool
        tool_response: serde_json::Value,
    },

    /// Claude Code notifications
    Notification {
        #[serde(flatten)]
        common: CommonEventData,

        /// The notification message
        message: String,
    },

    /// Main agent stopping
    Stop {
        #[serde(flatten)]
        common: CommonEventData,

        /// Whether stop hook is currently active (prevents infinite loops)
        stop_hook_active: bool,
    },

    /// Subagent (Task tool) stopping
    SubagentStop {
        #[serde(flatten)]
        common: CommonEventData,

        /// Whether stop hook is currently active (prevents infinite loops)
        stop_hook_active: bool,
    },

    /// Before memory compaction
    PreCompact {
        #[serde(flatten)]
        common: CommonEventData,

        /// Whether compaction was triggered manually or automatically
        trigger: CompactTrigger,

        /// Custom instructions for manual compaction
        #[serde(skip_serializing_if = "Option::is_none")]
        custom_instructions: Option<String>,
    },
}

/// Type of compaction trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    /// User-initiated via /compact command
    Manual,
    /// Automatic due to full context
    Auto,
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

impl HookEvent {
    /// Get the common event data
    pub fn common(&self) -> &CommonEventData {
        match self {
            HookEvent::PreToolUse { common, .. } => common,
            HookEvent::PostToolUse { common, .. } => common,
            HookEvent::Notification { common, .. } => common,
            HookEvent::Stop { common, .. } => common,
            HookEvent::SubagentStop { common, .. } => common,
            HookEvent::PreCompact { common, .. } => common,
        }
    }

    /// Get the tool name for tool-related events
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            HookEvent::PreToolUse { tool_name, .. } => Some(tool_name),
            HookEvent::PostToolUse { tool_name, .. } => Some(tool_name),
            _ => None,
        }
    }

    /// Get the tool input for tool-related events
    pub fn tool_input(&self) -> Option<&serde_json::Value> {
        match self {
            HookEvent::PreToolUse { tool_input, .. } => Some(tool_input),
            HookEvent::PostToolUse { tool_input, .. } => Some(tool_input),
            _ => None,
        }
    }

    /// Get the event name as a string
    pub fn event_name(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse { .. } => "PreToolUse",
            HookEvent::PostToolUse { .. } => "PostToolUse",
            HookEvent::Notification { .. } => "Notification",
            HookEvent::Stop { .. } => "Stop",
            HookEvent::SubagentStop { .. } => "SubagentStop",
            HookEvent::PreCompact { .. } => "PreCompact",
        }
    }

    /// Check if this is a tool-related event
    pub fn is_tool_event(&self) -> bool {
        matches!(
            self,
            HookEvent::PreToolUse { .. } | HookEvent::PostToolUse { .. }
        )
    }

    /// Check if this is a stop event
    pub fn is_stop_event(&self) -> bool {
        matches!(
            self,
            HookEvent::Stop { .. } | HookEvent::SubagentStop { .. }
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
    use pretty_assertions::assert_eq;

    #[test]
    fn test_hook_event_common_data() {
        let event = HookEvent::PreToolUse {
            common: CommonEventData {
                session_id: "test-session".to_string(),
                transcript_path: "/path/to/transcript".to_string(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
        };

        assert_eq!(event.common().session_id, "test-session");
        assert_eq!(event.tool_name(), Some("Bash"));
        assert!(event.is_tool_event());
        assert!(!event.is_stop_event());
    }

    #[test]
    fn test_hook_event_deserialization() {
        let json = r#"
        {
            "hook_event_name": "PreToolUse",
            "session_id": "test-session",
            "transcript_path": "/path/to/transcript",
            "tool_name": "Bash",
            "tool_input": {"command": "echo hello"}
        }
        "#;

        let event: HookEvent = serde_json::from_str(json).unwrap();

        match event {
            HookEvent::PreToolUse {
                common,
                tool_name,
                tool_input,
            } => {
                assert_eq!(common.session_id, "test-session");
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_input["command"], "echo hello");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_bash_tool_input_parsing() {
        let event = HookEvent::PreToolUse {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({
                "command": "cargo build",
                "description": "Build the project",
                "timeout": 60
            }),
        };

        let bash_input: BashToolInput = event.parse_tool_input().unwrap();
        assert_eq!(bash_input.command, "cargo build");
        assert_eq!(
            bash_input.description,
            Some("Build the project".to_string())
        );
        assert_eq!(bash_input.timeout, Some(60));
    }

    #[test]
    fn test_compact_trigger_serialization() {
        let trigger = CompactTrigger::Manual;
        let json = serde_json::to_string(&trigger).unwrap();
        assert_eq!(json, r#""manual""#);

        let trigger = CompactTrigger::Auto;
        let json = serde_json::to_string(&trigger).unwrap();
        assert_eq!(json, r#""auto""#);
    }

    #[test]
    fn test_notification_event() {
        let event = HookEvent::Notification {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
            },
            message: "Test notification".to_string(),
        };

        assert!(!event.is_tool_event());
        assert_eq!(event.tool_name(), None);
    }
}
