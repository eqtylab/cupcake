use serde::{Deserialize, Serialize};

/// The main response structure for Claude Code hooks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CupcakeResponse {
    #[serde(skip_serializing_if = "Option::is_none", rename = "hookSpecificOutput")]
    pub hook_specific_output: Option<HookSpecificOutput>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "continue")]
    pub continue_execution: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "stopReason")]
    pub stop_reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "suppressOutput")]
    pub suppress_output: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "systemMessage")]
    pub system_message: Option<String>,
}

impl CupcakeResponse {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Hook-specific output structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "hookEventName")]
pub enum HookSpecificOutput {
    PreToolUse {
        #[serde(rename = "permissionDecision")]
        permission_decision: PermissionDecision,
        #[serde(
            rename = "permissionDecisionReason",
            skip_serializing_if = "Option::is_none"
        )]
        permission_decision_reason: Option<String>,
        /// Updated input for modifying tool parameters (Factory AI specific)
        #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
        updated_input: Option<serde_json::Value>,
    },
    /// PermissionRequest response (newer API with nested decision object)
    PermissionRequest {
        /// Nested decision object containing behavior, updatedInput, and reason
        decision: PermissionRequestDecision,
    },
    UserPromptSubmit {
        #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
        additional_context: Option<String>,
    },
    SessionStart {
        #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
        additional_context: Option<String>,
    },
    PostToolUse {
        #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
        additional_context: Option<String>,
    },
    PreCompact {
        #[serde(rename = "customInstructions", skip_serializing_if = "Option::is_none")]
        custom_instructions: Option<String>,
    },
}

/// Permission decision for PreToolUse
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Decision behavior for PermissionRequest (newer API)
///
/// Only `allow` and `deny` are supported - there is no `ask` because
/// PermissionRequest IS the ask dialog hook (opportunity to bypass user prompt).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionRequestBehavior {
    Allow,
    Deny,
}

/// Decision object for PermissionRequest hook (nested structure)
///
/// For `allow`: optionally pass `updatedInput` to modify tool parameters
/// For `deny`: optionally pass `message` (shown to model) and `interrupt` (stops Claude)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionRequestDecision {
    /// The permission behavior: allow or deny
    pub behavior: PermissionRequestBehavior,

    /// Optional updated input for modifying tool parameters (used with allow)
    #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,

    /// Message explaining why permission was denied (used with deny)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Whether to interrupt/stop Claude entirely (used with deny)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<bool>,
}

/// Engine decision - maps our PolicyDecision to response actions
#[derive(Debug, Clone)]
pub enum EngineDecision {
    Allow { reason: Option<String> },
    Block { feedback: String },
    Ask { reason: String },
    Modify { reason: String, updated_input: serde_json::Value },
}
