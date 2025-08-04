//! EventFactory - Test data builder for Claude Code hook events
//!
//! This module provides a clean, builder-pattern API for creating valid
//! Claude Code hook event JSON payloads. It eliminates manual JSON construction
//! errors and makes test data creation trivial and reliable.

#![allow(dead_code)] // Many builder methods are provided for completeness

use serde_json::{json, Value};

/// Builder for creating Claude Code hook event JSON payloads
pub struct EventFactory;

impl EventFactory {
    /// Create a PreToolUse event builder
    pub fn pre_tool_use() -> PreToolUseBuilder {
        PreToolUseBuilder::default()
    }

    /// Create a PostToolUse event builder
    pub fn post_tool_use() -> PostToolUseBuilder {
        PostToolUseBuilder::default()
    }

    /// Create a UserPromptSubmit event builder
    pub fn user_prompt_submit() -> UserPromptSubmitBuilder {
        UserPromptSubmitBuilder::default()
    }

    /// Create a SessionStart event builder
    pub fn session_start() -> SessionStartBuilder {
        SessionStartBuilder::default()
    }

    /// Create a PreCompact event builder
    pub fn pre_compact() -> PreCompactBuilder {
        PreCompactBuilder::default()
    }

    /// Create a Stop event builder
    pub fn stop() -> StopBuilder {
        StopBuilder::default()
    }

    /// Create a SubagentStop event builder
    pub fn subagent_stop() -> SubagentStopBuilder {
        SubagentStopBuilder::default()
    }

    /// Create a Notification event builder
    pub fn notification() -> NotificationBuilder {
        NotificationBuilder::default()
    }
}

/// Common event data shared by all hooks
#[derive(Clone)]
struct CommonEventData {
    session_id: String,
    transcript_path: String,
    cwd: String,
}

impl Default for CommonEventData {
    fn default() -> Self {
        Self {
            session_id: "test-session-123".to_string(),
            transcript_path: "/tmp/test-transcript.jsonl".to_string(),
            cwd: "/home/user/project".to_string(),
        }
    }
}

/// Builder for PreToolUse events
pub struct PreToolUseBuilder {
    common: CommonEventData,
    tool_name: String,
    tool_input: Value,
}

impl Default for PreToolUseBuilder {
    fn default() -> Self {
        Self {
            common: CommonEventData::default(),
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "echo test"}),
        }
    }
}

impl PreToolUseBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = name.into();
        self
    }

    pub fn tool_input(mut self, input: Value) -> Self {
        self.tool_input = input;
        self
    }

    pub fn tool_input_command(mut self, command: impl Into<String>) -> Self {
        self.tool_input = json!({"command": command.into()});
        self
    }

    pub fn tool_input_file_path(mut self, path: impl Into<String>) -> Self {
        self.tool_input = json!({"file_path": path.into()});
        self
    }

    pub fn tool_input_description(mut self, desc: impl Into<String>) -> Self {
        self.tool_input["description"] = json!(desc.into());
        self
    }

    pub fn tool_input_timeout(mut self, timeout: u32) -> Self {
        self.tool_input["timeout"] = json!(timeout);
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "PreToolUse",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "tool_name": self.tool_name,
            "tool_input": self.tool_input
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "PreToolUse",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "tool_name": self.tool_name,
            "tool_input": self.tool_input
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for PostToolUse events
pub struct PostToolUseBuilder {
    common: CommonEventData,
    tool_name: String,
    tool_input: Value,
    tool_response: Value,
}

impl Default for PostToolUseBuilder {
    fn default() -> Self {
        Self {
            common: CommonEventData::default(),
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "echo test"}),
            tool_response: json!({"success": true, "output": "test\n"}),
        }
    }
}

