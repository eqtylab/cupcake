//! Contract tests for Claude Code JSON response compliance
//!
//! These tests verify that Cupcake produces 100% Claude Code spec-compliant responses

use cupcake::engine::events::claude_code::{
    CompactTrigger, NotificationPayload, PostToolUsePayload, PreCompactPayload, PreToolUsePayload,
    SessionSource, SessionStartPayload, StopPayload, SubagentStopPayload, UserPromptSubmitPayload,
};
use cupcake::engine::events::{ClaudeCodeEvent, CommonEventData};
use cupcake::engine::response::{claude_code::ClaudeCodeResponseBuilder, EngineDecision};
use serde_json::Value;

// Helper to create test events
fn create_pre_tool_use() -> ClaudeCodeEvent {
    ClaudeCodeEvent::PreToolUse(PreToolUsePayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        tool_name: "Bash".to_string(),
        tool_input: serde_json::json!({"command": "ls"}),
    })
}

fn create_post_tool_use() -> ClaudeCodeEvent {
    ClaudeCodeEvent::PostToolUse(PostToolUsePayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        tool_name: "Bash".to_string(),
        tool_input: serde_json::json!({"command": "ls"}),
        tool_response: serde_json::json!({
            "success": true,
            "output": "file1.txt\nfile2.txt"
        }),
    })
}

fn create_user_prompt_submit() -> ClaudeCodeEvent {
    ClaudeCodeEvent::UserPromptSubmit(UserPromptSubmitPayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        prompt: "Test prompt".to_string(),
    })
}

fn create_session_start() -> ClaudeCodeEvent {
    ClaudeCodeEvent::SessionStart(SessionStartPayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        source: SessionSource::Startup,
    })
}

fn create_pre_compact() -> ClaudeCodeEvent {
    ClaudeCodeEvent::PreCompact(PreCompactPayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        trigger: CompactTrigger::Manual,
        custom_instructions: Some("Test instructions".to_string()),
    })
}

fn create_notification() -> ClaudeCodeEvent {
    ClaudeCodeEvent::Notification(NotificationPayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        message: "Test notification".to_string(),
    })
}

fn create_stop() -> ClaudeCodeEvent {
    ClaudeCodeEvent::Stop(StopPayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        stop_hook_active: false,
    })
}

fn create_subagent_stop() -> ClaudeCodeEvent {
    ClaudeCodeEvent::SubagentStop(SubagentStopPayload {
        common: CommonEventData {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript".to_string(),
            cwd: "/home/user".to_string(),
        },
        stop_hook_active: false,
    })
}

