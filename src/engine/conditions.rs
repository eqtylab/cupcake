use crate::config::conditions::Condition;
use crate::Result;
use regex::Regex;
use std::collections::HashMap;

/// Context for condition evaluation with Claude Code hook integration
#[derive(Debug)]
pub struct EvaluationContext {
    /// Hook event type (PreToolUse, PostToolUse, etc.)
    pub event_type: String,
    /// Tool name being executed
    pub tool_name: String,
    /// Tool input parameters from Claude Code hook data
    pub tool_input: HashMap<String, serde_json::Value>,
    /// Session ID for state tracking
    pub session_id: String,
    /// Current working directory
    pub current_dir: std::path::PathBuf,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Timestamp of current evaluation
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Prompt text for UserPromptSubmit events
    pub prompt: Option<String>,
}

/// Result of condition evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionResult {
    /// Condition matched
    Match,
    /// Condition did not match
    NoMatch,
    /// Condition evaluation failed (treat as no match)
    Error(String),
}

impl ConditionResult {
    /// Check if this result represents a match
    pub fn is_match(&self) -> bool {
        matches!(self, ConditionResult::Match)
    }

    /// Check if this result represents an error
    pub fn is_error(&self) -> bool {
        matches!(self, ConditionResult::Error(_))
    }
}

/// Condition evaluator using 3-primitive model
pub struct ConditionEvaluator {
    /// Compiled regex cache for performance
    regex_cache: HashMap<String, Regex>,
}

impl ConditionEvaluator {
    /// Create new condition evaluator
    pub fn new() -> Self {
        Self {
            regex_cache: HashMap::new(),
        }
    }

