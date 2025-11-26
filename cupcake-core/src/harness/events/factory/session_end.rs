use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Reason for session end
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionEndReason {
    /// Session cleared with /clear command
    Clear,
    /// User logged out
    Logout,
    /// User exited while prompt input was visible
    PromptInputExit,
    /// Other exit reasons
    Other,
}

impl std::fmt::Display for SessionEndReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionEndReason::Clear => write!(f, "clear"),
            SessionEndReason::Logout => write!(f, "logout"),
            SessionEndReason::PromptInputExit => write!(f, "prompt_input_exit"),
            SessionEndReason::Other => write!(f, "other"),
        }
    }
}

/// Payload for SessionEnd hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// The reason for session end
    pub reason: SessionEndReason,
}