impl PostToolUseBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = name.into();
        self
    }

    pub fn tool_input(mut self, input: Value) -> Self {
        self.tool_input = input;
        self
    }

    pub fn tool_response(mut self, response: Value) -> Self {
        self.tool_response = response;
        self
    }

    pub fn tool_response_success(mut self, success: bool, output: impl Into<String>) -> Self {
        self.tool_response = json!({
            "success": success,
            "output": output.into()
        });
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "PostToolUse",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "tool_name": self.tool_name,
            "tool_input": self.tool_input,
            "tool_response": self.tool_response
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "PostToolUse",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "tool_name": self.tool_name,
            "tool_input": self.tool_input,
            "tool_response": self.tool_response
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for UserPromptSubmit events
pub struct UserPromptSubmitBuilder {
    common: CommonEventData,
    prompt: String,
}

impl Default for UserPromptSubmitBuilder {
    fn default() -> Self {
        Self {
            common: CommonEventData::default(),
            prompt: "Write a hello world function".to_string(),
        }
    }
}

impl UserPromptSubmitBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "prompt": self.prompt
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "prompt": self.prompt
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for SessionStart events
pub struct SessionStartBuilder {
    common: CommonEventData,
    source: String,
}

impl Default for SessionStartBuilder {
    fn default() -> Self {
        Self {
            common: CommonEventData::default(),
            source: "startup".to_string(),
        }
    }
}

impl SessionStartBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn source_startup(mut self) -> Self {
        self.source = "startup".to_string();
        self
    }

    pub fn source_resume(mut self) -> Self {
        self.source = "resume".to_string();
        self
    }

    pub fn source_clear(mut self) -> Self {
        self.source = "clear".to_string();
        self
    }

    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "SessionStart",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "source": self.source
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "SessionStart",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "source": self.source
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for PreCompact events
pub struct PreCompactBuilder {
    common: CommonEventData,
    trigger: String,
    custom_instructions: Option<String>,
}

impl Default for PreCompactBuilder {
    fn default() -> Self {
        Self {
            common: CommonEventData::default(),
            trigger: "manual".to_string(),
            custom_instructions: None,
        }
    }
}

impl PreCompactBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn trigger_manual(mut self) -> Self {
        self.trigger = "manual".to_string();
        self
    }

    pub fn trigger_auto(mut self) -> Self {
        self.trigger = "auto".to_string();
        self
    }

    pub fn custom_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.custom_instructions = Some(instructions.into());
        self
    }

    pub fn trigger(mut self, trigger: impl Into<String>) -> Self {
        self.trigger = trigger.into();
        self
    }

    pub fn build_json(self) -> String {
        let mut json = json!({
            "hook_event_name": "PreCompact",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "trigger": self.trigger
        });

        if let Some(instructions) = self.custom_instructions {
            json["custom_instructions"] = Value::String(instructions);
        }

        json.to_string()
    }

    pub fn build_value(self) -> Value {
        let mut json = json!({
            "hook_event_name": "PreCompact",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "trigger": self.trigger
        });

        if let Some(instructions) = self.custom_instructions {
            json["custom_instructions"] = Value::String(instructions);
        }

        json
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for Stop events
#[derive(Default)]
pub struct StopBuilder {
    common: CommonEventData,
    stop_hook_active: bool,
}

impl StopBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn stop_hook_active(mut self, active: bool) -> Self {
        self.stop_hook_active = active;
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "Stop",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "stop_hook_active": self.stop_hook_active
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "Stop",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "stop_hook_active": self.stop_hook_active
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for SubagentStop events
#[derive(Default)]
pub struct SubagentStopBuilder {
    common: CommonEventData,
    stop_hook_active: bool,
}

impl SubagentStopBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn stop_hook_active(mut self, active: bool) -> Self {
        self.stop_hook_active = active;
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "SubagentStop",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "stop_hook_active": self.stop_hook_active
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "SubagentStop",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "stop_hook_active": self.stop_hook_active
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

/// Builder for Notification events
pub struct NotificationBuilder {
    common: CommonEventData,
    message: String,
}

impl Default for NotificationBuilder {
    fn default() -> Self {
        Self {
            common: CommonEventData::default(),
            message: "Test notification".to_string(),
        }
    }
}

impl NotificationBuilder {
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.common.session_id = id.into();
        self
    }

    pub fn transcript_path(mut self, path: impl Into<String>) -> Self {
        self.common.transcript_path = path.into();
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.common.cwd = cwd.into();
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn build_json(self) -> String {
        json!({
            "hook_event_name": "Notification",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "message": self.message
        })
        .to_string()
    }

    pub fn build_value(self) -> Value {
        json!({
            "hook_event_name": "Notification",
            "session_id": self.common.session_id,
            "transcript_path": self.common.transcript_path,
            "cwd": self.common.cwd,
            "message": self.message
        })
    }

    pub fn build(self) -> cupcake::engine::events::ClaudeCodeEvent {
        let json = self.build_json();
        serde_json::from_str(&json).expect("EventFactory should produce valid events")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_pre_tool_use_builder() {
        let json_str = EventFactory::pre_tool_use()
            .tool_name("Bash")
            .tool_input_command("ls -la")
            .session_id("test-123")
            .build_json();

        let json: Value = serde_json::from_str(&json_str).expect("Valid JSON");
        assert_eq!(json["hook_event_name"], "PreToolUse");
        assert_eq!(json["tool_name"], "Bash");
        assert_eq!(json["tool_input"]["command"], "ls -la");
        assert_eq!(json["session_id"], "test-123");
    }

    #[test]
    fn test_post_tool_use_builder() {
        let json_str = EventFactory::post_tool_use()
            .tool_name("Write")
            .tool_input(json!({"file_path": "test.txt", "content": "Hello"}))
            .tool_response_success(true, "File written successfully")
            .build_json();

        let json: Value = serde_json::from_str(&json_str).expect("Valid JSON");
        assert_eq!(json["hook_event_name"], "PostToolUse");
        assert_eq!(json["tool_response"]["success"], true);
        assert_eq!(json["tool_response"]["output"], "File written successfully");
    }

    #[test]
    fn test_pre_compact_builder() {
        let json_str = EventFactory::pre_compact()
            .trigger_manual()
            .custom_instructions("Keep all TODO comments")
            .build_json();

        let json: Value = serde_json::from_str(&json_str).expect("Valid JSON");
        assert_eq!(json["hook_event_name"], "PreCompact");
        assert_eq!(json["trigger"], "manual");
        assert_eq!(json["custom_instructions"], "Keep all TODO comments");
    }

    #[test]
    fn test_session_start_sources() {
        // Test all three source types
        let startup = EventFactory::session_start().source_startup().build_value();
        assert_eq!(startup["source"], "startup");

        let resume = EventFactory::session_start().source_resume().build_value();
        assert_eq!(resume["source"], "resume");

        let clear = EventFactory::session_start().source_clear().build_value();
        assert_eq!(clear["source"], "clear");
    }
}
