use crate::common::event_factory::EventFactory;
use cupcake::engine::events::{
    claude_code::{BashToolInput, ReadToolInput},
    ClaudeCodeEvent, CompactTrigger,
};

#[test]
fn test_hook_event_deserialization() {
    // Test PreToolUse event
    let pre_tool_use_json = EventFactory::pre_tool_use()
        .session_id("test-session-123")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("cargo build")
        .tool_input_description("Build the project")
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&pre_tool_use_json).expect("Failed to deserialize PreToolUse event");

    match event {
        ClaudeCodeEvent::PreToolUse(payload) => {
            assert_eq!(payload.common.session_id, "test-session-123");
            assert_eq!(payload.common.transcript_path, "/path/to/transcript.md");
            assert_eq!(payload.tool_name, "Bash");
            assert_eq!(payload.tool_input["command"], "cargo build");
        }
        _ => panic!("Expected PreToolUse event"),
    }
}

#[test]
fn test_post_tool_use_event() {
    let post_tool_use_json = EventFactory::post_tool_use()
        .session_id("test-session-456")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .tool_name("Read")
        .tool_input(serde_json::json!({
            "file_path": "/path/to/file.rs",
            "limit": 100
        }))
        .tool_response(serde_json::json!({
            "content": "file content here",
            "lines_read": 50
        }))
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&post_tool_use_json).expect("Failed to deserialize PostToolUse event");

    match event {
        ClaudeCodeEvent::PostToolUse(payload) => {
            assert_eq!(payload.common.session_id, "test-session-456");
            assert_eq!(payload.tool_name, "Read");
            assert_eq!(payload.tool_input["file_path"], "/path/to/file.rs");
            assert_eq!(payload.tool_response["lines_read"], 50);
        }
        _ => panic!("Expected PostToolUse event"),
    }
}

#[test]
fn test_notification_event() {
    let notification_json = EventFactory::notification()
        .session_id("test-session-789")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .message("Claude needs permission to use the Bash tool")
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&notification_json).expect("Failed to deserialize Notification event");

    match event {
        ClaudeCodeEvent::Notification(payload) => {
            assert_eq!(payload.common.session_id, "test-session-789");
            assert_eq!(
                payload.message,
                "Claude needs permission to use the Bash tool"
            );
        }
        _ => panic!("Expected Notification event"),
    }
}

#[test]
fn test_stop_event() {
    let stop_json = EventFactory::stop()
        .session_id("test-session-stop")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .stop_hook_active(false)
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&stop_json).expect("Failed to deserialize Stop event");

    match event {
        ClaudeCodeEvent::Stop(payload) => {
            assert_eq!(payload.common.session_id, "test-session-stop");
            assert!(!payload.stop_hook_active);
        }
        _ => panic!("Expected Stop event"),
    }
}

#[test]
fn test_subagent_stop_event() {
    let subagent_stop_json = EventFactory::subagent_stop()
        .session_id("test-session-subagent")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .stop_hook_active(true)
        .build_json();

    let event: ClaudeCodeEvent = serde_json::from_str(&subagent_stop_json)
        .expect("Failed to deserialize SubagentStop event");

    match event {
        ClaudeCodeEvent::SubagentStop(payload) => {
            assert_eq!(payload.common.session_id, "test-session-subagent");
            assert!(payload.stop_hook_active);
        }
        _ => panic!("Expected SubagentStop event"),
    }
}

#[test]
fn test_pre_compact_event() {
    let pre_compact_json = EventFactory::pre_compact()
        .session_id("test-session-compact")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .trigger("manual")
        .custom_instructions("Save important context")
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&pre_compact_json).expect("Failed to deserialize PreCompact event");

    match event {
        ClaudeCodeEvent::PreCompact(payload) => {
            assert_eq!(payload.common.session_id, "test-session-compact");
            assert!(matches!(payload.trigger, CompactTrigger::Manual));
            assert_eq!(
                payload.custom_instructions,
                Some("Save important context".to_string())
            );
        }
        _ => panic!("Expected PreCompact event"),
    }
}

