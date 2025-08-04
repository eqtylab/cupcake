//! Environment variable sanitization for security
//!
//! This module provides secure environment variable filtering to prevent
//! sensitive information leakage while preserving necessary Claude Code
//! integration variables.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::collections::HashSet;

/// Hardcoded allow-list of environment variables
/// Only these variables are exposed to policy evaluation and action execution
static ALLOWED_ENV_VARS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut allowed = HashSet::new();

    // Claude Code integration variables
    allowed.insert("CLAUDE_PROJECT_DIR"); // Essential for policy discovery
    allowed.insert("CLAUDE_SESSION_ID"); // Required for stateful policies

    // Standard development environment
    allowed.insert("HOME");
    allowed.insert("USER");
    allowed.insert("SHELL");
    allowed.insert("LANG");
    allowed.insert("LC_ALL");
    allowed.insert("PATH");
    allowed.insert("PWD");

    // Development tooling
    allowed.insert("EDITOR");
    allowed.insert("VISUAL");
    allowed.insert("TERM");
    allowed.insert("COLORTERM");

    // Common CI/CD indicators
    allowed.insert("CI");
    allowed.insert("GITHUB_ACTIONS");
    allowed.insert("GITLAB_CI");
    allowed.insert("JENKINS_HOME");
    allowed.insert("TRAVIS");
    allowed.insert("CIRCLECI");

    // Node.js/npm
    allowed.insert("NODE_ENV");
    allowed.insert("npm_lifecycle_event");
    allowed.insert("npm_package_name");
    allowed.insert("npm_package_version");

    // Python
    allowed.insert("PYTHONPATH");
    allowed.insert("VIRTUAL_ENV");
    allowed.insert("CONDA_DEFAULT_ENV");

    // Rust
    allowed.insert("CARGO");
    allowed.insert("CARGO_HOME");
    allowed.insert("RUSTUP_HOME");

    // Git
    allowed.insert("GIT_AUTHOR_NAME");
    allowed.insert("GIT_AUTHOR_EMAIL");
    allowed.insert("GIT_COMMITTER_NAME");
    allowed.insert("GIT_COMMITTER_EMAIL");

    // OS indicators
    allowed.insert("OS");
    allowed.insert("OSTYPE");

    allowed
});

/// Provides filtered access to environment variables
pub struct SanitizedEnvironment;

impl SanitizedEnvironment {
    /// Get all allowed environment variables
    pub fn vars() -> HashMap<String, String> {
        std::env::vars()
            .filter(|(key, _)| ALLOWED_ENV_VARS.contains(key.as_str()))
            .collect()
    }

    /// Get a specific environment variable if allowed
    pub fn var(key: &str) -> Option<String> {
        if ALLOWED_ENV_VARS.contains(key) {
            std::env::var(key).ok()
        } else {
            None
        }
    }

    /// Check if a variable is in the allow-list
    pub fn is_allowed(key: &str) -> bool {
        ALLOWED_ENV_VARS.contains(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_vars_allowed() {
        assert!(SanitizedEnvironment::is_allowed("CLAUDE_PROJECT_DIR"));
        assert!(SanitizedEnvironment::is_allowed("CLAUDE_SESSION_ID"));
    }

    #[test]
    fn test_sensitive_vars_blocked() {
        // These should NOT be in the allow-list
        assert!(!SanitizedEnvironment::is_allowed("AWS_SECRET_ACCESS_KEY"));
        assert!(!SanitizedEnvironment::is_allowed("DATABASE_PASSWORD"));
        assert!(!SanitizedEnvironment::is_allowed("API_KEY"));
        assert!(!SanitizedEnvironment::is_allowed("GITHUB_TOKEN"));
        assert!(!SanitizedEnvironment::is_allowed("NPM_TOKEN"));
    }

    #[test]
    fn test_standard_vars_allowed() {
        assert!(SanitizedEnvironment::is_allowed("HOME"));
        assert!(SanitizedEnvironment::is_allowed("USER"));
        assert!(SanitizedEnvironment::is_allowed("PATH"));
        assert!(SanitizedEnvironment::is_allowed("PWD"));
    }

    #[test]
    fn test_vars_filtering() {
        // This test verifies the vars() method filters correctly
        let vars = SanitizedEnvironment::vars();

        // Should not contain any keys not in the allow-list
        for key in vars.keys() {
            assert!(
                ALLOWED_ENV_VARS.contains(key.as_str()),
                "Unexpected variable in sanitized environment: {key}"
            );
        }
    }

    #[test]
    fn test_var_method() {
        // Test allowed variable
        if std::env::var("HOME").is_ok() {
            assert!(SanitizedEnvironment::var("HOME").is_some());
        }

        // Test blocked variable
        std::env::set_var("TEST_SECRET_KEY", "secret123");
        assert!(SanitizedEnvironment::var("TEST_SECRET_KEY").is_none());
        std::env::remove_var("TEST_SECRET_KEY");
    }
}
