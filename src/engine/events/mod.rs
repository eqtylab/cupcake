//! Abstract event layer for multi-agent support
//!
//! This module provides the top-level abstraction for handling events
//! from different AI coding agents. Currently supports Claude Code,
//! but designed for extensibility.

pub mod claude_code;

// Re-export commonly used types
pub use claude_code::{ClaudeCodeEvent, CommonEventData, CompactTrigger, SessionSource};

use serde::{Deserialize, Serialize};

/// Top-level enum for all agent events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentEvent {
    /// Events from Claude Code agent
    ClaudeCode(claude_code::ClaudeCodeEvent),
    // Future: GitHub Copilot, Amazon CodeWhisperer, etc.
}

impl AgentEvent {
    /// Get the event name for logging and routing
    pub fn event_name(&self) -> &'static str {
        match self {
            AgentEvent::ClaudeCode(event) => event.event_name(),
        }
    }

    /// Get the common data (session_id, cwd, etc)
    pub fn common_data(&self) -> &claude_code::CommonEventData {
        match self {
            AgentEvent::ClaudeCode(event) => event.common(),
        }
    }
}
