use serde::{Deserialize, Serialize};

use super::actions::Action;
use super::conditions::Condition;

/// Top-level policy configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFile {
    /// Schema version for forward compatibility
    pub schema_version: String,

    /// Global settings
    #[serde(default)]
    pub settings: Settings,

    /// Array of policy definitions
    pub policies: Vec<Policy>,
}

/// Global settings for the policy engine
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Enable structured audit logging
    #[serde(default)]
    pub audit_logging: bool,

    /// Enable verbose debug logging
    #[serde(default)]
    pub debug_mode: bool,
}

/// Individual policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Human-readable policy name
    pub name: String,

    /// Optional longer description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Hook event when to evaluate this policy
    pub hook_event: HookEventType,

    /// Tool name pattern (regex) for PreToolUse/PostToolUse events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,

    /// Conditions that must all be true for policy to trigger
    pub conditions: Vec<Condition>,

    /// Action to take when all conditions match
    pub action: Action,
}

/// Hook event types that policies can respond to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HookEventType {
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
    SubagentStop,
    PreCompact,
}

impl Default for PolicyFile {
    fn default() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            settings: Settings::default(),
            policies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_policy_file_default() {
        let policy_file = PolicyFile::default();
        assert_eq!(policy_file.schema_version, "1.0");
        assert!(!policy_file.settings.audit_logging);
        assert!(!policy_file.settings.debug_mode);
        assert!(policy_file.policies.is_empty());
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(!settings.audit_logging);
        assert!(!settings.debug_mode);
    }
}
