//! Integration tests for Cursor harness-specific evaluation
//!
//! These tests verify the complete evaluation flow for Cursor using NATIVE event formats:
//! - Engine initialization with explicit Cursor harness
//! - Event processing with Cursor-native event schema (beforeShellExecution, afterFileEdit, etc.)
//! - Decision synthesis
//! - Response formatting matching Cursor hook expectations
//!
//! IMPORTANT: These tests use Cursor's native event structure, NOT Claude Code normalized events.
//! Cursor events have different names and field structures than Claude Code.

mod common;
use common::{create_test_project_for_harness, init_test_logging};

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_cursor_harness_deny_shell_execution() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create a deny policy for dangerous commands using Cursor's native event structure
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.test_shell_deny

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    # Access command directly (Cursor's native field structure)
    contains(input.command, "rm -rf /")
    decision := {
        "rule_id": "CURSOR-SHELL-DENY-001",
        "reason": "Dangerous command blocked",
        "severity": "CRITICAL"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/shell_deny.rego"),
        policy_content,
    )?;

    // Initialize engine with Cursor harness
    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Create Cursor's native beforeShellExecution event
    let event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "rm -rf /",
        "cwd": "/home/user/project",
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Deny decision
    match decision {
        FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Dangerous command blocked");
        }
        _ => panic!("Expected Deny decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_halt_on_prompt() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create a halt policy for prompts using Cursor's native event
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeSubmitPrompt"]
package cupcake.policies.test_prompt_halt

import rego.v1

halt contains decision if {
    input.hook_event_name == "beforeSubmitPrompt"
    # Access prompt directly (Cursor's native field)
    contains(input.prompt, "dangerous")
    decision := {
        "rule_id": "CURSOR-PROMPT-HALT-001",
        "reason": "Dangerous prompt blocked",
        "severity": "CRITICAL"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/prompt_halt.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Cursor's native beforeSubmitPrompt event
    let event = json!({
        "hook_event_name": "beforeSubmitPrompt",
        "prompt": "do something dangerous",
        "attachments": [],
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Halt { reason, .. } => {
            assert_eq!(reason, "Dangerous prompt blocked");
        }
        _ => panic!("Expected Halt decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_ask_file_read() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create an ask policy for file reads using Cursor's native event
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeReadFile"]
package cupcake.policies.test_file_read_ask

import rego.v1

ask contains decision if {
    input.hook_event_name == "beforeReadFile"
    # Access file_path directly (Cursor's native field)
    contains(input.file_path, ".env")
    decision := {
        "rule_id": "CURSOR-READ-ASK-001",
        "reason": "Accessing sensitive file",
        "question": "Are you sure you want to read .env file?",
        "severity": "MEDIUM"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/file_read.rego"),
        policy_content,
    )?;

    // Disable global config to avoid interference from global builtins
    let empty_global = TempDir::new()?;
    let config = EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: HarnessType::Cursor,
        wasm_max_memory: None,
        opa_path: None,
        skip_global_config: false,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Cursor's native beforeReadFile event
    let event = json!({
        "hook_event_name": "beforeReadFile",
        "file_path": "/home/user/.env",
        "content": "SECRET_KEY=abc123\nAPI_TOKEN=xyz789",
        "attachments": [],
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Ask { reason, .. } => {
            assert_eq!(reason, "Accessing sensitive file");
        }
        _ => panic!("Expected Ask decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_context_injection_limitations() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create a context injection policy
    // Note: Cursor has limited context injection support compared to Claude Code
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeSubmitPrompt"]
package cupcake.policies.test_context

import rego.v1

add_context contains msg if {
    input.hook_event_name == "beforeSubmitPrompt"
    input.prompt
    msg := "This is context from Cupcake"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/context.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "beforeSubmitPrompt",
        "prompt": "test prompt",
        "attachments": [],
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Allow with context
    // Note: Cursor's response builder may drop context if not supported by event type
    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context.len(), 1);
            assert_eq!(context[0], "This is context from Cupcake");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_mcp_execution() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create policy for MCP execution using Cursor's native event
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeMCPExecution"]
package cupcake.policies.test_mcp

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    # Access Cursor's MCP fields directly
    input.server_name == "untrusted-server"
    decision := {
        "rule_id": "CURSOR-MCP-DENY-001",
        "reason": "Untrusted MCP server blocked",
        "severity": "HIGH"
    }
}
"#;

    fs::write(
        project_dir.path().join(".cupcake/policies/cursor/mcp.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Cursor's native beforeMCPExecution event
    let event = json!({
        "hook_event_name": "beforeMCPExecution",
        "server_name": "untrusted-server",
        "tool_name": "dangerous_tool",
        "tool_input": {
            "param1": "value1"
        },
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Untrusted MCP server blocked");
        }
        _ => panic!("Expected Deny decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_file_edit_post_hook() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create policy for file edit validation using Cursor's native post-hook event
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["afterFileEdit"]
package cupcake.policies.test_edit_validation

import rego.v1

add_context contains msg if {
    input.hook_event_name == "afterFileEdit"
    # Access file_path directly (Cursor's native field)
    input.file_path
    msg := concat("", ["File edited: ", input.file_path])
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/edit.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Cursor's native afterFileEdit event
    let event = json!({
        "hook_event_name": "afterFileEdit",
        "file_path": "/home/user/project/src/main.rs",
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context.len(), 1);
            assert_eq!(context[0], "File edited: /home/user/project/src/main.rs");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_wildcard_routing() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create wildcard policy that matches all beforeShellExecution events
    let wildcard_policy = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.wildcard_audit

import rego.v1

add_context contains msg if {
    input.hook_event_name == "beforeShellExecution"
    msg := concat("", ["Audit: shell command '", input.command, "' executed"])
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/wildcard.rego"),
        wildcard_policy,
    )?;

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Test with different commands - wildcard should match all shell executions
    let commands = vec!["ls -la", "git status", "npm install", "cargo build"];

    for command in commands {
        let event = json!({
            "hook_event_name": "beforeShellExecution",
            "command": command,
            "cwd": "/home/user/project",
            "conversation_id": "cursor-conv-123",
            "generation_id": "cursor-gen-456",
            "workspace_roots": ["/home/user/project"]
        });

        let decision = engine.evaluate(&event, None).await?;

        match decision {
            FinalDecision::Allow { context } => {
                assert_eq!(context.len(), 1);
                assert_eq!(
                    context[0],
                    format!("Audit: shell command '{command}' executed")
                );
            }
            _ => panic!("Expected Allow with context for '{command}', got: {decision:?}"),
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_separate_user_agent_messages() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Cursor)?;

    // Create policy that uses agent_context for Cursor's dual-message capability
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.test_dual_message

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "sudo")
    decision := {
        "rule_id": "CURSOR-SUDO-DENY-001",
        "reason": "Sudo commands are not allowed",
        "agent_context": "The command contains 'sudo' which requires elevated privileges. Consider using Docker containers or adjusting file permissions instead. See security policy CURSOR-SUDO-DENY-001 for details.",
        "severity": "HIGH"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/cursor/dual_message.rego"),
        policy_content,
    )?;

    // Disable global config to avoid interference from global builtins
    let empty_global = TempDir::new()?;
    let config = EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: HarnessType::Cursor,
        wasm_max_memory: None,
        opa_path: None,
        skip_global_config: false,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": "sudo apt update",
        "cwd": "/home/user/project",
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Deny decision with agent messages
    match decision {
        FinalDecision::Deny {
            reason,
            agent_messages,
        } => {
            assert_eq!(reason, "Sudo commands are not allowed");
            assert_eq!(agent_messages.len(), 1);
            assert!(agent_messages[0].contains("elevated privileges"));
            assert!(agent_messages[0].contains("Docker containers"));
        }
        _ => panic!("Expected Deny decision, got: {decision:?}"),
    }

    Ok(())
}
