use serde::{Deserialize, Serialize};

/// Common fields present in all Cursor hook events
///
/// Based on Cursor's official hooks specification
///
/// Note: `hook_event_name` is handled by the CursorEvent enum's tag attribute,
/// so it's not included here to avoid conflicts with serde's tagged enum deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonCursorData {
    /// Unique identifier for the conversation
    pub conversation_id: String,

    /// Unique identifier for this specific generation
    pub generation_id: String,

    /// Array of workspace root paths
    #[serde(default)]
    pub workspace_roots: Vec<String>,

    /// The model configured for this generation (e.g., "claude-3-5-sonnet")
    #[serde(default)]
    pub model: Option<String>,

    /// Cursor application version (e.g., "1.7.2")
    #[serde(default)]
    pub cursor_version: Option<String>,

    /// Email address of the authenticated user, if available
    #[serde(default)]
    pub user_email: Option<String>,
}
