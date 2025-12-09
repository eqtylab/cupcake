use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's afterAgentResponse hook event
///
/// Called after the agent has completed an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterAgentResponsePayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// The assistant's final text response
    pub text: String,
}
