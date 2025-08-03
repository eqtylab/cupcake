use cupcake::config::conditions::Condition;
use cupcake::config::actions::Action;
use cupcake::config::types::{ComposedPolicy, HookEventType};
use cupcake::engine::events::{CommonEventData, HookEvent, CompactTrigger};
use cupcake::cli::commands::run::{ExecutionContextBuilder, EngineRunner};
use cupcake::engine::response::EngineDecision;

#[test]
fn test_precompact_manual_trigger_injects_instructions() {
    let builder = ExecutionContextBuilder::new();
    
    // Create PreCompact event with manual trigger
    let event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Manual,
        custom_instructions: Some("Keep all technical details".to_string()),
    };
    
    let eval_context = builder.build_evaluation_context(&event);
    let action_context = builder.build_action_context(&event);
    
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
        action:
            Action::InjectContext {
                context: Some("Preserve all TODO comments and FIXME markers.".to_string()),
                from_command: None,
                suppress_output: false,
                use_stdout: false,
            },
    };
    
    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(
        &[policy],
        &event,
        &eval_context,
        &action_context,
    ).unwrap();
    
    // Should allow and inject context
    assert!(matches!(result.final_decision, EngineDecision::Allow { .. }));
    assert_eq!(result.context_to_inject.len(), 1);
    assert_eq!(result.context_to_inject[0], "Preserve all TODO comments and FIXME markers.");
}

#[test]
fn test_precompact_auto_trigger_different_instructions() {
    let builder = ExecutionContextBuilder::new();
    
    // Create PreCompact event with auto trigger
    let event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Auto,
        custom_instructions: None,
    };
    
    let eval_context = builder.build_evaluation_context(&event);
    let action_context = builder.build_action_context(&event);
    
    // Create a policy that injects different instructions for auto compaction
    let policy = ComposedPolicy {
        name: "PreCompact Auto Instructions".to_string(),
        description: Some("Inject stronger preservation instructions for auto compaction".to_string()),
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
        action:
            Action::InjectContext {
                context: Some("CRITICAL: Preserve ALL ticket numbers, TODO comments, and implementation details!".to_string()),
                from_command: None,
                suppress_output: false,
                use_stdout: false,
            },
    };
    
    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(
        &[policy],
        &event,
        &eval_context,
        &action_context,
    ).unwrap();
    
    // Should allow and inject context
    assert!(matches!(result.final_decision, EngineDecision::Allow { .. }));
    assert_eq!(result.context_to_inject.len(), 1);
    assert_eq!(result.context_to_inject[0], "CRITICAL: Preserve ALL ticket numbers, TODO comments, and implementation details!");
}

#[test]
fn test_precompact_multiple_policies_combine_instructions() {
    let builder = ExecutionContextBuilder::new();
    
    // Create PreCompact event
    let event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Manual,
        custom_instructions: Some("User instructions".to_string()),
    };
    
    let eval_context = builder.build_evaluation_context(&event);
    let action_context = builder.build_action_context(&event);
    
    // Create multiple policies that all inject instructions
    let policy1 = ComposedPolicy {
        name: "PreCompact Policy 1".to_string(),
        description: Some("First policy".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![
            Condition::Match {
                field: "event_type".to_string(),
                value: "PreCompact".to_string(),
            },
        ],
        action:
            Action::InjectContext {
                context: Some("Instructions from Policy 1".to_string()),
                from_command: None,
                suppress_output: false,
                use_stdout: false,
            },
    };
    
    let policy2 = ComposedPolicy {
        name: "PreCompact Policy 2".to_string(),
        description: Some("Second policy".to_string()),
        hook_event: HookEventType::PreCompact,
        matcher: "*".to_string(),
        conditions: vec![
            Condition::Match {
                field: "event_type".to_string(),
                value: "PreCompact".to_string(),
            },
        ],
        action:
            Action::InjectContext {
                context: Some("Instructions from Policy 2".to_string()),
                from_command: None,
                suppress_output: false,
                use_stdout: false,
            },
    };
    
    // Run the engine with both policies
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(
        &[policy1, policy2],
        &event,
        &eval_context,
        &action_context,
    ).unwrap();
    
    // Should allow and inject all contexts
    assert!(matches!(result.final_decision, EngineDecision::Allow { .. }));
    assert_eq!(result.context_to_inject.len(), 2);
    assert!(result.context_to_inject.contains(&"Instructions from Policy 1".to_string()));
    assert!(result.context_to_inject.contains(&"Instructions from Policy 2".to_string()));
}

#[test]
fn test_precompact_block_compaction() {
    let builder = ExecutionContextBuilder::new();
    
    // Create PreCompact event
    let event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Auto,
        custom_instructions: None,
    };
    
    let eval_context = builder.build_evaluation_context(&event);
    let action_context = builder.build_action_context(&event);
    
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
        action:
            Action::BlockWithFeedback {
                feedback_message: "Auto compaction is disabled by policy".to_string(),
                include_context: false,
                suppress_output: false,
            },
    };
    
    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(
        &[policy],
        &event,
        &eval_context,
        &action_context,
    ).unwrap();
    
    // Should block
    assert!(matches!(result.final_decision, EngineDecision::Block { .. }));
    assert!(result.context_to_inject.is_empty());
}

#[test]
fn test_precompact_with_custom_instructions_condition() {
    let builder = ExecutionContextBuilder::new();
    
    // Create PreCompact event with custom instructions
    let event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Manual,
        custom_instructions: Some("Keep all security-related discussions".to_string()),
    };
    
    let eval_context = builder.build_evaluation_context(&event);
    let action_context = builder.build_action_context(&event);
    
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
        action:
            Action::InjectContext {
                context: Some("IMPORTANT: Preserve all security discussions, vulnerability reports, and threat models in detail.".to_string()),
                from_command: None,
                suppress_output: false,
                use_stdout: false,
            },
    };
    
    // Run the engine
    let mut engine = EngineRunner::new(Default::default(), false);
    let result = engine.run(
        &[policy],
        &event,
        &eval_context,
        &action_context,
    ).unwrap();
    
    // Should allow and inject context
    assert!(matches!(result.final_decision, EngineDecision::Allow { .. }));
    assert_eq!(result.context_to_inject.len(), 1);
    assert!(result.context_to_inject[0].contains("security discussions"));
}