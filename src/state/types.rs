use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single entry in the session state log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    /// When this entry was created
    pub timestamp: DateTime<Utc>,
    /// Event type (tool usage or custom event)
    pub event: StateEvent,
}

/// Types of state events that can be tracked
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateEvent {
    /// Tool usage automatically tracked by Cupcake
    ToolUsage(ToolUsageEntry),
    /// Custom event created by update_state action
    CustomEvent {
        name: String,
        data: HashMap<String, serde_json::Value>,
    },
}

/// Tool usage entry automatically tracked by Cupcake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageEntry {
    /// Name of the tool that was used
    pub tool_name: String,
    /// Tool input parameters
    pub input: HashMap<String, serde_json::Value>,
    /// Whether the tool execution was successful
    pub success: bool,
    /// Tool output (stdout/stderr for Bash, content for Read, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Duration of tool execution in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl StateEntry {
    /// Create a new tool usage entry
    pub fn new_tool_usage(
        tool_name: String,
        input: HashMap<String, serde_json::Value>,
        success: bool,
        output: Option<serde_json::Value>,
        duration_ms: Option<u64>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            event: StateEvent::ToolUsage(ToolUsageEntry {
                tool_name,
                input,
                success,
                output,
                duration_ms,
            }),
        }
    }

    /// Create a new custom event entry
    pub fn new_custom_event(name: String, data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            timestamp: Utc::now(),
            event: StateEvent::CustomEvent { name, data },
        }
    }

    /// Get the tool name if this is a tool usage entry
    pub fn tool_name(&self) -> Option<&str> {
        match &self.event {
            StateEvent::ToolUsage(entry) => Some(&entry.tool_name),
            StateEvent::CustomEvent { .. } => None,
        }
    }

    /// Get the custom event name if this is a custom event
    pub fn custom_event_name(&self) -> Option<&str> {
        match &self.event {
            StateEvent::ToolUsage(_) => None,
            StateEvent::CustomEvent { name, .. } => Some(name),
        }
    }

    /// Check if this entry represents a successful tool usage
    pub fn is_successful_tool_usage(&self) -> bool {
        match &self.event {
            StateEvent::ToolUsage(entry) => entry.success,
            StateEvent::CustomEvent { .. } => false,
        }
    }

    /// Get file path from tool input if available
    pub fn file_path(&self) -> Option<&str> {
        match &self.event {
            StateEvent::ToolUsage(entry) => entry.input.get("file_path").and_then(|v| v.as_str()),
            StateEvent::CustomEvent { .. } => None,
        }
    }

    /// Get command from tool input if available (for Bash tool)
    pub fn command(&self) -> Option<&str> {
        match &self.event {
            StateEvent::ToolUsage(entry) => entry.input.get("command").and_then(|v| v.as_str()),
            StateEvent::CustomEvent { .. } => None,
        }
    }
}

/// Session state containing all entries for a Claude Code session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session ID from Claude Code
    pub session_id: String,
    /// When this session was created
    pub created_at: DateTime<Utc>,
    /// Last time this session was updated
    pub updated_at: DateTime<Utc>,
    /// All state entries in chronological order
    pub entries: Vec<StateEntry>,
}

