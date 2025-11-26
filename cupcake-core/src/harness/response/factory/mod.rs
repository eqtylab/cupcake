// Factory AI-specific response builders
//!
//! This module contains specialized builders for each Factory AI Droid hook type,
//! ensuring 100% specification compliance with Factory AI's hook contract.

mod context_injection;
mod feedback_loop;
mod generic;
mod pre_tool_use;

pub use context_injection::ContextInjectionResponseBuilder;
pub use feedback_loop::FeedbackLoopResponseBuilder;
pub use generic::GenericResponseBuilder;
pub use pre_tool_use::PreToolUseResponseBuilder;

use crate::harness::events::factory::FactoryEvent;
use crate::harness::response::types::{CupcakeResponse, EngineDecision};

/// Central dispatcher for Factory AI responses
/// Routes to appropriate builder based on hook type
pub struct FactoryResponseBuilder;

impl FactoryResponseBuilder {
    /// Build response for a specific hook event
    pub fn build_response(
        decision: &EngineDecision,
        hook_event: &FactoryEvent,
        context_to_inject: Option<Vec<String>>,
        suppress_output: bool,
    ) -> CupcakeResponse {
        match hook_event {
            FactoryEvent::PreToolUse(_) => {
                PreToolUseResponseBuilder::build(decision, suppress_output)
            }
            FactoryEvent::PostToolUse(_)
            | FactoryEvent::Stop(_)
            | FactoryEvent::SubagentStop(_) => FeedbackLoopResponseBuilder::build(
                decision,
                context_to_inject,
                hook_event,
                suppress_output,
            ),
            FactoryEvent::UserPromptSubmit(_) => ContextInjectionResponseBuilder::build(
                decision,
                context_to_inject,
                suppress_output,
                true,
            ),
            FactoryEvent::SessionStart(_) => ContextInjectionResponseBuilder::build(
                decision,
                context_to_inject,
                suppress_output,
                false,
            ),
            FactoryEvent::PreCompact(_) => {
                // PreCompact is special - it uses stdout for instructions, not JSON
                GenericResponseBuilder::build_precompact(
                    decision,
                    context_to_inject,
                    suppress_output,
                )
            }
            FactoryEvent::Notification(_) | FactoryEvent::SessionEnd(_) => {
                GenericResponseBuilder::build(decision, suppress_output)
            }
        }
    }
}
