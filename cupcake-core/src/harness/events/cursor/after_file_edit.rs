use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Individual edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit {
    pub old_string: String,
    pub new_string: String,
}

/// Cursor's afterFileEdit hook event
///
/// Fired after a file has been edited by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterFileEditPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// Absolute path to the file that was edited
    pub file_path: String,

    /// Array of edit operations performed
    pub edits: Vec<FileEdit>,
}
