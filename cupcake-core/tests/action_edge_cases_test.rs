use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test that the new Path::is_file() logic correctly handles various edge cases
///
/// Skipped on Windows: Tests Unix-specific paths (/bin/echo) and shell behavior
#[tokio::test]
#[cfg(not(windows))]
async fn test_action_execution_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir).unwrap();

    // Create system policy
    create_system_policy(&system_dir);

    // Test markers for verification
    let shell_marker = temp_dir.path().join("shell_executed.txt");
    let relative_marker = temp_dir.path().join("relative_executed.txt");
    let args_marker = temp_dir.path().join("args_executed.txt");
    let python_marker = temp_dir.path().join("python_executed.txt");

    // Create rulebook with various command types
    let rulebook = format!(
        r#"
actions:
  by_rule_id:
    EDGE-001:
      # Case 1: Shell command with redirect (should use sh -c)
      - command: 'echo "shell command" > {}'
      # Case 2: Absolute path with arguments (should use sh -c) 
      - command: '/bin/echo "args test" > {}'
      # Case 3: Python command (should use sh -c)
      - command: 'echo "python test" > {}'
    EDGE-002:
      # Case 4: Relative script path (should resolve and execute as script if exists)
      - command: './test_relative.sh'
"#,
        shell_marker.display(),
        args_marker.display(),
        python_marker.display()
    );

    fs::write(cupcake_dir.join("rulebook.yml"), rulebook).unwrap();

    // Create a relative script that should be found and executed directly
    let scripts_dir = project_path.join(".");
    let relative_script = format!(
        r#"#!/bin/bash
echo "relative script" > {}
"#,
        relative_marker.display()
    );

    let relative_script_path = scripts_dir.join("test_relative.sh");
    fs::write(&relative_script_path, relative_script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&relative_script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&relative_script_path, perms).unwrap();
    }

    // Create test policy that triggers the actions
    let edge_policy = r#"package cupcake.policies.edge_test

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "edge-test")
    decision := {
        "reason": "Testing edge cases",
        "severity": "MEDIUM",
        "rule_id": "EDGE-001"
    }
}

deny contains decision if {
    contains(input.tool_input.command, "relative-test")
    decision := {
        "reason": "Testing relative script",
        "severity": "MEDIUM", 
        "rule_id": "EDGE-002"
    }
}
"#;

    fs::write(claude_dir.join("edge_test.rego"), edge_policy).unwrap();

    let engine = Engine::new(
        &project_path,
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )
    .await
    .unwrap();

    // Test Case 1-3: Shell commands
    let event1 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "edge-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision1 = engine.evaluate(&event1, None).await.unwrap();
    assert!(decision1.is_blocking(), "Expected blocking decision");

    // Wait for async actions to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify shell commands executed via sh -c
    assert!(
        shell_marker.exists(),
        "Shell command with redirect should have executed via sh -c"
    );
    assert!(
        args_marker.exists(),
        "Command with arguments should have executed via sh -c"
    );
    assert!(
        python_marker.exists(),
        "Python-style command should have executed via sh -c"
    );

    // Test Case 4: Relative script path
    let event2 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "relative-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision2 = engine.evaluate(&event2, None).await.unwrap();
    assert!(decision2.is_blocking(), "Expected blocking decision");

    // Wait for async action
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Verify relative script executed directly (not via sh -c)
    assert!(
        relative_marker.exists(),
        "Relative script should have been resolved and executed directly: {relative_script_path:?}"
    );

    // Verify contents to ensure correct execution
    let shell_content = fs::read_to_string(&shell_marker).unwrap();
    assert!(shell_content.contains("shell command"));

    let args_content = fs::read_to_string(&args_marker).unwrap();
    assert!(args_content.contains("args test"));

    let python_content = fs::read_to_string(&python_marker).unwrap();
    assert!(python_content.contains("python test"));

    let relative_content = fs::read_to_string(&relative_marker).unwrap();
    assert!(relative_content.contains("relative script"));
}

/// Test that non-existent script paths fallback to shell execution
///
/// Skipped on Windows: Tests Unix shell fallback behavior
#[tokio::test]
#[cfg(not(windows))]
async fn test_nonexistent_script_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir).unwrap();
    create_system_policy(&system_dir);

    let fallback_marker = temp_dir.path().join("fallback_executed.txt");

    // Create rulebook with a path that looks like a script but doesn't exist
    let rulebook = format!(
        r#"
actions:
  by_rule_id:
    FALLBACK-001:
      # This looks like a script path but doesn't exist - should fallback to sh -c
      - command: 'echo "fallback test" > {}'
"#,
        fallback_marker.display()
    );

    fs::write(cupcake_dir.join("rulebook.yml"), rulebook).unwrap();

    let fallback_policy = r#"package cupcake.policies.fallback_test

import rego.v1

# METADATA  
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "fallback-test")
    decision := {
        "reason": "Testing fallback",
        "severity": "MEDIUM",
        "rule_id": "FALLBACK-001"
    }
}
"#;

    fs::write(claude_dir.join("fallback_test.rego"), fallback_policy).unwrap();

    let engine = Engine::new(
        &project_path,
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )
    .await
    .unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "fallback-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    assert!(decision.is_blocking());

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Should have executed via shell fallback
    assert!(
        fallback_marker.exists(),
        "Non-existent script path should fallback to shell execution"
    );

    let content = fs::read_to_string(&fallback_marker).unwrap();
    assert!(content.contains("fallback test"));
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
        "modifications": collect_verbs("modify"),
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
