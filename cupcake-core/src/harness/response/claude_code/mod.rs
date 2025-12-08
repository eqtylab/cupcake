//! Claude Code-specific response builders
//!
//! This module contains specialized builders for each Claude Code hook type,
//! ensuring 100% specification compliance with the July 20 JSON hook contract.

mod context_injection;
mod feedback_loop;
mod generic;
mod permission_request;
mod pre_tool_use;

pub use context_injection::ContextInjectionResponseBuilder;
pub use feedback_loop::FeedbackLoopResponseBuilder;
pub use generic::GenericResponseBuilder;
pub use permission_request::PermissionRequestResponseBuilder;
pub use pre_tool_use::PreToolUseResponseBuilder;

use crate::harness::events::claude_code::ClaudeCodeEvent;
use crate::harness::response::types::{CupcakeResponse, EngineDecision};

/// Central dispatcher for Claude Code responses
/// Routes to appropriate builder based on hook type
pub struct ClaudeCodeResponseBuilder;

impl ClaudeCodeResponseBuilder {
    /// Build response for a specific hook event
    pub fn build_response(
        decision: &EngineDecision,
        hook_event: &ClaudeCodeEvent,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        match hook_event {
            ClaudeCodeEvent::PreToolUse(_) => {
                PreToolUseResponseBuilder::build(decision, suppress_output)
            }
            ClaudeCodeEvent::PermissionRequest(_) => {
                PermissionRequestResponseBuilder::build(decision, suppress_output)
            }
            ClaudeCodeEvent::PostToolUse(_)
            | ClaudeCodeEvent::Stop(_)
            | ClaudeCodeEvent::SubagentStop(_) => FeedbackLoopResponseBuilder::build(
                decision,
                context_to_inject,
                hook_event,
                suppress_output,
            ),
            ClaudeCodeEvent::UserPromptSubmit(_) => ContextInjectionResponseBuilder::build(
                decision,
                context_to_inject,
                suppress_output,
                true,
            ),
            ClaudeCodeEvent::SessionStart(_) => ContextInjectionResponseBuilder::build(
                decision,
                context_to_inject,
                suppress_output,
                false,
            ),
            ClaudeCodeEvent::PreCompact(_) => {
                // PreCompact is special - it uses stdout for instructions, not JSON
                GenericResponseBuilder::build_precompact(
                    decision,
                    context_to_inject,
                    suppress_output,
                )
            }
            ClaudeCodeEvent::Notification(_) | ClaudeCodeEvent::SessionEnd(_) => {
                GenericResponseBuilder::build(decision, suppress_output)
            }
        }
    }
}
