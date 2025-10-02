//! Tracing utilities for policy evaluation debugging
//!
//! Provides trace ID generation and context management for distributed tracing
//! of policy evaluations. Uses UUID v7 for time-based, sortable identifiers.

use uuid::Uuid;

/// Generate a unique trace ID for policy evaluation
///
/// Uses UUID v7 which is time-based and sortable, making it ideal for
/// distributed tracing and log correlation.
///
/// # Example
/// ```
/// use cupcake_core::engine::trace::generate_trace_id;
///
/// let trace_id = generate_trace_id();
/// tracing::info!(trace_id = %trace_id, "Starting evaluation");
/// ```
pub fn generate_trace_id() -> String {
    Uuid::now_v7().to_string()
}

/// Extract session ID from input if available
///
/// Claude Code events include a session_id that can be used for correlation.
/// This function safely extracts it from the input JSON.
pub fn extract_session_id(input: &serde_json::Value) -> Option<String> {
    input
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract event name from input
///
/// Supports both camelCase (hookEventName) and snake_case (hook_event_name)
/// for compatibility with different Claude Code versions.
pub fn extract_event_name(input: &serde_json::Value) -> Option<String> {
    input
        .get("hookEventName")
        .or_else(|| input.get("hook_event_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract tool name from input if available
pub fn extract_tool_name(input: &serde_json::Value) -> Option<String> {
    input
        .get("tool_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();

        // Should be unique
        assert_ne!(id1, id2);

        // Should be valid UUID format
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());

        // Should be UUID v7 (time-based and sortable)
        let uuid1 = Uuid::parse_str(&id1).unwrap();
        let uuid2 = Uuid::parse_str(&id2).unwrap();
        assert_eq!(uuid1.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(uuid2.get_version(), Some(uuid::Version::SortRand));
    }

    #[test]
    fn test_extract_session_id() {
        let input = json!({
            "session_id": "test-123",
            "hook_event_name": "PreToolUse"
        });

        assert_eq!(extract_session_id(&input), Some("test-123".to_string()));

        let input_no_session = json!({
            "hook_event_name": "PreToolUse"
        });

        assert_eq!(extract_session_id(&input_no_session), None);
    }

    #[test]
    fn test_extract_event_name() {
        // Test camelCase
        let input_camel = json!({
            "hookEventName": "PreToolUse"
        });
        assert_eq!(
            extract_event_name(&input_camel),
            Some("PreToolUse".to_string())
        );

        // Test snake_case
        let input_snake = json!({
            "hook_event_name": "PostToolUse"
        });
        assert_eq!(
            extract_event_name(&input_snake),
            Some("PostToolUse".to_string())
        );

        // Test both present (camelCase takes precedence)
        let input_both = json!({
            "hookEventName": "PreToolUse",
            "hook_event_name": "PostToolUse"
        });
        assert_eq!(
            extract_event_name(&input_both),
            Some("PreToolUse".to_string())
        );
    }

    #[test]
    fn test_extract_tool_name() {
        let input = json!({
            "tool_name": "Bash",
            "hook_event_name": "PreToolUse"
        });

        assert_eq!(extract_tool_name(&input), Some("Bash".to_string()));

        let input_no_tool = json!({
            "hook_event_name": "UserPromptSubmit"
        });

        assert_eq!(extract_tool_name(&input_no_tool), None);
    }
}
