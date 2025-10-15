//! Integration tests for Cursor harness-specific evaluation
//!
//! These tests verify the complete evaluation flow for Cursor:
//! - Engine initialization with explicit Cursor harness
//! - Event processing with Cursor-specific event format
//! - Decision synthesis
//! - Response formatting matching Cursor hook expectations

mod test_helpers;

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_cursor_harness_deny_shell_execution() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create a deny policy for dangerous commands
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.test_shell_deny

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")
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

    // Create Cursor-style event (normalized to PreToolUse with tool_name: Bash)
    // Cursor's beforeShellExecution maps to PreToolUse
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        },
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
        _ => panic!("Expected Deny decision, got: {:?}", decision),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_halt_on_prompt() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create a halt policy for prompts
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.test_prompt_halt

import rego.v1

halt contains decision if {
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

    // Cursor beforeSubmitPrompt -> UserPromptSubmit
    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "do something dangerous",
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Halt { reason, .. } => {
            assert_eq!(reason, "Dangerous prompt blocked");
        }
        _ => panic!("Expected Halt decision, got: {:?}", decision),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_ask_file_read() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create an ask policy for file reads
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Read"]
package cupcake.policies.test_file_read_ask

import rego.v1

ask contains decision if {
    input.tool_name == "Read"
    contains(input.tool_input.file_path, ".env")
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

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Cursor beforeReadFile -> PreToolUse with tool_name: Read
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": "/home/user/.env"
        },
        "conversation_id": "cursor-conv-123",
        "generation_id": "cursor-gen-456",
        "workspace_roots": ["/home/user/project"]
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Ask { reason, .. } => {
            assert_eq!(reason, "Accessing sensitive file");
        }
        _ => panic!("Expected Ask decision, got: {:?}", decision),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_context_injection_limitations() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create a context injection policy
    // Note: Cursor has limited context injection support compared to Claude Code
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.test_context

import rego.v1

add_context contains msg if {
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
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test prompt",
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
        _ => panic!("Expected Allow with context, got: {:?}", decision),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_mcp_execution() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create policy for MCP execution (Cursor-specific feature)
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["MCP"]
package cupcake.policies.test_mcp

import rego.v1

deny contains decision if {
    input.tool_name == "MCP"
    input.tool_input.server_name == "untrusted-server"
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

    // Cursor beforeMCPExecution -> PreToolUse with tool_name: MCP
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "MCP",
        "tool_input": {
            "server_name": "untrusted-server",
            "tool_name": "dangerous_tool"
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
        _ => panic!("Expected Deny decision, got: {:?}", decision),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_file_edit_post_hook() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create policy for file edit validation (post-hook)
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PostToolUse"]
#     required_tools: ["Edit"]
package cupcake.policies.test_edit_validation

import rego.v1

add_context contains msg if {
    input.tool_name == "Edit"
    input.tool_input.file_path
    msg := concat("", ["File edited: ", input.tool_input.file_path])
}
"#;

    fs::write(
        project_dir.path().join(".cupcake/policies/cursor/edit.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Cursor);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Cursor afterFileEdit -> PostToolUse with tool_name: Edit
    let event = json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "/home/user/project/src/main.rs",
            "old_string": "old",
            "new_string": "new"
        },
        "tool_response": {
            "success": true
        },
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
        _ => panic!("Expected Allow with context, got: {:?}", decision),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_harness_wildcard_routing() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create wildcard policy (matches any tool)
    let wildcard_policy = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.policies.wildcard_audit

import rego.v1

add_context contains msg if {
    msg := concat("", ["Audit: ", input.tool_name, " tool used"])
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

    // Test with different tools - wildcard should match all
    let tools = vec!["Bash", "Read", "Write", "Edit", "MCP"];

    for tool in tools {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": tool,
            "tool_input": {
                "command": "test"
            },
            "conversation_id": "cursor-conv-123",
            "generation_id": "cursor-gen-456",
            "workspace_roots": ["/home/user/project"]
        });

        let decision = engine.evaluate(&event, None).await?;

        match decision {
            FinalDecision::Allow { context } => {
                assert_eq!(context.len(), 1);
                assert_eq!(context[0], format!("Audit: {} tool used", tool));
            }
            _ => panic!("Expected Allow with context for {}, got: {:?}", tool, decision),
        }
    }

    Ok(())
}
