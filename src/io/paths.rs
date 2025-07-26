use crate::{CupcakeError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

/// Path management for Cupcake configuration files
#[derive(Debug, Clone)]
pub struct CupcakePaths {
    /// Configuration directory (.cupcake/)
    pub config_dir: PathBuf,
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
        let cache_dir = config_dir.join("cache");
        let audit_file = config_dir.join("audit.log");

        Ok(Self {
            config_dir,
            cache_dir,
            audit_file,
        })
    }

    /// Create paths for a specific project directory
    pub fn for_project(project_root: &std::path::Path) -> Self {
        let config_dir = project_root.join(".cupcake");
        let cache_dir = config_dir.join("cache");
        let audit_file = config_dir.join("audit.log");

        Self {
            config_dir,
            cache_dir,
            audit_file,
        }
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
        std::fs::create_dir_all(&self.cache_dir)?;
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
        assert!(paths.cache_dir.ends_with("cache"));
        assert!(paths.audit_file.ends_with("audit.log"));
    }

    #[test]
    fn test_project_paths() {
        let project_root = Path::new("/tmp/test-project");
        let paths = CupcakePaths::for_project(project_root);

        assert_eq!(paths.config_dir, project_root.join(".cupcake"));
        assert_eq!(paths.cache_dir, project_root.join(".cupcake/cache"));
        assert_eq!(paths.audit_file, project_root.join(".cupcake/audit.log"));
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
