use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's stop hook event
///
/// Fired when the agent loop ends.
/// Can optionally auto-submit a follow-up user message to keep iterating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// Status of the agent loop: "completed", "aborted", or "error"
    pub status: String,

    /// How many times the stop hook has already triggered an automatic follow-up
    /// for this conversation (starts at 0). Maximum of 5 auto follow-ups enforced.
    #[serde(default)]
    pub loop_count: u32,
}
