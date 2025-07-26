use cupcake::config::actions::Action;
use cupcake::config::conditions::{Condition, StateQueryFilter};
use cupcake::config::loader::LoadedConfiguration;
use cupcake::config::types::{ComposedPolicy, HookEventType, Settings};
use cupcake::engine::actions::{ActionContext, ActionExecutor, ActionResult};
use cupcake::engine::conditions::{ConditionEvaluator, ConditionResult, EvaluationContext};
use cupcake::engine::evaluation::PolicyEvaluator;
use cupcake::engine::events::{HookEvent, CommonEventData};
use cupcake::engine::response::EngineDecision;
use cupcake::state::manager::StateManager;
use cupcake::state::types::{SessionState, StateEntry};
use std::collections::HashMap;
use tempfile::tempdir;

/// Test InjectContext action on its own
#[test]
fn test_inject_context_action() {
    let mut executor = ActionExecutor::new();
    let temp_dir = tempdir().unwrap();
    let mut state_manager = StateManager::new(temp_dir.path()).unwrap();
    
    // Create InjectContext action
    let action = Action::InjectContext {
        context: "Remember: Always run tests before committing".to_string(),
        use_stdout: true,
    };
    
    // Create action context
    let context = ActionContext::new(
        "UserPromptSubmit".to_string(),
        HashMap::new(),
        std::env::current_dir().unwrap(),
        HashMap::new(),
        "test-session".to_string(),
    );
    
    // Execute action
    let result = executor.execute(&action, &context, Some(&mut state_manager));
    
    match result {
        ActionResult::Success { feedback, .. } => {
            assert_eq!(feedback, Some("Remember: Always run tests before committing".to_string()));
        }
        _ => panic!("Expected Success result"),
    }
}

/// Test StateQuery condition evaluation without state
#[test]
fn test_state_query_no_state() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create StateQuery condition looking for recent npm test
    let condition = Condition::StateQuery {
        filter: StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("npm test".to_string()),
            result: Some("success".to_string()),
            within_minutes: Some(30),
        },
        expect_exists: true,
    };
    
    // Create evaluation context without state
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::current_dir().unwrap(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: None,
        prompt: Some("Help me commit my changes".to_string()),
    };
    
    // Should not match since no state is loaded
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::NoMatch);
}

/// Test StateQuery condition evaluation with matching state
#[test]
fn test_state_query_with_matching_state() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create StateQuery condition
    let condition = Condition::StateQuery {
        filter: StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("npm test".to_string()),
            result: Some("success".to_string()),
            within_minutes: Some(30),
        },
        expect_exists: true,
    };
    
    // Create session state with matching tool usage
    let mut session_state = SessionState::new("test-session".to_string());
    let mut tool_input = HashMap::new();
    tool_input.insert("command".to_string(), serde_json::json!("npm test"));
    
    session_state.add_entry(StateEntry::new_tool_usage(
        "Bash".to_string(),
        tool_input,
        true, // success
        Some(serde_json::json!("All tests passed")),
        Some(1000),
    ));
    
    // Create evaluation context with state
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::current_dir().unwrap(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: Some(session_state),
        prompt: Some("Help me commit my changes".to_string()),
    };
    
    // Should match
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

/// Test StateQuery with time constraint
#[test]
fn test_state_query_time_constraint() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create StateQuery condition - must be within 5 minutes
    let condition = Condition::StateQuery {
        filter: StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("npm test".to_string()),
            result: Some("success".to_string()),
            within_minutes: Some(5),
        },
        expect_exists: true,
    };
    
    // Create session state with old tool usage (10 minutes ago)
    let mut session_state = SessionState::new("test-session".to_string());
    let mut tool_input = HashMap::new();
    tool_input.insert("command".to_string(), serde_json::json!("npm test"));
    
    let mut old_entry = StateEntry::new_tool_usage(
        "Bash".to_string(),
        tool_input,
        true,
        None,
        None,
    );
    
    // Manually set timestamp to 10 minutes ago
    old_entry.timestamp = chrono::Utc::now() - chrono::Duration::minutes(10);
    session_state.add_entry(old_entry);
    
    // Create evaluation context
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::current_dir().unwrap(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: Some(session_state),
        prompt: Some("Help me commit my changes".to_string()),
    };
    
    // Should not match due to time constraint
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::NoMatch);
}

/// Test StateQuery with expect_exists = false
#[test]
fn test_state_query_expect_not_exists() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create StateQuery condition - expect NO recent force push
    let condition = Condition::StateQuery {
        filter: StateQueryFilter {
            tool: "Bash".to_string(),
            command_contains: Some("git push --force".to_string()),
            result: None, // Don't care about result
            within_minutes: Some(60),
        },
        expect_exists: false, // We want to ensure this HASN'T happened
    };
    
    // Create session state without any force pushes
    let mut session_state = SessionState::new("test-session".to_string());
    let mut tool_input = HashMap::new();
    tool_input.insert("command".to_string(), serde_json::json!("git status"));
    
    session_state.add_entry(StateEntry::new_tool_usage(
        "Bash".to_string(),
        tool_input,
        true,
        None,
        None,
    ));
    
    // Create evaluation context
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::current_dir().unwrap(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: Some(session_state),
        prompt: Some("Help me push my changes".to_string()),
    };
    
    // Should match since no force push exists
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

