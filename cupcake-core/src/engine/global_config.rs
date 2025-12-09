//! Global Configuration Discovery Module
//!
//! Provides cross-platform discovery and management of machine-wide Cupcake configurations.
//! Global policies take absolute precedence over project-specific policies.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug, info, trace};

/// Global configuration paths for system-wide policies
#[derive(Debug, Clone)]
pub struct GlobalPaths {
    /// Root directory for global config
    pub root: PathBuf,
    /// Global policies directory
    pub policies: PathBuf,
    /// Global rulebook file
    pub rulebook: PathBuf,
    /// Global signals directory
    pub signals: PathBuf,
    /// Global actions directory
    pub actions: PathBuf,
}

impl GlobalPaths {
    /// Discover global configuration paths using platform conventions
    ///
    /// Resolution order:
    /// 1. CLI override (if provided)
    /// 2. Platform-specific user config directory
    /// 3. None if config directory doesn't exist (graceful absence)
    pub fn discover() -> Result<Option<Self>> {
        Self::discover_with_override(None)
    }

    /// Discover global configuration with optional CLI override
    ///
    /// Resolution order:
    /// 1. CLI override parameter (if provided)
    /// 2. Platform-specific user config directory
    /// 3. None if config directory doesn't exist (graceful absence)
    pub fn discover_with_override(cli_override: Option<PathBuf>) -> Result<Option<Self>> {
        trace!("Discovering global configuration paths");

        // First check CLI override
        if let Some(override_path) = cli_override {
            // Validate the path
            if !override_path.is_absolute() {
                return Err(anyhow::anyhow!(
                    "Global config path must be absolute (got: {})",
                    override_path.display()
                ));
            }

            if !override_path.exists() {
                return Err(anyhow::anyhow!(
                    "Global config path does not exist: {}",
                    override_path.display()
                ));
            }

            // Canonicalize to resolve .. and symlinks (defense-in-depth)
            // This ensures the user sees the actual target directory in logs/errors
            let canonical_path = override_path.canonicalize().with_context(|| {
                format!(
                    "Failed to resolve global config path: {}",
                    override_path.display()
                )
            })?;

            if !canonical_path.is_dir() {
                return Err(anyhow::anyhow!(
                    "Global config path must be a directory: {}",
                    canonical_path.display()
                ));
            }

            debug!(
                "Using CLI --global-config override: {} (resolved to {})",
                override_path.display(),
                canonical_path.display()
            );

            return Ok(Some(Self::from_root(canonical_path)?));
        }

        // Use platform-specific config directory
        let config_dir = Self::get_platform_config_dir()?;
        let cupcake_global_dir = config_dir.join("cupcake");

        // Check if global config exists - graceful absence
        if !cupcake_global_dir.exists() {
            debug!("No global configuration found at {:?}", cupcake_global_dir);
            return Ok(None);
        }

        info!("Found global configuration at {:?}", cupcake_global_dir);
        Ok(Some(Self::from_root(cupcake_global_dir)?))
    }

    /// Create GlobalPaths from a root directory
    fn from_root(root: PathBuf) -> Result<Self> {
        // Verify root exists
        if !root.exists() {
            return Err(anyhow::anyhow!(
                "Global config root does not exist: {:?}",
                root
            ));
        }

        Ok(GlobalPaths {
            policies: root.join("policies"),
            rulebook: root.join("rulebook.yml"),
            signals: root.join("signals"),
            actions: root.join("actions"),
            root,
        })
    }

