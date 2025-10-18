use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's stop hook event
///
/// Fired when the agent loop ends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// Status of the agent loop: "completed", "aborted", or "error"
    pub status: String,
}
