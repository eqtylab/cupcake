use serde::{Deserialize, Serialize};

mod notification;
mod permission_request;
mod post_tool_use;
mod pre_compact;
mod pre_tool_use;
mod session_end;
mod session_start;
mod stop;
mod subagent_stop;
mod user_prompt_submit;

pub use notification::{NotificationPayload, NotificationType};
pub use permission_request::PermissionRequestPayload;
pub use post_tool_use::PostToolUsePayload;
pub use pre_compact::PreCompactPayload;
pub use pre_tool_use::PreToolUsePayload;
pub use session_end::{SessionEndPayload, SessionEndReason};
pub use session_start::SessionStartPayload;
pub use stop::StopPayload;
pub use subagent_stop::SubagentStopPayload;
pub use user_prompt_submit::UserPromptSubmitPayload;

/// Trait for all event payloads - ensures access to common data
pub trait EventPayload {
    fn common(&self) -> &CommonEventData;
}

/// Marker trait for events that can inject context via stdout
pub trait InjectsContext {}

/// Permission mode for Claude Code sessions
///
/// Indicates the current permission level for the session, which affects
/// how Claude Code handles tool permissions.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default permission mode - user is prompted for dangerous operations
    #[default]
    Default,
    /// Plan mode - Claude creates plans without executing
    Plan,
    /// Accept edits mode - file edits are auto-approved
    AcceptEdits,
    /// Bypass permissions - all tool calls auto-approved (dangerous)
    BypassPermissions,
}

impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionMode::Default => write!(f, "default"),
            PermissionMode::Plan => write!(f, "plan"),
            PermissionMode::AcceptEdits => write!(f, "acceptEdits"),
            PermissionMode::BypassPermissions => write!(f, "bypassPermissions"),
        }
    }
}

/// Common fields present in all Claude Code hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonEventData {
    /// Unique identifier for the Claude Code session
    pub session_id: String,

    /// Path to the session transcript file
    pub transcript_path: String,

    /// Current working directory when the hook is invoked
    pub cwd: String,

    /// Current permission mode for the session
    #[serde(default)]
    pub permission_mode: PermissionMode,
}

/// All possible Claude Code hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name", rename_all = "PascalCase")]
pub enum ClaudeCodeEvent {
    /// Before tool execution
    PreToolUse(PreToolUsePayload),

    /// After tool execution (success only)
    PostToolUse(PostToolUsePayload),

    /// Claude Code notifications
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

    /// Permission request for tool execution (newer API)
    ///
    /// PermissionRequest is the newer, cleaner hook for tool permission decisions.
    /// It provides the same functionality as PreToolUse but with:
    /// - A `tool_use_id` field for tracking specific tool invocations
    /// - A cleaner response format with nested `decision` object
    PermissionRequest(PermissionRequestPayload),
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

impl std::fmt::Display for CompactTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompactTrigger::Manual => write!(f, "manual"),
            CompactTrigger::Auto => write!(f, "auto"),
        }
    }
}

/// Source of session start event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    /// Normal startup
    Startup,
    /// Resumed via --resume, --continue, or /resume
    Resume,
    /// After /clear command
    Clear,
    /// After compact (auto or manual)
    Compact,
}

impl std::fmt::Display for SessionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionSource::Startup => write!(f, "startup"),
            SessionSource::Resume => write!(f, "resume"),
            SessionSource::Clear => write!(f, "clear"),
            SessionSource::Compact => write!(f, "compact"),
        }
    }
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

impl ClaudeCodeEvent {
    /// Get the common event data
    pub fn common(&self) -> &CommonEventData {
        match self {
            ClaudeCodeEvent::PreToolUse(payload) => &payload.common,
            ClaudeCodeEvent::PostToolUse(payload) => &payload.common,
            ClaudeCodeEvent::Notification(payload) => &payload.common,
            ClaudeCodeEvent::Stop(payload) => &payload.common,
            ClaudeCodeEvent::SubagentStop(payload) => &payload.common,
            ClaudeCodeEvent::PreCompact(payload) => &payload.common,
            ClaudeCodeEvent::UserPromptSubmit(payload) => &payload.common,
            ClaudeCodeEvent::SessionStart(payload) => &payload.common,
            ClaudeCodeEvent::SessionEnd(payload) => &payload.common,
            ClaudeCodeEvent::PermissionRequest(payload) => &payload.common,
        }
    }