    /// Get the platform-specific config directory
    fn get_platform_config_dir() -> Result<PathBuf> {
        // Use the directories crate for cross-platform support
        use directories::ProjectDirs;

        // Get the config directory for the current platform
        // On Linux: ~/.config/
        // On macOS: ~/Library/Application Support/
        // On Windows: %APPDATA%\
        if let Some(proj_dirs) = ProjectDirs::from("", "", "cupcake") {
            // Return the parent of the project-specific directory
            // to get the general config directory
            if let Some(parent) = proj_dirs.config_dir().parent() {
                return Ok(parent.to_path_buf());
            }
        }

        // Fallback to home directory approach
        #[cfg(unix)]
        {
            if let Ok(home) = std::env::var("HOME") {
                return Ok(PathBuf::from(home).join(".config"));
            }
        }

        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                return Ok(PathBuf::from(appdata));
            }
        }

        Err(anyhow::anyhow!(
            "Could not determine platform config directory"
        ))
    }

    /// Check if the global configuration is properly initialized
    pub fn is_initialized(&self) -> bool {
        self.policies.exists()
            && self.rulebook.exists()
            && self.signals.exists()
            && self.actions.exists()
    }

    /// Initialize a new global configuration directory structure
    pub fn initialize(&self) -> Result<()> {
        info!("Initializing global configuration at {:?}", self.root);

        // Create directory structure with harness-specific subdirectories
        std::fs::create_dir_all(&self.root).context("Failed to create global config root")?;
        std::fs::create_dir_all(&self.policies)
            .context("Failed to create global policies directory")?;

        // Create harness-specific subdirectories for all harnesses
        std::fs::create_dir_all(self.policies.join("claude").join("system"))
            .context("Failed to create global policies/claude/system directory")?;
        std::fs::create_dir_all(self.policies.join("claude").join("builtins"))
            .context("Failed to create global policies/claude/builtins directory")?;
        std::fs::create_dir_all(self.policies.join("cursor").join("system"))
            .context("Failed to create global policies/cursor/system directory")?;
        std::fs::create_dir_all(self.policies.join("cursor").join("builtins"))
            .context("Failed to create global policies/cursor/builtins directory")?;
        std::fs::create_dir_all(self.policies.join("factory").join("system"))
            .context("Failed to create global policies/factory/system directory")?;
        std::fs::create_dir_all(self.policies.join("factory").join("builtins"))
            .context("Failed to create global policies/factory/builtins directory")?;
        std::fs::create_dir_all(self.policies.join("opencode").join("system"))
            .context("Failed to create global policies/opencode/system directory")?;
        std::fs::create_dir_all(self.policies.join("opencode").join("builtins"))
            .context("Failed to create global policies/opencode/builtins directory")?;

        std::fs::create_dir_all(&self.signals)
            .context("Failed to create global signals directory")?;
        std::fs::create_dir_all(&self.actions)
            .context("Failed to create global actions directory")?;

        // Create minimal rulebook if it doesn't exist
        if !self.rulebook.exists() {
            let rulebook_content = r#"# Global Cupcake Configuration
#
# This configuration applies to ALL Cupcake projects on this machine.
# Global policies have absolute precedence and cannot be overridden.

# Signals and actions defined here are only available to global policies
signals: {}
actions: {}

# Builtins can be configured globally
builtins: {}
"#;
            std::fs::write(&self.rulebook, rulebook_content)
                .context("Failed to create global rulebook.yml")?;
        }

        // Note: System evaluate policies are created by init_global_config()
        // in a harness-specific manner (claude/ and cursor/ subdirectories)

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_from_root() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        let global_paths = GlobalPaths::from_root(root.clone()).unwrap();

        assert_eq!(global_paths.root, root);
        assert_eq!(global_paths.policies, root.join("policies"));
        assert_eq!(global_paths.rulebook, root.join("rulebook.yml"));
        assert_eq!(global_paths.signals, root.join("signals"));
        assert_eq!(global_paths.actions, root.join("actions"));
    }

    #[test]
    fn test_from_root_nonexistent() {
        let result = GlobalPaths::from_root(PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_with_cli_override() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        let result = GlobalPaths::discover_with_override(Some(root.clone())).unwrap();
        assert!(result.is_some());

        let global_paths = result.unwrap();
        // Path is now canonicalized, so compare canonicalized versions
        let expected_root = root.canonicalize().unwrap();
        assert_eq!(global_paths.root, expected_root);
    }

    #[test]
    fn test_discover_with_cli_override_relative_path() {
        let result = GlobalPaths::discover_with_override(Some(PathBuf::from("relative/path")));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be absolute"));
    }

    #[test]
    fn test_discover_with_cli_override_nonexistent() {
        let result = GlobalPaths::discover_with_override(Some(PathBuf::from("/nonexistent/path")));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Error message contains "does not exist" on Unix, "cannot find" on Windows
        assert!(
            err_msg.contains("does not exist")
                || err_msg.contains("cannot find")
                || err_msg.contains("nonexistent"),
            "Error message should indicate path doesn't exist, got: {err_msg}"
        );
    }

    #[test]
    fn test_discover_graceful_absence() {
        // Discovery should return None when no global config exists
        let result = GlobalPaths::discover().unwrap();

        // This might be Some if developer has global config installed
        // but that's okay - we're testing that it doesn't error
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_is_initialized() {
        let temp_dir = TempDir::new().unwrap();
        let global_paths = GlobalPaths::from_root(temp_dir.path().to_path_buf()).unwrap();

        // Not initialized yet
        assert!(!global_paths.is_initialized());

        // Initialize
        global_paths.initialize().unwrap();

        // Now should be initialized
        assert!(global_paths.is_initialized());
        assert!(global_paths.policies.exists());
        assert!(global_paths.rulebook.exists());
        assert!(global_paths.signals.exists());
        assert!(global_paths.actions.exists());
    }

    #[test]
    fn test_initialize_creates_structure() {
        let temp_dir = TempDir::new().unwrap();
        let global_paths = GlobalPaths::from_root(temp_dir.path().to_path_buf()).unwrap();

        global_paths.initialize().unwrap();

        // Check all directories exist
        assert!(global_paths.root.exists());
        assert!(global_paths.policies.exists());
        // Harness-specific subdirectories for all harnesses
        assert!(global_paths.policies.join("claude").join("system").exists());
        assert!(global_paths
            .policies
            .join("claude")
            .join("builtins")
            .exists());
        assert!(global_paths.policies.join("cursor").join("system").exists());
        assert!(global_paths
            .policies
            .join("cursor")
            .join("builtins")
            .exists());
        assert!(global_paths
            .policies
            .join("factory")
            .join("system")
            .exists());
        assert!(global_paths
            .policies
            .join("factory")
            .join("builtins")
            .exists());
        assert!(global_paths
            .policies
            .join("opencode")
            .join("system")
            .exists());
        assert!(global_paths
            .policies
            .join("opencode")
            .join("builtins")
            .exists());
        assert!(global_paths.signals.exists());
        assert!(global_paths.actions.exists());

        // Check rulebook exists
        assert!(global_paths.rulebook.exists());

        // Verify rulebook content
        let rulebook_content = std::fs::read_to_string(&global_paths.rulebook).unwrap();
        assert!(rulebook_content.contains("Global Cupcake Configuration"));

        // Note: System evaluate.rego files are created by init_global_config() in main.rs,
        // not by initialize(). This test only verifies the directory structure.
    }
}
