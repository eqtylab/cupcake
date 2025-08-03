//! Test the EventFactory itself to ensure it produces valid JSON

mod common;
use common::EventFactory;
use serde_json::Value;

#[test]
fn test_event_factory_produces_valid_json() {
    // Test PreToolUse
    let pre_tool_json = EventFactory::pre_tool_use()
        .tool_name("Bash")
        .tool_input_command("echo hello")
        .build_json();
    
    let parsed: Value = serde_json::from_str(&pre_tool_json)
        .expect("PreToolUse should produce valid JSON");
    assert_eq!(parsed["hook_event_name"], "PreToolUse");
    
    // Test PostToolUse
    let post_tool_json = EventFactory::post_tool_use()
        .tool_response_success(true, "Command executed")
        .build_json();
    
    let parsed: Value = serde_json::from_str(&post_tool_json)
        .expect("PostToolUse should produce valid JSON");
    assert_eq!(parsed["hook_event_name"], "PostToolUse");
    
    // Test UserPromptSubmit
    let prompt_json = EventFactory::user_prompt_submit()
        .prompt("Test prompt")
        .build_json();
    
    let parsed: Value = serde_json::from_str(&prompt_json)
        .expect("UserPromptSubmit should produce valid JSON");
    assert_eq!(parsed["prompt"], "Test prompt");
    
    // Test PreCompact with custom instructions
    let compact_json = EventFactory::pre_compact()
        .trigger_manual()
        .custom_instructions("Keep TODO comments")
        .build_json();
    
    let parsed: Value = serde_json::from_str(&compact_json)
        .expect("PreCompact should produce valid JSON");
    assert_eq!(parsed["trigger"], "manual");
    assert_eq!(parsed["custom_instructions"], "Keep TODO comments");
}