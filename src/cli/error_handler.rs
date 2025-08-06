//! Error handling module for fail-closed behavior
//!
//! This module ensures that any error during hook processing results in a
//! spec-compliant blocking response, preventing unsafe fail-open behavior.

use crate::engine::events::ClaudeCodeEvent;
use crate::engine::response::{CupcakeResponse, HookSpecificOutput, PermissionDecision};
use crate::CupcakeError;

/// Handle run command errors with fail-closed behavior using event type string
///
/// This function ensures that any error results in a blocking response that
/// prevents the operation from proceeding. The response format depends on
/// the hook event type to maintain Claude Code spec compliance.
pub fn handle_run_command_error_with_type(error: CupcakeError, event_type: &str) -> ! {
    // Log the error for debugging
    eprintln!("Cupcake error (failing closed): {error}");
    
    // Generate appropriate blocking response based on event type string
    let response = generate_blocking_response_for_type(event_type, &error.to_string());
    
    // Output the blocking response
    if let Ok(json) = serde_json::to_string(&response) {
        println!("{json}");
    } else {
        // Even if JSON serialization fails, output a minimal blocking response
        println!(r#"{{"continue":false,"error":"Failed to serialize error response"}}"#);
    }
    
    // Exit with success code (0) as required by Claude Code hooks
    std::process::exit(0);
}

/// Handle run command errors with fail-closed behavior
///
/// This function ensures that any error results in a blocking response that
/// prevents the operation from proceeding. The response format depends on
/// the hook event type to maintain Claude Code spec compliance.
pub fn handle_run_command_error(error: CupcakeError, event: &ClaudeCodeEvent) -> ! {
    // Log the error for debugging
    eprintln!("Cupcake error (failing closed): {error}");
    
    // Generate appropriate blocking response based on event type
    let response = generate_blocking_response(event, &error.to_string());
    
    // Output the blocking response
    if let Ok(json) = serde_json::to_string(&response) {
        println!("{json}");
    } else {
        // Even if JSON serialization fails, output a minimal blocking response
        println!(r#"{{"continue":false,"error":"Failed to serialize error response"}}"#);
    }
    
    // Exit with success code (0) as required by Claude Code hooks
    std::process::exit(0);
}

/// Generate a blocking response based on event type string
fn generate_blocking_response_for_type(event_type: &str, error_message: &str) -> CupcakeResponse {
    let mut response = CupcakeResponse::empty();
    
    match event_type {
        "PreToolUse" => {
            // PreToolUse uses permission decision format
            response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                permission_decision: PermissionDecision::Deny,
                permission_decision_reason: Some(format!("Cupcake error: {error_message}")),
            });
        }
        "UserPromptSubmit" | "SessionStart" => {
            // These events use continue/stopReason format
            response.continue_execution = Some(false);
            response.stop_reason = Some(format!("Cupcake error: {error_message}"));
        }
        "PostToolUse" | "Stop" | "SubagentStop" | "PreCompact" => {
            // These events also use generic blocking format
            response.continue_execution = Some(false);
            response.stop_reason = Some(format!("Cupcake error: {error_message}"));
        }
        "Notification" => {
            // Notification hooks cannot block, just return empty response
            // Error will be logged to stderr
        }
        _ => {
            // Unknown event type - use generic blocking format
            response.continue_execution = Some(false);
            response.stop_reason = Some(format!("Cupcake error: {error_message}"));
        }
    }
    
    response
}

/// Generate a blocking response appropriate for the event type
fn generate_blocking_response(event: &ClaudeCodeEvent, error_message: &str) -> CupcakeResponse {
    let mut response = CupcakeResponse::empty();
    
    match event {
        ClaudeCodeEvent::PreToolUse(_) => {
            // PreToolUse uses permission decision format
            response.hook_specific_output = Some(HookSpecificOutput::PreToolUse {
                permission_decision: PermissionDecision::Deny,
                permission_decision_reason: Some(format!("Cupcake error: {error_message}")),
            });
        }
        ClaudeCodeEvent::UserPromptSubmit(_) | ClaudeCodeEvent::SessionStart(_) => {
            // These events use continue/stopReason format
            response.continue_execution = Some(false);
            response.stop_reason = Some(format!("Cupcake error: {error_message}"));
        }
        ClaudeCodeEvent::PostToolUse(_) 
        | ClaudeCodeEvent::Stop(_) 
        | ClaudeCodeEvent::SubagentStop(_) => {
            // These events also use generic blocking format
            response.continue_execution = Some(false);
            response.stop_reason = Some(format!("Cupcake error: {error_message}"));
        }
        ClaudeCodeEvent::PreCompact(_) => {
            // PreCompact can block by using exit code 2, but we'll use JSON for consistency
            response.continue_execution = Some(false);
            response.stop_reason = Some(format!("Cupcake error: {error_message}"));
        }
        ClaudeCodeEvent::Notification(_) => {
            // Notification hooks cannot block, just return empty response
            // Error will be logged to stderr
        }
    }
    
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::claude_code::{PreToolUsePayload, CommonEventData};
    
    #[test]
    fn test_blocking_response_for_pre_tool_use() {
        let event = ClaudeCodeEvent::PreToolUse(PreToolUsePayload {
            common: CommonEventData {
                session_id: "test".to_string(),
                transcript_path: "/tmp/test".to_string(),
                cwd: "/tmp".to_string(),
            },
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "rm -rf /"}),
        });
        
        let response = generate_blocking_response(&event, "Test error");
        
        match response.hook_specific_output {
            Some(HookSpecificOutput::PreToolUse { permission_decision, permission_decision_reason }) => {
                assert_eq!(permission_decision, PermissionDecision::Deny);
                assert!(permission_decision_reason.unwrap().contains("Test error"));
            }
            _ => panic!("Expected PreToolUse hook output"),
        }
    }
}