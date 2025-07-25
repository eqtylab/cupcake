use cupcake::config::types::{ComposedPolicy, HookEventType};
use cupcake::config::loader::PolicyLoader;
use cupcake::engine::evaluation::PolicyEvaluator;
use cupcake::engine::conditions::EvaluationContext;
use cupcake::engine::events::{CommonEventData, HookEvent};
use cupcake::config::conditions::Condition;
use cupcake::config::actions::Action;
use std::collections::HashMap;
use tempfile::tempdir;
use std::fs;
use chrono::Utc;

#[test]
fn test_empty_matcher_for_user_prompt_submit() {
    // Create a policy with empty matcher for UserPromptSubmit
    let policy = ComposedPolicy {
        name: "Block sensitive prompts".to_string(),
        description: Some("Block prompts containing API keys".to_string()),
        hook_event: HookEventType::UserPromptSubmit,
        matcher: "".to_string(), // Empty string matcher
        conditions: vec![Condition::Pattern {
            field: "prompt".to_string(),
            regex: r"sk-[a-zA-Z0-9]{16,}".to_string(),
        }],
        action: Action::BlockWithFeedback {
            feedback_message: "Detected API key in prompt!".to_string(),
            include_context: false,
        },
    };

    // Create UserPromptSubmit event
    let hook_event = HookEvent::UserPromptSubmit {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.md".to_string(),
            cwd: "/home/user".to_string(),
        },
        prompt: "My API key is sk-1234567890abcdef123456".to_string(),
    };

    // Create evaluation context
    let context = EvaluationContext {
        event_type: "UserPromptSubmit".to_string(),
        tool_name: String::new(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::path::PathBuf::from("/home/user"),
        env_vars: HashMap::new(),
        timestamp: Utc::now(),
        full_session_state: None,
        prompt: Some("My API key is sk-1234567890abcdef123456".to_string()),
    };

    // Evaluate the policy
    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&[policy], &hook_event, &context).unwrap();

    // Should block due to API key detection
    match result.decision {
        cupcake::engine::response::EngineDecision::Block { feedback } => {
            assert!(feedback.contains("Detected API key"));
        }
        _ => panic!("Expected block decision"),
    }
}

#[test]
fn test_empty_matcher_only_matches_non_tool_events() {
    // Create a policy with empty matcher
    let policy = ComposedPolicy {
        name: "Test empty matcher".to_string(),
        description: None,
        hook_event: HookEventType::PreToolUse,
        matcher: "".to_string(), // Empty string matcher
        conditions: vec![Condition::Match {
            field: "event_type".to_string(),
            value: "PreToolUse".to_string(),
        }],
        action: Action::ProvideFeedback {
            message: "Should not match".to_string(),
            include_context: false,
        },
    };

    // Create PreToolUse event (has tool name)
    let hook_event = HookEvent::PreToolUse {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.md".to_string(),
            cwd: "/home/user".to_string(),
        },
        tool_name: "Bash".to_string(),
        tool_input: serde_json::json!({"command": "ls"}),
    };

    // Create evaluation context
    let context = EvaluationContext {
        event_type: "PreToolUse".to_string(),
        tool_name: "Bash".to_string(),
        tool_input: HashMap::new(),
        session_id: "test-session".to_string(),
        current_dir: std::path::PathBuf::from("/home/user"),
        env_vars: HashMap::new(),
        timestamp: Utc::now(),
        full_session_state: None,
        prompt: None,
    };

    // Evaluate the policy
    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&[policy], &hook_event, &context).unwrap();

    // Should not match because PreToolUse has a tool name but policy has empty matcher
    assert_eq!(result.matched_policies.len(), 0);
    assert_eq!(result.feedback_messages.len(), 0);
}

#[test]
fn test_yaml_loading_with_empty_matcher() {
    let temp_dir = tempdir().unwrap();
    let guardrails_dir = temp_dir.path().join("guardrails");
    let policies_dir = guardrails_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    // Create root config
    let root_config = r#"
settings:
  timeout_ms: 5000
  debug: false

imports:
  - policies/*.yaml
"#;
    fs::write(guardrails_dir.join("cupcake.yaml"), root_config).unwrap();

    // Create policy file with empty string matcher
    let policy_yaml = r#"
UserPromptSubmit:
  "":  # Empty string matcher for non-tool events
    - name: "Check for secrets"
      description: "Block prompts containing secrets"
      conditions:
        - type: pattern
          field: prompt
          regex: "(password|secret|key)\\s*[:=]\\s*\\S+"
      action:
        type: block_with_feedback
        feedback_message: "Detected potential secret in prompt"
        include_context: false
"#;
    fs::write(policies_dir.join("prompt-policies.yaml"), policy_yaml).unwrap();

    // Load the configuration
    let mut loader = PolicyLoader::new();
    let loaded = loader.load_configuration_from_directory(temp_dir.path()).unwrap();

    // Verify the policy was loaded correctly
    assert_eq!(loaded.policies.len(), 1);
    let policy = &loaded.policies[0];
    assert_eq!(policy.name, "Check for secrets");
    assert_eq!(policy.hook_event, HookEventType::UserPromptSubmit);
    assert_eq!(policy.matcher, "");
}