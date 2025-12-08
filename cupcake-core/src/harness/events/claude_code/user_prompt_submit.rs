use super::{CommonEventData, EventPayload, InjectsContext};
use serde::{Deserialize, Serialize};

/// Payload for UserPromptSubmit hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitPayload {
    #[serde(flatten)]
    pub common: CommonEventData,

    /// The prompt submitted by the user
    pub prompt: String,
}

impl EventPayload for UserPromptSubmitPayload {
    fn common(&self) -> &CommonEventData {
        &self.common
    }
}

// UserPromptSubmit can inject context via stdout
impl InjectsContext for UserPromptSubmitPayload {}

impl UserPromptSubmitPayload {
    /// Get the user's prompt
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Check if prompt contains a specific substring
    pub fn contains(&self, substring: &str) -> bool {
        self.prompt.contains(substring)
    }

    /// Get prompt length
    pub fn len(&self) -> usize {
        self.prompt.len()
    }

    /// Check if prompt is empty
    pub fn is_empty(&self) -> bool {
        self.prompt.is_empty()
    }

    /// Get first N characters of prompt
    pub fn preview(&self, n: usize) -> &str {
        let end = self.prompt.len().min(n);
        &self.prompt[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_prompt_submit_payload() {
        let payload = UserPromptSubmitPayload {
            common: CommonEventData {
                session_id: "test-123".to_string(),
                transcript_path: "/tmp/transcript".to_string(),
                cwd: "/home/user".to_string(),
                permission_mode: Default::default(),
            },
            prompt: "Write a function to calculate factorial".to_string(),
        };

        assert_eq!(payload.common().session_id, "test-123");
        assert_eq!(payload.prompt(), "Write a function to calculate factorial");
        assert!(payload.contains("factorial"));
        assert!(!payload.contains("fibonacci"));
        assert_eq!(payload.len(), 39);
        assert!(!payload.is_empty());
        assert_eq!(payload.preview(10), "Write a fu");
    }
}
