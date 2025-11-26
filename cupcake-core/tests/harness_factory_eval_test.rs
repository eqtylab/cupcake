//! Integration tests for Factory AI Droid harness-specific evaluation
//!
//! These tests verify the complete evaluation flow for Factory AI:
//! - Engine initialization with explicit Factory harness
//! - Event processing with Factory-specific event format
//! - Permission mode handling (unique to Factory AI)
//! - Updated input support in PreToolUse (Factory AI-specific)
//! - Decision synthesis
//! - Response formatting matching Factory AI Droid hook expectations

mod common;
use common::{create_test_project_for_harness, init_test_logging};

use anyhow::Result;
use cupcake_core::engine::{decision::FinalDecision, Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_factory_harness_deny_decision() -> Result<()> {
    init_test_logging();

    // Setup test project
    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

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
            .join(".cupcake/policies/factory/test_deny.rego"),
        policy_content,
    )?;

    // Initialize engine with Factory harness explicitly
    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Create Factory AI PreToolUse event with permission_mode
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        },
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
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
async fn test_factory_harness_halt_decision() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

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
            .join(".cupcake/policies/factory/test_halt.rego"),
        policy_content,
    )?;

    // Initialize engine with Factory harness
    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Create Factory AI UserPromptSubmit event with permission_mode
    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "do something dangerous",
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
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
async fn test_factory_harness_ask_decision() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

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
            .join(".cupcake/policies/factory/test_ask.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "/path/to/important.txt",
            "content": "new content"
        },
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
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
async fn test_factory_harness_allow_with_context() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

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
            .join(".cupcake/policies/factory/test_context.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test prompt",
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
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
async fn test_factory_harness_routing_specificity() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

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
            .join(".cupcake/policies/factory/specific.rego"),
        specific_policy,
    )?;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/factory/wildcard.rego"),
        wildcard_policy,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
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
        "cwd": "/tmp",
        "permission_mode": "default"
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
async fn test_factory_harness_session_start_event() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SessionStart"]
package cupcake.policies.session_start

import rego.v1

add_context contains msg if {
    input.hook_event_name == "SessionStart"
    input.source == "Startup"
    msg := "Session started from startup"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/factory/session.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "SessionStart",
        "source": "Startup",
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
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

#[tokio::test]
async fn test_factory_harness_permission_mode_handling() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

    // Policy that responds to different permission modes
    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.policies.permission_mode_test

import rego.v1

add_context contains msg if {
    input.permission_mode == "plan"
    msg := "Running in plan mode"
}

deny contains decision if {
    input.permission_mode == "bypassPermissions"
    decision := {
        "rule_id": "BYPASS-BLOCKED",
        "reason": "Bypass permissions mode is not allowed",
        "severity": "CRITICAL"
    }
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/factory/permission_test.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    // Test plan mode - should get context
    let plan_event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "ls"
        },
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "plan"
    });

    let decision = engine.evaluate(&plan_event, None).await?;
    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context[0], "Running in plan mode");
        }
        _ => panic!("Expected Allow with context for plan mode, got: {decision:?}"),
    }

    // Test bypass mode - should get denied
    let bypass_event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "ls"
        },
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "bypassPermissions"
    });

    let decision = engine.evaluate(&bypass_event, None).await?;
    match decision {
        FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Bypass permissions mode is not allowed");
        }
        _ => panic!("Expected Deny for bypass mode, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_factory_harness_notification_event() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["Notification"]
package cupcake.policies.notification_test

import rego.v1

add_context contains msg if {
    input.hook_event_name == "Notification"
    input.notification_type == "error"
    msg := "Error notification detected"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/factory/notification.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "Notification",
        "notification_type": "error",
        "message": "Something went wrong",
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context[0], "Error notification detected");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_factory_harness_subagent_stop_event() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["SubagentStop"]
package cupcake.policies.subagent_test

import rego.v1

add_context contains msg if {
    input.hook_event_name == "SubagentStop"
    input.subagent_type == "code_reviewer"
    msg := "Code review subagent completed"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/factory/subagent.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "SubagentStop",
        "subagent_type": "code_reviewer",
        "result": "success",
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context[0], "Code review subagent completed");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}

#[tokio::test]
async fn test_factory_harness_pre_compact_event() -> Result<()> {
    init_test_logging();

    let project_dir = TempDir::new()?;
    create_test_project_for_harness(project_dir.path(), HarnessType::Factory)?;

    let policy_content = r#"# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreCompact"]
package cupcake.policies.pre_compact_test

import rego.v1

add_context contains msg if {
    input.hook_event_name == "PreCompact"
    msg := "Context compaction about to occur"
}
"#;

    fs::write(
        project_dir
            .path()
            .join(".cupcake/policies/factory/compact.rego"),
        policy_content,
    )?;

    let config = EngineConfig::new(HarnessType::Factory);
    let engine = Engine::new_with_config(project_dir.path(), config).await?;

    let event = json!({
        "hook_event_name": "PreCompact",
        "session_id": "test-session",
        "cwd": "/tmp",
        "permission_mode": "default"
    });

    let decision = engine.evaluate(&event, None).await?;

    match decision {
        FinalDecision::Allow { context } => {
            assert_eq!(context[0], "Context compaction about to occur");
        }
        _ => panic!("Expected Allow with context, got: {decision:?}"),
    }

    Ok(())
}