#[test]
fn test_pre_tool_use_allow_contract() {
    let event = create_pre_tool_use();
    let decision = EngineDecision::Allow {
        reason: Some("Allowed by policy".to_string()),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    // Verify contract fields
    assert_eq!(json["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(json["hookSpecificOutput"]["permissionDecision"], "allow");
    assert_eq!(
        json["hookSpecificOutput"]["permissionDecisionReason"],
        "Allowed by policy"
    );

    // Verify no extra fields
    assert_eq!(json.get("continue"), None);
    assert_eq!(json.get("stopReason"), None);
    assert_eq!(json.get("decision"), None);
    assert_eq!(json.get("reason"), None);
}

#[test]
fn test_pre_tool_use_deny_contract() {
    let event = create_pre_tool_use();
    let decision = EngineDecision::Block {
        feedback: "Dangerous command".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    assert_eq!(json["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(json["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(
        json["hookSpecificOutput"]["permissionDecisionReason"],
        "Dangerous command"
    );
}

#[test]
fn test_pre_tool_use_ask_contract() {
    let event = create_pre_tool_use();
    let decision = EngineDecision::Ask {
        reason: "Please confirm this action".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    assert_eq!(json["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(json["hookSpecificOutput"]["permissionDecision"], "ask");
    assert_eq!(
        json["hookSpecificOutput"]["permissionDecisionReason"],
        "Please confirm this action"
    );
}

#[test]
fn test_post_tool_use_block_contract() {
    let event = create_post_tool_use();
    let decision = EngineDecision::Block {
        feedback: "Output format incorrect - please return JSON".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    // Feedback loop format
    assert_eq!(json["decision"], "block");
    assert_eq!(
        json["reason"],
        "Output format incorrect - please return JSON"
    );

    // Should NOT have continue/stopReason
    assert_eq!(json.get("continue"), None);
    assert_eq!(json.get("stopReason"), None);
    assert_eq!(json.get("hookSpecificOutput"), None);
}

#[test]
fn test_stop_block_contract() {
    let event = create_stop();
    let decision = EngineDecision::Block {
        feedback: "Task incomplete - add error handling".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    // Feedback loop format
    assert_eq!(json["decision"], "block");
    assert_eq!(json["reason"], "Task incomplete - add error handling");

    // Should NOT have continue/stopReason
    assert_eq!(json.get("continue"), None);
    assert_eq!(json.get("stopReason"), None);
}

#[test]
fn test_subagent_stop_block_contract() {
    let event = create_subagent_stop();
    let decision = EngineDecision::Block {
        feedback: "Subagent task needs validation".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    // Feedback loop format
    assert_eq!(json["decision"], "block");
    assert_eq!(json["reason"], "Subagent task needs validation");
}

#[test]
fn test_user_prompt_submit_context_injection_contract() {
    let event = create_user_prompt_submit();
    let decision = EngineDecision::Allow { reason: None };
    let context = vec![
        "Policy reminder: follow security guidelines".to_string(),
        "Today's date: 2025-08-04".to_string(),
    ];
    let response =
        ClaudeCodeResponseBuilder::build_response(&decision, &event, Some(context), false);

    let json: Value = serde_json::to_value(&response).unwrap();

    assert_eq!(
        json["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    assert_eq!(
        json["hookSpecificOutput"]["additionalContext"],
        "Policy reminder: follow security guidelines\nToday's date: 2025-08-04"
    );
}

#[test]
fn test_user_prompt_submit_block_contract() {
    let event = create_user_prompt_submit();
    let decision = EngineDecision::Block {
        feedback: "Prompt contains sensitive information".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    // UserPromptSubmit uses decision: "block" format in hookSpecificOutput
    assert_eq!(json["hookSpecificOutput"]["decision"], "block");
    assert_eq!(
        json["hookSpecificOutput"]["decisionReason"],
        "Prompt contains sensitive information"
    );

    // Should NOT have continue/stopReason at top level
    assert_eq!(json.get("continue"), None);
    assert_eq!(json.get("stopReason"), None);
}

#[test]
fn test_session_start_context_injection_contract() {
    let event = create_session_start();
    let decision = EngineDecision::Allow { reason: None };
    let context = vec!["Welcome to the session".to_string()];
    let response =
        ClaudeCodeResponseBuilder::build_response(&decision, &event, Some(context), false);

    let json: Value = serde_json::to_value(&response).unwrap();

    assert_eq!(
        json["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    assert_eq!(
        json["hookSpecificOutput"]["additionalContext"],
        "Welcome to the session"
    );
}

#[test]
fn test_notification_block_contract() {
    let event = create_notification();
    let decision = EngineDecision::Block {
        feedback: "Notification not allowed".to_string(),
    };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json: Value = serde_json::to_value(&response).unwrap();

    // Generic events use continue/stopReason
    assert_eq!(json["continue"], false);
    assert_eq!(json["stopReason"], "Notification not allowed");

    // Should NOT have decision/reason or hookSpecificOutput
    assert_eq!(json.get("decision"), None);
    assert_eq!(json.get("reason"), None);
    assert_eq!(json.get("hookSpecificOutput"), None);
}

#[test]
fn test_suppress_output_contract() {
    // Test that suppress_output works for all event types
    let event = create_pre_tool_use();
    let decision = EngineDecision::Allow { reason: None };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, true);

    let json: Value = serde_json::to_value(&response).unwrap();
    assert_eq!(json["suppressOutput"], true);

    // Test with feedback loop event
    let event = create_post_tool_use();
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, true);

    let json: Value = serde_json::to_value(&response).unwrap();
    assert_eq!(json["suppressOutput"], true);
}

#[test]
fn test_allow_produces_minimal_response() {
    // Allow should produce minimal response for feedback events
    let event = create_post_tool_use();
    let decision = EngineDecision::Allow { reason: None };
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json_str = serde_json::to_string(&response).unwrap();
    assert_eq!(json_str, "{}"); // Empty JSON

    // Same for Stop
    let event = create_stop();
    let response = ClaudeCodeResponseBuilder::build_response(&decision, &event, None, false);

    let json_str = serde_json::to_string(&response).unwrap();
    assert_eq!(json_str, "{}"); // Empty JSON
}

#[test]
fn test_pre_compact_special_case() {
    // PreCompact with context should be handled specially in run/mod.rs
    // But when called directly, it should return a generic response
    let event = create_pre_compact();
    let decision = EngineDecision::Allow { reason: None };
    let context = vec!["Preserve all TODOs".to_string()];

    let response =
        ClaudeCodeResponseBuilder::build_response(&decision, &event, Some(context), false);
    let json_str = serde_json::to_string(&response).unwrap();

    // Should return empty response (special handling is in run/mod.rs)
    assert_eq!(json_str, "{}");
}
