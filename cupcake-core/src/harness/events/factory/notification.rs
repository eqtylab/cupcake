use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Payload for Notification hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// The notification message
    pub message: String,
}