    /// Get the tool name for tool-related events
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            ClaudeCodeEvent::PreToolUse(payload) => Some(&payload.tool_name),
            ClaudeCodeEvent::PostToolUse(payload) => Some(&payload.tool_name),
            ClaudeCodeEvent::PermissionRequest(payload) => Some(&payload.tool_name),
            _ => None,
        }
    }

    /// Get the tool input for tool-related events
    pub fn tool_input(&self) -> Option<&serde_json::Value> {
        match self {
            ClaudeCodeEvent::PreToolUse(payload) => Some(&payload.tool_input),
            ClaudeCodeEvent::PostToolUse(payload) => Some(&payload.tool_input),
            ClaudeCodeEvent::PermissionRequest(payload) => Some(&payload.tool_input),
            _ => None,
        }
    }

    /// Get the event name as a string
    pub fn event_name(&self) -> &'static str {
        match self {
            ClaudeCodeEvent::PreToolUse { .. } => "PreToolUse",
            ClaudeCodeEvent::PostToolUse(_) => "PostToolUse",
            ClaudeCodeEvent::Notification { .. } => "Notification",
            ClaudeCodeEvent::Stop { .. } => "Stop",
            ClaudeCodeEvent::SubagentStop { .. } => "SubagentStop",
            ClaudeCodeEvent::PreCompact { .. } => "PreCompact",
            ClaudeCodeEvent::UserPromptSubmit { .. } => "UserPromptSubmit",
            ClaudeCodeEvent::SessionStart { .. } => "SessionStart",
            ClaudeCodeEvent::SessionEnd { .. } => "SessionEnd",
            ClaudeCodeEvent::PermissionRequest { .. } => "PermissionRequest",
        }
    }

    /// Check if this is a tool-related event
    pub fn is_tool_event(&self) -> bool {
        matches!(
            self,
            ClaudeCodeEvent::PreToolUse(_)
                | ClaudeCodeEvent::PostToolUse(_)
                | ClaudeCodeEvent::PermissionRequest(_)
        )
    }

    /// Check if this is a stop event
    pub fn is_stop_event(&self) -> bool {
        matches!(
            self,
            ClaudeCodeEvent::Stop { .. } | ClaudeCodeEvent::SubagentStop { .. }
        )
    }

    /// Check if this is a permission event (pre-execution decision)
    pub fn is_permission_event(&self) -> bool {
        matches!(
            self,
            ClaudeCodeEvent::PreToolUse(_) | ClaudeCodeEvent::PermissionRequest(_)
        )
    }

    /// Get the permission mode for the session
    pub fn permission_mode(&self) -> &PermissionMode {
        &self.common().permission_mode
    }

    /// Get the tool use ID for tool-related events
    ///
    /// Returns the unique identifier for this tool invocation.
    /// Available for PreToolUse, PostToolUse, and PermissionRequest events.
    pub fn tool_use_id(&self) -> Option<&str> {
        match self {
            ClaudeCodeEvent::PreToolUse(payload) => payload.tool_use_id.as_deref(),
            ClaudeCodeEvent::PostToolUse(payload) => payload.tool_use_id.as_deref(),
            ClaudeCodeEvent::PermissionRequest(payload) => Some(&payload.tool_use_id),
            _ => None,
        }
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
        let event = ClaudeCodeEvent::PreToolUse(PreToolUsePayload {
            common: CommonEventData {
                session_id: "test-session".to_string(),
                transcript_path: "/path/to/transcript".to_string(),
                cwd: "/home/user/project".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
            tool_use_id: None,
        });

        assert_eq!(event.common().session_id, "test-session");
        assert_eq!(event.common().cwd, "/home/user/project");
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
            "cwd": "/home/user/project",
            "tool_name": "Bash",
            "tool_input": {"command": "echo hello"}
        }
        "#;

        let event: ClaudeCodeEvent = serde_json::from_str(json).unwrap();

        match event {
            ClaudeCodeEvent::PreToolUse(payload) => {
                assert_eq!(payload.common.session_id, "test-session");
                assert_eq!(payload.common.cwd, "/home/user/project");
                assert_eq!(payload.tool_name, "Bash");
                assert_eq!(payload.tool_input["command"], "echo hello");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_bash_tool_input_parsing() {
        let event = ClaudeCodeEvent::PreToolUse(PreToolUsePayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
                cwd: "/home/user/project".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({
                "command": "cargo build",
                "description": "Build the project",
                "timeout": 60
            }),
            tool_use_id: None,
        });

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
        let event = ClaudeCodeEvent::Notification(NotificationPayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
                cwd: "/home/user/project".to_string(),
                permission_mode: Default::default(),
            },
            message: "Test notification".to_string(),
            notification_type: None,
        });

        assert!(!event.is_tool_event());
        assert_eq!(event.tool_name(), None);
    }

    #[test]
    fn test_user_prompt_submit_event() {
        let json = r#"
        {
            "hook_event_name": "UserPromptSubmit",
            "session_id": "test-session",
            "transcript_path": "/path/to/transcript",
            "cwd": "/home/user/project",
            "prompt": "Write a function to calculate factorial"
        }
        "#;

        let event: ClaudeCodeEvent = serde_json::from_str(json).unwrap();

        match &event {
            ClaudeCodeEvent::UserPromptSubmit(payload) => {
                assert_eq!(payload.common.session_id, "test-session");
                assert_eq!(payload.common.cwd, "/home/user/project");
                assert_eq!(payload.prompt, "Write a function to calculate factorial");
                assert_eq!(event.event_name(), "UserPromptSubmit");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_session_start_event() {
        // Test all source types according to Claude Code hooks.md
        let test_cases = vec![
            ("startup", SessionSource::Startup),
            ("resume", SessionSource::Resume),
            ("clear", SessionSource::Clear),
            ("compact", SessionSource::Compact),
        ];

        for (source_str, expected_source) in test_cases {
            let json = format!(
                r#"
                {{
                    "hook_event_name": "SessionStart",
                    "session_id": "test-session",
                    "transcript_path": "~/.claude/projects/.../transcript.jsonl",
                    "cwd": "/home/user/project",
                    "source": "{source_str}"
                }}
                "#
            );

            let event: ClaudeCodeEvent = serde_json::from_str(&json).unwrap();

            match &event {
                ClaudeCodeEvent::SessionStart(payload) => {
                    assert_eq!(payload.common.session_id, "test-session");
                    assert_eq!(payload.common.cwd, "/home/user/project");
                    assert_eq!(event.event_name(), "SessionStart");
                    match (&payload.source, &expected_source) {
                        (SessionSource::Startup, SessionSource::Startup) => (),
                        (SessionSource::Resume, SessionSource::Resume) => (),
                        (SessionSource::Clear, SessionSource::Clear) => (),
                        (SessionSource::Compact, SessionSource::Compact) => (),
                        _ => panic!("Source mismatch"),
                    }
                }
                _ => panic!("Wrong event type"),
            }
        }
    }

    #[test]
    fn test_permission_request_event() {
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

        let event: ClaudeCodeEvent = serde_json::from_str(json).unwrap();

        match &event {
            ClaudeCodeEvent::PermissionRequest(payload) => {
                assert_eq!(payload.common.session_id, "test-session");
                assert_eq!(payload.common.cwd, "/home/user/project");
                assert_eq!(payload.tool_name, "Bash");
                assert_eq!(payload.tool_use_id, "toolu_123");
                assert_eq!(payload.tool_input["command"], "rm -rf /tmp");
                assert_eq!(event.event_name(), "PermissionRequest");
                assert!(event.is_tool_event());
                assert!(event.is_permission_event());
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_permission_event_helper() {
        // PreToolUse is a permission event
        let pre_tool = ClaudeCodeEvent::PreToolUse(PreToolUsePayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
                cwd: "/home".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_use_id: None,
        });
        assert!(pre_tool.is_permission_event());

        // PermissionRequest is a permission event
        let perm_req = ClaudeCodeEvent::PermissionRequest(PermissionRequestPayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
                cwd: "/home".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_use_id: "toolu_xyz".to_string(),
        });
        assert!(perm_req.is_permission_event());

        // PostToolUse is NOT a permission event
        let post_tool = ClaudeCodeEvent::PostToolUse(PostToolUsePayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/path".to_string(),
                cwd: "/home".to_string(),
                permission_mode: Default::default(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_response: serde_json::json!({"success": true}),
            tool_use_id: None,
        });
        assert!(!post_tool.is_permission_event());
    }
}
