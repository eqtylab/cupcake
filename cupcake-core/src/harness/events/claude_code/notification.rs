use super::{CommonEventData, EventPayload};
use serde::{Deserialize, Serialize};

/// Type of notification sent by Claude Code
///
/// Used to filter notifications and run different hooks for different types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    /// Permission requests from Claude Code
    PermissionPrompt,
    /// When Claude is waiting for user input (after 60+ seconds idle)
    IdlePrompt,
    /// Authentication success notifications
    AuthSuccess,
    /// When Claude Code needs input for MCP tool elicitation
    ElicitationDialog,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationType::PermissionPrompt => write!(f, "permission_prompt"),
            NotificationType::IdlePrompt => write!(f, "idle_prompt"),
            NotificationType::AuthSuccess => write!(f, "auth_success"),
            NotificationType::ElicitationDialog => write!(f, "elicitation_dialog"),
        }
    }
}

/// Payload for Notification hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// The notification message
    pub message: String,

    /// Type of notification (for filtering hooks)
    #[serde(default)]
    pub notification_type: Option<NotificationType>,
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

    /// Get the notification type if present
    pub fn notification_type(&self) -> Option<&NotificationType> {
        self.notification_type.as_ref()
    }

    /// Check if this is a permission prompt notification
    pub fn is_permission_prompt(&self) -> bool {
        matches!(
            self.notification_type,
            Some(NotificationType::PermissionPrompt)
        )
    }

    /// Check if this is an idle prompt notification
    pub fn is_idle_prompt(&self) -> bool {
        matches!(self.notification_type, Some(NotificationType::IdlePrompt))
    }

    /// Check if this is an auth success notification
    pub fn is_auth_success(&self) -> bool {
        matches!(self.notification_type, Some(NotificationType::AuthSuccess))
    }

    /// Check if this is an elicitation dialog notification
    pub fn is_elicitation_dialog(&self) -> bool {
        matches!(
            self.notification_type,
            Some(NotificationType::ElicitationDialog)
        )
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
                permission_mode: Default::default(),
            },
            message: "Build completed successfully".to_string(),
            notification_type: None,
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert_eq!(payload.message(), "Build completed successfully");
        assert!(payload.contains("completed"));
        assert!(!payload.contains("failed"));
        assert_eq!(payload.len(), 28);
        assert!(!payload.is_empty());
        assert!(!payload.is_permission_prompt());
    }

    #[test]
    fn test_notification_with_type() {
        let payload = NotificationPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            message: "Claude needs your permission to use Bash".to_string(),
            notification_type: Some(NotificationType::PermissionPrompt),
        };

        assert!(payload.is_permission_prompt());
        assert!(!payload.is_idle_prompt());
        assert!(!payload.is_auth_success());
        assert!(!payload.is_elicitation_dialog());
        assert_eq!(
            payload.notification_type(),
            Some(&NotificationType::PermissionPrompt)
        );
    }

    #[test]
    fn test_notification_type_serialization() {
        // Test that notification_type serializes to snake_case
        let notification_type = NotificationType::PermissionPrompt;
        let json = serde_json::to_string(&notification_type).unwrap();
        assert_eq!(json, r#""permission_prompt""#);

        let notification_type = NotificationType::IdlePrompt;
        let json = serde_json::to_string(&notification_type).unwrap();
        assert_eq!(json, r#""idle_prompt""#);

        let notification_type = NotificationType::AuthSuccess;
        let json = serde_json::to_string(&notification_type).unwrap();
        assert_eq!(json, r#""auth_success""#);

        let notification_type = NotificationType::ElicitationDialog;
        let json = serde_json::to_string(&notification_type).unwrap();
        assert_eq!(json, r#""elicitation_dialog""#);
    }

    #[test]
    fn test_notification_deserialization() {
        let json = r#"
        {
            "session_id": "abc123",
            "transcript_path": "/path/to/transcript",
            "cwd": "/home/user",
            "hook_event_name": "Notification",
            "message": "Claude needs your permission",
            "notification_type": "permission_prompt"
        }
        "#;

        let payload: NotificationPayload = serde_json::from_str(json).unwrap();
        assert_eq!(
            payload.notification_type,
            Some(NotificationType::PermissionPrompt)
        );
        assert!(payload.is_permission_prompt());
    }
}
