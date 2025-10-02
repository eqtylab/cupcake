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
    },
    UserPromptSubmit {
        #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
        additional_context: Option<String>,
    },
    SessionStart {
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

/// Engine decision - maps our PolicyDecision to response actions
#[derive(Debug, Clone)]
pub enum EngineDecision {
    Allow { reason: Option<String> },
    Block { feedback: String },
    Ask { reason: String },
}
