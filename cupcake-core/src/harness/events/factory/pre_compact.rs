use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Type of compaction trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    /// User-initiated via /compact command
    Manual,
    /// Automatic due to full context
    Auto,
}

impl std::fmt::Display for CompactTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompactTrigger::Manual => write!(f, "manual"),
            CompactTrigger::Auto => write!(f, "auto"),
        }
    }
}

/// Payload for PreCompact hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactPayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// Whether this is manual or automatic compaction
    pub trigger: CompactTrigger,

    /// Custom instructions (only for manual compaction)
    #[serde(default)]
    pub custom_instructions: String,
}
