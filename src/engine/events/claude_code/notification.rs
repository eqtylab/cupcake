use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};

/// Payload for Notification hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// The notification message
    pub message: String,
}

impl EventPayload for NotificationPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

impl NotificationPayload {
    /// Get the notification message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Check if the message contains a specific substring
    pub fn contains(&self, substring: &str) -> bool {
        self.message.contains(substring)
    }

    /// Get message length
    pub fn len(&self) -> usize {
        self.message.len()
    }

    /// Check if message is empty
    pub fn is_empty(&self) -> bool {
        self.message.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_payload() {
        let payload = NotificationPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
            },
            message: "Build completed successfully".to_string(),
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert_eq!(payload.message(), "Build completed successfully");
        assert!(payload.contains("completed"));
        assert!(!payload.contains("failed"));
        assert_eq!(payload.len(), 28);
        assert!(!payload.is_empty());
    }
}
