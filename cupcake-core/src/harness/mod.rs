// Place this code in cupcake-rewrite/src/harness/mod.rs

pub mod events;
pub mod response;
pub mod types;

use crate::engine::decision::FinalDecision;
use anyhow::Result;
use events::claude_code::ClaudeCodeEvent;
use events::cursor::CursorEvent;
use events::factory::FactoryEvent;
use events::opencode::OpenCodeEvent;
use response::{
    ClaudeCodeResponseBuilder, CursorResponseBuilder, EngineDecision, FactoryResponseBuilder,
    OpenCodeResponse,
};
use serde_json::Value;

/// The ClaudeHarness - a pure translator
pub struct ClaudeHarness;

/// The CursorHarness - a pure translator for Cursor events
pub struct CursorHarness;

/// The FactoryHarness - a pure translator for Factory AI events
pub struct FactoryHarness;

/// The OpenCodeHarness - a pure translator for OpenCode events
pub struct OpenCodeHarness;

impl ClaudeHarness {
    /// Parse the raw hook event from stdin
    pub fn parse_event(input: &str) -> Result<ClaudeCodeEvent> {
        Ok(serde_json::from_str(input)?)
    }

    /// Format the response for this specific harness
    pub fn format_response(event: &ClaudeCodeEvent, decision: &FinalDecision) -> Result<Value> {
        // 1. Convert our new FinalDecision into the old EngineDecision format
        //    that the response builders expect.
        let engine_decision = Self::adapt_decision(decision);

        // 2. Extract context from FinalDecision for the response builder
        let context = Self::extract_context(decision);

        // 3. Use the spec-compliant response builder with extracted context
        let cupcake_response = ClaudeCodeResponseBuilder::build_response(
            &engine_decision,
            event,
            context,
            false, // suppress_output can be made configurable later
        );

        // 3. Convert the final response to a JSON Value.
        Ok(serde_json::to_value(cupcake_response)?)
    }

