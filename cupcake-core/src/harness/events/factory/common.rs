use serde::{Deserialize, Serialize};

/// Permission mode for Factory AI Droid
/// Indicates the current permission mode when the hook is invoked
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Normal permission checks
    Default,
    /// Planning mode
    Plan,
    /// Auto-accept edits
    AcceptEdits,
    /// Bypass all permission checks
    BypassPermissions,
}

impl Default for PermissionMode {
    fn default() -> Self {
        PermissionMode::Default
    }
}

/// Common fields present in all Factory AI Droid hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonFactoryData {
    /// Unique identifier for the Droid session
    pub session_id: String,

    /// Path to the session transcript file
    pub transcript_path: String,

    /// Current working directory when the hook is invoked
    pub cwd: String,

    /// Current permission mode
    pub permission_mode: PermissionMode,
}
