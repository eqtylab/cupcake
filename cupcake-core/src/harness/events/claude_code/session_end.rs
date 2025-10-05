use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};

/// Reason why a Claude Code session ended
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionEndReason {
    /// Session cleared with /clear command
    Clear,
    /// User logged out
    Logout,
    /// User exited while prompt input was visible
    #[serde(rename = "prompt_input_exit")]
    PromptInputExit,
    /// Other exit reasons
    Other,
}

/// Payload for SessionEnd hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Reason for session termination
    pub reason: SessionEndReason,
}

impl EventPayload for SessionEndPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl SessionEndPayload {
    /// Check if session ended due to clear command
    pub fn is_clear(&self) -> bool {
        matches!(self.reason, SessionEndReason::Clear)
    }

    /// Check if session ended due to logout
    pub fn is_logout(&self) -> bool {
        matches!(self.reason, SessionEndReason::Logout)
    }

    /// Check if session ended from prompt input exit
    pub fn is_prompt_input_exit(&self) -> bool {
        matches!(self.reason, SessionEndReason::PromptInputExit)
    }

    /// Get reason as string
    pub fn reason_str(&self) -> &'static str {
        match self.reason {
            SessionEndReason::Clear => "clear",
            SessionEndReason::Logout => "logout",
            SessionEndReason::PromptInputExit => "prompt_input_exit",
            SessionEndReason::Other => "other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_end_deserialization() {
        let test_cases = vec![
            ("clear", SessionEndReason::Clear),
            ("logout", SessionEndReason::Logout),
            ("prompt_input_exit", SessionEndReason::PromptInputExit),
            ("other", SessionEndReason::Other),
        ];

        for (reason_str, expected_reason) in test_cases {
            let json = format!(
                r#"
                {{
                    "session_id": "test-session",
                    "transcript_path": "/path/to/transcript",
                    "cwd": "/home/user/project",
                    "reason": "{reason_str}"
                }}
                "#
            );

            let payload: SessionEndPayload = serde_json::from_str(&json).unwrap();
            assert_eq!(payload.common.session_id, "test-session");
            assert_eq!(payload.reason, expected_reason);
        }
    }

    #[test]
    fn test_session_end_helpers() {
        let clear_payload = SessionEndPayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            reason: SessionEndReason::Clear,
        };

        assert!(clear_payload.is_clear());
        assert!(!clear_payload.is_logout());
        assert_eq!(clear_payload.reason_str(), "clear");
    }
}
