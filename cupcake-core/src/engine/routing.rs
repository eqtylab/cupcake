//! Routing module - Maps events to policy subsets
//!
//! Implements the NEW_GUIDING_FINAL.md metadata-driven routing:
//! "Route: Cupcake uses the event data to consult its internal routing map,
//! instantly identifying the small subset of policy units that are relevant"
//!
//! Updated for Host-Side Indexing via OPA metadata

use super::RoutingDirective;

/// Create a routing key from a metadata directive for the routing map
/// This determines how policies are indexed for fast lookup via Host-Side Indexing
pub fn create_routing_key_from_metadata(directive: &RoutingDirective) -> Vec<String> {
    let mut keys = Vec::new();

    // If no events specified, this is likely a system/aggregation policy
    // System policies don't need routing - they're called directly
    if directive.required_events.is_empty() {
        tracing::debug!("Directive has no required_events - policy will not be routed");
        return keys;
    }

    // Generate keys for each event/tool combination
    for event in &directive.required_events {
        if directive.required_tools.is_empty() {
            // Event with no tool constraints (UserPromptSubmit, Stop, etc.)
            keys.push(event.clone());
        } else {
            // Event with tool constraints - create key for each tool
            for tool in &directive.required_tools {
                if tool == "*" {
                    // Wildcard - matches all tools for this event
                    keys.push(format!("{event}:*"));
                } else {
                    // Specific tool
                    keys.push(format!("{event}:{tool}"));
                }
            }
        }
    }

    keys
}

/// Create all routing keys for a metadata directive (handles multiple events/tools)
/// This is the primary function used by the engine for building the routing map
pub fn create_all_routing_keys_from_metadata(directive: &RoutingDirective) -> Vec<String> {
    create_routing_key_from_metadata(directive)
}

/// Create an event key for routing lookup
pub fn create_event_key(event_name: &str, tool_name: Option<&str>) -> String {
    match tool_name {
        Some(tool) => format!("{event_name}:{tool}"),
        None => event_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_routing_key_no_tools() {
        let directive = RoutingDirective {
            required_events: vec!["UserPromptSubmit".to_string()],
            required_tools: vec![],
            required_signals: vec![],
        };

        let keys = create_routing_key_from_metadata(&directive);
        assert_eq!(keys, vec!["UserPromptSubmit"]);
    }

    #[test]
    fn test_create_routing_key_with_tool() {
        let directive = RoutingDirective {
            required_events: vec!["PreToolUse".to_string()],
            required_tools: vec!["Bash".to_string()],
            required_signals: vec![],
        };

        let keys = create_routing_key_from_metadata(&directive);
        assert_eq!(keys, vec!["PreToolUse:Bash"]);
    }

    #[test]
    fn test_create_routing_key_wildcard() {
        let directive = RoutingDirective {
            required_events: vec!["PreToolUse".to_string()],
            required_tools: vec!["*".to_string()],
            required_signals: vec![],
        };

        let keys = create_routing_key_from_metadata(&directive);
        assert_eq!(keys, vec!["PreToolUse:*"]);
    }

    #[test]
    fn test_create_all_routing_keys_multiple_tools() {
        let directive = RoutingDirective {
            required_events: vec!["PreToolUse".to_string()],
            required_tools: vec!["Bash".to_string(), "Shell".to_string(), "Exec".to_string()],
            required_signals: vec![],
        };

        let keys = create_all_routing_keys_from_metadata(&directive);
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"PreToolUse:Bash".to_string()));
        assert!(keys.contains(&"PreToolUse:Shell".to_string()));
        assert!(keys.contains(&"PreToolUse:Exec".to_string()));
    }

    #[test]
    fn test_create_routing_keys_multiple_events() {
        let directive = RoutingDirective {
            required_events: vec!["PreToolUse".to_string(), "PostToolUse".to_string()],
            required_tools: vec!["Bash".to_string()],
            required_signals: vec![],
        };

        let keys = create_all_routing_keys_from_metadata(&directive);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"PreToolUse:Bash".to_string()));
        assert!(keys.contains(&"PostToolUse:Bash".to_string()));
    }
}

// Aligns with NEW_GUIDING_FINAL.md:
// - Implements Host-Side Indexing via OPA metadata parsing
// - Enables instant policy subset selection (O(1) lookups)
// - Creates efficient routing keys from metadata directives
// - Supports wildcard (*) matching for all tools
// - Handles multiple events and tools per policy
// - Foundation for the metadata-driven Hybrid Model
