mod common;
use common::event_factory::EventFactory;
use cupcake::config::actions::Action;
use cupcake::config::conditions::Condition;
use cupcake::config::loader::PolicyLoader;
use cupcake::config::types::{ComposedPolicy, HookEventType};
use cupcake::engine::evaluation::PolicyEvaluator;
use cupcake::engine::events::AgentEvent;
use cupcake::cli::commands::run::ExecutionContextBuilder;
use std::fs;
use tempfile::tempdir;

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
            suppress_output: false,
        },
    };

    // Create UserPromptSubmit event
    let hook_event = EventFactory::user_prompt_submit()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.md")
        .cwd("/home/user")
        .prompt("My API key is sk-1234567890abcdef123456")
        .build();

    // Create evaluation context
    let context_builder = ExecutionContextBuilder::new();
    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());
    let context = context_builder.build_evaluation_context(&agent_event);

    // Evaluate policy
    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&[policy], &hook_event, &context);

    // Should match and trigger block action
    assert!(result.is_ok());
    let eval_result = result.unwrap();
    assert_eq!(eval_result.matched_policies.len(), 1);
    assert_eq!(eval_result.decision, cupcake::engine::response::EngineDecision::Block {
        feedback: "Detected API key in prompt!".to_string()
    });
}

#[test]
fn test_empty_matcher_with_pre_tool_use() {
    // Create policy with empty matcher for PreToolUse
    let policy = ComposedPolicy {
        name: "Block rm -rf".to_string(),
        description: Some("Prevent dangerous rm commands".to_string()),
        hook_event: HookEventType::PreToolUse,
        matcher: "".to_string(), // Empty matcher
        conditions: vec![
            Condition::Match {
                field: "tool_name".to_string(),
                value: "Bash".to_string(),
            },
            Condition::Pattern {
                field: "tool_input.command".to_string(),
                regex: r"rm\s+-rf\s+/".to_string(),
            },
        ],
        action: Action::BlockWithFeedback {
            feedback_message: "Dangerous command blocked!".to_string(),
            include_context: false,
            suppress_output: false,
        },
    };

    // Create PreToolUse event with dangerous command
    let hook_event = EventFactory::pre_tool_use()
        .session_id("test-session")
        .tool_name("Bash")
        .tool_input_command("rm -rf /")
        .build();

    let context_builder = ExecutionContextBuilder::new();
    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());
    let context = context_builder.build_evaluation_context(&agent_event);

    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&[policy], &hook_event, &context);

    assert!(result.is_ok());
    let eval_result = result.unwrap();
    assert_eq!(eval_result.matched_policies.len(), 1);
    assert_eq!(eval_result.decision, cupcake::engine::response::EngineDecision::Block {
        feedback: "Dangerous command blocked!".to_string()
    });
}

#[test]
fn test_empty_matcher_session_start() {
    // Create policy for SessionStart with empty matcher
    let policy = ComposedPolicy {
        name: "Log all sessions".to_string(),
        description: Some("Log when sessions start".to_string()),
        hook_event: HookEventType::SessionStart,
        matcher: "".to_string(),
        conditions: vec![], // No conditions, matches all SessionStart events
        action: Action::ProvideFeedback {
            message: "Session started: {{session_id}}".to_string(),
            include_context: false,
            suppress_output: false,
        },
    };

    let hook_event = EventFactory::session_start()
        .session_id("test-123")
        .source_startup()
        .build();

    let context_builder = ExecutionContextBuilder::new();
    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());
    let context = context_builder.build_evaluation_context(&agent_event);

    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&[policy], &hook_event, &context);

    assert!(result.is_ok());
    let eval_result = result.unwrap();
    assert_eq!(eval_result.matched_policies.len(), 1);
}

#[test]
fn test_empty_matcher_from_yaml() {
    let dir = tempdir().unwrap();
    let policy_path = dir.path().join("policy.yaml");
    
    // YAML with empty matcher string
    let yaml_content = r#"
rules:
  - name: "Block sensitive files"
    description: "Prevent access to sensitive files"
    hook_event: PreToolUse
    matcher: ""  # Empty matcher
    if:
      match:
        field: tool_name
        value: Read
    pattern:
      field: tool_input.file_path
      regex: "(passwords|secrets|keys)\\.txt"
    then:
      actions:
        - block_with_feedback:
            feedback_message: "Access to sensitive files is not allowed"
"#;

    fs::write(&policy_path, yaml_content).unwrap();

    let mut loader = PolicyLoader::new();
    let config = loader.load_configuration(&policy_path).unwrap();
    
    assert_eq!(config.policies.len(), 1);
    assert_eq!(config.policies[0].matcher, "");
    
    // Test with actual event
    let hook_event = EventFactory::pre_tool_use()
        .tool_name("Read")
        .tool_input_file_path("/etc/passwords.txt")
        .build();

    let context_builder = ExecutionContextBuilder::new();
    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());
    let context = context_builder.build_evaluation_context(&agent_event);

    let mut evaluator = PolicyEvaluator::new();
    let result = evaluator.evaluate(&config.policies, &hook_event, &context);

    assert!(result.is_ok());
    let eval_result = result.unwrap();
    assert_eq!(eval_result.matched_policies.len(), 1);
}

#[test]
fn test_empty_matcher_all_hook_types() {
    // Test that empty matcher works for all hook event types
    let hook_types = vec![
        HookEventType::PreToolUse,
        HookEventType::PostToolUse,
        HookEventType::UserPromptSubmit,
        HookEventType::SessionStart,
        HookEventType::Stop,
        HookEventType::SubagentStop,
        HookEventType::PreCompact,
        HookEventType::Notification,
    ];

    for hook_type in hook_types {
        let policy = ComposedPolicy {
            name: format!("Test policy for {:?}", hook_type),
            description: None,
            hook_event: hook_type.clone(),
            matcher: "".to_string(),
            conditions: vec![],
            action: Action::ProvideFeedback {
                message: "Matched".to_string(),
                include_context: false,
                suppress_output: false,
            },
        };

        // Create appropriate event for each type
        let hook_event = match hook_type {
            HookEventType::PreToolUse => EventFactory::pre_tool_use().tool_name("Test").build(),
            HookEventType::PostToolUse => EventFactory::post_tool_use().tool_name("Test").build(),
            HookEventType::UserPromptSubmit => EventFactory::user_prompt_submit().prompt("Test").build(),
            HookEventType::SessionStart => EventFactory::session_start().build(),
            HookEventType::Stop => EventFactory::stop().build(),
            HookEventType::SubagentStop => EventFactory::subagent_stop().build(),
            HookEventType::PreCompact => EventFactory::pre_compact().build(),
            HookEventType::Notification => EventFactory::notification().message("Test").build(),
        };

        let context_builder = ExecutionContextBuilder::new();
        let agent_event = AgentEvent::ClaudeCode(hook_event.clone());
        let context = context_builder.build_evaluation_context(&agent_event);

        let mut evaluator = PolicyEvaluator::new();
        let result = evaluator.evaluate(&[policy], &hook_event, &context);

        assert!(result.is_ok(), "Failed for hook type: {:?}", hook_type);
        let eval_result = result.unwrap();
        assert_eq!(eval_result.matched_policies.len(), 1, "No match for hook type: {:?}", hook_type);
    }
}