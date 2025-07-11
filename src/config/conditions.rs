use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Condition types for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Match against tool input command (Bash tool)
    CommandRegex { 
        value: String,
        #[serde(default)]
        flags: Vec<String>,
    },
    
    /// Match against file paths using regex
    FilepathRegex { 
        value: String,
        #[serde(default)]
        flags: Vec<String>,
    },
    
    /// Match against file paths using glob patterns
    FilepathGlob { 
        value: String,
    },
    
    /// Match file content using regex (for Edit/Write tools)
    FileContentRegex { 
        value: String,
        #[serde(default)]
        flags: Vec<String>,
    },
    
    /// Logical NOT operator
    Not { 
        condition: Box<Condition>,
    },
    
    /// Logical AND operator
    And { 
        conditions: Vec<Condition>,
    },
    
    /// Logical OR operator
    Or { 
        conditions: Vec<Condition>,
    },
    
    /// Check if a state exists (tool usage tracking)
    StateExists { 
        #[serde(flatten)]
        query: StateQuery,
    },
    
    /// Check if a state does NOT exist
    StateMissing { 
        #[serde(flatten)]
        query: StateQuery,
    },
    
    /// Query state with specific criteria
    StateQuery { 
        #[serde(flatten)]
        query: StateQuery,
    },
    
    /// Check if a file exists on filesystem
    FileExists { 
        path: String,
    },
    
    /// Check if file was modified within timeframe
    FileModifiedWithin { 
        path: String,
        minutes: u32,
    },
    
    /// Check environment variable value
    EnvVarEquals { 
        name: String,
        value: String,
    },
    
    /// Check if working directory contains pattern
    WorkingDirContains { 
        value: String,
    },
    
    /// Time window constraint
    TimeWindow { 
        start: String,  // HH:MM format
        end: String,    // HH:MM format
        #[serde(skip_serializing_if = "Option::is_none")]
        timezone: Option<String>,
    },
    
    /// Day of week constraint
    DayOfWeek { 
        days: Vec<String>,  // ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    },
    
    /// Check if message contains text (for Notification events)
    MessageContains { 
        value: String,
    },
}

/// State query parameters for tracking tool usage and custom events
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateQuery {
    /// Tool name (Read, Write, Edit, Bash, etc.) - for automatic tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    
    /// File path for file-based tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Custom event name - for explicit update_state actions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    
    /// Command pattern for Bash tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_contains: Option<String>,
    
    /// Result of the operation (success, failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    
    /// Time constraint in minutes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within_minutes: Option<u32>,
    
    /// Since last occurrence of another event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    
    /// Additional query parameters
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_condition_serialization() {
        let condition = Condition::CommandRegex {
            value: "git\\s+commit".to_string(),
            flags: vec!["multiline".to_string()],
        };
        
        let toml = toml::to_string(&condition).unwrap();
        let deserialized: Condition = toml::from_str(&toml).unwrap();
        
        match deserialized {
            Condition::CommandRegex { value, flags } => {
                assert_eq!(value, "git\\s+commit");
                assert_eq!(flags, vec!["multiline"]);
            }
            _ => panic!("Wrong condition type"),
        }
    }

    #[test]
    fn test_state_query_default() {
        let query = StateQuery::default();
        assert!(query.tool.is_none());
        assert!(query.path.is_none());
        assert!(query.event.is_none());
        assert!(query.additional.is_empty());
    }

    #[test]
    fn test_nested_condition() {
        let condition = Condition::And {
            conditions: vec![
                Condition::FilepathRegex {
                    value: "\\.rs$".to_string(),
                    flags: vec![],
                },
                Condition::Not {
                    condition: Box::new(Condition::FilepathGlob {
                        value: "test/**".to_string(),
                    }),
                },
            ],
        };
        
        let toml = toml::to_string(&condition).unwrap();
        let _deserialized: Condition = toml::from_str(&toml).unwrap();
    }
}