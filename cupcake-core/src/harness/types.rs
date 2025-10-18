//! Harness type definitions
//!
//! This module defines the harness types that Cupcake supports.
//! Each harness represents a different AI coding agent with its own
//! event schema, response format, and capabilities.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported AI coding agent harnesses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HarnessType {
    /// Claude Code (claude.ai/code) - Anthropic's official CLI
    #[serde(rename = "claude")]
    ClaudeCode,

    /// Cursor (cursor.sh) - AI-powered code editor
    #[serde(rename = "cursor")]
    Cursor,
}

impl HarnessType {
    /// Get the harness name as a string (lowercase)
    pub fn as_str(&self) -> &'static str {
        match self {
            HarnessType::ClaudeCode => "claude",
            HarnessType::Cursor => "cursor",
        }
    }

    /// Get the display name (proper casing)
    pub fn display_name(&self) -> &'static str {
        match self {
            HarnessType::ClaudeCode => "Claude Code",
            HarnessType::Cursor => "Cursor",
        }
    }

    /// Get the policy directory name for this harness
    pub fn policy_dir(&self) -> &'static str {
        match self {
            HarnessType::ClaudeCode => "claude",
            HarnessType::Cursor => "cursor",
        }
    }
}

impl fmt::Display for HarnessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for HarnessType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claudecode" | "claude-code" => Ok(HarnessType::ClaudeCode),
            "cursor" => Ok(HarnessType::Cursor),
            _ => Err(format!(
                "Unknown harness type: '{s}'. Valid options: claude, cursor"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_type_parsing() {
        assert_eq!(
            "claude".parse::<HarnessType>().unwrap(),
            HarnessType::ClaudeCode
        );
        assert_eq!(
            "claudecode".parse::<HarnessType>().unwrap(),
            HarnessType::ClaudeCode
        );
        assert_eq!(
            "claude-code".parse::<HarnessType>().unwrap(),
            HarnessType::ClaudeCode
        );
        assert_eq!(
            "cursor".parse::<HarnessType>().unwrap(),
            HarnessType::Cursor
        );
        assert_eq!(
            "CLAUDE".parse::<HarnessType>().unwrap(),
            HarnessType::ClaudeCode
        );
        assert_eq!(
            "CURSOR".parse::<HarnessType>().unwrap(),
            HarnessType::Cursor
        );
    }

    #[test]
    fn test_harness_type_invalid() {
        assert!("invalid".parse::<HarnessType>().is_err());
        assert!("windsurf".parse::<HarnessType>().is_err());
    }

    #[test]
    fn test_harness_type_display() {
        assert_eq!(HarnessType::ClaudeCode.to_string(), "claude");
        assert_eq!(HarnessType::Cursor.to_string(), "cursor");
        assert_eq!(HarnessType::ClaudeCode.display_name(), "Claude Code");
        assert_eq!(HarnessType::Cursor.display_name(), "Cursor");
    }

    #[test]
    fn test_policy_dir() {
        assert_eq!(HarnessType::ClaudeCode.policy_dir(), "claude");
        assert_eq!(HarnessType::Cursor.policy_dir(), "cursor");
    }
}
