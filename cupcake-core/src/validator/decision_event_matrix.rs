//! Decision-Event Compatibility Matrix
//!
//! This module defines which decision verbs are valid for which Claude Code events.
//! Based on the official Claude Code hooks specification.

use std::collections::HashMap;

/// Decision verbs that policies can use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionVerb {
    Halt,
    Deny,
    Block,
    Ask,
    Modify,
    AddContext,
}

impl DecisionVerb {
    /// Get all decision verbs
    pub fn all() -> Vec<Self> {
        vec![
            Self::Halt,
            Self::Deny,
            Self::Block,
            Self::Ask,
            Self::Modify,
            Self::AddContext,
        ]
    }

    /// Parse decision verb from string (as it appears in Rego)
    pub fn from_rego_name(name: &str) -> Option<Self> {
        match name {
            "halt" => Some(Self::Halt),
            "deny" => Some(Self::Deny),
            "block" => Some(Self::Block),
            "ask" => Some(Self::Ask),
            "modify" => Some(Self::Modify),
            "add_context" => Some(Self::AddContext),
            _ => None,
        }
    }

    /// Get the Rego name for this verb (as it appears in policies)
    pub fn rego_name(&self) -> &'static str {
        match self {
            Self::Halt => "halt",
            Self::Deny => "deny",
            Self::Block => "block",
            Self::Ask => "ask",
            Self::Modify => "modify",
            Self::AddContext => "add_context",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Halt => "Immediate cessation (highest priority)",
            Self::Deny => "Block action with feedback to Claude",
            Self::Block => "Block action (post-execution feedback)",
            Self::Ask => "Request user confirmation",
            Self::Modify => "Modify tool input before execution",
            Self::AddContext => "Inject additional context",
        }
    }
}

/// Compatibility matrix for decision verbs and events
pub struct DecisionEventMatrix {
    compatibility: HashMap<&'static str, Vec<DecisionVerb>>,
}

impl DecisionEventMatrix {
    /// Create the authoritative compatibility matrix
    pub fn new() -> Self {
        let mut compatibility = HashMap::new();

        // PreToolUse: Supports all decision types including Modify
        compatibility.insert(
            "PreToolUse",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Deny,
                DecisionVerb::Block,
                DecisionVerb::Ask,
                DecisionVerb::Modify,
                DecisionVerb::AddContext,
            ],
        );

