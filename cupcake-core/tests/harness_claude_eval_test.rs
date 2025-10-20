//! Integration tests for Claude Code harness-specific evaluation
//!
//! These tests verify the complete evaluation flow for Claude Code:
//! - Engine initialization with explicit ClaudeCode harness
//! - Event processing with Claude-specific event format
//! - Decision synthesis
//! - Response formatting matching Claude Code hook expectations

mod test_helpers;

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_claude_harness_deny_decision() -> Result<()> {
    test_helpers::init_test_logging();

    // Setup test project
    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create a deny policy for testing
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.test_deny

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")
    decision := {
        "rule_id": "TEST-DENY-001",
        "reason": "Dangerous command blocked",
        "severity": "CRITICAL"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/test_deny.rego"),
        policy_content,
    )?;

    // Initialize engine with ClaudeCode harness explicitly
    let config = EngineConfig::new(HarnessType::ClaudeCode);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Create Claude Code PreToolUse event
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        },
        "session_id": "test-session",
        "cwd": "/tmp"
    });

    // Evaluate the event
    let decision = engine.evaluate(&event, None).await?;

    // Verify we got a Deny decision
    match decision {
        FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Dangerous command blocked");
        }
        _ => panic!("Expected Deny decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_claude_harness_halt_decision() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create a halt policy
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.test_halt

import rego.v1

halt contains decision if {
    contains(input.prompt, "dangerous")
    decision := {
        "rule_id": "TEST-HALT-001",
        "reason": "Dangerous prompt blocked",
        "severity": "CRITICAL"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/test_halt.rego"),
        policy_content,
    )?;

    // Initialize engine with ClaudeCode harness
    let config = EngineConfig::new(HarnessType::ClaudeCode);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Create Claude Code UserPromptSubmit event
    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "do something dangerous",
        "session_id": "test-session",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Halt decision
    match decision {
        FinalDecision::Halt { reason, .. } => {
            assert_eq!(reason, "Dangerous prompt blocked");
        }
        _ => panic!("Expected Halt decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_claude_harness_ask_decision() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create an ask policy
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write"]
package cupcake.policies.test_ask

import rego.v1

ask contains decision if {
    input.tool_name == "Write"
    contains(input.tool_input.file_path, "important")
    decision := {
        "rule_id": "TEST-ASK-001",
        "reason": "Modifying important file",
        "question": "Are you sure you want to modify this important file?",
        "severity": "MEDIUM"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/test_ask.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::ClaudeCode);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "/path/to/important.txt",
            "content": "new content"
        },
        "session_id": "test-session",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Ask decision
    match decision {
        FinalDecision::Ask { reason, .. } => {
            assert_eq!(reason, "Modifying important file");
        }
        _ => panic!("Expected Ask decision, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_claude_harness_allow_with_context() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create a context injection policy
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.test_context

import rego.v1

add_context contains msg if {
    input.prompt
    msg := "This is additional context from Cupcake"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/test_context.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::ClaudeCode);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test prompt",
        "session_id": "test-session",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Allow with context
    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context.len(), 1);
            assert_eq!(context[0], "This is additional context from Cupcake");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_claude_harness_routing_specificity() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    // Create specific tool policy
    let specific_policy = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit"]
package cupcake.policies.specific_edit

import rego.v1

deny contains decision if {
    input.tool_name == "Edit"
    decision := {
        "rule_id": "SPECIFIC-EDIT-001",
        "reason": "Specific Edit policy fired",
        "severity": "HIGH"
    }
}
"#;

    // Create wildcard policy (only event, no tool)
    let wildcard_policy = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.policies.wildcard

import rego.v1

add_context contains msg if {
    msg := "Wildcard policy also sees this event"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/specific.rego"),
        specific_policy,
    )?;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/wildcard.rego"),
        wildcard_policy,
    )?;

    let config = EngineConfig::new(HarnessType::ClaudeCode);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Send Edit tool event - should match both specific and wildcard policies
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "/test.txt",
            "old_string": "old",
            "new_string": "new"
        },
        "session_id": "test-session",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await?;

    // Verify Deny from specific policy won (higher priority than context)
    match decision {
        FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Specific Edit policy fired");
        }
        _ => panic!("Expected Deny from specific policy, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_claude_harness_session_start_event() -> Result<()> {
    test_helpers::init_test_logging();

    let project_dir = TempDir::new()?;
    test_helpers::create_test_project(project_dir.path())?;

    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.policies.session_start

import rego.v1

add_context contains msg if {
    input.hook_event_name == "SessionStart"
    input.source == "startup"
    msg := "Session started from startup"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/claude/session.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::ClaudeCode);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "SessionStart",
        "source": "startup",
        "session_id": "test-session",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context.len(), 1);
            assert_eq!(context[0], "Session started from startup");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}
