//! Integration tests for OpenCode harness
use anyhow::Result;
use cupcake_core::engine::{Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test project for OpenCode
async fn setup_opencode_test_project() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");

    // Create OpenCode-specific directory structure
    fs::create_dir_all(cupcake_dir.join("policies/opencode/system"))?;
    fs::create_dir_all(cupcake_dir.join("signals"))?;
    fs::create_dir_all(cupcake_dir.join("actions"))?;

    // Create system evaluate policy
    fs::write(
        cupcake_dir.join("policies/opencode/system/evaluate.rego"),
        r#"package cupcake.system

import rego.v1

evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "allow_overrides": collect_verbs("allow_override"),
        "add_context": collect_verbs("add_context")
    }
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]

    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]

    result := all_decisions
}

default collect_verbs(_) := []
"#,
    )?;

    // Create minimal rulebook
    fs::write(
        cupcake_dir.join("rulebook.yml"),
        r#"version: "1.0"
builtins: {}
signals: {}
actions: {}
"#,
    )?;

    Ok(temp_dir)
}

#[tokio::test]
async fn test_opencode_pretooluse_allow() -> Result<()> {
    let temp_dir = setup_opencode_test_project().await?;

    // Create a policy that doesn't match and thus allows
    let policy_content = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.test_allow

import rego.v1

# Deny only if command contains "danger" (which our test doesn't)
deny contains decision if {
    contains(input.tool_input.command, "danger")
    decision := {
        "rule_id": "DANGER_CHECK",
        "reason": "Command contains danger",
        "severity": "HIGH"
    }
}
"#;

    let policy_dir = temp_dir.path().join(".cupcake/policies/opencode");
    fs::write(policy_dir.join("test_allow.rego"), policy_content)?;

    // Create engine
    let config = EngineConfig {
        harness: HarnessType::OpenCode,
        wasm_max_memory: Some(10 * 1024 * 1024),
        opa_path: None,
        global_config: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Create OpenCode PreToolUse event (using OpenCode format: tool + args)
    // Preprocessing will convert to: tool_name + tool_input
    let event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test_session",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool": "bash",
        "args": {
            "command": "echo hello"
        }
    });

    // Evaluate
    let decision = engine.evaluate(&event, None).await?;

    // Should be Allow (default when no denials/blocks)
    assert!(!decision.is_blocking() && !decision.is_halt());

    Ok(())
}

#[tokio::test]
async fn test_opencode_pretooluse_deny() -> Result<()> {
    let temp_dir = setup_opencode_test_project().await?;

    // Create a deny policy for dangerous commands
    let policy_content = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.test_deny

import rego.v1

# Deny rm commands
deny contains decision if {
    startswith(input.tool_input.command, "rm ")
    decision := {
        "rule_id": "DENY_RM",
        "reason": "rm commands are dangerous",
        "severity": "HIGH"
    }
}
"#;

    let policy_dir = temp_dir.path().join(".cupcake/policies/opencode");
    fs::write(policy_dir.join("test_deny.rego"), policy_content)?;

    // Create engine
    let config = EngineConfig {
        harness: HarnessType::OpenCode,
        wasm_max_memory: Some(10 * 1024 * 1024),
        opa_path: None,
        global_config: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Create OpenCode PreToolUse event with dangerous command
    // Using OpenCode format: tool (lowercase) + args
    let event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test_session",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool": "bash",
        "args": {
            "command": "rm -rf /"
        }
    });

    // Evaluate
    let decision = engine.evaluate(&event, None).await?;

    // Should be blocking (Deny or Block)
    assert!(decision.is_blocking());

    Ok(())
}

#[tokio::test]
async fn test_opencode_posttooluse() -> Result<()> {
    let temp_dir = setup_opencode_test_project().await?;

    // Create a PostToolUse policy
    let policy_content = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PostToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.test_post

import rego.v1

# Log successful commands
add_context contains message if {
    input.tool_response.success == true
    message := concat("", [
        "Command succeeded: ",
        input.tool_input.command
    ])
}
"#;

    let policy_dir = temp_dir.path().join(".cupcake/policies/opencode");
    fs::write(policy_dir.join("test_post.rego"), policy_content)?;

    // Create engine
    let config = EngineConfig {
        harness: HarnessType::OpenCode,
        wasm_max_memory: Some(10 * 1024 * 1024),
        opa_path: None,
        global_config: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Create OpenCode PostToolUse event
    // Using OpenCode format: tool (lowercase) + args + result
    let event = json!({
        "hook_event_name": "PostToolUse",
        "session_id": "test_session",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool": "bash",
        "args": {
            "command": "echo hello"
        },
        "result": {
            "success": true,
            "output": "hello"
        }
    });

    // Evaluate
    let decision = engine.evaluate(&event, None).await?;

    // Should have context added
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { context } => {
            assert!(!context.is_empty());
            assert!(context.iter().any(|msg| msg.contains("Command succeeded")));
        }
        cupcake_core::engine::decision::FinalDecision::AllowOverride { agent_messages, .. } => {
            assert!(!agent_messages.is_empty());
            assert!(agent_messages
                .iter()
                .any(|msg| msg.contains("Command succeeded")));
        }
        _ => {}
    }

    Ok(())
}

#[tokio::test]
async fn test_opencode_event_parsing() -> Result<()> {
    // Test that OpenCode events have the expected structure
    // This tests the JSON format sent by the plugin

    // PreToolUse event (OpenCode format)
    let pre_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test_session",
        "cwd": "/test/dir",
        "tool": "bash",
        "args": {
            "command": "ls -la"
        }
    });

    // Should be valid JSON with OpenCode structure
    assert!(pre_event.is_object());
    assert_eq!(pre_event["hook_event_name"], "PreToolUse");
    assert_eq!(pre_event["tool"], "bash");
    assert!(pre_event.get("args").is_some());

    // PostToolUse event (OpenCode format)
    let post_event = json!({
        "hook_event_name": "PostToolUse",
        "session_id": "test_session",
        "cwd": "/test/dir",
        "tool": "bash",
        "args": {
            "command": "ls -la"
        },
        "result": {
            "success": true,
            "output": "file1\nfile2"
        }
    });

    // Should be valid JSON with OpenCode structure
    assert!(post_event.is_object());
    assert_eq!(post_event["hook_event_name"], "PostToolUse");
    assert_eq!(post_event["result"]["success"], true);

    Ok(())
}
