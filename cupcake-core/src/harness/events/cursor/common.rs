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
    pub workspace_roots: Vec<String>,
}
