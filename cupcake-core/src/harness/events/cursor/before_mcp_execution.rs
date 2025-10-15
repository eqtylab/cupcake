use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's beforeMCPExecution hook event
///
/// Fired before any MCP tool is executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeforeMCPExecutionPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// The name of the MCP tool being invoked
    pub tool_name: String,

    /// The JSON parameters being passed to the tool
    pub tool_input: serde_json::Value,

    /// Server URL (if MCP server)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Command string (if command-based MCP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}
