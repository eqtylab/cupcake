use super::CommonFactoryData;
use serde::{Deserialize, Serialize};

/// Payload for PreToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUsePayload {
    #[serde(flatten)]
    pub common: CommonFactoryData,

    /// Name of the tool being called
    pub tool_name: String,

    /// Input parameters for the tool
    pub tool_input: serde_json::Value,
}

impl PreToolUsePayload {
    /// Extract tool input as a specific type
    pub fn parse_tool_input<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.tool_input.clone())
    }

    /// Check if this is a specific tool
    pub fn is_tool(&self, name: &str) -> bool {
        self.tool_name == name
    }

    /// Get tool input as a string if it's a simple command
    pub fn get_command(&self) -> Option<String> {
        self.tool_input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get file path from tool input if present
    pub fn get_file_path(&self) -> Option<String> {
        self.tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}