#[test]
fn test_pre_compact_auto_trigger() {
    let pre_compact_auto_json = EventFactory::pre_compact()
        .session_id("test-session-auto")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .trigger("auto")
        .build_json();

    let event: ClaudeCodeEvent = serde_json::from_str(&pre_compact_auto_json)
        .expect("Failed to deserialize PreCompact auto event");

    match event {
        ClaudeCodeEvent::PreCompact(payload) => {
            assert_eq!(payload.common.session_id, "test-session-auto");
            assert!(matches!(payload.trigger, CompactTrigger::Auto));
            assert_eq!(payload.custom_instructions, None);
        }
        _ => panic!("Expected PreCompact event"),
    }
}

#[test]
fn test_hook_event_helper_methods() {
    let event = EventFactory::pre_tool_use()
        .session_id("test-session")
        .transcript_path("/path/to/transcript.md")
        .cwd("/tmp")
        .tool_name("Bash")
        .tool_input_command("echo hello")
        .build();

    // Test helper methods
    assert_eq!(event.common().session_id, "test-session");
    assert_eq!(event.tool_name(), Some("Bash"));
    assert!(event.tool_input().is_some());
    assert!(event.is_tool_event());
    assert!(!event.is_stop_event());

    let notification_event = EventFactory::notification()
        .session_id("test-session")
        .message("Test notification")
        .build();

    assert!(!notification_event.is_tool_event());
    assert_eq!(notification_event.tool_name(), None);
}

#[test]
fn test_tool_input_parsing() {
    // Test Bash tool input parsing
    let bash_event = EventFactory::pre_tool_use()
        .tool_name("Bash")
        .tool_input_command("cargo test")
        .tool_input_description("Run tests")
        .tool_input_timeout(60)
        .build();

    let bash_input: BashToolInput = bash_event.parse_tool_input().unwrap();
    assert_eq!(bash_input.command, "cargo test");
    assert_eq!(bash_input.description, Some("Run tests".to_string()));
    assert_eq!(bash_input.timeout, Some(60));

    // Test Read tool input parsing
    let read_event = EventFactory::pre_tool_use()
        .tool_name("Read")
        .tool_input(serde_json::json!({
            "file_path": "/tmp/test.rs",
            "limit": 50,
            "offset": 100
        }))
        .build();

    let read_input: ReadToolInput = read_event.parse_tool_input().unwrap();
    assert_eq!(read_input.file_path, "/tmp/test.rs");
    assert_eq!(read_input.limit, Some(50));
    assert_eq!(read_input.offset, Some(100));
}

#[test]
fn test_user_prompt_submit_event() {
    let prompt_json = EventFactory::user_prompt_submit()
        .session_id("test-prompt-session")
        .prompt("Help me write a function")
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&prompt_json).expect("Failed to deserialize UserPromptSubmit event");

    match event {
        ClaudeCodeEvent::UserPromptSubmit(payload) => {
            assert_eq!(payload.common.session_id, "test-prompt-session");
            assert_eq!(payload.prompt, "Help me write a function");
        }
        _ => panic!("Expected UserPromptSubmit event"),
    }
}

#[test]
fn test_session_start_event() {
    let session_json = EventFactory::session_start()
        .session_id("test-start-session")
        .source("startup")
        .build_json();

    let event: ClaudeCodeEvent =
        serde_json::from_str(&session_json).expect("Failed to deserialize SessionStart event");

    match event {
        ClaudeCodeEvent::SessionStart(payload) => {
            assert_eq!(payload.common.session_id, "test-start-session");
            assert!(payload.is_startup());
        }
        _ => panic!("Expected SessionStart event"),
    }
}

#[test]
fn test_complete_event_serialization() {
    // Test that we can serialize and deserialize all event types
    let events = vec![
        EventFactory::pre_tool_use().tool_name("Test").build(),
        EventFactory::post_tool_use().tool_name("Test").build(),
        EventFactory::notification().message("Test").build(),
        EventFactory::stop().build(),
        EventFactory::subagent_stop().build(),
        EventFactory::pre_compact().build(),
        EventFactory::user_prompt_submit().prompt("Test").build(),
        EventFactory::session_start().build(),
    ];

    for event in events {
        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: ClaudeCodeEvent =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Basic sanity check
        assert_eq!(event.event_name(), deserialized.event_name());
    }
}
