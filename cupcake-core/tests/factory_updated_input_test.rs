//! Tests for Factory AI Droid's updatedInput feature
//!
//! The updatedInput feature allows policies to modify tool parameters before execution.
//! This is unique to Factory AI Droid and not supported in Claude Code or Cursor.
//!
//! These tests verify:
//! - Response builder correctly includes updatedInput in JSON output
//! - Updated input is only included with Allow decisions
//! - Updated input is properly formatted for Factory AI Droid consumption

use cupcake_core::harness::response::factory::PreToolUseResponseBuilder;
use cupcake_core::harness::response::types::{
    EngineDecision, HookSpecificOutput, PermissionDecision,
};
use serde_json::json;

#[test]
fn test_pre_tool_use_with_updated_input() {
    let decision = EngineDecision::Allow {
        reason: Some("Modified for safety".to_string()),
    };

    let updated_input = json!({
        "command": "ls -la /safe/path",
        "description": "Modified command"
    });

    let response = PreToolUseResponseBuilder::build_with_updated_input(
        &decision,
        Some(updated_input.clone()),
        false,
    );

    // Verify response structure
    assert!(response.hook_specific_output.is_some());

    if let Some(HookSpecificOutput::PreToolUse {
        permission_decision,
        permission_decision_reason,
        updated_input: returned_input,
    }) = response.hook_specific_output
    {
        assert_eq!(permission_decision, PermissionDecision::Allow);
        assert_eq!(
            permission_decision_reason,
            Some("Modified for safety".to_string())
        );
        assert!(returned_input.is_some());
        assert_eq!(returned_input.unwrap(), updated_input);
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}

#[test]
fn test_pre_tool_use_without_updated_input() {
    let decision = EngineDecision::Allow {
        reason: Some("Command is safe".to_string()),
    };

    let response = PreToolUseResponseBuilder::build(&decision, false);

    // Verify response structure without updated_input
    assert!(response.hook_specific_output.is_some());

    if let Some(HookSpecificOutput::PreToolUse {
        permission_decision,
        permission_decision_reason,
        updated_input,
    }) = response.hook_specific_output
    {
        assert_eq!(permission_decision, PermissionDecision::Allow);
        assert_eq!(
            permission_decision_reason,
            Some("Command is safe".to_string())
        );
        assert!(updated_input.is_none());
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}

#[test]
fn test_deny_does_not_include_updated_input() {
    let decision = EngineDecision::Block {
        feedback: "Command blocked".to_string(),
    };

    // Even if we try to provide updated_input, it should be ignored for deny
    let updated_input = json!({"command": "safe command"});

    let response =
        PreToolUseResponseBuilder::build_with_updated_input(&decision, Some(updated_input), false);

    if let Some(HookSpecificOutput::PreToolUse {
        permission_decision,
        permission_decision_reason: _,
        updated_input,
    }) = response.hook_specific_output
    {
        assert_eq!(permission_decision, PermissionDecision::Deny);
        // Should not include updated_input when denying
        assert!(updated_input.is_none());
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}

#[test]
fn test_ask_does_not_include_updated_input() {
    let decision = EngineDecision::Ask {
        reason: "Confirm this action".to_string(),
    };

    let updated_input = json!({"command": "safe command"});

    let response =
        PreToolUseResponseBuilder::build_with_updated_input(&decision, Some(updated_input), false);

    if let Some(HookSpecificOutput::PreToolUse {
        permission_decision,
        permission_decision_reason: _,
        updated_input,
    }) = response.hook_specific_output
    {
        assert_eq!(permission_decision, PermissionDecision::Ask);
        // Should not include updated_input when asking
        assert!(updated_input.is_none());
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}

#[test]
fn test_updated_input_preserves_complex_json() {
    let decision = EngineDecision::Allow {
        reason: Some("Modified parameters".to_string()),
    };

    // Test with complex nested JSON structure
    let updated_input = json!({
        "command": "docker run",
        "args": ["--rm", "-it"],
        "env": {
            "PATH": "/usr/local/bin",
            "HOME": "/home/user"
        },
        "volumes": [
            {"host": "/tmp", "container": "/data"},
            {"host": "/home", "container": "/mnt"}
        ],
        "nested": {
            "deep": {
                "structure": "preserved"
            }
        }
    });

    let response = PreToolUseResponseBuilder::build_with_updated_input(
        &decision,
        Some(updated_input.clone()),
        false,
    );

    if let Some(HookSpecificOutput::PreToolUse {
        updated_input: returned_input,
        ..
    }) = response.hook_specific_output
    {
        assert!(returned_input.is_some());
        let returned = returned_input.unwrap();

        // Verify structure preservation
        assert_eq!(returned["command"], "docker run");
        assert_eq!(returned["args"], json!(["--rm", "-it"]));
        assert_eq!(returned["env"]["PATH"], "/usr/local/bin");
        assert_eq!(returned["volumes"][0]["host"], "/tmp");
        assert_eq!(returned["nested"]["deep"]["structure"], "preserved");
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}

#[test]
fn test_updated_input_json_serialization() {
    let decision = EngineDecision::Allow {
        reason: Some("Command modified".to_string()),
    };

    let updated_input = json!({
        "file_path": "/path/to/file.txt",
        "content": "new content\nwith\nnewlines",
        "permissions": 0o644
    });

    let response =
        PreToolUseResponseBuilder::build_with_updated_input(&decision, Some(updated_input), false);

    // Serialize to JSON to verify it's valid
    let json_str = serde_json::to_string(&response).expect("Should serialize");

    // Deserialize back to verify structure
    let deserialized: serde_json::Value =
        serde_json::from_str(&json_str).expect("Should deserialize");

    // Verify the updatedInput field exists and is properly nested
    assert!(deserialized["hookSpecificOutput"]["updatedInput"].is_object());
    assert_eq!(
        deserialized["hookSpecificOutput"]["updatedInput"]["file_path"],
        "/path/to/file.txt"
    );
    assert_eq!(
        deserialized["hookSpecificOutput"]["updatedInput"]["content"],
        "new content\nwith\nnewlines"
    );
}

#[test]
fn test_suppress_output_with_updated_input() {
    let decision = EngineDecision::Allow {
        reason: Some("Modified silently".to_string()),
    };

    let updated_input = json!({"command": "ls"});

    let response = PreToolUseResponseBuilder::build_with_updated_input(
        &decision,
        Some(updated_input.clone()),
        true, // suppress_output
    );

    // Verify both suppress_output and updated_input are present
    assert_eq!(response.suppress_output, Some(true));

    if let Some(HookSpecificOutput::PreToolUse {
        updated_input: returned_input,
        ..
    }) = response.hook_specific_output
    {
        assert_eq!(returned_input, Some(updated_input));
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}

/// Test for EngineDecision::Modify handling
#[test]
fn test_modify_decision_generates_allow_with_updated_input() {
    let updated_input = json!({
        "command": "ls -la /safe/path",
        "description": "Sanitized command"
    });

    let decision = EngineDecision::Modify {
        reason: "Command sanitized for safety".to_string(),
        updated_input: updated_input.clone(),
    };

    let response = PreToolUseResponseBuilder::build_with_updated_input(
        &decision, None, // updated_input comes from decision
        false,
    );

    // Verify response structure
    assert!(response.hook_specific_output.is_some());

    if let Some(HookSpecificOutput::PreToolUse {
        permission_decision,
        permission_decision_reason,
        updated_input: returned_input,
    }) = response.hook_specific_output
    {
        // Modify implies Allow with updatedInput
        assert_eq!(permission_decision, PermissionDecision::Allow);
        assert_eq!(
            permission_decision_reason,
            Some("Command sanitized for safety".to_string())
        );
        assert!(returned_input.is_some());
        assert_eq!(returned_input.unwrap(), updated_input);
    } else {
        panic!("Expected PreToolUse hook specific output");
    }
}
