// Place this code in cupcake-rewrite/src/harness/mod.rs

pub mod events;
pub mod response;

use crate::engine::decision::FinalDecision;
use anyhow::Result;
use events::claude_code::ClaudeCodeEvent;
use response::{ClaudeCodeResponseBuilder, EngineDecision};
use serde_json::Value;

/// The ClaudeHarness - a pure translator
pub struct ClaudeHarness;

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
            FinalDecision::Halt { reason } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Deny { reason } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Block { reason } => EngineDecision::Block {
                feedback: reason.clone(),
            },
            FinalDecision::Ask { reason } => EngineDecision::Ask {
                reason: reason.clone(),
            },
            FinalDecision::AllowOverride { reason } => EngineDecision::Allow {
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
