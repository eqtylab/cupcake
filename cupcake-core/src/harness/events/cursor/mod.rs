//! Cursor hook events
//!
//! This module provides event structures for Cursor's hooks system.
//! Field names use snake_case to match Cursor's JSON format exactly.

mod after_agent_response;
mod after_agent_thought;
mod after_file_edit;
mod after_mcp_execution;
mod after_shell_execution;
mod before_mcp_execution;
mod before_read_file;
mod before_shell_execution;
mod before_submit_prompt;
mod common;
mod stop;

pub use after_agent_response::AfterAgentResponsePayload;
pub use after_agent_thought::AfterAgentThoughtPayload;
pub use after_file_edit::{AfterFileEditPayload, FileEdit};
pub use after_mcp_execution::AfterMCPExecutionPayload;
pub use after_shell_execution::AfterShellExecutionPayload;
pub use before_mcp_execution::BeforeMCPExecutionPayload;
pub use before_read_file::{Attachment, BeforeReadFilePayload};
pub use before_shell_execution::BeforeShellExecutionPayload;
pub use before_submit_prompt::{BeforeSubmitPromptPayload, PromptAttachment};
pub use common::CommonCursorData;
pub use stop::StopPayload;

use serde::{Deserialize, Serialize};

/// All possible Cursor hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name", rename_all = "camelCase")]
pub enum CursorEvent {
    BeforeShellExecution(BeforeShellExecutionPayload),
    AfterShellExecution(AfterShellExecutionPayload),
    BeforeMCPExecution(BeforeMCPExecutionPayload),
    AfterMCPExecution(AfterMCPExecutionPayload),
    AfterFileEdit(AfterFileEditPayload),
    BeforeReadFile(BeforeReadFilePayload),
    BeforeSubmitPrompt(BeforeSubmitPromptPayload),
    AfterAgentResponse(AfterAgentResponsePayload),
    AfterAgentThought(AfterAgentThoughtPayload),
    Stop(StopPayload),
}

impl CursorEvent {
    /// Get the event name as a string
    pub fn event_name(&self) -> &'static str {
        match self {
            CursorEvent::BeforeShellExecution(_) => "beforeShellExecution",
            CursorEvent::AfterShellExecution(_) => "afterShellExecution",
            CursorEvent::BeforeMCPExecution(_) => "beforeMCPExecution",
            CursorEvent::AfterMCPExecution(_) => "afterMCPExecution",
            CursorEvent::AfterFileEdit(_) => "afterFileEdit",
            CursorEvent::BeforeReadFile(_) => "beforeReadFile",
            CursorEvent::BeforeSubmitPrompt(_) => "beforeSubmitPrompt",
            CursorEvent::AfterAgentResponse(_) => "afterAgentResponse",
            CursorEvent::AfterAgentThought(_) => "afterAgentThought",
            CursorEvent::Stop(_) => "stop",
        }
    }

    /// Get the conversation ID (common across all events)
    pub fn conversation_id(&self) -> &str {
        match self {
            CursorEvent::BeforeShellExecution(p) => &p.common.conversation_id,
            CursorEvent::AfterShellExecution(p) => &p.common.conversation_id,
            CursorEvent::BeforeMCPExecution(p) => &p.common.conversation_id,
            CursorEvent::AfterMCPExecution(p) => &p.common.conversation_id,
            CursorEvent::AfterFileEdit(p) => &p.common.conversation_id,
            CursorEvent::BeforeReadFile(p) => &p.common.conversation_id,
            CursorEvent::BeforeSubmitPrompt(p) => &p.common.conversation_id,
            CursorEvent::AfterAgentResponse(p) => &p.common.conversation_id,
            CursorEvent::AfterAgentThought(p) => &p.common.conversation_id,
            CursorEvent::Stop(p) => &p.common.conversation_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_before_shell_execution_parsing() {
        let json = r#"{
            "hook_event_name": "beforeShellExecution",
            "conversation_id": "conv-123",
            "generation_id": "gen-456",
            "workspace_roots": ["/home/user/project"],
            "command": "ls -la",
            "cwd": "/home/user/project"
        }"#;

        let event: CursorEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_name(), "beforeShellExecution");
        assert_eq!(event.conversation_id(), "conv-123");
    }
}
