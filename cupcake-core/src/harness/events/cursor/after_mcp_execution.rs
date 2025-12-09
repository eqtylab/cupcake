use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's afterMCPExecution hook event
///
/// Fired after an MCP tool executes; includes the tool's input parameters and full JSON result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterMCPExecutionPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// Name of the MCP tool that was executed
    pub tool_name: String,

    /// JSON params string passed to the tool
    pub tool_input: String,

    /// JSON string of the tool response
    pub result_json: String,

    /// Duration in milliseconds spent executing the MCP tool (excludes approval wait time)
    pub duration: u64,
}
