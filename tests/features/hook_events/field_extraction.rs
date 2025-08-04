use crate::common::event_factory::EventFactory;
use cupcake::cli::commands::run::ExecutionContextBuilder;
use cupcake::config::conditions::Condition;
use cupcake::engine::conditions::{ConditionEvaluator, ConditionResult};
use cupcake::engine::events::AgentEvent;
use serde_json::json;

#[test]
fn test_precompact_trigger_extraction() {
    let builder = ExecutionContextBuilder::new();

    // Test manual trigger
    let manual_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger("manual")
        .custom_instructions("Preserve all TODO comments")
        .build();

    let agent_event = AgentEvent::ClaudeCode(manual_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.trigger, Some("manual".to_string()));
    assert_eq!(
        context.custom_instructions,
        Some("Preserve all TODO comments".to_string())
    );

    // Test auto trigger with no custom instructions
    let auto_event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/tmp/transcript.jsonl")
        .cwd("/home/user")
        .trigger("auto")
        .build();

    let agent_event = AgentEvent::ClaudeCode(auto_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.trigger, Some("auto".to_string()));
    assert_eq!(context.custom_instructions, None);
}

#[test]
fn test_precompact_condition_evaluation() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();

    let event = EventFactory::pre_compact()
        .session_id("test-session")
        .trigger_manual()
        .custom_instructions("Important context")
        .build();

    let agent_event = AgentEvent::ClaudeCode(event);
    let context = builder.build_evaluation_context(&agent_event);

    // Test trigger field condition
    let trigger_condition = Condition::Match {
        field: "trigger".to_string(),
        value: "manual".to_string(),
    };

    let result = evaluator.evaluate(&trigger_condition, &context);
    assert!(matches!(result, ConditionResult::Match));

    // Test custom_instructions presence
    let instruction_pattern = Condition::Pattern {
        field: "custom_instructions".to_string(),
        regex: "^Important.*".to_string(),
    };

    let result = evaluator.evaluate(&instruction_pattern, &context);
    assert!(matches!(result, ConditionResult::Match));
}

#[test]
fn test_stop_hook_active_extraction() {
    let builder = ExecutionContextBuilder::new();

    // Test Stop event with hook active
    let stop_event = EventFactory::stop()
        .session_id("test-session")
        .stop_hook_active(true)
        .build();

    let agent_event = AgentEvent::ClaudeCode(stop_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.stop_hook_active, Some(true));

    // Test SubagentStop event with hook inactive
    let subagent_event = EventFactory::subagent_stop()
        .session_id("test-session")
        .stop_hook_active(false)
        .build();

    let agent_event = AgentEvent::ClaudeCode(subagent_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.stop_hook_active, Some(false));
}

#[test]
fn test_session_source_extraction() {
    let builder = ExecutionContextBuilder::new();

    // Test startup source
    let startup_event = EventFactory::session_start()
        .session_id("test-session")
        .source_startup()
        .build();

    let agent_event = AgentEvent::ClaudeCode(startup_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.source, Some("startup".to_string()));

    // Test resume source
    let resume_event = EventFactory::session_start()
        .session_id("test-session")
        .source_resume()
        .build();

    let agent_event = AgentEvent::ClaudeCode(resume_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.source, Some("resume".to_string()));

    // Test clear source
    let clear_event = EventFactory::session_start()
        .session_id("test-session")
        .source_clear()
        .build();

    let agent_event = AgentEvent::ClaudeCode(clear_event);
    let context = builder.build_evaluation_context(&agent_event);
    assert_eq!(context.source, Some("clear".to_string()));
}

#[test]
fn test_tool_response_extraction() {
    let builder = ExecutionContextBuilder::new();

    // Test successful Write operation
    let write_event = EventFactory::post_tool_use()
        .session_id("test-session")
        .tool_name("Write")
        .tool_input(json!({
            "file_path": "/app/file.txt",
            "content": "Hello world"
        }))
        .tool_response_success(true, "File written successfully")
        .build();

    let agent_event = AgentEvent::ClaudeCode(write_event);
    let context = builder.build_evaluation_context(&agent_event);

    assert_eq!(context.tool_name, "Write");
    assert!(context.tool_response.is_some());

    let response = context.tool_response.unwrap();
    assert_eq!(response["success"], true);
    assert_eq!(response["output"], "File written successfully");
}

