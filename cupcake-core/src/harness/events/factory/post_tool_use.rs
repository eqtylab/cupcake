use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Payload for PostToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUsePayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// Name of the tool that was called
    pub tool_name: String,

    /// Input parameters that were used
    pub tool_input: serde_json::Value,

    /// Response from the tool execution
    pub tool_response: serde_json::Value,
}
