//! Claude Code hooks configuration
//!
//! Provides the standard Cupcake hook configuration for Claude Code integration.

use serde_json::{json, Value};

/// Build the standard Cupcake hook configuration for Claude Code
///
/// This configuration follows the July 20 Claude Code specification with:
/// - Nested array format for hooks
/// - Matcher field for tool events (PreToolUse, PostToolUse, PreCompact)
/// - No matcher field for non-tool events (UserPromptSubmit, Notification, etc.)
/// - Timeout values in seconds per Claude Code spec
/// - `managed_by: "cupcake"` marker for idempotent sync
pub fn build_cupcake_hooks() -> Value {
    json!({
        "PreToolUse": [
            {
                "matcher": "*",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PreToolUse",
                        "timeout": 5  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "PostToolUse": [
            {
                "matcher": "*",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PostToolUse",
                        "timeout": 2  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "UserPromptSubmit": [
            {
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event UserPromptSubmit",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "Notification": [
            {
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event Notification",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "Stop": [
            {
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event Stop",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "SubagentStop": [
            {
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event SubagentStop",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "PreCompact": [
            {
                "matcher": "manual",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PreCompact",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            },
            {
                "matcher": "auto",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PreCompact",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ],
        "SessionStart": [
            {
                "matcher": "startup",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event SessionStart",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            },
            {
                "matcher": "resume",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event SessionStart",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            },
            {
                "matcher": "clear",
                "managed_by": "cupcake",  // Our ownership marker
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event SessionStart",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_managed_by_marker_present() {
        let hooks = build_cupcake_hooks();
        
        // Check each event type has managed_by marker
        for event_type in &["PreToolUse", "PostToolUse", "UserPromptSubmit", 
                            "Notification", "Stop", "SubagentStop", 
                            "PreCompact", "SessionStart"] {
            let event_array = hooks.get(event_type)
                .expect(&format!("{} should exist", event_type))
                .as_array()
                .expect(&format!("{} should be an array", event_type));
            
            for hook_obj in event_array {
                assert_eq!(
                    hook_obj.get("managed_by").and_then(|v| v.as_str()),
                    Some("cupcake"),
                    "{} hook should have managed_by: cupcake marker",
                    event_type
                );
            }
        }
    }

    #[test]
    fn test_precompact_intelligent_matchers() {
        let hooks = build_cupcake_hooks();
        let precompact = hooks["PreCompact"].as_array().unwrap();
        
        // Should have exactly 2 entries
        assert_eq!(precompact.len(), 2, "PreCompact should have 2 matchers");
        
        // Check matchers
        let matchers: Vec<&str> = precompact
            .iter()
            .filter_map(|h| h.get("matcher").and_then(|v| v.as_str()))
            .collect();
        
        assert!(matchers.contains(&"manual"), "Should have manual matcher");
        assert!(matchers.contains(&"auto"), "Should have auto matcher");
        
        // Both should have managed_by marker
        for hook in precompact {
            assert_eq!(
                hook.get("managed_by").and_then(|v| v.as_str()),
                Some("cupcake")
            );
        }
    }

    #[test]
    fn test_session_start_intelligent_matchers() {
        let hooks = build_cupcake_hooks();
        let session_start = hooks["SessionStart"].as_array().unwrap();
        
        // Should have exactly 3 entries
        assert_eq!(session_start.len(), 3, "SessionStart should have 3 matchers");
        
        // Check matchers
        let matchers: Vec<&str> = session_start
            .iter()
            .filter_map(|h| h.get("matcher").and_then(|v| v.as_str()))
            .collect();
        
        assert!(matchers.contains(&"startup"), "Should have startup matcher");
        assert!(matchers.contains(&"resume"), "Should have resume matcher");
        assert!(matchers.contains(&"clear"), "Should have clear matcher");
        
        // All should have managed_by marker
        for hook in session_start {
            assert_eq!(
                hook.get("managed_by").and_then(|v| v.as_str()),
                Some("cupcake")
            );
        }
    }
}
