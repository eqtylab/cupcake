//! Merge logic for modification decisions.
//!
//! Handles deep merging of multiple modifications with priority-based conflict resolution.

use serde_json::Value;

use super::super::decision::ModificationObject;

/// Merge multiple modifications with priority-based conflict resolution.
///
/// Algorithm:
/// 1. Sort modifications by priority (descending - highest first)
/// 2. Deep merge `updated_input` objects
/// 3. For conflicting keys, highest priority wins (first-wins after sort)
/// 4. Aggregate reasons from all modifications
pub fn merge_modifications(modifications: &[ModificationObject]) -> (String, Value, Vec<String>) {
    if modifications.is_empty() {
        return (
            "No modifications".to_string(),
            Value::Object(serde_json::Map::new()),
            Vec::new(),
        );
    }

    // Sort by priority descending (highest first)
    let mut sorted: Vec<&ModificationObject> = modifications.iter().collect();
    sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

    // Deep merge updated_inputs
    let mut merged = Value::Object(serde_json::Map::new());
    for modification in &sorted {
        merged = deep_merge(merged, modification.updated_input.clone());
    }

    // Aggregate reasons
    let reason = if sorted.len() == 1 {
        sorted[0].reason.clone()
    } else {
        let reasons: Vec<String> = sorted
            .iter()
            .map(|m| format!("[{}] {}", m.rule_id, m.reason))
            .collect();
        format!("Multiple modifications applied: {}", reasons.join("; "))
    };

    let agent_messages = collect_modification_agent_messages(modifications);

    (reason, merged, agent_messages)
}

/// Deep merge two JSON values.
///
/// For objects: recursively merge, with `base` taking precedence for conflicts.
/// For other types: `base` wins (first-wins after priority sort).
pub fn deep_merge(base: Value, incoming: Value) -> Value {
    match (base, incoming) {
        (Value::Object(mut base_map), Value::Object(incoming_map)) => {
            for (key, incoming_value) in incoming_map {
                if let Some(base_value) = base_map.get(&key) {
                    // Key exists in base - recursively merge if both are objects
                    if base_value.is_object() && incoming_value.is_object() {
                        let merged = deep_merge(base_value.clone(), incoming_value);
                        base_map.insert(key, merged);
                    }
                    // Otherwise base wins (already in place)
                } else {
                    // Key doesn't exist in base - add from incoming
                    base_map.insert(key, incoming_value);
                }
            }
            Value::Object(base_map)
        }
        (base, _) => base, // Base (higher priority) wins for non-objects
    }
}

/// Collect agent-specific messages from modification decisions.
pub fn collect_modification_agent_messages(modifications: &[ModificationObject]) -> Vec<String> {
    modifications
        .iter()
        .filter_map(|m| m.agent_context.clone())
        .collect()
}
