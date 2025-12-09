use serde::{Deserialize, Serialize};

use super::common::CommonCursorData;

/// Cursor's afterShellExecution hook event
///
/// Fired after a shell command executes; useful for auditing or collecting metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterShellExecutionPayload {
    #[serde(flatten)]
    pub common: CommonCursorData,

    /// The full terminal command that was executed
    pub command: String,

    /// Full output captured from the terminal
    pub output: String,

    /// Duration in milliseconds spent executing the shell command (excludes approval wait time)
    pub duration: u64,
}
