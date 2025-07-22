use cupcake::engine::events::{
    BashToolInput, CommonEventData, CompactTrigger, HookEvent, ReadToolInput,
};
use serde_json;

#[test]
fn test_hook_event_deserialization() {
    // Test PreToolUse event
    let pre_tool_use_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session-123",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "cargo build",
            "description": "Build the project"
        }
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(pre_tool_use_json).expect("Failed to deserialize PreToolUse event");

    match event {
        HookEvent::PreToolUse {
            common,
            tool_name,
            tool_input,
        } => {
            assert_eq!(common.session_id, "test-session-123");
            assert_eq!(common.transcript_path, "/path/to/transcript.md");
            assert_eq!(tool_name, "Bash");
            assert_eq!(tool_input["command"], "cargo build");
        }
        _ => panic!("Expected PreToolUse event"),
    }
}

#[test]
fn test_post_tool_use_event() {
    let post_tool_use_json = r#"
    {
        "hook_event_name": "PostToolUse",
        "session_id": "test-session-456",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "tool_name": "Read",
        "tool_input": {
            "file_path": "/path/to/file.rs",
            "limit": 100
        },
        "tool_response": {
            "content": "file content here",
            "lines_read": 50
        }
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(post_tool_use_json).expect("Failed to deserialize PostToolUse event");

    match event {
        HookEvent::PostToolUse {
            common,
            tool_name,
            tool_input,
            tool_response,
        } => {
            assert_eq!(common.session_id, "test-session-456");
            assert_eq!(tool_name, "Read");
            assert_eq!(tool_input["file_path"], "/path/to/file.rs");
            assert_eq!(tool_response["lines_read"], 50);
        }
        _ => panic!("Expected PostToolUse event"),
    }
}

#[test]
fn test_notification_event() {
    let notification_json = r#"
    {
        "hook_event_name": "Notification",
        "session_id": "test-session-789",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "message": "Claude needs permission to use the Bash tool"
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(notification_json).expect("Failed to deserialize Notification event");

    match event {
        HookEvent::Notification { common, message } => {
            assert_eq!(common.session_id, "test-session-789");
            assert_eq!(message, "Claude needs permission to use the Bash tool");
        }
        _ => panic!("Expected Notification event"),
    }
}

#[test]
fn test_stop_event() {
    let stop_json = r#"
    {
        "hook_event_name": "Stop",
        "session_id": "test-session-stop",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "stop_hook_active": false
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(stop_json).expect("Failed to deserialize Stop event");

    match event {
        HookEvent::Stop {
            common,
            stop_hook_active,
        } => {
            assert_eq!(common.session_id, "test-session-stop");
            assert!(!stop_hook_active);
        }
        _ => panic!("Expected Stop event"),
    }
}

#[test]
fn test_subagent_stop_event() {
    let subagent_stop_json = r#"
    {
        "hook_event_name": "SubagentStop",
        "session_id": "test-session-subagent",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "stop_hook_active": true
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(subagent_stop_json).expect("Failed to deserialize SubagentStop event");

    match event {
        HookEvent::SubagentStop {
            common,
            stop_hook_active,
        } => {
            assert_eq!(common.session_id, "test-session-subagent");
            assert!(stop_hook_active);
        }
        _ => panic!("Expected SubagentStop event"),
    }
}

#[test]
fn test_pre_compact_event() {
    let pre_compact_json = r#"
    {
        "hook_event_name": "PreCompact",
        "session_id": "test-session-compact",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "trigger": "manual",
        "custom_instructions": "Save important context"
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(pre_compact_json).expect("Failed to deserialize PreCompact event");

    match event {
        HookEvent::PreCompact {
            common,
            trigger,
            custom_instructions,
        } => {
            assert_eq!(common.session_id, "test-session-compact");
            assert!(matches!(trigger, CompactTrigger::Manual));
            assert_eq!(
                custom_instructions,
                Some("Save important context".to_string())
            );
        }
        _ => panic!("Expected PreCompact event"),
    }
}

#[test]
fn test_pre_compact_auto_trigger() {
    let pre_compact_auto_json = r#"
    {
        "hook_event_name": "PreCompact",
        "session_id": "test-session-auto",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "trigger": "auto"
    }
    "#;

    let event: HookEvent = serde_json::from_str(pre_compact_auto_json)
        .expect("Failed to deserialize PreCompact auto event");

    match event {
        HookEvent::PreCompact {
            common,
            trigger,
            custom_instructions,
        } => {
            assert_eq!(common.session_id, "test-session-auto");
            assert!(matches!(trigger, CompactTrigger::Auto));
            assert_eq!(custom_instructions, None);
        }
        _ => panic!("Expected PreCompact event"),
    }
}

#[test]
fn test_hook_event_helper_methods() {
    let pre_tool_use_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo hello"
        }
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(pre_tool_use_json).expect("Failed to deserialize event");

    // Test helper methods
    assert_eq!(event.common().session_id, "test-session");
    assert_eq!(event.tool_name(), Some("Bash"));
    assert!(event.tool_input().is_some());
    assert!(event.is_tool_event());
    assert!(!event.is_stop_event());

    let notification_json = r#"
    {
        "hook_event_name": "Notification",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "message": "Test notification"
    }
    "#;

    let notification_event: HookEvent =
        serde_json::from_str(notification_json).expect("Failed to deserialize notification");

    assert_eq!(notification_event.tool_name(), None);
    assert!(!notification_event.is_tool_event());
    assert!(!notification_event.is_stop_event());

    let stop_json = r#"
    {
        "hook_event_name": "Stop",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "stop_hook_active": false
    }
    "#;

    let stop_event: HookEvent =
        serde_json::from_str(stop_json).expect("Failed to deserialize stop event");

    assert!(!stop_event.is_tool_event());
    assert!(stop_event.is_stop_event());
}

#[test]
fn test_tool_input_parsing() {
    let bash_event_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "tool_name": "Bash",
        "tool_input": {
            "command": "cargo test",
            "description": "Run tests",
            "timeout": 300
        }
    }
    "#;

    let event: HookEvent =
        serde_json::from_str(bash_event_json).expect("Failed to deserialize Bash event");

    let bash_input: BashToolInput = event
        .parse_tool_input()
        .expect("Failed to parse Bash tool input");
    assert_eq!(bash_input.command, "cargo test");
    assert_eq!(bash_input.description, Some("Run tests".to_string()));
    assert_eq!(bash_input.timeout, Some(300));

    let read_event_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "tool_name": "Read",
        "tool_input": {
            "file_path": "/path/to/file.rs",
            "limit": 50,
            "offset": 10
        }
    }
    "#;

    let read_event: HookEvent =
        serde_json::from_str(read_event_json).expect("Failed to deserialize Read event");

    let read_input: ReadToolInput = read_event
        .parse_tool_input()
        .expect("Failed to parse Read tool input");
    assert_eq!(read_input.file_path, "/path/to/file.rs");
    assert_eq!(read_input.limit, Some(50));
    assert_eq!(read_input.offset, Some(10));
}

#[test]
fn test_compact_trigger_serialization() {
    // Test manual trigger
    let manual_trigger = CompactTrigger::Manual;
    let json = serde_json::to_string(&manual_trigger).expect("Failed to serialize manual trigger");
    assert_eq!(json, r#""manual""#);

    let deserialized: CompactTrigger =
        serde_json::from_str(&json).expect("Failed to deserialize manual trigger");
    assert!(matches!(deserialized, CompactTrigger::Manual));

    // Test auto trigger
    let auto_trigger = CompactTrigger::Auto;
    let json = serde_json::to_string(&auto_trigger).expect("Failed to serialize auto trigger");
    assert_eq!(json, r#""auto""#);

    let deserialized: CompactTrigger =
        serde_json::from_str(&json).expect("Failed to deserialize auto trigger");
    assert!(matches!(deserialized, CompactTrigger::Auto));
}

#[test]
fn test_common_event_data() {
    let common_data = CommonEventData {
        session_id: "test-session-common".to_string(),
        transcript_path: "/path/to/transcript.md".to_string(),
        cwd: "/home/user/project".to_string(),
    };

    let json = serde_json::to_string(&common_data).expect("Failed to serialize common event data");
    let deserialized: CommonEventData =
        serde_json::from_str(&json).expect("Failed to deserialize common event data");

    assert_eq!(deserialized.session_id, common_data.session_id);
    assert_eq!(deserialized.transcript_path, common_data.transcript_path);
}
