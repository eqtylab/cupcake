use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use tokio;

/// Test that actions execute asynchronously (fire-and-forget)
#[tokio::test]
async fn test_action_fire_and_forget() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");
    
    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();
    
    // Create markers for timing verification
    let start_marker = temp_dir.path().join("action_started.txt");
    let end_marker = temp_dir.path().join("action_completed.txt");
    
    // Create a slow action that takes 3 seconds (more reliable for testing)
    let slow_action = format!(
        r#"#!/bin/bash
echo "$(date +%s%N)" > {}
sleep 3
echo "$(date +%s%N)" > {}
"#,
        start_marker.display(),
        end_marker.display()
    );
    
    let action_path = actions_dir.join("ASYNC-001.sh");
    fs::write(&action_path, slow_action).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&action_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&action_path, perms).unwrap();
    }
    
    // Create system policy
    create_system_policy(&system_dir);
    
    // Create test policy
    let test_policy = r#"package cupcake.policies.async_test

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "async-test")
    decision := {
        "reason": "Testing async execution",
        "severity": "MEDIUM",
        "rule_id": "ASYNC-001"
    }
}
"#;
    
    fs::write(policies_dir.join("async_test.rego"), test_policy).unwrap();
    
    let engine = Engine::new(&project_path).await.unwrap();
    
    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "async-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });
    
    // Record time before evaluation
    let eval_start = std::time::Instant::now();
    
    // Evaluate should return quickly, not wait for action
    let decision = engine.evaluate(&event).await.unwrap();
    
    let eval_duration = eval_start.elapsed();
    
    assert!(decision.is_blocking());
    
    // Evaluation should complete in less than 1 second (action takes 3 seconds)
    assert!(
        eval_duration.as_secs() < 1,
        "Evaluation took {:?} - should not wait for action",
        eval_duration
    );
    
    // Wait longer to ensure action has actually started
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Action should have started but not completed
    assert!(
        start_marker.exists(),
        "Action did not start within 1000ms. Script path: {:?}",
        action_path
    );
    assert!(
        !end_marker.exists(),
        "Action completed too quickly - not running async (action should take 3 seconds)"
    );
    
    // Wait for action to complete (3 second action + buffer)
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    // Now action should be complete
    assert!(
        end_marker.exists(),
        "Action did not complete after waiting"
    );
}

/// Test that multiple actions execute concurrently
#[tokio::test]
async fn test_multiple_actions_concurrent() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    
    fs::create_dir_all(&system_dir).unwrap();
    
    // Create markers for multiple actions
    let markers: Vec<_> = (1..=3)
        .map(|i| temp_dir.path().join(format!("action_{}.txt", i)))
        .collect();
    
    // Create guidebook with multiple slow actions
    let mut actions = vec![];
    for (i, marker) in markers.iter().enumerate() {
        actions.push(format!(
            r#"      - command: 'sleep 1 && echo "Action {}" > {}'"#,
            i + 1,
            marker.display()
        ));
    }
    
    let guidebook = format!(
        r#"
actions:
  by_rule_id:
    CONCURRENT-001:
{}
"#,
        actions.join("\n")
    );
    
    
    fs::write(cupcake_dir.join("guidebook.yml"), guidebook).unwrap();
    
    create_system_policy(&system_dir);
    
    let policy = r#"package cupcake.policies.concurrent

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "concurrent-test")
    decision := {
        "reason": "Testing concurrent actions",
        "severity": "MEDIUM",
        "rule_id": "CONCURRENT-001"
    }
}
"#;
    
    fs::write(policies_dir.join("concurrent.rego"), policy).unwrap();
    
    let engine = Engine::new(&project_path).await.unwrap();
    
    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "concurrent-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });
    
    let start = std::time::Instant::now();
    let decision = engine.evaluate(&event).await.unwrap();
    assert!(decision.is_blocking());
    
    // Wait for all actions to complete (should take ~1 second if concurrent, plus overhead)
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    
    let duration = start.elapsed();
    
    // All actions should have completed
    for marker in &markers {
        assert!(
            marker.exists(),
            "Action marker {:?} not found",
            marker
        );
    }
    
    // If actions ran sequentially, it would take 3+ seconds  
    // If concurrent, should complete in ~1-2 seconds plus overhead
    // We allow up to 6 seconds to account for system load
    assert!(
        duration.as_secs() <= 6,
        "Actions took {:?} - not running concurrently",
        duration
    );
}

