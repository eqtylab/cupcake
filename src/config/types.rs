use serde::{Deserialize, Serialize};

use super::actions::Action;
use super::conditions::Condition;

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

/// Hook event types that policies can respond to
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum HookEventType {
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
    SubagentStop,
    PreCompact,
}

impl std::fmt::Display for HookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEventType::PreToolUse => write!(f, "PreToolUse"),
            HookEventType::PostToolUse => write!(f, "PostToolUse"),
            HookEventType::Notification => write!(f, "Notification"),
            HookEventType::Stop => write!(f, "Stop"),
            HookEventType::SubagentStop => write!(f, "SubagentStop"),
            HookEventType::PreCompact => write!(f, "PreCompact"),
        }
    }
}

// =============================================================================
// YAML-Based Policy Types (Plan 005)
// =============================================================================

/// Root configuration file structure for guardrails/cupcake.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootConfig {
    /// Global settings for the policy engine
    #[serde(default)]
    pub settings: Settings,

    /// Glob patterns for importing policy fragment files
    #[serde(default)]
    pub imports: Vec<String>,
}

impl Default for RootConfig {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            imports: vec!["policies/*.yaml".to_string()],
        }
    }
}

/// Simplified policy structure for YAML fragments (without hook_event/matcher)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlPolicy {
    /// Human-readable policy name
    pub name: String,

    /// Optional longer description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Conditions that must all be true for policy to trigger
    pub conditions: Vec<Condition>,

    /// Action to take when all conditions match
    pub action: Action,
}

/// Type alias for the "Grouped Map" structure of a single policy file
/// Structure: { "HookEvent": { "Matcher": [Policy, Policy, ...] } }
pub type PolicyFragment =
    std::collections::HashMap<String, std::collections::HashMap<String, Vec<YamlPolicy>>>;

/// Final composed policy structure for engine consumption
/// This restores the hook_event and matcher fields from the YAML structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposedPolicy {
    /// Human-readable policy name
    pub name: String,

    /// Optional longer description
    pub description: Option<String>,

    /// Hook event when to evaluate this policy (restored from YAML structure)
    pub hook_event: HookEventType,

    /// Tool name pattern (restored from YAML structure)
    pub matcher: String,

    /// Conditions that must all be true for policy to trigger
    pub conditions: Vec<Condition>,

    /// Action to take when all conditions match
    pub action: Action,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(!settings.audit_logging);
        assert!(!settings.debug_mode);
    }
}
