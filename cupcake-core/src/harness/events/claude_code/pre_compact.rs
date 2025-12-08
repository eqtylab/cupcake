use super::{CommonEventData, CompactTrigger, EventPayload, InjectsContext};
use serde::{Deserialize, Serialize};

/// Payload for PreCompact hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Whether compaction was triggered manually or automatically
    pub trigger: CompactTrigger,

    /// Custom instructions for manual compaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

impl EventPayload for PreCompactPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

// PreCompact can inject context via stdout
impl InjectsContext for PreCompactPayload {}

impl PreCompactPayload {
    /// Check if this is a manual compaction
    pub fn is_manual(&self) -> bool {
        matches!(self.trigger, CompactTrigger::Manual)
    }

    /// Check if this is an automatic compaction
    pub fn is_auto(&self) -> bool {
        matches!(self.trigger, CompactTrigger::Auto)
    }

    /// Get custom instructions if present
    pub fn instructions(&self) -> Option<&str> {
        self.custom_instructions.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_compact_payload() {
        let manual_payload = PreCompactPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            trigger: CompactTrigger::Manual,
            custom_instructions: Some("Keep technical details".to_string()),
        };

        assert_eq!(manual_payload.common().session_id, "test-123");
        assert!(manual_payload.is_manual());
        assert!(!manual_payload.is_auto());
        assert_eq!(
            manual_payload.instructions(),
            Some("Keep technical details")
        );

        let auto_payload = PreCompactPayload {
            common: manual_payload.common.clone(),
            trigger: CompactTrigger::Auto,
            custom_instructions: None,
        };

        assert!(!auto_payload.is_manual());
        assert!(auto_payload.is_auto());
        assert_eq!(auto_payload.instructions(), None);
    }
}
