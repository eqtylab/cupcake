use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};

/// Payload for SubagentStop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Whether stop hook is currently active (prevents infinite loops)
    pub stop_hook_active: bool,
}

impl EventPayload for SubagentStopPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl SubagentStopPayload {
    /// Check if we should allow the subagent to stop
    /// When stop_hook_active is true, we should allow stop to prevent infinite loops
    pub fn should_allow_stop(&self) -> bool {
        self.stop_hook_active
    }

    /// Check if this is the first stop attempt
    pub fn is_first_attempt(&self) -> bool {
        !self.stop_hook_active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_stop_payload() {
        let payload = SubagentStopPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            stop_hook_active: false,
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert!(!payload.stop_hook_active);
        assert!(!payload.should_allow_stop());
        assert!(payload.is_first_attempt());

        let active_payload = SubagentStopPayload {
            common: payload.common.clone(),
            stop_hook_active: true,
        };

        assert!(active_payload.should_allow_stop());
        assert!(!active_payload.is_first_attempt());
    }
}
