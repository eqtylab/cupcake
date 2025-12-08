use super::{CommonEventData, EventPayload, InjectsContext, SessionSource};
use serde::{Deserialize, Serialize};

/// Payload for SessionStart hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// Source of the session start
    pub source: SessionSource,
}

impl EventPayload for SessionStartPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

// SessionStart can inject context via stdout
impl InjectsContext for SessionStartPayload {}

impl SessionStartPayload {
    /// Check if this is a normal startup
    pub fn is_startup(&self) -> bool {
        matches!(self.source, SessionSource::Startup)
    }

    /// Check if this is a resumed session
    pub fn is_resume(&self) -> bool {
        matches!(self.source, SessionSource::Resume)
    }

    /// Check if this is after a clear command
    pub fn is_clear(&self) -> bool {
        matches!(self.source, SessionSource::Clear)
    }

    /// Check if this is after a compact operation
    pub fn is_compact(&self) -> bool {
        matches!(self.source, SessionSource::Compact)
    }

    /// Get source as string
    pub fn source_str(&self) -> &'static str {
        match self.source {
            SessionSource::Startup => "startup",
            SessionSource::Resume => "resume",
            SessionSource::Clear => "clear",
            SessionSource::Compact => "compact",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_start_payload() {
        let startup_payload = SessionStartPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            source: SessionSource::Startup,
        };

        assert_eq!(startup_payload.common().session_id, "test-123");
        assert!(startup_payload.is_startup());
        assert!(!startup_payload.is_resume());
        assert!(!startup_payload.is_clear());
        assert_eq!(startup_payload.source_str(), "startup");

        let resume_payload = SessionStartPayload {
            common: startup_payload.common.clone(),
            source: SessionSource::Resume,
        };

        assert!(!resume_payload.is_startup());
        assert!(resume_payload.is_resume());
        assert!(!resume_payload.is_clear());
        assert_eq!(resume_payload.source_str(), "resume");
    }
}
