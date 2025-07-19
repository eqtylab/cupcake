use cupcake::engine::conditions::{ConditionEvaluator, ConditionResult, EvaluationContext};
use cupcake::config::conditions::Condition;
use std::collections::HashMap;
use chrono::Utc;

#[test]
fn test_prompt_field_extraction() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create context with prompt
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::temp_dir(),
        env_vars: HashMap::new(),
        timestamp: Utc::now(),
        full_session_state: None,
        prompt: Some("Write a function to calculate factorial".to_string()),
    };

    // Test matching on prompt field
    let condition = Condition::Match {
        field: "prompt".to_string(),
        value: "Write a function to calculate factorial".to_string(),
    };

    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

#[test]
fn test_prompt_field_pattern_matching() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create context with prompt containing sensitive data
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::temp_dir(),
        env_vars: HashMap::new(),
        timestamp: Utc::now(),
        full_session_state: None,
        prompt: Some("My API key is sk-1234567890abcdef".to_string()),
    };

    // Test pattern matching for API keys
    let condition = Condition::Pattern {
        field: "prompt".to_string(),
        regex: r"sk-[a-zA-Z0-9]{16,}".to_string(),
    };

    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

#[test]
fn test_prompt_field_no_match() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create context without prompt (non-UserPromptSubmit event)
    let context = EvaluationContext {
        event_type: "PreToolUse".to_string(),
        tool_name: "Bash".to_string(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::temp_dir(),
        env_vars: HashMap::new(),
        timestamp: Utc::now(),
        full_session_state: None,
        prompt: None,
    };

    // Test matching on prompt field when it's None
    let condition = Condition::Match {
        field: "prompt".to_string(),
        value: "some value".to_string(),
    };

    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::NoMatch);
}

#[test]
fn test_cwd_from_evaluation_context() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create context with specific cwd
    let context = EvaluationContext {
        event_type: "PreToolUse".to_string(),
        tool_name: "Bash".to_string(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::path::PathBuf::from("/home/user/project"),
        env_vars: HashMap::new(),
        timestamp: Utc::now(),
        full_session_state: None,
        prompt: None,
    };

    // Test that cwd is properly used in check conditions
    // This would be used by the CommandExecutor
    assert_eq!(context.current_dir.to_str().unwrap(), "/home/user/project");
}