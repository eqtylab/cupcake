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
pub fn build_cupcake_hooks() -> Value {
    json!({
        "PreToolUse": [
            {
                "matcher": "*",
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
                "matcher": "*",
                "hooks": [
                    {
                        "type": "command",
                        "command": "cupcake run --event PreCompact",
                        "timeout": 1  // timeout in seconds per Claude Code spec
                    }
                ]
            }
        ]
    })
}