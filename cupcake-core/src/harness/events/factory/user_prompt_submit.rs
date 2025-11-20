use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Payload for UserPromptSubmit hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// The user's prompt text
    pub prompt: String,
}
