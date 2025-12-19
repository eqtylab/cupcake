//! Configuration for input preprocessing behavior
//!
//! This module defines how preprocessing should behave, allowing
//! fine-grained control over what normalizations are applied.

/// Result of preprocessing, including which operations were actually applied
#[derive(Debug, Clone, Default)]
pub struct PreprocessResult {
    /// List of operations that were actually performed
    pub applied_operations: Vec<String>,
}

impl PreprocessResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            applied_operations: Vec::new(),
        }
    }

    /// Record that an operation was applied
    pub fn record(&mut self, operation: &str) {
        self.applied_operations.push(operation.to_string());
    }

    /// Get the list of applied operations
    pub fn operations(&self) -> &[String] {
        &self.applied_operations
    }
}

/// Main configuration for preprocessing behavior
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// Enable whitespace normalization for commands
    ///
    /// When enabled, collapses multiple spaces, converts tabs to spaces,
    /// and trims leading/trailing whitespace from Bash commands.
    pub normalize_whitespace: bool,

    /// Log all transformations for audit purposes
    ///
    /// When enabled, logs original → normalized transformations
    /// at DEBUG level for security auditing.
    pub audit_transformations: bool,

    /// Enable script inspection - when a shell command executes a script,
    /// load and attach its content for policy evaluation (TOB-2 defense)
    ///
    /// When enabled, detects script execution patterns (e.g., ./script.sh,
    /// bash script.sh, python script.py) and loads the script content,
    /// making it available as input.executed_script_content for policies.
    pub enable_script_inspection: bool,

    /// Enable symlink resolution for file paths (TOB-4 defense)
    ///
    /// When enabled, detects symbolic links in file operation tool calls and
    /// resolves them to their canonical target paths. This prevents bypass attacks
    /// where attackers create symlinks to protected directories (e.g., ln -s .cupcake/ tmp/).
    ///
    /// The resolved path is attached as input.resolved_file_path and input.is_symlink
    /// for policies to check. Performance impact is negligible (~15-30μs per operation).
    ///
    /// Enabled by default for defense-in-depth security.
    pub enable_symlink_resolution: bool,
    // Future fields:
    // /// Enable command substitution detection
    // pub detect_substitution: bool,
    //
    // /// Enable inline function detection
    // pub detect_functions: bool,
    //
    // /// Tool-specific configurations
    // pub tool_configs: HashMap<String, ToolConfig>,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            normalize_whitespace: true,      // Enable by default for security
            audit_transformations: true,     // Enable audit trail by default
            enable_script_inspection: false, // Opt-in for script inspection (performance consideration)
            enable_symlink_resolution: true, // Enable by default (performance is negligible ~30μs)
        }
    }
}

impl PreprocessConfig {
    /// Create a minimal configuration (normalization only, no logging)
    pub fn minimal() -> Self {
        Self {
            normalize_whitespace: true,
            audit_transformations: false,
            enable_script_inspection: false,
            enable_symlink_resolution: true, // Still enabled for security
        }
    }

    /// Create a disabled configuration (no preprocessing)
    pub fn disabled() -> Self {
        Self {
            normalize_whitespace: false,
            audit_transformations: false,
            enable_script_inspection: false,
            enable_symlink_resolution: false, // All preprocessing disabled
        }
    }

    /// Create a debug configuration (all features, verbose logging)
    pub fn debug() -> Self {
        Self {
            normalize_whitespace: true,
            audit_transformations: true,
            enable_script_inspection: true, // Enable in debug mode for maximum inspection
            enable_symlink_resolution: true,
        }
    }

    /// Create a configuration with script inspection enabled
    pub fn with_script_inspection() -> Self {
        Self {
            normalize_whitespace: true,
            audit_transformations: true,
            enable_script_inspection: true,
            enable_symlink_resolution: true,
        }
    }

    /// Create a configuration with symlink resolution enabled
    pub fn with_symlink_resolution() -> Self {
        Self {
            normalize_whitespace: true,
            audit_transformations: true,
            enable_script_inspection: false,
            enable_symlink_resolution: true,
        }
    }
}

// Future: Tool-specific configuration
// #[derive(Debug, Clone)]
// pub struct ToolConfig {
//     /// Apply normalization to this tool
//     pub enabled: bool,
//
//     /// Tool-specific rules
//     pub rules: Vec<Rule>,
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PreprocessConfig::default();
        assert!(config.normalize_whitespace);
        assert!(config.audit_transformations);
        assert!(!config.enable_script_inspection); // Off by default for performance
        assert!(config.enable_symlink_resolution); // On by default for security
    }

    #[test]
    fn test_minimal_config() {
        let config = PreprocessConfig::minimal();
        assert!(config.normalize_whitespace);
        assert!(!config.audit_transformations);
        assert!(!config.enable_script_inspection);
        assert!(config.enable_symlink_resolution); // Still enabled for security
    }

    #[test]
    fn test_disabled_config() {
        let config = PreprocessConfig::disabled();
        assert!(!config.normalize_whitespace);
        assert!(!config.audit_transformations);
        assert!(!config.enable_script_inspection);
        assert!(!config.enable_symlink_resolution); // All disabled
    }

    #[test]
    fn test_debug_config() {
        let config = PreprocessConfig::debug();
        assert!(config.normalize_whitespace);
        assert!(config.audit_transformations);
        assert!(config.enable_script_inspection); // Enabled in debug mode
        assert!(config.enable_symlink_resolution);
    }

    #[test]
    fn test_with_script_inspection() {
        let config = PreprocessConfig::with_script_inspection();
        assert!(config.normalize_whitespace);
        assert!(config.audit_transformations);
        assert!(config.enable_script_inspection);
        assert!(config.enable_symlink_resolution);
    }

    #[test]
    fn test_with_symlink_resolution() {
        let config = PreprocessConfig::with_symlink_resolution();
        assert!(config.normalize_whitespace);
        assert!(config.audit_transformations);
        assert!(!config.enable_script_inspection); // Off by default
        assert!(config.enable_symlink_resolution); // Enabled
    }
}