    /// Evaluate a condition against the given context
    pub fn evaluate(
        &mut self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> ConditionResult {
        match condition {
            Condition::Match { field, value } => self.evaluate_match(field, value, context),
            Condition::Pattern { field, regex } => self.evaluate_pattern(field, regex, context),
            Condition::Check {
                spec,
                expect_success,
            } => self.evaluate_check(spec.as_ref(), *expect_success, context),
            Condition::Not { condition } => self.evaluate_not(condition, context),
            Condition::And { conditions } => self.evaluate_and(conditions, context),
            Condition::Or { conditions } => self.evaluate_or(conditions, context),
        }
    }

    /// Evaluate Match condition - direct string comparison
    fn evaluate_match(
        &self,
        field: &str,
        value: &str,
        context: &EvaluationContext,
    ) -> ConditionResult {
        match self.extract_field(field, context) {
            Some(field_value) => {
                if field_value == value {
                    ConditionResult::Match
                } else {
                    ConditionResult::NoMatch
                }
            }
            None => ConditionResult::NoMatch,
        }
    }

    /// Evaluate Pattern condition - regex matching
    fn evaluate_pattern(
        &mut self,
        field: &str,
        regex: &str,
        context: &EvaluationContext,
    ) -> ConditionResult {
        let field_value = match self.extract_field(field, context) {
            Some(value) => value,
            None => return ConditionResult::NoMatch,
        };

        match self.get_or_compile_regex(regex) {
            Ok(regex_obj) => {
                if regex_obj.is_match(&field_value) {
                    ConditionResult::Match
                } else {
                    ConditionResult::NoMatch
                }
            }
            Err(e) => ConditionResult::Error(format!("Invalid regex '{}': {}", regex, e)),
        }
    }

    /// Evaluate Check condition - secure command execution (Plan 008)
    fn evaluate_check(
        &self,
        spec: &crate::config::actions::CommandSpec,
        expect_success: bool,
        context: &EvaluationContext,
    ) -> ConditionResult {
        // Create template variables from context for secure substitution
        let mut template_vars = std::collections::HashMap::new();
        
        // Add basic context variables
        template_vars.insert("tool_name".to_string(), context.tool_name.clone());
        template_vars.insert("session_id".to_string(), context.session_id.clone());
        template_vars.insert("event_type".to_string(), context.event_type.clone());
        
        // Add tool input variables
        for (key, value) in &context.tool_input {
            if let Some(str_value) = value.as_str() {
                template_vars.insert(format!("tool_input.{}", key), str_value.to_string());
            }
        }
        
        // Add environment variables
        for (key, value) in &context.env_vars {
            template_vars.insert(format!("env.{}", key), value.clone());
        }

        // Create secure CommandExecutor
        let command_executor = crate::engine::command_executor::CommandExecutor::new(template_vars);

        // Build secure CommandGraph 
        let graph = match command_executor.build_graph(spec) {
            Ok(graph) => graph,
            Err(e) => return ConditionResult::Error(format!("Command graph construction failed: {}", e)),
        };

        // Execute with secure, shell-free process spawning
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build() {
            Ok(rt) => rt,
            Err(e) => return ConditionResult::Error(format!("Failed to create async runtime: {}", e)),
        };

        let execution_result = rt.block_on(async {
            command_executor.execute_graph(&graph).await
        });

        match execution_result {
            Ok(result) => {
                // Check if result matches expectation
                if result.success == expect_success {
                    ConditionResult::Match
                } else {
                    ConditionResult::NoMatch
                }
            }
            Err(e) => ConditionResult::Error(format!("Secure command execution failed: {}", e)),
        }
    }

    /// Extract field value from evaluation context
    fn extract_field(&self, field: &str, context: &EvaluationContext) -> Option<String> {
        match field {
            // Event-level fields
            "event_type" => Some(context.event_type.clone()),
            "tool_name" => Some(context.tool_name.clone()),
            "session_id" => Some(context.session_id.clone()),
            "prompt" => context.prompt.clone(),

            // Tool input fields (dot notation)
            field_name if field_name.starts_with("tool_input.") => {
                let key = &field_name[11..]; // Remove "tool_input." prefix
                context.tool_input.get(key).and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                })
            }

            // Environment variables (dot notation)
            field_name if field_name.starts_with("env.") => {
                let key = &field_name[4..]; // Remove "env." prefix
                context.env_vars.get(key).cloned()
            }

            // Direct tool input access (legacy support)
            _ => context.tool_input.get(field).and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            }),
        }
    }

    // Legacy template expansion removed in Plan 008
    // Template substitution now handled securely by CommandExecutor

    /// Evaluate logical NOT condition
    fn evaluate_not(
        &mut self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> ConditionResult {
        match self.evaluate(condition, context) {
            ConditionResult::Match => ConditionResult::NoMatch,
            ConditionResult::NoMatch => ConditionResult::Match,
            err => err, // Preserve errors
        }
    }

    /// Evaluate logical AND condition
    fn evaluate_and(
        &mut self,
        conditions: &[Condition],
        context: &EvaluationContext,
    ) -> ConditionResult {
        for condition in conditions {
            match self.evaluate(condition, context) {
                ConditionResult::Match => continue,
                other => return other, // Return first non-match or error
            }
        }
        ConditionResult::Match
    }

    /// Evaluate logical OR condition
    fn evaluate_or(
        &mut self,
        conditions: &[Condition],
        context: &EvaluationContext,
    ) -> ConditionResult {
        let mut last_error = None;

        for condition in conditions {
            match self.evaluate(condition, context) {
                ConditionResult::Match => return ConditionResult::Match,
                ConditionResult::NoMatch => continue,
                ConditionResult::Error(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // If we had errors but no matches, return the last error
        if let Some(error) = last_error {
            ConditionResult::Error(error)
        } else {
            ConditionResult::NoMatch
        }
    }

    /// Get or compile regex with caching
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)
                .map_err(|e| crate::CupcakeError::Condition(format!("Invalid regex: {}", e)))?;

            self.regex_cache.insert(pattern.to_string(), regex);
        }

        self.regex_cache
            .get(pattern)
            .ok_or_else(|| crate::CupcakeError::Condition("Regex cache inconsistency".to_string()))
    }


    // Legacy insecure command conversion removed in Plan 008
    // All command execution now uses secure CommandExecutor with zero shell involvement
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(dead_code)] // Temporarily disabled for Plan 008 transition
mod tests_disabled {
    use super::*;
    use chrono::Utc;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;

    fn create_test_context() -> EvaluationContext {
        let mut tool_input = HashMap::new();
        tool_input.insert(
            "command".to_string(),
            serde_json::Value::String("git commit -m 'test'".to_string()),
        );
        tool_input.insert(
            "file_path".to_string(),
            serde_json::Value::String("src/main.rs".to_string()),
        );
        tool_input.insert(
            "content".to_string(),
            serde_json::Value::String("fn main() {\n    println!(\"Hello\");\n}".to_string()),
        );

        let mut env_vars = HashMap::new();
        env_vars.insert("NODE_ENV".to_string(), "development".to_string());
        env_vars.insert("USER".to_string(), "testuser".to_string());

        EvaluationContext {
            event_type: "PreToolUse".to_string(),
            tool_name: "Bash".to_string(),
            tool_input,
            session_id: "test-session-123".to_string(),
            current_dir: std::env::temp_dir(),
            env_vars,
            timestamp: Utc::now(),
            prompt: None,
        }
    }

    #[test]
    fn test_match_condition_tool_name() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Match {
            field: "tool_name".to_string(),
            value: "Bash".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_match_condition_no_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Match {
            field: "tool_name".to_string(),
            value: "Edit".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::NoMatch);
    }

