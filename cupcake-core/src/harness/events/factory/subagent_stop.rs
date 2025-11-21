use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Payload for SubagentStop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// Whether a stop hook is already active (prevents infinite loops)
    pub stop_hook_active: bool,
}
