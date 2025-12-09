//! Cursor-specific response builders
//!
//! This module contains specialized builders for each Cursor hook type,
//! ensuring compliance with Cursor's hook response schemas.
//!
//! CRITICAL: Cursor's beforeSubmitPrompt does NOT support context injection.
//! See before_submit_prompt.rs for details.

mod after_agent_response;
mod after_agent_thought;
mod after_file_edit;
mod after_mcp_execution;
mod after_shell_execution;
mod before_mcp_execution;
mod before_read_file;
mod before_shell_execution;
mod before_submit_prompt;
mod stop;

use crate::harness::events::cursor::CursorEvent;
use crate::harness::response::types::EngineDecision;
use serde_json::Value;

/// Central dispatcher for Cursor responses
/// Routes to appropriate builder based on hook type
pub struct CursorResponseBuilder;

impl CursorResponseBuilder {
    /// Build response for a specific Cursor hook event
    ///
    /// Unlike Claude Code, Cursor has simpler response schemas:
    /// - beforeSubmitPrompt: Only supports {continue: true/false, user_message?: string}
    /// - beforeReadFile: Only supports {permission: "allow"|"deny"}
    /// - after* events: Fire-and-forget, return empty {}
    /// - Other events: Support full permission model with messages
    ///
    /// agent_messages: Optional technical details for the agent (separate from user message)
    ///
    /// Returns raw JSON Value to be serialized to stdout
    pub fn build_response(
        decision: &EngineDecision,
        event: &CursorEvent,
        agent_messages: Option<Vec<String>>,
    ) -> Value {
        match event {
            CursorEvent::BeforeShellExecution(_) => {
                before_shell_execution::build(decision, agent_messages)
            }
            CursorEvent::AfterShellExecution(_) => {
                after_shell_execution::build(decision, agent_messages)
            }
            CursorEvent::BeforeMCPExecution(_) => {
                before_mcp_execution::build(decision, agent_messages)
            }
            CursorEvent::AfterMCPExecution(_) => {
                after_mcp_execution::build(decision, agent_messages)
            }
            CursorEvent::AfterFileEdit(_) => after_file_edit::build(decision, agent_messages),
            CursorEvent::BeforeReadFile(_) => before_read_file::build(decision, agent_messages),
            CursorEvent::BeforeSubmitPrompt(_) => {
                before_submit_prompt::build(decision, agent_messages)
            }
            CursorEvent::AfterAgentResponse(_) => {
                after_agent_response::build(decision, agent_messages)
            }
            CursorEvent::AfterAgentThought(_) => {
                after_agent_thought::build(decision, agent_messages)
            }
            CursorEvent::Stop(_) => stop::build(decision, agent_messages),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::events::cursor::*;

    #[test]
    fn test_before_shell_execution_allow() {
        let event = CursorEvent::BeforeShellExecution(BeforeShellExecutionPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            command: "ls".to_string(),
            cwd: "/test".to_string(),
        });

        let decision = EngineDecision::Allow { reason: None };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response["permission"], "allow");
    }

    #[test]
    fn test_before_shell_execution_deny() {
        let event = CursorEvent::BeforeShellExecution(BeforeShellExecutionPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            command: "rm -rf /".to_string(),
            cwd: "/test".to_string(),
        });

        let decision = EngineDecision::Block {
            feedback: "Dangerous command".to_string(),
        };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response["permission"], "deny");
        assert_eq!(response["user_message"], "Dangerous command");
    }

    #[test]
    fn test_before_submit_prompt_allow() {
        let event = CursorEvent::BeforeSubmitPrompt(BeforeSubmitPromptPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            prompt: "Hello".to_string(),
            attachments: vec![],
        });

        let decision = EngineDecision::Allow { reason: None };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response["continue"], true);
    }

    #[test]
    fn test_before_submit_prompt_deny() {
        let event = CursorEvent::BeforeSubmitPrompt(BeforeSubmitPromptPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            prompt: "Malicious prompt".to_string(),
            attachments: vec![],
        });

        let decision = EngineDecision::Block {
            feedback: "Blocked".to_string(),
        };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response["continue"], false);
        assert_eq!(response["user_message"], "Blocked");
    }

    #[test]
    fn test_before_read_file_allow() {
        let event = CursorEvent::BeforeReadFile(BeforeReadFilePayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            file_path: "/test/file.txt".to_string(),
            content: "file content".to_string(),
            attachments: vec![],
        });

        let decision = EngineDecision::Allow { reason: None };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response["permission"], "allow");
    }

    #[test]
    fn test_after_file_edit_returns_empty() {
        let event = CursorEvent::AfterFileEdit(AfterFileEditPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            file_path: "/test/file.txt".to_string(),
            edits: vec![],
        });

        let decision = EngineDecision::Allow { reason: None };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response, serde_json::json!({}));
    }

    #[test]
    fn test_stop_allow_returns_empty() {
        let event = CursorEvent::Stop(StopPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            status: "completed".to_string(),
            loop_count: 0,
        });

        let decision = EngineDecision::Allow { reason: None };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(response, serde_json::json!({}));
    }

    #[test]
    fn test_stop_block_returns_followup_message() {
        let event = CursorEvent::Stop(StopPayload {
            common: CommonCursorData {
                conversation_id: "conv-123".to_string(),
                generation_id: "gen-456".to_string(),
                workspace_roots: vec!["/test".to_string()],
                model: None,
                cursor_version: None,
                user_email: None,
            },
            status: "completed".to_string(),
            loop_count: 2,
        });

        let decision = EngineDecision::Block {
            feedback: "Tests are failing. Please fix them.".to_string(),
        };
        let response = CursorResponseBuilder::build_response(&decision, &event, None);

        assert_eq!(
            response,
            serde_json::json!({
                "followup_message": "Tests are failing. Please fix them."
            })
        );
    }
}