    /// This is the ADAPTER function. It's the bridge between the new engine
    /// and the old, correct response builders.
    fn adapt_decision(decision: &FinalDecision) -> EngineDecision {
        match decision {
            FinalDecision::Halt { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Deny { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Block { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Ask { reason, .. } => EngineDecision::Ask {
                reason: reason.clone(),
            },
            FinalDecision::AllowOverride { reason, .. } => EngineDecision::Allow {
                reason: Some(reason.clone()),
            },
            FinalDecision::Allow { context } => EngineDecision::Allow {
                reason: if !context.is_empty() {
                    Some(context.join("\n"))
                } else {
                    None
                },
            },
        }
    }

    /// Extract context information from FinalDecision for response building
    fn extract_context(decision: &FinalDecision) -> Option<Vec<String>> {
        match decision {
            FinalDecision::Allow { context } => {
                if context.is_empty() {
                    None
                } else {
                    Some(context.clone())
                }
            }
            // All other decision types don't carry additional context
            // The reason is already captured in the EngineDecision
            _ => None,
        }
    }
}

impl CursorHarness {
    /// Parse the raw hook event from stdin (Cursor format)
    pub fn parse_event(input: &str) -> Result<CursorEvent> {
        Ok(serde_json::from_str(input)?)
    }

    /// Format the response for Cursor harness
    ///
    /// IMPORTANT: Cursor has more limited response capabilities:
    /// - beforeSubmitPrompt: Only {continue: true/false} - NO context injection
    /// - beforeReadFile: Only {permission: "allow"|"deny"} - minimal schema
    /// - Other events: Full permission model with messages
    pub fn format_response(event: &CursorEvent, decision: &FinalDecision) -> Result<Value> {
        // 1. Convert FinalDecision to EngineDecision format
        let engine_decision = Self::adapt_decision(decision);

        // 2. Extract agent messages for separate user/agent messaging
        let agent_messages = Self::extract_agent_messages(decision);

        // 3. Use Cursor's response builder with agent messages
        let response =
            CursorResponseBuilder::build_response(&engine_decision, event, agent_messages);

        // 4. Return as JSON Value
        Ok(response)
    }

    /// Adapt FinalDecision to EngineDecision (same logic as ClaudeHarness)
    fn adapt_decision(decision: &FinalDecision) -> EngineDecision {
        match decision {
            FinalDecision::Halt { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Deny { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Block { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Ask { reason, .. } => EngineDecision::Ask {
                reason: reason.clone(),
            },
            FinalDecision::AllowOverride { reason, .. } => EngineDecision::Allow {
                reason: Some(reason.clone()),
            },
            FinalDecision::Allow { context } => EngineDecision::Allow {
                reason: if !context.is_empty() {
                    Some(context.join("\n"))
                } else {
                    None
                },
            },
        }
    }

    /// Extract agent messages from FinalDecision
    /// These are used by Cursor to populate the agentMessage field separately from userMessage
    fn extract_agent_messages(decision: &FinalDecision) -> Option<Vec<String>> {
        match decision {
            FinalDecision::Halt { agent_messages, .. }
            | FinalDecision::Deny { agent_messages, .. }
            | FinalDecision::Block { agent_messages, .. }
            | FinalDecision::Ask { agent_messages, .. }
            | FinalDecision::AllowOverride { agent_messages, .. } => {
                if agent_messages.is_empty() {
                    None
                } else {
                    Some(agent_messages.clone())
                }
            }
            FinalDecision::Allow { .. } => None,
        }
    }
}

impl FactoryHarness {
    /// Parse the raw hook event from stdin (Factory AI format)
    pub fn parse_event(input: &str) -> Result<FactoryEvent> {
        Ok(serde_json::from_str(input)?)
    }

    /// Format the response for Factory AI harness
    ///
    /// Factory AI supports the same capabilities as Claude Code with additional features:
    /// - updatedInput for PreToolUse (allows modifying tool parameters)
    /// - permission_mode field in all events
    pub fn format_response(event: &FactoryEvent, decision: &FinalDecision) -> Result<Value> {
        // 1. Convert FinalDecision to EngineDecision format
        let engine_decision = Self::adapt_decision(decision);

        // 2. Extract context for separate context injection
        let context = Self::extract_context(decision);

        // 3. Use Factory's response builder with extracted context
        let cupcake_response = FactoryResponseBuilder::build_response(
            &engine_decision,
            event,
            context,
            false, // suppress_output can be made configurable later
        );

        // 4. Return as JSON Value
        Ok(serde_json::to_value(cupcake_response)?)
    }

    /// Adapt FinalDecision to EngineDecision (same logic as Claude and Cursor)
    fn adapt_decision(decision: &FinalDecision) -> EngineDecision {
        match decision {
            FinalDecision::Halt { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Deny { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Block { reason, .. } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Ask { reason, .. } => EngineDecision::Ask {
                reason: reason.clone(),
            },
            FinalDecision::AllowOverride { reason, .. } => EngineDecision::Allow {
                reason: Some(reason.clone()),
            },
            FinalDecision::Allow { context } => EngineDecision::Allow {
                reason: if !context.is_empty() {
                    Some(context.join("\n"))
                } else {
                    None
                },
            },
        }
    }

    /// Extract context information from FinalDecision for response building
    fn extract_context(decision: &FinalDecision) -> Option<Vec<String>> {
        match decision {
            FinalDecision::Allow { context } => {
                if context.is_empty() {
                    None
                } else {
                    Some(context.clone())
                }
            }
            // All other decision types don't carry additional context
            _ => None,
        }
    }
}

impl OpenCodeHarness {
    /// Parse the raw hook event from stdin (OpenCode format)
    pub fn parse_event(input: &str) -> Result<OpenCodeEvent> {
        Ok(serde_json::from_str(input)?)
    }

    /// Format the response for OpenCode harness
    ///
    /// OpenCode uses a simple JSON response format:
    /// {
    ///   "decision": "allow"|"deny"|"block"|"ask",
    ///   "reason": "...",
    ///   "context": ["..."]
    /// }
    ///
    /// The TypeScript plugin will interpret this and either:
    /// - Throw an error (deny/block/ask)
    /// - Return normally (allow)
    pub fn format_response(_event: &OpenCodeEvent, decision: &FinalDecision) -> Result<Value> {
        let response = match decision {
            FinalDecision::Halt { reason, .. } => OpenCodeResponse::block(reason.clone()),
            FinalDecision::Deny { reason, .. } => OpenCodeResponse::deny(reason.clone()),
            FinalDecision::Block { reason, .. } => OpenCodeResponse::block(reason.clone()),
            FinalDecision::Ask { reason, .. } => {
                // OpenCode plugin will convert "ask" to deny with approval message
                OpenCodeResponse::ask(reason.clone())
            }
            FinalDecision::AllowOverride { reason, .. } => {
                // Allow with reason - context injection TBD in Phase 2
                OpenCodeResponse::allow_with_context(vec![reason.clone()])
            }
            FinalDecision::Allow { context } => {
                if context.is_empty() {
                    OpenCodeResponse::allow()
                } else {
                    OpenCodeResponse::allow_with_context(context.clone())
                }
            }
        };

        Ok(response.to_json_value())
    }
}
