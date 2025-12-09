use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's afterAgentThought hook event
///
/// Called after the agent completes a thinking block.
/// Useful for observing the agent's reasoning process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterAgentThoughtPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// Fully aggregated thinking text for the completed block
    pub text: String,

    /// Duration in milliseconds for the thinking block (optional)
    #[serde(default)]
    pub duration_ms: Option<u64>,
}