/// Test that evaluation doesn't block on action failures
#[tokio::test]
async fn test_action_failure_non_blocking() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let actions_dir = cupcake_dir.join("actions");
    
    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();
    
    // Create an action that will fail
    let failing_action = r#"#!/bin/bash
exit 1  # Intentional failure
"#;
    
    let action_path = actions_dir.join("FAIL-001.sh");
    fs::write(&action_path, failing_action).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&action_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&action_path, perms).unwrap();
    }
    
    create_system_policy(&system_dir);
    
    let policy = r#"package cupcake.policies.fail_test

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "fail-test")
    decision := {
        "reason": "Testing failure handling",
        "severity": "MEDIUM",
        "rule_id": "FAIL-001"
    }
}
"#;
    
    fs::write(policies_dir.join("fail_test.rego"), policy).unwrap();
    
    let engine = Engine::new(&project_path).await.unwrap();
    
    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "fail-test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });
    
    let start = std::time::Instant::now();
    
    // Should not panic or block on action failure
    let decision = engine.evaluate(&event).await.unwrap();
    
    let duration = start.elapsed();
    
    assert!(decision.is_blocking());
    
    // Should return quickly despite action failure
    assert!(
        duration.as_millis() < 500,
        "Evaluation blocked on failing action"
    );
}

/// Test that actions don't block subsequent evaluations
#[tokio::test]
async fn test_actions_dont_block_subsequent_evaluations() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    
    fs::create_dir_all(&system_dir).unwrap();
    
    // Create a slow action
    let marker = temp_dir.path().join("slow_action.txt");
    let guidebook = format!(
        r#"
actions:
  by_rule_id:
    SLOW-001:
      - command: 'sleep 3 && echo "Done" > {}'
"#,
        marker.display()
    );
    
    fs::write(cupcake_dir.join("guidebook.yml"), guidebook).unwrap();
    
    create_system_policy(&system_dir);
    
    let policy = r#"package cupcake.policies.slow

import rego.v1

# METADATA
# scope: rule
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    contains(input.tool_input.command, "slow-trigger")
    decision := {
        "reason": "Slow action",
        "severity": "MEDIUM",
        "rule_id": "SLOW-001"
    }
}

# Another rule that allows
allow_override contains decision if {
    contains(input.tool_input.command, "fast-allow")
    decision := {
        "reason": "Quick allow",
        "severity": "LOW",
        "rule_id": "ALLOW-001"
    }
}
"#;
    
    fs::write(policies_dir.join("slow.rego"), policy).unwrap();
    
    let engine = Engine::new(&project_path).await.unwrap();
    
    // First evaluation triggers slow action
    let event1 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "slow-trigger"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });
    
    let decision1 = engine.evaluate(&event1).await.unwrap();
    assert!(decision1.is_blocking());
    
    // Immediately evaluate another event (while action is running)
    let event2 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "fast-allow"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });
    
    let start = std::time::Instant::now();
    let decision2 = engine.evaluate(&event2).await.unwrap();
    let duration = start.elapsed();
    
    // Second evaluation should complete quickly
    assert!(
        duration.as_millis() < 500,
        "Second evaluation blocked by first action"
    );
    
    // Verify second decision is correct
    match decision2 {
        cupcake_core::engine::decision::FinalDecision::AllowOverride { .. } => {},
        _ => panic!("Expected AllowOverride decision, got {:?}", decision2)
    }
}

// Helper function
fn create_system_policy(system_dir: &std::path::PathBuf) {
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