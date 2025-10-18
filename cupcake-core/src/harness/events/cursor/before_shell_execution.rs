use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's beforeShellExecution hook event
///
/// Fired before any shell command is executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeforeShellExecutionPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// The full shell command to be executed
    pub command: String,

    /// Current working directory
    pub cwd: String,
}
