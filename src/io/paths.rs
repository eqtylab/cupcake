use crate::{CupcakeError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

/// Path management for Cupcake configuration and state files
#[derive(Debug, Clone)]
pub struct CupcakePaths {
    /// Configuration directory (.cupcake/)
    pub config_dir: PathBuf,
    /// State files directory (.cupcake/state/)
    pub state_dir: PathBuf,
    /// Cache directory (.cupcake/cache/)
    pub cache_dir: PathBuf,
    /// Audit log file (.cupcake/audit.log)
    pub audit_file: PathBuf,
}

impl CupcakePaths {
    /// Create new paths instance using standard directories
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", "cupcake").ok_or_else(|| {
            CupcakeError::Path("Failed to determine project directories".to_string())
        })?;

        let config_dir = dirs.config_dir().to_path_buf();
        let state_dir = config_dir.join("state");
        let cache_dir = config_dir.join("cache");
        let audit_file = config_dir.join("audit.log");

        Ok(Self {
            config_dir,
            state_dir,
            cache_dir,
            audit_file,
        })
    }

    /// Create paths for a specific project directory
    pub fn for_project(project_root: &std::path::Path) -> Self {
        let config_dir = project_root.join(".cupcake");
        let state_dir = config_dir.join("state");
        let cache_dir = config_dir.join("cache");
        let audit_file = config_dir.join("audit.log");

        Self {
            config_dir,
            state_dir,
            cache_dir,
            audit_file,
        }
    }

    /// Get policy file path for project
    pub fn policy_file(&self, project_root: &std::path::Path) -> PathBuf {
        project_root.join("cupcake.toml")
    }

    /// Get user policy file path
    pub fn user_policy_file(&self) -> PathBuf {
        self.config_dir.join("cupcake.toml")
    }

    /// Get state file path for a session
    pub fn state_file(&self, session_id: &str) -> PathBuf {
        self.state_dir.join(format!("{}.json", session_id))
    }

    /// Get policy cache file path
    pub fn policy_cache_file(&self, project_root: &std::path::Path) -> PathBuf {
        self.cache_dir.join(format!(
            "policy-{}.cache",
            project_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        ))
    }

    /// Get Claude Code settings file path
    pub fn claude_settings_file(&self) -> PathBuf {
        self.config_dir.join(".claude").join("settings.json")
    }

    /// Get default Claude Code settings file path
    pub fn default_claude_settings() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("", "", "claude").ok_or_else(|| {
            CupcakeError::Path("Failed to determine Claude directories".to_string())
        })?;

        Ok(dirs.config_dir().join("settings.json"))
    }

    /// Ensure all directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.state_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Clean up old state files
    pub fn cleanup_old_state_files(&self, max_age_days: u64) -> Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - (max_age_days * 24 * 60 * 60);

        if !self.state_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(modified_secs) = modified.duration_since(UNIX_EPOCH) {
                            if modified_secs.as_secs() < cutoff {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for CupcakePaths {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to current directory if platform directories fail
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Self::for_project(&current_dir)
        })
    }
}

/// Utilities for working with paths
pub struct PathUtils;

impl PathUtils {
    /// Find project root by looking for cupcake.toml or .git
    pub fn find_project_root() -> Option<PathBuf> {
        let current_dir = std::env::current_dir().ok()?;
        let mut dir = current_dir.as_path();

        loop {
            if dir.join("cupcake.toml").exists() || dir.join(".git").exists() {
                return Some(dir.to_path_buf());
            }

            dir = dir.parent()?;
        }
    }

    /// Check if path is safe (no traversal attacks)
    pub fn is_safe_path(path: &std::path::Path) -> bool {
        !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    }

    /// Normalize path separators for cross-platform compatibility
    pub fn normalize_path(path: &str) -> String {
        if cfg!(windows) {
            path.replace('/', "\\")
        } else {
            path.replace('\\', "/")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_cupcake_paths_creation() {
        let paths = CupcakePaths::new();
        assert!(paths.is_ok());

        let paths = paths.unwrap();
        assert!(paths.config_dir.ends_with("cupcake"));
        assert!(paths.state_dir.ends_with("state"));
        assert!(paths.cache_dir.ends_with("cache"));
        assert!(paths.audit_file.ends_with("audit.log"));
    }

    #[test]
    fn test_project_paths() {
        let project_root = Path::new("/tmp/test-project");
        let paths = CupcakePaths::for_project(project_root);

        assert_eq!(paths.config_dir, project_root.join(".cupcake"));
        assert_eq!(paths.state_dir, project_root.join(".cupcake/state"));
        assert_eq!(paths.cache_dir, project_root.join(".cupcake/cache"));
        assert_eq!(paths.audit_file, project_root.join(".cupcake/audit.log"));
    }

    #[test]
    fn test_policy_file_paths() {
        let project_root = Path::new("/tmp/test-project");
        let paths = CupcakePaths::for_project(project_root);

        assert_eq!(
            paths.policy_file(project_root),
            project_root.join("cupcake.toml")
        );
        assert!(paths.user_policy_file().ends_with("cupcake.toml"));
    }

    #[test]
    fn test_state_file_path() {
        let project_root = Path::new("/tmp/test-project");
        let paths = CupcakePaths::for_project(project_root);

        let state_file = paths.state_file("test-session-123");
        assert!(state_file.ends_with("test-session-123.json"));
        assert!(state_file.parent().unwrap().ends_with("state"));
    }

    #[test]
    fn test_path_utils_safety() {
        assert!(PathUtils::is_safe_path(Path::new("safe/path/file.txt")));
        assert!(!PathUtils::is_safe_path(Path::new("../../../etc/passwd")));
        assert!(!PathUtils::is_safe_path(Path::new(
            "safe/../unsafe/file.txt"
        )));
    }

    #[test]
    fn test_path_normalization() {
        if cfg!(windows) {
            assert_eq!(PathUtils::normalize_path("path/to/file"), "path\\to\\file");
        } else {
            assert_eq!(PathUtils::normalize_path("path\\to\\file"), "path/to/file");
        }
    }
}
