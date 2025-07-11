use serde::{Deserialize, Serialize};

/// Condition types for policy evaluation using 3-primitive model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Field matching - direct string comparison
    /// Example: { type = "match", field = "tool_name", value = "Bash" }
    Match {
        /// Field to extract from context (e.g., "tool_name", "event_type")
        field: String,
        /// Value to match exactly
        value: String,
    },

    /// Pattern matching - regex on extracted field
    /// Example: { type = "pattern", field = "tool_input.file_path", regex = "\\.tsx?$" }
    Pattern {
        /// Field to extract from context (e.g., "tool_input.command", "tool_input.file_path")
        field: String,
        /// Regex pattern to match against field value
        regex: String,
    },

    /// Command execution - run shell command for complex checks
    /// Example: { type = "check", command = "[ $(date +%u) -le 5 ]", expect_success = true }
    Check {
        /// Shell command to execute (supports template variables)
        command: String,
        /// Whether exit code 0 means condition matches (true) or doesn't match (false)
        #[serde(default = "default_expect_success")]
        expect_success: bool,
    },

    /// Logical NOT operator
    Not { condition: Box<Condition> },

    /// Logical AND operator
    And { conditions: Vec<Condition> },

    /// Logical OR operator
    Or { conditions: Vec<Condition> },
}

/// Default value for expect_success field
fn default_expect_success() -> bool {
    true
}

/// State query parameters for tracking tool usage and custom events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateQueryFilter {
    /// Tool name (Read, Write, Edit, Bash, etc.) - for automatic tracking
    pub tool: String,
    /// Command pattern for Bash tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_contains: Option<String>,
    /// Result of the operation (success, failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Time constraint in minutes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within_minutes: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_condition_serialization() {
        let condition = Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: "git\\s+commit".to_string(),
        };

        let toml = toml::to_string(&condition).unwrap();
        let deserialized: Condition = toml::from_str(&toml).unwrap();

        match deserialized {
            Condition::Pattern { field, regex } => {
                assert_eq!(field, "tool_input.command");
                assert_eq!(regex, "git\\s+commit");
            }
            _ => panic!("Wrong condition type"),
        }
    }

    #[test]
    fn test_nested_condition() {
        let condition = Condition::And {
            conditions: vec![
                Condition::Pattern {
                    field: "tool_input.file_path".to_string(),
                    regex: "\\.rs$".to_string(),
                },
                Condition::Not {
                    condition: Box::new(Condition::Pattern {
                        field: "tool_input.file_path".to_string(),
                        regex: "test/".to_string(),
                    }),
                },
            ],
        };

        let toml = toml::to_string(&condition).unwrap();
        let _deserialized: Condition = toml::from_str(&toml).unwrap();
    }
}