        // PermissionRequest: Fires when user is shown a permission dialog
        // NO Ask - PermissionRequest IS the ask dialog (opportunity to bypass)
        // Supports: Halt, Deny, Block, Modify (to auto-approve/deny/modify)
        compatibility.insert(
            "PermissionRequest",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Deny,
                DecisionVerb::Block,
                DecisionVerb::Modify,
                // No Ask - doesn't make sense when this IS the ask dialog
                // No AddContext - permission dialogs don't support context injection
            ],
        );

        // PostToolUse: Block (feedback loop), context
        // NO Ask - tool already executed
        compatibility.insert(
            "PostToolUse",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Block,
                DecisionVerb::AddContext,
            ],
        );

        // Stop/SubagentStop: Block (prevent stopping)
        // NO Ask - doesn't make sense for stop events
        compatibility.insert(
            "Stop",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Block,
            ],
        );
        compatibility.insert(
            "SubagentStop",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Block,
            ],
        );

        // UserPromptSubmit: Block (prevent prompt), context
        // NO Ask - doesn't make sense to ask about user's own prompt
        compatibility.insert(
            "UserPromptSubmit",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Block,
                DecisionVerb::AddContext,
            ],
        );

        // SessionStart: Context injection ONLY
        // NO blocking, NO asking - just loads context at session start
        compatibility.insert("SessionStart", vec![DecisionVerb::AddContext]);

        // SessionEnd: No decision control at all
        // Cannot prevent session termination
        compatibility.insert("SessionEnd", vec![]);

        // PreCompact: Context injection (custom instructions)
        compatibility.insert("PreCompact", vec![DecisionVerb::AddContext]);

        // Notification: Block (though rarely used)
        compatibility.insert(
            "Notification",
            vec![
                DecisionVerb::Halt,
                DecisionVerb::Block,
            ],
        );

        Self { compatibility }
    }

    /// Check if a decision verb is compatible with an event
    pub fn is_compatible(&self, event: &str, verb: DecisionVerb) -> bool {
        self.compatibility
            .get(event)
            .map(|verbs| verbs.contains(&verb))
            .unwrap_or(false)
    }

    /// Get all compatible verbs for an event
    pub fn compatible_verbs(&self, event: &str) -> Vec<DecisionVerb> {
        self.compatibility.get(event).cloned().unwrap_or_default()
    }

    /// Get incompatible verbs for an event
    pub fn incompatible_verbs(&self, event: &str) -> Vec<DecisionVerb> {
        let compatible = self.compatible_verbs(event);
        DecisionVerb::all()
            .into_iter()
            .filter(|verb| !compatible.contains(verb))
            .collect()
    }

    /// Check if a verb-event combination is deprecated (but still allowed)
    ///
    /// Returns a deprecation message if the combination is deprecated, None otherwise.
    /// Deprecated combinations still work but should be migrated to preferred alternatives.
    pub fn deprecation_warning(&self, event: &str, verb: DecisionVerb) -> Option<&'static str> {
        match (event, verb) {
            ("PreToolUse", DecisionVerb::Block) | ("PermissionRequest", DecisionVerb::Block) => {
                Some(
                    "Use 'deny' instead of 'block' for pre-execution permission events. \
                     'deny' is for pre-execution rejection (the bouncer); \
                     'block' is for post-execution feedback loops (the inspector). \
                     This will become an error in a future version.",
                )
            }
            _ => None,
        }
    }

    /// Get a helpful error message for an incompatible combination
    pub fn incompatibility_reason(&self, event: &str, verb: DecisionVerb) -> String {
        match (event, verb) {
            ("SessionStart", DecisionVerb::Ask | DecisionVerb::Block | DecisionVerb::Deny | DecisionVerb::Halt) => {
                format!(
                    "SessionStart events only support context injection. '{}' decisions cannot block or prompt for session startup.",
                    verb.rego_name()
                )
            }
            ("SessionEnd", _) => {
                "SessionEnd events do not support decision control. They run for cleanup only and cannot block session termination.".to_string()
            }
            ("PostToolUse", DecisionVerb::Ask) => {
                "PostToolUse events do not support 'ask' decisions because the tool has already executed. Use 'block' for feedback loops.".to_string()
            }
            ("UserPromptSubmit", DecisionVerb::Ask) => {
                "UserPromptSubmit events do not support 'ask' decisions. Use 'block' to prevent prompt processing or 'add_context' to inject information.".to_string()
            }
            ("Stop" | "SubagentStop", DecisionVerb::Ask) => {
                format!("{event} events do not support 'ask' decisions. Use 'block' to prevent stopping.")
            }
            ("PreCompact", DecisionVerb::Ask | DecisionVerb::Block | DecisionVerb::Deny | DecisionVerb::Halt | DecisionVerb::Modify) => {
                format!(
                    "PreCompact events only support 'add_context' for custom instructions. '{}' decisions are not supported.",
                    verb.rego_name()
                )
            }
            (_, DecisionVerb::Modify) => {
                format!(
                    "'modify' decisions are only supported for PreToolUse events. {} events do not support tool input modification.",
                    event
                )
            }
            _ => {
                format!(
                    "'{}' decisions are not supported for {} events. Supported: {}",
                    verb.rego_name(),
                    event,
                    self.compatible_verbs(event)
                        .iter()
                        .map(|v| v.rego_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl Default for DecisionEventMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_tool_use_supports_all() {
        let matrix = DecisionEventMatrix::new();

        // PreToolUse should support everything
        for verb in DecisionVerb::all() {
            assert!(
                matrix.is_compatible("PreToolUse", verb),
                "PreToolUse should support {verb:?}"
            );
        }
    }

    #[test]
    fn test_session_start_only_context() {
        let matrix = DecisionEventMatrix::new();

        assert!(matrix.is_compatible("SessionStart", DecisionVerb::AddContext));
        assert!(!matrix.is_compatible("SessionStart", DecisionVerb::Ask));
        assert!(!matrix.is_compatible("SessionStart", DecisionVerb::Block));
        assert!(!matrix.is_compatible("SessionStart", DecisionVerb::Deny));
        assert!(!matrix.is_compatible("SessionStart", DecisionVerb::Halt));
    }

    #[test]
    fn test_post_tool_use_no_ask() {
        let matrix = DecisionEventMatrix::new();

        assert!(!matrix.is_compatible("PostToolUse", DecisionVerb::Ask));
        assert!(matrix.is_compatible("PostToolUse", DecisionVerb::Block));
        assert!(matrix.is_compatible("PostToolUse", DecisionVerb::AddContext));
    }

    #[test]
    fn test_incompatibility_reasons() {
        let matrix = DecisionEventMatrix::new();

        let reason = matrix.incompatibility_reason("SessionStart", DecisionVerb::Ask);
        assert!(reason.contains("SessionStart"));
        assert!(reason.contains("context injection"));

        let reason = matrix.incompatibility_reason("PostToolUse", DecisionVerb::Ask);
        assert!(reason.contains("already executed"));
    }

    #[test]
    fn test_verb_parsing() {
        assert_eq!(DecisionVerb::from_rego_name("ask"), Some(DecisionVerb::Ask));
        assert_eq!(
            DecisionVerb::from_rego_name("add_context"),
            Some(DecisionVerb::AddContext)
        );
        assert_eq!(
            DecisionVerb::from_rego_name("modify"),
            Some(DecisionVerb::Modify)
        );
        assert_eq!(DecisionVerb::from_rego_name("invalid"), None);
    }

    #[test]
    fn test_modify_only_pre_tool_use() {
        let matrix = DecisionEventMatrix::new();

        // Modify should only be supported for PreToolUse
        assert!(matrix.is_compatible("PreToolUse", DecisionVerb::Modify));
        assert!(!matrix.is_compatible("PostToolUse", DecisionVerb::Modify));
        assert!(!matrix.is_compatible("UserPromptSubmit", DecisionVerb::Modify));
        assert!(!matrix.is_compatible("SessionStart", DecisionVerb::Modify));
        assert!(!matrix.is_compatible("Stop", DecisionVerb::Modify));
    }

    #[test]
    fn test_all_events_have_entries() {
        let matrix = DecisionEventMatrix::new();
        let events = vec![
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "UserPromptSubmit",
            "SessionStart",
            "SessionEnd",
            "Stop",
            "SubagentStop",
            "PreCompact",
            "Notification",
        ];

        for event in events {
            // Should not panic - all events have entries
            let _verbs = matrix.compatible_verbs(event);
        }
    }

    #[test]
    fn test_deprecation_warning_block_pretooluse() {
        let matrix = DecisionEventMatrix::new();

        // block + PreToolUse is deprecated
        let warning = matrix.deprecation_warning("PreToolUse", DecisionVerb::Block);
        assert!(warning.is_some(), "block + PreToolUse should be deprecated");
        assert!(warning.unwrap().contains("deny"));

        // block + PostToolUse is NOT deprecated (correct usage)
        let warning = matrix.deprecation_warning("PostToolUse", DecisionVerb::Block);
        assert!(
            warning.is_none(),
            "block + PostToolUse should NOT be deprecated"
        );

        // deny + PreToolUse is NOT deprecated (correct usage)
        let warning = matrix.deprecation_warning("PreToolUse", DecisionVerb::Deny);
        assert!(
            warning.is_none(),
            "deny + PreToolUse should NOT be deprecated"
        );
    }

    #[test]
    fn test_permission_request_compatibility() {
        let matrix = DecisionEventMatrix::new();

        // PermissionRequest supports: Halt, Deny, Block, Modify
        assert!(matrix.is_compatible("PermissionRequest", DecisionVerb::Halt));
        assert!(matrix.is_compatible("PermissionRequest", DecisionVerb::Deny));
        assert!(matrix.is_compatible("PermissionRequest", DecisionVerb::Block));
        assert!(matrix.is_compatible("PermissionRequest", DecisionVerb::Modify));

        // PermissionRequest does NOT support Ask (it IS the ask dialog)
        assert!(
            !matrix.is_compatible("PermissionRequest", DecisionVerb::Ask),
            "PermissionRequest should NOT support Ask - it IS the ask dialog"
        );

        // PermissionRequest does NOT support AddContext
        assert!(
            !matrix.is_compatible("PermissionRequest", DecisionVerb::AddContext),
            "PermissionRequest should NOT support AddContext"
        );
    }

    #[test]
    fn test_deprecation_warning_block_permission_request() {
        let matrix = DecisionEventMatrix::new();

        // block + PermissionRequest is deprecated (same as PreToolUse)
        let warning = matrix.deprecation_warning("PermissionRequest", DecisionVerb::Block);
        assert!(
            warning.is_some(),
            "block + PermissionRequest should be deprecated"
        );
        assert!(warning.unwrap().contains("deny"));

        // deny + PermissionRequest is NOT deprecated (correct usage)
        let warning = matrix.deprecation_warning("PermissionRequest", DecisionVerb::Deny);
        assert!(
            warning.is_none(),
            "deny + PermissionRequest should NOT be deprecated"
        );
    }
}
