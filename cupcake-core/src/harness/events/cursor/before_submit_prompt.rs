use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Attachment for prompt events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAttachment {
    #[serde(rename = "type")]
    pub attachment_type: String, // "file" or "rule"

    /// The file path (Cursor uses camelCase "filePath" in JSON)
    #[serde(rename = "filePath")]
    pub file_path: String,
}

/// Cursor's beforeSubmitPrompt hook event
///
/// Fired right after user hits send but before backend request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeforeSubmitPromptPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// The user's prompt text
    pub prompt: String,

    /// Any file or rule attachments
    #[serde(default)]
    pub attachments: Vec<PromptAttachment>,
}