/// Test full policy evaluation with StateQuery and InjectContext
#[test]
fn test_policy_with_state_query_and_inject_context() {
    let temp_dir = tempdir().unwrap();
    let mut state_manager = StateManager::new(temp_dir.path()).unwrap();
    
    // First, add some tool usage to state
    let mut tool_input = HashMap::new();
    tool_input.insert("command".to_string(), serde_json::json!("npm test"));
    
    state_manager.add_tool_usage(
        "test-session",
        "Bash".to_string(),
        tool_input.clone(),
        false, // tests failed
        Some(serde_json::json!("2 tests failed")),
        None,
    ).unwrap();
    
    // Create policy that checks if tests failed recently and injects context
    let policy = ComposedPolicy {
        name: "remind-to-fix-tests".to_string(),
        description: Some("Remind to fix failing tests before committing".to_string()),
        hook_event: HookEventType::UserPromptSubmit,
        matcher: "*".to_string(),
        conditions: vec![
            // Check if prompt mentions commit
            Condition::Pattern {
                field: "prompt".to_string(),
                regex: r"(?i)(commit|push)".to_string(),
            },
            // Check if tests failed recently
            Condition::StateQuery {
                filter: StateQueryFilter {
                    tool: "Bash".to_string(),
                    command_contains: Some("npm test".to_string()),
                    result: Some("failure".to_string()),
                    within_minutes: Some(30),
                },
                expect_exists: true,
            },
        ],
        action: Action::InjectContext {
            context: "⚠️ Recent test failures detected! Please fix the failing tests before committing. Run 'npm test' to see the failures.".to_string(),
            use_stdout: true,
        },
    };
    
    // Create configuration
    let config = LoadedConfiguration {
        policies: vec![policy],
        settings: Settings::default(),
    };
    
    // Create hook event
    let hook_event = HookEvent::UserPromptSubmit {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: temp_dir.path().to_str().unwrap().to_string(),
        },
        prompt: "I want to commit my changes now".to_string(),
    };
    
    // Build evaluation context with state
    let session_state = state_manager.get_session_state("test-session").unwrap();
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: temp_dir.path().to_path_buf(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: Some(session_state.clone()),
        prompt: Some("I want to commit my changes now".to_string()),
    };
    
    // Evaluate policy
    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&config.policies, &hook_event, &context).unwrap();
    
    // Should match and return Allow decision
    assert_eq!(result.matched_policies.len(), 1);
    assert!(matches!(result.decision, EngineDecision::Allow { .. }));
    
    // Execute the action
    let mut executor = ActionExecutor::new();
    let action_context = ActionContext::new(
        "UserPromptSubmit".to_string(),
        HashMap::new(),
        temp_dir.path().to_path_buf(),
        HashMap::new(),
        "test-session".to_string(),
    );
    
    let action_result = executor.execute(
        &result.matched_policies[0].action,
        &action_context,
        Some(&mut state_manager),
    );
    
    // Should get the warning context
    match action_result {
        ActionResult::Success { feedback, .. } => {
            assert!(feedback.unwrap().contains("Recent test failures detected"));
        }
        _ => panic!("Expected Success result"),
    }
}

/// Test complex StateQuery with multiple tool checks
#[test]
fn test_complex_state_query_multiple_tools() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create AND condition: tests passed AND linting passed
    let condition = Condition::And {
        conditions: vec![
            Condition::StateQuery {
                filter: StateQueryFilter {
                    tool: "Bash".to_string(),
                    command_contains: Some("npm test".to_string()),
                    result: Some("success".to_string()),
                    within_minutes: Some(30),
                },
                expect_exists: true,
            },
            Condition::StateQuery {
                filter: StateQueryFilter {
                    tool: "Bash".to_string(),
                    command_contains: Some("npm run lint".to_string()),
                    result: Some("success".to_string()),
                    within_minutes: Some(30),
                },
                expect_exists: true,
            },
        ],
    };
    
    // Create session state with both successful runs
    let mut session_state = SessionState::new("test-session".to_string());
    
    // Add test run
    let mut test_input = HashMap::new();
    test_input.insert("command".to_string(), serde_json::json!("npm test"));
    session_state.add_entry(StateEntry::new_tool_usage(
        "Bash".to_string(),
        test_input,
        true,
        None,
        None,
    ));
    
    // Add lint run
    let mut lint_input = HashMap::new();
    lint_input.insert("command".to_string(), serde_json::json!("npm run lint"));
    session_state.add_entry(StateEntry::new_tool_usage(
        "Bash".to_string(),
        lint_input,
        true,
        None,
        None,
    ));
    
    // Create evaluation context
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::current_dir().unwrap(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: Some(session_state),
        prompt: Some("Ready to commit".to_string()),
    };
    
    // Should match since both conditions are met
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

/// Test StateQuery for file edits
#[test]
fn test_state_query_file_edits() {
    let mut evaluator = ConditionEvaluator::new();
    
    // Create condition: check if README was edited recently
    let condition = Condition::StateQuery {
        filter: StateQueryFilter {
            tool: "Edit".to_string(),
            command_contains: None, // Edit doesn't have commands
            result: Some("success".to_string()),
            within_minutes: Some(10),
        },
        expect_exists: true,
    };
    
    // Create session state with Edit tool usage
    let mut session_state = SessionState::new("test-session".to_string());
    let mut edit_input = HashMap::new();
    edit_input.insert("file_path".to_string(), serde_json::json!("README.md"));
    edit_input.insert("old_string".to_string(), serde_json::json!("old content"));
    edit_input.insert("new_string".to_string(), serde_json::json!("new content"));
    
    session_state.add_entry(StateEntry::new_tool_usage(
        "Edit".to_string(),
        edit_input,
        true,
        None,
        Some(500),
    ));
    
    // Create evaluation context
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::env::current_dir().unwrap(),
        env_vars: HashMap::new(),
        timestamp: chrono::Utc::now(),
        full_session_state: Some(session_state),
        prompt: Some("Update docs".to_string()),
    };
    
    // Should match
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
}