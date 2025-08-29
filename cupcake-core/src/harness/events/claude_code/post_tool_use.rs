//! PostToolUse event payload

use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Payload for PostToolUse events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUsePayload {
    /// Common event data
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Name of the tool that was executed
    pub tool_name: String,

    /// Input parameters that were provided to the tool
    pub tool_input: Value,

    /// Response from the tool execution
    pub tool_response: Value,
}

impl EventPayload for PostToolUsePayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl PostToolUsePayload {
    /// Check if the tool execution was successful
    pub fn was_successful(&self) -> Option<bool> {
        self.tool_response.get("success").and_then(|v| v.as_bool())
    }

    /// Get the tool output if available
    pub fn get_output(&self) -> Option<&str> {
        self.tool_response.get("output").and_then(|v| v.as_str())
    }

    /// Get error message if the tool failed
    pub fn get_error(&self) -> Option<&str> {
        self.tool_response.get("error").and_then(|v| v.as_str())
    }
}
