use crate::state::types::{SessionState, StateEntry};
use crate::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages session state files in .cupcake/state/ directory
pub struct StateManager {
    /// Base directory for state files (.cupcake/)
    state_dir: PathBuf,
    /// In-memory cache of loaded session states
    cache: HashMap<String, SessionState>,
}

impl StateManager {
    /// Create new state manager for the given directory
    pub fn new(base_dir: &Path) -> Result<Self> {
        let state_dir = base_dir.join(".cupcake").join("state");

        // Create state directory if it doesn't exist
        if !state_dir.exists() {
            fs::create_dir_all(&state_dir)?;
        }

        Ok(Self {
            state_dir,
            cache: HashMap::new(),
        })
    }

    /// Create new state manager using current working directory
    pub fn new_in_current_dir() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::new(&current_dir)
    }

    /// Get or create session state for the given session ID
    pub fn get_session_state(&mut self, session_id: &str) -> Result<&mut SessionState> {
        if !self.cache.contains_key(session_id) {
            let state = self.load_session_state(session_id)?;
            self.cache.insert(session_id.to_string(), state);
        }

        self.cache.get_mut(session_id).ok_or_else(|| {
            crate::CupcakeError::State("Session state cache inconsistency".to_string())
        })
    }

    /// Load session state from disk or create new one
    fn load_session_state(&self, session_id: &str) -> Result<SessionState> {
        let file_path = self.get_session_file_path(session_id);

        if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            let state: SessionState = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(SessionState::new(session_id.to_string()))
        }
    }

    /// Save session state to disk
    pub fn save_session_state(&self, session_id: &str) -> Result<()> {
        if let Some(state) = self.cache.get(session_id) {
            let file_path = self.get_session_file_path(session_id);
            let content = serde_json::to_string_pretty(state)?;
            fs::write(&file_path, content)?;
        }
        Ok(())
    }

    /// Add entry to session state and save to disk
    pub fn add_entry(&mut self, session_id: &str, entry: StateEntry) -> Result<()> {
        let state = self.get_session_state(session_id)?;
        state.add_entry(entry);
        self.save_session_state(session_id)?;
        Ok(())
    }

    /// Add tool usage entry to session state
    pub fn add_tool_usage(
        &mut self,
        session_id: &str,
        tool_name: String,
        input: HashMap<String, serde_json::Value>,
        success: bool,
        output: Option<serde_json::Value>,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        let entry = StateEntry::new_tool_usage(tool_name, input, success, output, duration_ms);
        self.add_entry(session_id, entry)
    }

    /// Add custom event to session state
    pub fn add_custom_event(
        &mut self,
        session_id: &str,
        event_name: String,
        data: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let entry = StateEntry::new_custom_event(event_name, data);
        self.add_entry(session_id, entry)
    }

    /// Check if a file has been read in the session
    pub fn has_read_file(&mut self, session_id: &str, file_path: &str) -> Result<bool> {
        let state = self.get_session_state(session_id)?;
        Ok(state.has_read_file(file_path))
    }

    /// Check if a command matching pattern has been run successfully
    pub fn has_run_command_matching(
        &mut self,
        session_id: &str,
        pattern: &regex::Regex,
    ) -> Result<bool> {
        let state = self.get_session_state(session_id)?;
        Ok(state.has_run_command_matching(pattern))
    }

    /// Check if a custom event exists in the session
    pub fn has_custom_event(&mut self, session_id: &str, event_name: &str) -> Result<bool> {
        let state = self.get_session_state(session_id)?;
        Ok(state.has_custom_event(event_name))
    }

    /// Get session statistics
    pub fn get_session_stats(&mut self, session_id: &str) -> Result<SessionStats> {
        let state = self.get_session_state(session_id)?;
        Ok(SessionStats {
            total_entries: state.entry_count(),
            tool_usage_count: state.tool_usage_count(),
            custom_event_count: state.custom_event_count(),
            created_at: state.created_at,
            updated_at: state.updated_at,
        })
    }

    /// List all session IDs that have state files
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let mut sessions = Vec::new();

        if self.state_dir.exists() {
            for entry in fs::read_dir(&self.state_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                        sessions.push(file_name.to_string());
                    }
                }
            }
        }

        sessions.sort();
        Ok(sessions)
    }

    /// Clean up old session files (older than specified days)
    pub fn cleanup_old_sessions(&self, days_old: u32) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::days(days_old as i64);
        let mut cleaned_count = 0;

        if self.state_dir.exists() {
            for entry in fs::read_dir(&self.state_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    // Check file modification time
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            let modified_datetime: DateTime<Utc> = modified.into();
                            if modified_datetime < cutoff_time && fs::remove_file(&path).is_ok() {
                                cleaned_count += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(cleaned_count)
    }

    /// Delete session state file
    pub fn delete_session(&mut self, session_id: &str) -> Result<bool> {
        let file_path = self.get_session_file_path(session_id);

        // Remove from cache
        self.cache.remove(session_id);

        // Remove file if it exists
        if file_path.exists() {
            fs::remove_file(&file_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get file path for session state file
    fn get_session_file_path(&self, session_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}.json", session_id))
    }

    /// Get the state directory path
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    /// Clear in-memory cache (forces reload from disk on next access)
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get number of cached sessions
    pub fn cached_session_count(&self) -> usize {
        self.cache.len()
    }
}

/// Statistics about a session
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_entries: usize,
    pub tool_usage_count: usize,
    pub custom_event_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_manager() -> (StateManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path()).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_state_manager_creation() {
        let (manager, _temp_dir) = create_test_manager();

        assert!(manager.state_dir().exists());
        assert!(manager.state_dir().is_dir());
        assert_eq!(manager.cached_session_count(), 0);
    }

    #[test]
    fn test_get_session_state_new() {
        let (mut manager, _temp_dir) = create_test_manager();

        let state = manager.get_session_state("test-session").unwrap();
        assert_eq!(state.session_id, "test-session");
        assert_eq!(state.entry_count(), 0);
        assert_eq!(manager.cached_session_count(), 1);
    }

    #[test]
    fn test_add_tool_usage() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut input = HashMap::new();
        input.insert(
            "file_path".to_string(),
            serde_json::Value::String("test.rs".to_string()),
        );

        manager
            .add_tool_usage(
                "test-session",
                "Read".to_string(),
                input,
                true,
                None,
                Some(50),
            )
            .unwrap();

        let state = manager.get_session_state("test-session").unwrap();
        assert_eq!(state.entry_count(), 1);
        assert_eq!(state.tool_usage_count(), 1);
        assert!(state.has_read_file("test.rs"));
    }

    #[test]
    fn test_add_custom_event() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut data = HashMap::new();
        data.insert(
            "feature".to_string(),
            serde_json::Value::String("auth".to_string()),
        );

        manager
            .add_custom_event("test-session", "FeatureComplete".to_string(), data)
            .unwrap();

        let state = manager.get_session_state("test-session").unwrap();
        assert_eq!(state.entry_count(), 1);
        assert_eq!(state.custom_event_count(), 1);
        assert!(state.has_custom_event("FeatureComplete"));
    }

    #[test]
    fn test_save_and_load_session() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Add some data
        let mut input = HashMap::new();
        input.insert(
            "file_path".to_string(),
            serde_json::Value::String("test.rs".to_string()),
        );
        manager
            .add_tool_usage("test-session", "Read".to_string(), input, true, None, None)
            .unwrap();

        // Save session
        manager.save_session_state("test-session").unwrap();

        // Clear cache and reload
        manager.clear_cache();
        let state = manager.get_session_state("test-session").unwrap();

        assert_eq!(state.entry_count(), 1);
        assert!(state.has_read_file("test.rs"));
    }

    #[test]
    fn test_has_read_file() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut input = HashMap::new();
        input.insert(
            "file_path".to_string(),
            serde_json::Value::String("README.md".to_string()),
        );

        manager
            .add_tool_usage("test-session", "Read".to_string(), input, true, None, None)
            .unwrap();

        assert!(manager.has_read_file("test-session", "README.md").unwrap());
        assert!(!manager.has_read_file("test-session", "other.md").unwrap());
    }

    #[test]
    fn test_has_run_command_matching() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut input = HashMap::new();
        input.insert(
            "command".to_string(),
            serde_json::Value::String("git commit -m 'test'".to_string()),
        );

        manager
            .add_tool_usage("test-session", "Bash".to_string(), input, true, None, None)
            .unwrap();

        let git_pattern = regex::Regex::new(r"git\s+commit").unwrap();
        let npm_pattern = regex::Regex::new(r"npm\s+test").unwrap();

        assert!(manager
            .has_run_command_matching("test-session", &git_pattern)
            .unwrap());
        assert!(!manager
            .has_run_command_matching("test-session", &npm_pattern)
            .unwrap());
    }

    #[test]
    fn test_has_custom_event() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut data = HashMap::new();
        data.insert(
            "test".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        manager
            .add_custom_event("test-session", "TestEvent".to_string(), data)
            .unwrap();

        assert!(manager
            .has_custom_event("test-session", "TestEvent")
            .unwrap());
        assert!(!manager
            .has_custom_event("test-session", "OtherEvent")
            .unwrap());
    }

    #[test]
    fn test_get_session_stats() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Add tool usage
        let mut input = HashMap::new();
        input.insert(
            "file_path".to_string(),
            serde_json::Value::String("test.rs".to_string()),
        );
        manager
            .add_tool_usage("test-session", "Read".to_string(), input, true, None, None)
            .unwrap();

        // Add custom event
        let mut data = HashMap::new();
        data.insert(
            "test".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        manager
            .add_custom_event("test-session", "TestEvent".to_string(), data)
            .unwrap();

        let stats = manager.get_session_stats("test-session").unwrap();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.tool_usage_count, 1);
        assert_eq!(stats.custom_event_count, 1);
    }

    #[test]
    fn test_list_sessions() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Create some sessions
        manager.get_session_state("session-1").unwrap();
        manager.get_session_state("session-2").unwrap();
        manager.get_session_state("session-3").unwrap();

        // Save them
        manager.save_session_state("session-1").unwrap();
        manager.save_session_state("session-2").unwrap();
        manager.save_session_state("session-3").unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 3);
        assert!(sessions.contains(&"session-1".to_string()));
        assert!(sessions.contains(&"session-2".to_string()));
        assert!(sessions.contains(&"session-3".to_string()));
    }

    #[test]
    fn test_delete_session() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Create and save a session
        manager.get_session_state("test-session").unwrap();
        manager.save_session_state("test-session").unwrap();

        // Verify it exists
        let sessions = manager.list_sessions().unwrap();
        assert!(sessions.contains(&"test-session".to_string()));

        // Delete it
        let deleted = manager.delete_session("test-session").unwrap();
        assert!(deleted);

        // Verify it's gone
        let sessions = manager.list_sessions().unwrap();
        assert!(!sessions.contains(&"test-session".to_string()));
        assert_eq!(manager.cached_session_count(), 0);
    }

    #[test]
    fn test_clear_cache() {
        let (mut manager, _temp_dir) = create_test_manager();

        manager.get_session_state("test-session").unwrap();
        assert_eq!(manager.cached_session_count(), 1);

        manager.clear_cache();
        assert_eq!(manager.cached_session_count(), 0);
    }
}
