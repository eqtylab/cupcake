use crate::config::conditions::StateQueryFilter;
use crate::state::types::{SessionState, StateEvent, ToolUsageEntry};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use std::collections::HashMap;

/// State query engine for evaluating state-based conditions
pub struct StateQuery<'a> {
    state: &'a SessionState,
}

impl<'a> StateQuery<'a> {
    /// Create new state query for the given session state
    pub fn new(state: &'a SessionState) -> Self {
        Self { state }
    }

    /// Check if a tool usage exists for the given tool and path
    pub fn tool_usage_exists(&self, tool: &str, path: &str) -> bool {
        self.state.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                StateEvent::ToolUsage(tool_entry)
                if tool_entry.tool_name == tool
                && tool_entry.success
                && self.matches_path(&tool_entry.input, path)
            )
        })
    }

    /// Check if a tool usage is missing (inverse of exists)
    pub fn tool_usage_missing(&self, tool: &str, path: &str) -> bool {
        !self.tool_usage_exists(tool, path)
    }

    /// Check if a custom event exists
    pub fn custom_event_exists(&self, event_name: &str) -> bool {
        self.state.has_custom_event(event_name)
    }

    /// Check if a custom event is missing
    pub fn custom_event_missing(&self, event_name: &str) -> bool {
        !self.custom_event_exists(event_name)
    }

    /// Execute a complex state query with filters
    pub fn execute_query(&self, query: &StateQueryFilter) -> bool {
        for entry in &self.state.entries {
            if let StateEvent::ToolUsage(tool_entry) = &entry.event {
                if self.matches_query_filters(tool_entry, &entry.timestamp, query) {
                    return true;
                }
            }
        }
        false
    }

    /// Get all file paths that have been read successfully
    pub fn get_read_files(&self) -> Vec<String> {
        self.state
            .entries
            .iter()
            .filter_map(|entry| match &entry.event {
                StateEvent::ToolUsage(tool_entry)
                    if tool_entry.tool_name == "Read" && tool_entry.success =>
                {
                    tool_entry
                        .input
                        .get("file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }
                _ => None,
            })
            .collect()
    }

    /// Get all commands that have been run successfully
    pub fn get_successful_commands(&self) -> Vec<String> {
        self.state
            .entries
            .iter()
            .filter_map(|entry| match &entry.event {
                StateEvent::ToolUsage(tool_entry)
                    if tool_entry.tool_name == "Bash" && tool_entry.success =>
                {
                    tool_entry
                        .input
                        .get("command")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }
                _ => None,
            })
            .collect()
    }

    /// Get all custom events of a specific type
    pub fn get_custom_events(
        &self,
        event_name: &str,
    ) -> Vec<(&DateTime<Utc>, &HashMap<String, serde_json::Value>)> {
        self.state.get_custom_events(event_name)
    }

    /// Get the most recent tool usage of a specific type
    pub fn get_latest_tool_usage(&self, tool_name: &str) -> Option<&ToolUsageEntry> {
        self.state.get_latest_tool_usage(tool_name)
    }

    /// Check if any command matching a pattern has been run successfully
    pub fn has_command_matching(&self, pattern: &Regex) -> bool {
        self.state.has_run_command_matching(pattern)
    }

    /// Get tool usage entries within a time window
    pub fn get_tool_usage_within(&self, tool: &str, minutes: u32) -> Vec<&ToolUsageEntry> {
        let cutoff_time = Utc::now() - Duration::minutes(minutes as i64);

        self.state
            .entries
            .iter()
            .filter_map(|entry| {
                if entry.timestamp >= cutoff_time {
                    match &entry.event {
                        StateEvent::ToolUsage(tool_entry) if tool_entry.tool_name == tool => {
                            Some(tool_entry)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if tool was used successfully within time window
    pub fn tool_used_within(&self, tool: &str, minutes: u32) -> bool {
        !self.get_tool_usage_within(tool, minutes).is_empty()
    }

    /// Get count of tool usages
    pub fn count_tool_usage(&self, tool: &str) -> usize {
        self.state.get_tool_usage(tool).len()
    }

    /// Get count of successful tool usages
    pub fn count_successful_tool_usage(&self, tool: &str) -> usize {
        self.state
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    &entry.event,
                    StateEvent::ToolUsage(tool_entry)
                    if tool_entry.tool_name == tool && tool_entry.success
                )
            })
            .count()
    }

    /// Get count of failed tool usages
    pub fn count_failed_tool_usage(&self, tool: &str) -> usize {
        self.state
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    &entry.event,
                    StateEvent::ToolUsage(tool_entry)
                    if tool_entry.tool_name == tool && !tool_entry.success
                )
            })
            .count()
    }

    /// Check if entry matches query filters
    fn matches_query_filters(
        &self,
        tool_entry: &ToolUsageEntry,
        timestamp: &DateTime<Utc>,
        query: &StateQueryFilter,
    ) -> bool {
        // Check tool name
        if tool_entry.tool_name != query.tool {
            return false;
        }

        // Check command contains filter
        if let Some(command_contains) = &query.command_contains {
            if tool_entry.tool_name == "Bash" {
                if let Some(command) = tool_entry.input.get("command").and_then(|v| v.as_str()) {
                    if !command.contains(command_contains) {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false; // command_contains only applies to Bash tool
            }
        }

        // Check result filter
        if let Some(result) = &query.result {
            let expected_success = match result.as_str() {
                "success" => true,
                "failure" => false,
                _ => return false, // Invalid result value
            };

            if tool_entry.success != expected_success {
                return false;
            }
        }

        // Check time window filter
        if let Some(within_minutes) = query.within_minutes {
            let cutoff_time = Utc::now() - Duration::minutes(within_minutes as i64);
            if *timestamp < cutoff_time {
                return false;
            }
        }

        true
    }

    /// Check if tool input matches the given path pattern
    fn matches_path(&self, input: &HashMap<String, serde_json::Value>, path: &str) -> bool {
        // For most tools, check file_path parameter
        if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
            return file_path == path;
        }

        // For Bash tool, could match against command (not implemented here)
        // For other tools, might have different path parameters

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::types::{SessionState, StateEntry};
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn create_test_session() -> SessionState {
        let mut state = SessionState::new("test-session".to_string());

        // Add successful Read
        let mut read_input = HashMap::new();
        read_input.insert(
            "file_path".to_string(),
            serde_json::Value::String("README.md".to_string()),
        );
        let read_entry = StateEntry::new_tool_usage(
            "Read".to_string(),
            read_input,
            true,
            Some(serde_json::Value::String(
                "# Project\nDescription".to_string(),
            )),
            Some(50),
        );
        state.add_entry(read_entry);

        // Add successful Bash command
        let mut bash_input = HashMap::new();
        bash_input.insert(
            "command".to_string(),
            serde_json::Value::String("git commit -m 'test'".to_string()),
        );
        let bash_entry = StateEntry::new_tool_usage(
            "Bash".to_string(),
            bash_input,
            true,
            Some(serde_json::Value::String(
                "Committed successfully".to_string(),
            )),
            Some(1000),
        );
        state.add_entry(bash_entry);

        // Add failed npm test
        let mut npm_input = HashMap::new();
        npm_input.insert(
            "command".to_string(),
            serde_json::Value::String("npm test".to_string()),
        );
        let npm_entry = StateEntry::new_tool_usage(
            "Bash".to_string(),
            npm_input,
            false,
            Some(serde_json::Value::String("Tests failed".to_string())),
            Some(5000),
        );
        state.add_entry(npm_entry);

        // Add custom event
        let mut event_data = HashMap::new();
        event_data.insert(
            "feature".to_string(),
            serde_json::Value::String("auth".to_string()),
        );
        let custom_entry = StateEntry::new_custom_event("FeatureComplete".to_string(), event_data);
        state.add_entry(custom_entry);

        state
    }

    #[test]
    fn test_tool_usage_exists() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        assert!(query.tool_usage_exists("Read", "README.md"));
        assert!(!query.tool_usage_exists("Read", "other.md"));
        assert!(!query.tool_usage_exists("Write", "README.md"));
    }

    #[test]
    fn test_tool_usage_missing() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        assert!(!query.tool_usage_missing("Read", "README.md"));
        assert!(query.tool_usage_missing("Read", "other.md"));
        assert!(query.tool_usage_missing("Write", "README.md"));
    }

    #[test]
    fn test_custom_event_exists() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        assert!(query.custom_event_exists("FeatureComplete"));
        assert!(!query.custom_event_exists("OtherEvent"));
    }

    #[test]
    fn test_get_read_files() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        let read_files = query.get_read_files();
        assert_eq!(read_files.len(), 1);
        assert!(read_files.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_get_successful_commands() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        let commands = query.get_successful_commands();
        assert_eq!(commands.len(), 1);
        assert!(commands.contains(&"git commit -m 'test'".to_string()));
        // npm test should not be included because it failed
        assert!(!commands.contains(&"npm test".to_string()));
    }

    #[test]
    fn test_has_command_matching() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        let git_pattern = Regex::new(r"git\s+commit").unwrap();
        let test_pattern = Regex::new(r"npm\s+test").unwrap();
        let build_pattern = Regex::new(r"cargo\s+build").unwrap();

        assert!(query.has_command_matching(&git_pattern));
        assert!(!query.has_command_matching(&test_pattern)); // Failed command doesn't count
        assert!(!query.has_command_matching(&build_pattern));
    }

    #[test]
    fn test_count_tool_usage() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        assert_eq!(query.count_tool_usage("Bash"), 2); // Both git and npm commands
        assert_eq!(query.count_tool_usage("Read"), 1);
        assert_eq!(query.count_tool_usage("Write"), 0);

        assert_eq!(query.count_successful_tool_usage("Bash"), 1); // Only git command
        assert_eq!(query.count_failed_tool_usage("Bash"), 1); // Only npm command
    }

    #[test]
    fn test_execute_query_basic() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        // Query for successful git commands
        let git_query = StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("git".to_string()),
            result: Some("success".to_string()),
            within_minutes: None,
        };

        assert!(query.execute_query(&git_query));

        // Query for successful npm commands (should fail because npm test failed)
        let npm_query = StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("npm".to_string()),
            result: Some("success".to_string()),
            within_minutes: None,
        };

        assert!(!query.execute_query(&npm_query));
    }

    #[test]
    fn test_execute_query_with_result_filter() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        // Query for failed npm commands
        let failed_npm_query = StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("npm".to_string()),
            result: Some("failure".to_string()),
            within_minutes: None,
        };

        assert!(query.execute_query(&failed_npm_query));

        // Query for failed git commands (should not exist)
        let failed_git_query = StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("git".to_string()),
            result: Some("failure".to_string()),
            within_minutes: None,
        };

        assert!(!query.execute_query(&failed_git_query));
    }

    #[test]
    fn test_execute_query_tool_filter() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        // Query for Read tool usage
        let read_query = StateQueryFilter {
            tool: "Read".to_string(),
            command_contains: None,
            result: Some("success".to_string()),
            within_minutes: None,
        };

        assert!(query.execute_query(&read_query));

        // Query for Write tool usage (should not exist)
        let write_query = StateQueryFilter {
            tool: "Write".to_string(),
            command_contains: None,
            result: Some("success".to_string()),
            within_minutes: None,
        };

        assert!(!query.execute_query(&write_query));
    }

    #[test]
    fn test_get_latest_tool_usage() {
        let state = create_test_session();
        let query = StateQuery::new(&state);

        // Latest Bash usage should be the npm test (second one added)
        let latest_bash = query.get_latest_tool_usage("Bash");
        assert!(latest_bash.is_some());
        assert_eq!(latest_bash.unwrap().tool_name, "Bash");
        assert!(!latest_bash.unwrap().success); // npm test failed

        // Latest Read usage should be the README.md read
        let latest_read = query.get_latest_tool_usage("Read");
        assert!(latest_read.is_some());
        assert_eq!(latest_read.unwrap().tool_name, "Read");
        assert!(latest_read.unwrap().success);

        // No Write usage
        let latest_write = query.get_latest_tool_usage("Write");
        assert!(latest_write.is_none());
    }
}