#[test]
fn test_tool_response_condition_evaluation() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();

    // Test successful tool execution
    let successful_event = EventFactory::post_tool_use()
        .session_id("test-session")
        .tool_name("Bash")
        .tool_input(json!({
            "command": "echo 'test'"
        }))
        .tool_response(json!({
            "success": true,
            "output": "test\n",
            "exit_code": 0
        }))
        .build();

    let agent_event = AgentEvent::ClaudeCode(successful_event);
    let context = builder.build_evaluation_context(&agent_event);

    // Test tool_response.success field
    let success_condition = Condition::Match {
        field: "tool_response.success".to_string(),
        value: "true".to_string(),
    };

    let result = evaluator.evaluate(&success_condition, &context);
    assert!(matches!(result, ConditionResult::Match));

    // Test tool_response.exit_code field
    let exit_code_condition = Condition::Match {
        field: "tool_response.exit_code".to_string(),
        value: "0".to_string(),
    };

    let result = evaluator.evaluate(&exit_code_condition, &context);
    assert!(matches!(result, ConditionResult::Match));
}

#[test]
fn test_enterprise_tool_response_pattern() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();

    // Test pattern matching in tool response for security scanning
    let scan_event = EventFactory::post_tool_use()
        .session_id("enterprise-session")
        .tool_name("SecurityScan")
        .tool_input(json!({
            "target": "/secure/zone"
        }))
        .tool_response(json!({
            "success": true,
            "findings": "No vulnerabilities detected",
            "severity": "low"
        }))
        .build();

    let agent_event = AgentEvent::ClaudeCode(scan_event);
    let context = builder.build_evaluation_context(&agent_event);

    // Test pattern in tool_response.findings
    let findings_pattern = Condition::Pattern {
        field: "tool_response.findings".to_string(),
        regex: "(?i)no vulnerabilities".to_string(),
    };

    let result = evaluator.evaluate(&findings_pattern, &context);
    assert!(matches!(result, ConditionResult::Match));
}

#[test]
fn test_complex_nested_response_extraction() {
    let builder = ExecutionContextBuilder::new();

    // Test deeply nested tool response
    let complex_event = EventFactory::post_tool_use()
        .session_id("complex-session")
        .tool_name("APICall")
        .tool_input(json!({
            "endpoint": "/api/v1/data",
            "method": "GET"
        }))
        .tool_response(json!({
            "success": true,
            "data": {
                "user": {
                    "id": 123,
                    "permissions": ["read", "write", "admin"]
                },
                "metadata": {
                    "timestamp": "2024-01-01T00:00:00Z",
                    "version": "1.0"
                }
            },
            "headers": {
                "content-type": "application/json",
                "x-rate-limit": "100"
            }
        }))
        .build();

    let agent_event = AgentEvent::ClaudeCode(complex_event);
    let context = builder.build_evaluation_context(&agent_event);

    let response = context.tool_response.as_ref().unwrap();
    assert_eq!(response["data"]["user"]["id"], 123);
    assert!(response["data"]["user"]["permissions"].is_array());
}

#[test]
fn test_failed_tool_response_handling() {
    let builder = ExecutionContextBuilder::new();

    // Test failed Write operation with permission error
    let failed_write_event = EventFactory::post_tool_use()
        .session_id("enterprise-session")
        .tool_name("Write")
        .tool_input(json!({
            "file_path": "/protected/file.txt",
            "content": "sensitive data"
        }))
        .tool_response(json!({
            "success": false,
            "error": "Permission denied"
        }))
        .build();

    let agent_event = AgentEvent::ClaudeCode(failed_write_event);
    let context = builder.build_evaluation_context(&agent_event);

    assert_eq!(context.tool_name, "Write");

    let response = context.tool_response.as_ref().unwrap();
    assert_eq!(response["success"], false);
    assert_eq!(response["error"], "Permission denied");
}

#[test]
fn test_stop_hook_condition_evaluation() {
    let mut evaluator = ConditionEvaluator::new();
    let builder = ExecutionContextBuilder::new();

    // Create Stop event with hook active (should allow stop to prevent infinite loop)
    let stop_event_in_loop = EventFactory::stop()
        .session_id("loop-detection")
        .stop_hook_active(true)
        .build();

    let agent_event = AgentEvent::ClaudeCode(stop_event_in_loop);
    let context = builder.build_evaluation_context(&agent_event);

    // Test condition: if stop_hook_active is true, allow the stop
    let stop_active_condition = Condition::Match {
        field: "stop_hook_active".to_string(),
        value: "true".to_string(),
    };

    let result = evaluator.evaluate(&stop_active_condition, &context);
    assert!(matches!(result, ConditionResult::Match));

    // This would be used in a policy to allow stop when hook is active:
    // if: { match: { field: "stop_hook_active", value: "true" } }
    // then: { actions: [{ allow: { reason: "Preventing infinite loop" } }] }
}
