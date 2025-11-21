use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Payload for Stop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// Whether a stop hook is already active (prevents infinite loops)
    pub stop_hook_active: bool,
}
