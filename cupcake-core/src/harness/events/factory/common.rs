use serde::{Deserialize, Serialize};

/// Permission mode for Factory AI Droid
/// Stored as a string to accommodate Factory's various autonomy modes
/// (e.g., "default", "plan", "auto-medium", "auto-full", etc.)
pub type PermissionMode = String;

/// Common fields present in all Factory AI Droid hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