    #[test]
    fn test_pattern_condition_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: r"git\s+commit".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_pattern_condition_no_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: r"git\s+push".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::NoMatch);
    }

    #[test]
    fn test_pattern_filepath_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Pattern {
            field: "tool_input.file_path".to_string(),
            regex: r"\.rs$".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_check_condition_success() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Check {
            spec: Box::new(crate::config::actions::CommandSpec::Array(Box::new(crate::config::actions::ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }))),
            expect_success: true,
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_check_condition_failure() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Check {
            spec: Box::new(crate::config::actions::CommandSpec::Array(Box::new(crate::config::actions::ArrayCommandSpec {
                command: vec!["false".to_string()],
                args: None,
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }))),
            expect_success: true,
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::NoMatch);
    }

    #[test]
    fn test_check_condition_expect_failure() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Check {
            spec: Box::new(crate::config::actions::CommandSpec::Array(Box::new(crate::config::actions::ArrayCommandSpec {
                command: vec!["false".to_string()],
                args: None,
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }))),
            expect_success: false,
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_template_variable_expansion() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Check {
            spec: Box::new(crate::config::actions::CommandSpec::Array(Box::new(crate::config::actions::ArrayCommandSpec {
                command: vec!["test".to_string()],
                args: Some(vec!["{{tool_name}}".to_string(), "=".to_string(), "Bash".to_string()]),
                working_dir: None,
                env: None,
                pipe: None,
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }))),
            expect_success: true,
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_env_variable_extraction() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Match {
            field: "env.NODE_ENV".to_string(),
            value: "development".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_logical_and_all_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::And {
            conditions: vec![
                Condition::Match {
                    field: "tool_name".to_string(),
                    value: "Bash".to_string(),
                },
                Condition::Pattern {
                    field: "tool_input.file_path".to_string(),
                    regex: r"\.rs$".to_string(),
                },
            ],
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_logical_and_partial_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::And {
            conditions: vec![
                Condition::Match {
                    field: "tool_name".to_string(),
                    value: "Bash".to_string(),
                },
                Condition::Pattern {
                    field: "tool_input.file_path".to_string(),
                    regex: r"\.js$".to_string(), // This won't match
                },
            ],
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::NoMatch);
    }

    #[test]
    fn test_logical_or_partial_match() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Or {
            conditions: vec![
                Condition::Match {
                    field: "tool_name".to_string(),
                    value: "Edit".to_string(), // This won't match
                },
                Condition::Pattern {
                    field: "tool_input.file_path".to_string(),
                    regex: r"\.rs$".to_string(), // This will match
                },
            ],
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_logical_not() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Not {
            condition: Box::new(Condition::Match {
                field: "tool_name".to_string(),
                value: "Edit".to_string(), // This won't match, so NOT will match
            }),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }

    #[test]
    fn test_invalid_regex() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Pattern {
            field: "tool_input.command".to_string(),
            regex: r"[invalid".to_string(), // Invalid regex
        };

        let result = evaluator.evaluate(&condition, &context);
        assert!(result.is_error());
    }

    #[test]
    fn test_field_extraction_nonexistent() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Match {
            field: "nonexistent_field".to_string(),
            value: "anything".to_string(),
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::NoMatch);
    }

    #[test]
    fn test_complex_logical_conditions() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        // Test complex nested logical condition: (tool_name=Bash AND file_path=*.rs) OR env.NODE_ENV=production
        let condition = Condition::Or {
            conditions: vec![
                Condition::And {
                    conditions: vec![
                        Condition::Match {
                            field: "tool_name".to_string(),
                            value: "Bash".to_string(),
                        },
                        Condition::Pattern {
                            field: "tool_input.file_path".to_string(),
                            regex: r"\.rs$".to_string(),
                        },
                    ],
                },
                Condition::Match {
                    field: "env.NODE_ENV".to_string(),
                    value: "production".to_string(), // This won't match
                },
            ],
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match); // First AND condition should match
    }

    #[test]
    fn test_check_with_file_path_template() {
        let mut evaluator = ConditionEvaluator::new();
        let context = create_test_context();

        let condition = Condition::Check {
            spec: Box::new(crate::config::actions::CommandSpec::Array(Box::new(crate::config::actions::ArrayCommandSpec {
                command: vec!["echo".to_string()],
                args: Some(vec!["{{tool_input.file_path}}".to_string()]),
                working_dir: None,
                env: None,
                pipe: Some(vec![crate::config::actions::PipeCommand {
                    cmd: vec!["grep".to_string(), "-q".to_string(), "\\.rs$".to_string()],
                }]),
                redirect_stdout: None,
                append_stdout: None,
                redirect_stderr: None,
                merge_stderr: None,
                on_success: None,
                on_failure: None,
            }))),
            expect_success: true,
        };

        let result = evaluator.evaluate(&condition, &context);
        assert_eq!(result, ConditionResult::Match);
    }
}
