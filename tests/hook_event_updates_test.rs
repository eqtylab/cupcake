use cupcake::engine::events::{CommonEventData, HookEvent};
use serde_json;

#[test]
fn test_cwd_field_in_all_events() {
    // Test PreToolUse with cwd
    let pre_tool_use_json = r#"
    {
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/home/user/project",
        "tool_name": "Bash",
        "tool_input": {
            "command": "cargo build"
        }
    }
    "#;

    let event: HookEvent = serde_json::from_str(pre_tool_use_json).unwrap();
    assert_eq!(event.common().cwd, "/home/user/project");

    // Test PostToolUse with cwd
    let post_tool_use_json = r#"
    {
        "hook_event_name": "PostToolUse",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/usr/local/bin",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "test.txt",
            "content": "test"
        },
        "tool_response": {
            "success": true
        }
    }
    "#;

    let event: HookEvent = serde_json::from_str(post_tool_use_json).unwrap();
    assert_eq!(event.common().cwd, "/usr/local/bin");

    // Test Notification with cwd
    let notification_json = r#"
    {
        "hook_event_name": "Notification",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/tmp",
        "message": "Test notification"
    }
    "#;

    let event: HookEvent = serde_json::from_str(notification_json).unwrap();
    assert_eq!(event.common().cwd, "/tmp");

    // Test Stop with cwd
    let stop_json = r#"
    {
        "hook_event_name": "Stop",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/home/user",
        "stop_hook_active": false
    }
    "#;

    let event: HookEvent = serde_json::from_str(stop_json).unwrap();
    assert_eq!(event.common().cwd, "/home/user");

    // Test SubagentStop with cwd
    let subagent_stop_json = r#"
    {
        "hook_event_name": "SubagentStop",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/workspace",
        "stop_hook_active": true
    }
    "#;

    let event: HookEvent = serde_json::from_str(subagent_stop_json).unwrap();
    assert_eq!(event.common().cwd, "/workspace");

    // Test PreCompact with cwd
    let pre_compact_json = r#"
    {
        "hook_event_name": "PreCompact",
        "session_id": "test-session",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/var/log",
        "trigger": "manual",
        "custom_instructions": "Save state"
    }
    "#;

    let event: HookEvent = serde_json::from_str(pre_compact_json).unwrap();
    assert_eq!(event.common().cwd, "/var/log");
}

#[test]
fn test_user_prompt_submit_event() {
    let json = r#"
    {
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session-prompt",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/home/user/project",
        "prompt": "Write a function to calculate factorial"
    }
    "#;

    let event: HookEvent = serde_json::from_str(json).unwrap();

    match event {
        HookEvent::UserPromptSubmit { common, prompt } => {
            assert_eq!(common.session_id, "test-session-prompt");
            assert_eq!(common.transcript_path, "/path/to/transcript.md");
            assert_eq!(common.cwd, "/home/user/project");
            assert_eq!(prompt, "Write a function to calculate factorial");
        }
        _ => panic!("Expected UserPromptSubmit event"),
    }
}

#[test]
fn test_user_prompt_submit_with_secrets() {
    let json = r#"
    {
        "hook_event_name": "UserPromptSubmit",
        "session_id": "test-session-secret",
        "transcript_path": "/path/to/transcript.md",
        "cwd": "/secure/location",
        "prompt": "My password is secret123"
    }
    "#;

    let event: HookEvent = serde_json::from_str(json).unwrap();

    match event {
        HookEvent::UserPromptSubmit { common, prompt } => {
            assert_eq!(common.cwd, "/secure/location");
            assert!(prompt.contains("password"));
        }
        _ => panic!("Expected UserPromptSubmit event"),
    }
}

#[test]
fn test_user_prompt_submit_event_name() {
    let event = HookEvent::UserPromptSubmit {
        common: CommonEventData {
            session_id: "test".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/home".to_string(),
        },
        prompt: "Test prompt".to_string(),
    };

    assert_eq!(event.event_name(), "UserPromptSubmit");
    assert!(!event.is_tool_event());
    assert!(!event.is_stop_event());
}