use cupcake::engine::events::HookEvent;

mod common;
use common::event_factory::EventFactory;

#[test]
fn test_cwd_field_in_all_events() {
    // Test PreToolUse with cwd
    let event = EventFactory::pre_tool_use()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/home/user/project")
        .tool_name("Bash")
        .tool_input_command("cargo build")
        .build();
    assert_eq!(event.common().cwd, "/home/user/project");

    // Test PostToolUse with cwd
    let event = EventFactory::post_tool_use()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/usr/local/bin")
        .tool_name("Write")
        .tool_input(serde_json::json!({
            "file_path": "test.txt",
            "content": "test"
        }))
        .tool_response(serde_json::json!({
            "success": true
        }))
        .build();
    assert_eq!(event.common().cwd, "/usr/local/bin");

    // Test Notification with cwd
    let event = EventFactory::notification()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .message("Test notification")
        .build();
    assert_eq!(event.common().cwd, "/tmp");

    // Test Stop with cwd
    let event = EventFactory::stop()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/home/user")
        .stop_hook_active(false)
        .build();
    assert_eq!(event.common().cwd, "/home/user");

    // Test SubagentStop with cwd
    let event = EventFactory::subagent_stop()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/workspace")
        .stop_hook_active(true)
        .build();
    assert_eq!(event.common().cwd, "/workspace");

    // Test PreCompact with cwd
    let event = EventFactory::pre_compact()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/var/log")
        .trigger("manual")
        .custom_instructions("Save state")
        .build();
    assert_eq!(event.common().cwd, "/var/log");
}

#[test]
fn test_user_prompt_submit_event() {
    let event = EventFactory::user_prompt_submit()
        .session_id("test-session-prompt")
        .transcript_path("/path/to/transcript.md")
        .cwd("/home/user/project")
        .prompt("Write a function to calculate factorial")
        .build();

    match event {
        HookEvent::UserPromptSubmit(payload) => {
            assert_eq!(payload.common.session_id, "test-session-prompt");
            assert_eq!(payload.common.transcript_path, "/path/to/transcript.md");
            assert_eq!(payload.common.cwd, "/home/user/project");
            assert_eq!(payload.prompt, "Write a function to calculate factorial");
        }
        _ => panic!("Expected UserPromptSubmit event"),
    }
}

#[test]
fn test_user_prompt_submit_with_secrets() {
    let event = EventFactory::user_prompt_submit()
        .session_id("test-session-secret")
        .transcript_path("/path/to/transcript.md")
        .cwd("/secure/location")
        .prompt("My password is secret123")
        .build();

    match event {
        HookEvent::UserPromptSubmit(payload) => {
            assert_eq!(payload.common.cwd, "/secure/location");
            assert!(payload.prompt.contains("password"));
        }
        _ => panic!("Expected UserPromptSubmit event"),
    }
}

#[test]
fn test_user_prompt_submit_event_name() {
    let event = EventFactory::user_prompt_submit()
        .session_id("test")
        .transcript_path("/path")
        .cwd("/home")
        .prompt("Test prompt")
        .build();

    assert_eq!(event.event_name(), "UserPromptSubmit");
    assert!(!event.is_tool_event());
    assert!(!event.is_stop_event());
}