impl SessionState {
    /// Create a new empty session state
    pub fn new(session_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            created_at: now,
            updated_at: now,
            entries: Vec::new(),
        }
    }

    /// Add a new entry to the session state
    pub fn add_entry(&mut self, entry: StateEntry) {
        self.entries.push(entry);
        self.updated_at = Utc::now();
    }

    /// Get all tool usage entries for a specific tool
    pub fn get_tool_usage(&self, tool_name: &str) -> Vec<&ToolUsageEntry> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.event {
                StateEvent::ToolUsage(tool_entry) if tool_entry.tool_name == tool_name => {
                    Some(tool_entry)
                }
                _ => None,
            })
            .collect()
    }

    /// Get all custom events with a specific name
    pub fn get_custom_events(
        &self,
        event_name: &str,
    ) -> Vec<(&DateTime<Utc>, &HashMap<String, serde_json::Value>)> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.event {
                StateEvent::CustomEvent { name, data } if name == event_name => {
                    Some((&entry.timestamp, data))
                }
                _ => None,
            })
            .collect()
    }

    /// Check if a specific file has been read in this session
    pub fn has_read_file(&self, file_path: &str) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                StateEvent::ToolUsage(tool_entry)
                if tool_entry.tool_name == "Read"
                && tool_entry.success
                && tool_entry.input.get("file_path").and_then(|v| v.as_str()) == Some(file_path)
            )
        })
    }

    /// Check if a command matching a pattern has been run successfully
    pub fn has_run_command_matching(&self, pattern: &regex::Regex) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                StateEvent::ToolUsage(tool_entry)
                if tool_entry.tool_name == "Bash"
                && tool_entry.success
                && tool_entry.input.get("command")
                    .and_then(|v| v.as_str())
                    .map(|cmd| pattern.is_match(cmd))
                    .unwrap_or(false)
            )
        })
    }

    /// Get the most recent entry of a specific type
    pub fn get_latest_tool_usage(&self, tool_name: &str) -> Option<&ToolUsageEntry> {
        self.entries
            .iter()
            .rev() // Start from most recent
            .find_map(|entry| match &entry.event {
                StateEvent::ToolUsage(tool_entry) if tool_entry.tool_name == tool_name => {
                    Some(tool_entry)
                }
                _ => None,
            })
    }

    /// Check if a custom event exists
    pub fn has_custom_event(&self, event_name: &str) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                StateEvent::CustomEvent { name, .. } if name == event_name
            )
        })
    }

    /// Get number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get number of tool usage entries
    pub fn tool_usage_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.event, StateEvent::ToolUsage(_)))
            .count()
    }

    /// Get number of custom events
    pub fn custom_event_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.event, StateEvent::CustomEvent { .. }))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn create_test_tool_input(file_path: &str) -> HashMap<String, serde_json::Value> {
        let mut input = HashMap::new();
        input.insert(
            "file_path".to_string(),
            serde_json::Value::String(file_path.to_string()),
        );
        input
    }

    fn create_test_bash_input(command: &str) -> HashMap<String, serde_json::Value> {
        let mut input = HashMap::new();
        input.insert(
            "command".to_string(),
            serde_json::Value::String(command.to_string()),
        );
        input
    }

    #[test]
    fn test_state_entry_creation() {
        let entry = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("test.rs"),
            true,
            None,
            Some(50),
        );

        assert_eq!(entry.tool_name(), Some("Read"));
        assert_eq!(entry.file_path(), Some("test.rs"));
        assert!(entry.is_successful_tool_usage());
        assert_eq!(entry.custom_event_name(), None);
    }

    #[test]
    fn test_custom_event_creation() {
        let mut data = HashMap::new();
        data.insert(
            "feature".to_string(),
            serde_json::Value::String("auth".to_string()),
        );

        let entry = StateEntry::new_custom_event("FeatureComplete".to_string(), data);

        assert_eq!(entry.custom_event_name(), Some("FeatureComplete"));
        assert_eq!(entry.tool_name(), None);
        assert!(!entry.is_successful_tool_usage());
    }

    #[test]
    fn test_session_state_creation() {
        let state = SessionState::new("test-session-123".to_string());

        assert_eq!(state.session_id, "test-session-123");
        assert_eq!(state.entries.len(), 0);
        assert_eq!(state.entry_count(), 0);
        assert_eq!(state.tool_usage_count(), 0);
        assert_eq!(state.custom_event_count(), 0);
    }

    #[test]
    fn test_session_state_add_entries() {
        let mut state = SessionState::new("test-session".to_string());

        // Add tool usage entry
        let tool_entry = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("README.md"),
            true,
            None,
            None,
        );
        state.add_entry(tool_entry);

        // Add custom event
        let mut data = HashMap::new();
        data.insert(
            "test".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        let custom_entry = StateEntry::new_custom_event("TestEvent".to_string(), data);
        state.add_entry(custom_entry);

        assert_eq!(state.entry_count(), 2);
        assert_eq!(state.tool_usage_count(), 1);
        assert_eq!(state.custom_event_count(), 1);
    }

    #[test]
    fn test_has_read_file() {
        let mut state = SessionState::new("test-session".to_string());

        // Add successful read
        let read_entry = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("README.md"),
            true,
            None,
            None,
        );
        state.add_entry(read_entry);

        // Add failed read
        let failed_read = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("other.md"),
            false,
            None,
            None,
        );
        state.add_entry(failed_read);

        assert!(state.has_read_file("README.md"));
        assert!(!state.has_read_file("other.md")); // Failed reads don't count
        assert!(!state.has_read_file("nonexistent.md"));
    }

    #[test]
    fn test_has_run_command_matching() {
        let mut state = SessionState::new("test-session".to_string());

        // Add successful command
        let bash_entry = StateEntry::new_tool_usage(
            "Bash".to_string(),
            create_test_bash_input("git commit -m 'test'"),
            true,
            None,
            None,
        );
        state.add_entry(bash_entry);

        // Add failed command
        let failed_bash = StateEntry::new_tool_usage(
            "Bash".to_string(),
            create_test_bash_input("npm test"),
            false,
            None,
            None,
        );
        state.add_entry(failed_bash);

        let git_pattern = regex::Regex::new(r"git\s+commit").unwrap();
        let test_pattern = regex::Regex::new(r"npm\s+test").unwrap();
        let other_pattern = regex::Regex::new(r"cargo\s+build").unwrap();

        assert!(state.has_run_command_matching(&git_pattern));
        assert!(!state.has_run_command_matching(&test_pattern)); // Failed commands don't count
        assert!(!state.has_run_command_matching(&other_pattern));
    }

    #[test]
    fn test_has_custom_event() {
        let mut state = SessionState::new("test-session".to_string());

        let mut data = HashMap::new();
        data.insert(
            "feature".to_string(),
            serde_json::Value::String("auth".to_string()),
        );

        let custom_entry = StateEntry::new_custom_event("FeatureComplete".to_string(), data);
        state.add_entry(custom_entry);

        assert!(state.has_custom_event("FeatureComplete"));
        assert!(!state.has_custom_event("OtherEvent"));
    }

    #[test]
    fn test_get_tool_usage() {
        let mut state = SessionState::new("test-session".to_string());

        // Add multiple Read entries
        let read1 = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("file1.rs"),
            true,
            None,
            None,
        );
        let read2 = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("file2.rs"),
            true,
            None,
            None,
        );
        let bash = StateEntry::new_tool_usage(
            "Bash".to_string(),
            create_test_bash_input("echo test"),
            true,
            None,
            None,
        );

        state.add_entry(read1);
        state.add_entry(read2);
        state.add_entry(bash);

        let read_entries = state.get_tool_usage("Read");
        assert_eq!(read_entries.len(), 2);

        let bash_entries = state.get_tool_usage("Bash");
        assert_eq!(bash_entries.len(), 1);

        let write_entries = state.get_tool_usage("Write");
        assert_eq!(write_entries.len(), 0);
    }

    #[test]
    fn test_serialization() {
        let mut state = SessionState::new("test-session".to_string());

        let tool_entry = StateEntry::new_tool_usage(
            "Read".to_string(),
            create_test_tool_input("test.rs"),
            true,
            Some(serde_json::Value::String("file content".to_string())),
            Some(100),
        );
        state.add_entry(tool_entry);

        // Test JSON serialization
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.session_id, deserialized.session_id);
        assert_eq!(state.entries.len(), deserialized.entries.len());
    }
}
