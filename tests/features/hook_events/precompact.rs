use crate::common::event_factory::EventFactory;
use cupcake::cli::commands::run::EngineRunner;
use cupcake::config::actions::Action;
use cupcake::config::conditions::Condition;
use cupcake::config::types::{ComposedPolicy, HookEventType};
use cupcake::engine::events::AgentEvent;
use cupcake::engine::response::EngineDecision;

#[test]
fn test_precompact_manual_trigger_injects_instructions() {
    // Create PreCompact event with manual trigger
    let hook_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger_manual()
        .custom_instructions("Keep all technical details")
        .build();

    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());

    // Create a policy that injects instructions for manual compaction
    let policy = ComposedPolicy {
        name: "PreCompact Manual Instructions".to_string(),
        description: Some("Inject preservation instructions for manual compaction".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![
            Condition::Match {
                field: "event_type".to_string(),
                value: "PreCompact".to_string(),
            },
            Condition::Match {
                field: "trigger".to_string(),
                value: "manual".to_string(),
            },
        ],
        action: Action::inject_context("Preserve all TODO comments and FIXME markers."),
    };

    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(&[policy], &agent_event).unwrap();

    // Should allow and inject context
    assert!(matches!(
        result.final_decision,
        EngineDecision::Allow { .. }
    ));
    assert_eq!(result.context_to_inject.len(), 1);
    assert_eq!(
        result.context_to_inject[0],
        "Preserve all TODO comments and FIXME markers."
    );
}

#[test]
fn test_precompact_auto_trigger_different_instructions() {
    // Create PreCompact event with auto trigger
    let hook_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger_auto()
        .build();

    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());

    // Create a policy that injects different instructions for auto compaction
    let policy = ComposedPolicy {
        name: "PreCompact Auto Instructions".to_string(),
        description: Some(
            "Inject stronger preservation instructions for auto compaction".to_string(),
        ),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![
            Condition::Match {
                field: "event_type".to_string(),
                value: "PreCompact".to_string(),
            },
            Condition::Match {
                field: "trigger".to_string(),
                value: "auto".to_string(),
            },
        ],
        action: Action::inject_context(
            "CRITICAL: Preserve ALL ticket numbers, TODO comments, and implementation details!",
        ),
    };

    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(&[policy], &agent_event).unwrap();

    // Should allow and inject context
    assert!(matches!(
        result.final_decision,
        EngineDecision::Allow { .. }
    ));
    assert_eq!(result.context_to_inject.len(), 1);
    assert_eq!(
        result.context_to_inject[0],
        "CRITICAL: Preserve ALL ticket numbers, TODO comments, and implementation details!"
    );
}

#[test]
fn test_precompact_multiple_policies_combine_instructions() {
    // Create PreCompact event
    let hook_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger_manual()
        .custom_instructions("User instructions")
        .build();

    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());

    // Create multiple policies that all inject instructions
    let policy1 = ComposedPolicy {
        name: "PreCompact Policy 1".to_string(),
        description: Some("First policy".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![Condition::Match {
            field: "event_type".to_string(),
            value: "PreCompact".to_string(),
        }],
        action: Action::inject_context("Instructions from Policy 1"),
    };

    let policy2 = ComposedPolicy {
        name: "PreCompact Policy 2".to_string(),
        description: Some("Second policy".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![Condition::Match {
            field: "event_type".to_string(),
            value: "PreCompact".to_string(),
        }],
        action: Action::inject_context("Instructions from Policy 2"),
    };

    // Run the engine with both policies
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(&[policy1, policy2], &agent_event).unwrap();

    // Should allow and inject all contexts
    assert!(matches!(
        result.final_decision,
        EngineDecision::Allow { .. }
    ));
    assert_eq!(result.context_to_inject.len(), 2);
    assert!(result
        .context_to_inject
        .contains(&"Instructions from Policy 1".to_string()));
    assert!(result
        .context_to_inject
        .contains(&"Instructions from Policy 2".to_string()));
}

#[test]
fn test_precompact_block_compaction() {
    // Create PreCompact event
    let hook_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger_auto()
        .build();

    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());

    // Create a policy that blocks auto compaction
    let policy = ComposedPolicy {
        name: "Block Auto Compaction".to_string(),
        description: Some("Prevent automatic compaction".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![
            Condition::Match {
                field: "event_type".to_string(),
                value: "PreCompact".to_string(),
            },
            Condition::Match {
                field: "trigger".to_string(),
                value: "auto".to_string(),
            },
        ],
        action: Action::block_with_feedback("Auto compaction is disabled by policy"),
    };

    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(&[policy], &agent_event).unwrap();

    // Should block
    assert!(matches!(
        result.final_decision,
        EngineDecision::Block { .. }
    ));
    assert!(result.context_to_inject.is_empty());
}

#[test]
fn test_precompact_with_custom_instructions_condition() {
    // Create PreCompact event with custom instructions
    let hook_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger_manual()
        .custom_instructions("Keep all security-related discussions")
        .build();

    let agent_event = AgentEvent::ClaudeCode(hook_event.clone());

    // Create a policy that matches on custom instructions content
    let policy = ComposedPolicy {
        name: "Security Preservation".to_string(),
        description: Some("Extra preservation for security content".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![
            Condition::Match {
                field: "event_type".to_string(),
                value: "PreCompact".to_string(),
            },
            Condition::Pattern {
                field: "custom_instructions".to_string(),
                regex: r"security".to_string(),
            },
        ],
        action: Action::inject_context("IMPORTANT: Preserve all security discussions, vulnerability reports, and threat models in detail."),
    };

    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(&[policy], &agent_event).unwrap();

    // Should allow and inject context
    assert!(matches!(
        result.final_decision,
        EngineDecision::Allow { .. }
    ));
    assert_eq!(result.context_to_inject.len(), 1);
    assert!(result.context_to_inject[0].contains("security discussions"));
}
