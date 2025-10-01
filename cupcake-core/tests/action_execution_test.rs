use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Convert a path to bash-compatible format (for use in shell scripts on Windows)
#[cfg(windows)]
fn path_for_bash(path: &PathBuf) -> String {
    let path_str = path.display().to_string();
    if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
        let drive = path_str.chars().next().unwrap().to_lowercase();
        let path_part = &path_str[2..].replace('\\', "/");
        format!("/{}{}", drive, path_part)
    } else {
        path_str.replace('\\', "/")
    }
}

#[cfg(not(windows))]
fn path_for_bash(path: &PathBuf) -> String {
    path.display().to_string()
}

/// Test that actions execute when a deny decision is triggered
#[tokio::test]
async fn test_action_execution_on_deny() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();

    // Create a marker file that the action will write to
    let action_marker = temp_dir.path().join("action_executed.txt");

    // Create action script for DENY-001 rule
    let action_script = format!(
        r#"#!/bin/bash
echo "Action executed for DENY-001" > {}
echo "Command was: $1" >> {}
date >> {}
"#,
        path_for_bash(&action_marker),
        path_for_bash(&action_marker),
        path_for_bash(&action_marker)
    );

    let action_path = actions_dir.join("DENY-001.sh");
    fs::write(&action_path, action_script).unwrap();

    // Make action executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&action_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&action_path, perms).unwrap();
    }

    // Create system evaluation policy
    create_system_policy(&system_dir);

    // Create test policy that denies specific commands
    let test_policy = r#"package cupcake.policies.test_deny

import rego.v1

# METADATA
# scope: rule
# title: Test Deny Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "dangerous")
    decision := {
        "reason": "Dangerous command blocked",
        "severity": "HIGH",
        "rule_id": "DENY-001"
    }
}
"#;

    fs::write(policies_dir.join("test_deny.rego"), test_policy).unwrap();

    // Initialize engine
    let engine = Engine::new(&project_path).await.unwrap();

    // Create event that will trigger the deny rule
    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf dangerous"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    // Evaluate - this should trigger the action
    let decision = engine.evaluate(&event, None).await.unwrap();

    // Verify we got a deny decision
    assert!(decision.is_blocking(), "Expected blocking decision");

    // Wait for the async action to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify the action executed by checking the marker file
    assert!(
        action_marker.exists(),
        "Action marker file not created - action did not execute"
    );

    let marker_content = fs::read_to_string(&action_marker).unwrap();
    assert!(
        marker_content.contains("Action executed for DENY-001"),
        "Action output incorrect: {marker_content}"
    );
}

/// Test that halt decisions trigger appropriate actions
#[tokio::test]
async fn test_action_execution_on_halt() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Setup directories
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();

    // Create action marker
    let halt_marker = temp_dir.path().join("halt_triggered.txt");

    // Create action for HALT-001
    let halt_action = format!(
        r#"#!/bin/bash
echo "EMERGENCY HALT ACTION TRIGGERED" > {}
echo "Timestamp: $(date)" >> {}
"#,
        path_for_bash(&halt_marker),
        path_for_bash(&halt_marker)
    );

    let action_path = actions_dir.join("HALT-001.sh");
    fs::write(&action_path, halt_action).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&action_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&action_path, perms).unwrap();
    }

    create_system_policy(&system_dir);

    // Create policy with halt rule
    let halt_policy = r#"package cupcake.policies.test_halt

import rego.v1

# METADATA
# scope: rule
# title: Test Halt Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

halt contains decision if {
    contains(input.tool_input.command, "rm -rf /")
    decision := {
        "reason": "CATASTROPHIC COMMAND - EMERGENCY HALT",
        "severity": "CRITICAL",
        "rule_id": "HALT-001"
    }
}
"#;

    fs::write(policies_dir.join("test_halt.rego"), halt_policy).unwrap();

    let engine = Engine::new(&project_path).await.unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();

    assert!(decision.is_halt(), "Expected halt decision");

    // Wait for async action
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    assert!(halt_marker.exists(), "Halt action did not execute");

    let content = fs::read_to_string(&halt_marker).unwrap();
    assert!(content.contains("EMERGENCY HALT ACTION TRIGGERED"));
}

/// Test that multiple actions can be triggered for a single rule
#[tokio::test]
async fn test_multiple_actions_per_rule() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();

    // Create guidebook with multiple actions for one rule
    let marker1 = temp_dir.path().join("cupcake_test_action1.txt");
    let marker2 = temp_dir.path().join("cupcake_test_action2.txt");
    let guidebook = format!(
        r#"
actions:
  by_rule_id:
    MULTI-001:
      - command: 'echo "First action" > {}'
      - command: 'echo "Second action" > {}'
"#,
        path_for_bash(&marker1),
        path_for_bash(&marker2)
    );

    fs::write(cupcake_dir.join("guidebook.yml"), guidebook).unwrap();

    create_system_policy(&system_dir);

    let multi_policy = r#"package cupcake.policies.test_multi

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "multi-trigger")
    decision := {
        "reason": "Multi-action test",
        "severity": "MEDIUM",
        "rule_id": "MULTI-001"
    }
}
"#;

    fs::write(policies_dir.join("test_multi.rego"), multi_policy).unwrap();

    let engine = Engine::new(&project_path).await.unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "multi-trigger test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    assert!(decision.is_blocking());

    // Wait for async actions
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify both actions executed
    let marker1 = temp_dir.path().join("cupcake_test_action1.txt");
    let marker2 = temp_dir.path().join("cupcake_test_action2.txt");
    assert!(marker1.exists(), "First action did not execute");
    assert!(marker2.exists(), "Second action did not execute");

    // No cleanup needed - TempDir handles it
}

/// Test general denial actions (on_any_denial)
#[tokio::test]
async fn test_on_any_denial_actions() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");

    fs::create_dir_all(&system_dir).unwrap();

    let general_marker = temp_dir.path().join("any_denial.txt");

    // Create guidebook with on_any_denial action
    let guidebook = format!(
        r#"
actions:
  on_any_denial:
    - command: 'echo "Some denial occurred" > {}'
"#,
        path_for_bash(&general_marker)
    );

    fs::write(cupcake_dir.join("guidebook.yml"), guidebook).unwrap();

    create_system_policy(&system_dir);

    // Policy with a different rule ID
    let policy = r#"package cupcake.policies.test_general

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "test")
    decision := {
        "reason": "Test denial",
        "severity": "LOW",
        "rule_id": "RANDOM-123"
    }
}
"#;

    fs::write(policies_dir.join("test_general.rego"), policy).unwrap();

    let engine = Engine::new(&project_path).await.unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "test command"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    assert!(decision.is_blocking());

    // Wait for async action
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    assert!(
        general_marker.exists(),
        "General denial action did not execute"
    );
}

// Helper function to create the system evaluation policy
fn create_system_policy(system_dir: &Path) {
    let system_policy = r#"package cupcake.system

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
"#;

    fs::write(system_dir.join("evaluate.rego"), system_policy).unwrap();
}
