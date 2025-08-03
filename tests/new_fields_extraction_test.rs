use chrono::Utc;
use cupcake::engine::conditions::{ConditionEvaluator, ConditionResult, EvaluationContext};
use cupcake::config::conditions::Condition;
use cupcake::engine::events::{CommonEventData, HookEvent, CompactTrigger};
use cupcake::cli::commands::run::ExecutionContextBuilder;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_precompact_trigger_extraction() {
    let builder = ExecutionContextBuilder::new();
    
    // Test manual trigger
    let manual_event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Manual,
        custom_instructions: Some("Preserve all TODO comments".to_string()),
    };
    
    let context = builder.build_evaluation_context(&manual_event);
    assert_eq!(context.trigger, Some("manual".to_string()));
    assert_eq!(context.custom_instructions, Some("Preserve all TODO comments".to_string()));
    
    // Test auto trigger with no custom instructions
    let auto_event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Auto,
        custom_instructions: None,
    };
    
    let context = builder.build_evaluation_context(&auto_event);
    assert_eq!(context.trigger, Some("auto".to_string()));
    assert_eq!(context.custom_instructions, None);
}

#[test]
fn test_precompact_condition_evaluation() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();
    
    let event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Manual,
        custom_instructions: Some("Keep ticket numbers".to_string()),
    };
    
    let context = builder.build_evaluation_context(&event);
    
    // Test trigger matching
    let trigger_condition = Condition::Match {
        field: "trigger".to_string(),
        value: "manual".to_string(),
    };
    let result = evaluator.evaluate(&trigger_condition, &context);
    assert_eq!(result, ConditionResult::Match);
    
    // Test custom instructions pattern matching
    let instructions_condition = Condition::Pattern {
        field: "custom_instructions".to_string(),
        regex: r"ticket.*numbers".to_string(),
    };
    let result = evaluator.evaluate(&instructions_condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

#[test]
fn test_posttooluse_tool_response_extraction() {
    let builder = ExecutionContextBuilder::new();
    
    let event = HookEvent::PostToolUse {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        tool_name: "Write".to_string(),
        tool_input: json!({
            "file_path": "/app/file.txt",
            "content": "Hello world"
        }),
        tool_response: json!({
            "success": true,
            "filePath": "/app/file.txt"
        }),
    };
    
    let context = builder.build_evaluation_context(&event);
    assert!(context.tool_response.is_some());
    
    // Verify tool_response content
    let tool_response = context.tool_response.unwrap();
    assert_eq!(tool_response["success"], json!(true));
    assert_eq!(tool_response["filePath"], json!("/app/file.txt"));
}

#[test]
fn test_posttooluse_tool_response_condition_evaluation() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();
    
    let event = HookEvent::PostToolUse {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        tool_name: "Bash".to_string(),
        tool_input: json!({
            "command": "echo 'test'"
        }),
        tool_response: json!({
            "success": true,
            "stdout": "test\n",
            "exit_code": 0
        }),
    };
    
    let context = builder.build_evaluation_context(&event);
    
    // Test tool_response.success field access
    let success_condition = Condition::Match {
        field: "tool_response.success".to_string(),
        value: "true".to_string(),
    };
    let result = evaluator.evaluate(&success_condition, &context);
    assert_eq!(result, ConditionResult::Match);
    
    // Test tool_response.exit_code field access
    let exit_code_condition = Condition::Match {
        field: "tool_response.exit_code".to_string(),
        value: "0".to_string(),
    };
    let result = evaluator.evaluate(&exit_code_condition, &context);
    assert_eq!(result, ConditionResult::Match);
    
    // Test tool_response.stdout pattern matching
    let stdout_condition = Condition::Pattern {
        field: "tool_response.stdout".to_string(),
        regex: r"test".to_string(),
    };
    let result = evaluator.evaluate(&stdout_condition, &context);
    assert_eq!(result, ConditionResult::Match);
}

#[test]
fn test_stop_hook_active_extraction() {
    let builder = ExecutionContextBuilder::new();
    
    // Test Stop event with stop_hook_active = true
    let stop_event = HookEvent::Stop {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        stop_hook_active: true,
    };
    
    let context = builder.build_evaluation_context(&stop_event);
    assert_eq!(context.stop_hook_active, Some(true));
    
    // Test SubagentStop event with stop_hook_active = false
    let subagent_stop_event = HookEvent::SubagentStop {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        stop_hook_active: false,
    };
    
    let context = builder.build_evaluation_context(&subagent_stop_event);
    assert_eq!(context.stop_hook_active, Some(false));
}

