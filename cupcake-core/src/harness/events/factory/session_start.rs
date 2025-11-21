use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Source of session start event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    /// Normal startup
    Startup,
    /// Resumed via --resume, --continue, or /resume
    Resume,
    /// After /clear command
    Clear,
    /// After compact (auto or manual)
    Compact,
}

impl std::fmt::Display for SessionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionSource::Startup => write!(f, "startup"),
            SessionSource::Resume => write!(f, "resume"),
            SessionSource::Clear => write!(f, "clear"),
            SessionSource::Compact => write!(f, "compact"),
        }
    }
}

/// Payload for SessionStart hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// The source of the session start
    pub source: SessionSource,
}