#[test]
fn test_stop_hook_active_condition_evaluation() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();
    
    let event = HookEvent::Stop {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        stop_hook_active: true,
    };
    
    let context = builder.build_evaluation_context(&event);
    
    // Test stop_hook_active condition evaluation
    let condition = Condition::Match {
        field: "stop_hook_active".to_string(),
        value: "true".to_string(),
    };
    let result = evaluator.evaluate(&condition, &context);
    assert_eq!(result, ConditionResult::Match);
    
    // Test inverse condition
    let false_condition = Condition::Match {
        field: "stop_hook_active".to_string(),
        value: "false".to_string(),
    };
    let result = evaluator.evaluate(&false_condition, &context);
    assert_eq!(result, ConditionResult::NoMatch);
}

#[test]
fn test_field_extraction_for_non_applicable_events() {
    let builder = ExecutionContextBuilder::new();
    
    // Test that PreToolUse doesn't have the new fields
    let pretooluse_event = HookEvent::PreToolUse {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        tool_name: "Bash".to_string(),
        tool_input: json!({"command": "ls"}),
    };
    
    let context = builder.build_evaluation_context(&pretooluse_event);
    assert_eq!(context.tool_response, None);
    assert_eq!(context.stop_hook_active, None);
    assert_eq!(context.trigger, None);
    assert_eq!(context.custom_instructions, None);
    
    // Test that UserPromptSubmit doesn't have the new fields
    let user_prompt_event = HookEvent::UserPromptSubmit {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/home/user".to_string(),
        },
        prompt: "Hello".to_string(),
    };
    
    let context = builder.build_evaluation_context(&user_prompt_event);
    assert_eq!(context.tool_response, None);
    assert_eq!(context.stop_hook_active, None);
    assert_eq!(context.trigger, None);
    assert_eq!(context.custom_instructions, None);
}

#[test]
fn test_enterprise_use_case_precompact_auto_policy() {
    // Test the use case mentioned in the review: "If PreCompact is auto, inject preservation instruction"
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();
    
    let auto_compact_event = HookEvent::PreCompact {
        common: CommonEventData {
            session_id: "enterprise-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/enterprise/project".to_string(),
        },
        trigger: CompactTrigger::Auto,
        custom_instructions: None,
    };
    
    let context = builder.build_evaluation_context(&auto_compact_event);
    
    // Condition: trigger == "auto"
    let auto_trigger_condition = Condition::Match {
        field: "trigger".to_string(),
        value: "auto".to_string(),
    };
    let result = evaluator.evaluate(&auto_trigger_condition, &context);
    assert_eq!(result, ConditionResult::Match);
    
    // This proves the policy engine can now differentiate between manual and auto compaction
    // and could inject "Preserve all TODO comments and ticket numbers" for auto compactions
}

#[test]
fn test_enterprise_use_case_posttooluse_validation() {
    // Test the use case: "Check tool_response from Write to ensure success: true"
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();
    
    let failed_write_event = HookEvent::PostToolUse {
        common: CommonEventData {
            session_id: "enterprise-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/enterprise/project".to_string(),
        },
        tool_name: "Write".to_string(),
        tool_input: json!({
            "file_path": "/protected/file.txt",
            "content": "sensitive data"
        }),
        tool_response: json!({
            "success": false,
            "error": "Permission denied"
        }),
    };
    
    let context = builder.build_evaluation_context(&failed_write_event);
    
    // Condition: tool_response.success == "true"
    let success_condition = Condition::Match {
        field: "tool_response.success".to_string(),
        value: "true".to_string(),
    };
    let result = evaluator.evaluate(&success_condition, &context);
    assert_eq!(result, ConditionResult::NoMatch); // Failed write should not match
    
    // Condition: tool_response.success == "false" 
    let failure_condition = Condition::Match {
        field: "tool_response.success".to_string(),
        value: "false".to_string(),
    };
    let result = evaluator.evaluate(&failure_condition, &context);
    assert_eq!(result, ConditionResult::Match); // This would trigger a policy response
}

#[test]
fn test_enterprise_use_case_stop_loop_prevention() {
    // Test the use case: "Prevent infinite loops by checking stop_hook_active"
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();
    
    let stop_event_in_loop = HookEvent::Stop {
        common: CommonEventData {
            session_id: "enterprise-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            cwd: "/enterprise/project".to_string(),
        },
        stop_hook_active: true, // Already in a stop hook loop
    };
    
    let context = builder.build_evaluation_context(&stop_event_in_loop);
    
    // Condition: stop_hook_active == "false" (safe to continue)
    let safe_condition = Condition::Match {
        field: "stop_hook_active".to_string(),
        value: "false".to_string(),
    };
    let result = evaluator.evaluate(&safe_condition, &context);
    assert_eq!(result, ConditionResult::NoMatch); // Already in loop, not safe
    
    // Condition: stop_hook_active == "true" (already in loop, don't trigger more)
    let loop_condition = Condition::Match {
        field: "stop_hook_active".to_string(),
        value: "true".to_string(),
    };
    let result = evaluator.evaluate(&loop_condition, &context);
    assert_eq!(result, ConditionResult::Match); // In loop, policy can detect and avoid triggering
}