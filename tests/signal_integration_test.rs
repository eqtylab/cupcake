use cupcake_rego::engine::Engine;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_end_to_end_signal_integration() {
    // Create a temporary test project structure
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    // Create .cupcake directory structure
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let signals_dir = cupcake_dir.join("signals");
    let actions_dir = cupcake_dir.join("actions");
    
    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&signals_dir).unwrap();
    fs::create_dir_all(&actions_dir).unwrap();
    
    // Create system evaluation policy
    let system_policy = r#"package cupcake.system

import rego.v1

# METADATA
# scope: rule
# title: System Aggregation Policy
# authors: ["Cupcake Engine"]

# Collect all decision verbs from the policy hierarchy
# Uses walk() for automatic policy discovery

halts := collect_verbs("halt")
denies := collect_verbs("deny") 
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

# Single evaluation entrypoint for the engine
evaluate := {
    "halts": halts,
    "denies": denies,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
}

# Collect all instances of a decision verb across all policies
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
"#;
    
    fs::write(system_dir.join("evaluate.rego"), system_policy).unwrap();
    
    // Create test policy that uses both string and structured signals
    let test_policy = r#"package cupcake.policies.integration_test

import rego.v1

# METADATA
# scope: rule
# title: Integration Test Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch", "test_status"]

# Rule using string signal access
deny contains decision if {
    input.signals.git_branch == "main"
    contains(input.tool_input.command, "dangerous_command")
    
    decision := {
        "reason": "Cannot run dangerous commands on main branch",
        "severity": "HIGH",
        "rule_id": "TEST-001"
    }
}

# Rule using structured signal access
ask contains decision if {
    input.signals.test_status.passing == false
    input.signals.test_status.coverage < 90
    contains(input.tool_input.command, "deploy")
    
    decision := {
        "reason": sprintf("Deploy with failing tests and %.1f%% coverage?", [input.signals.test_status.coverage]),
        "severity": "MEDIUM", 
        "rule_id": "TEST-002"
    }
}

# Rule using array access from structured signal
deny contains decision if {
    count(input.signals.test_status.failed_tests) > 3
    contains(input.tool_input.command, "release")
    
    decision := {
        "reason": sprintf("Cannot release with %d failing tests", [count(input.signals.test_status.failed_tests)]),
        "severity": "HIGH",
        "rule_id": "TEST-003"
    }
}

# Context using both signal types
add_context contains sprintf("Branch: %s, Test Coverage: %.1f%%", 
                            [input.signals.git_branch, input.signals.test_status.coverage]) if {
    input.signals.git_branch
    input.signals.test_status.coverage
}
"#;
    
    fs::write(policies_dir.join("integration_test.rego"), test_policy).unwrap();
    
    // Create string signal (git_branch) - outputs JSON string
    let git_branch_signal = r#"#!/bin/bash
echo '"main"'
"#;
    
    let git_branch_path = signals_dir.join("git_branch.sh");
    fs::write(&git_branch_path, git_branch_signal).unwrap();
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&git_branch_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&git_branch_path, perms).unwrap();
    }
    
    // Create structured signal (test_status) - outputs JSON object
    let test_status_signal = r#"#!/bin/bash
echo '{
  "passing": false,
  "coverage": 85.5,
  "duration": 45.2,
  "failed_tests": ["test_a", "test_b", "test_c", "test_d"],
  "environment": "ci"
}'
"#;
    
    let test_status_path = signals_dir.join("test_status.sh");
    fs::write(&test_status_path, test_status_signal).unwrap();
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&test_status_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&test_status_path, perms).unwrap();
    }
    
    // Initialize engine with test project
    let mut engine = Engine::new(&project_path).await.unwrap();
    
    // Test 1: String signal access (git_branch == "main")
    let event1 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "dangerous_command --force"
        },
        "session_id": "test-1",
        "cwd": "/tmp"
    });
    
    let decision1 = engine.evaluate(&event1).await.unwrap();
    assert!(decision1.is_blocking(), "Should deny dangerous command on main branch");
    if let Some(reason) = decision1.reason() {
        assert!(reason.contains("Cannot run dangerous commands on main branch"));
    }
    
    // Test 2: Structured signal access (test_status.passing == false, coverage < 90)
    let event2 = json!({
        "hookEventName": "PreToolUse", 
        "tool_name": "Bash",
        "tool_input": {
            "command": "deploy --production"
        },
        "session_id": "test-2",
        "cwd": "/tmp"
    });
    
    let decision2 = engine.evaluate(&event2).await.unwrap();
    assert!(decision2.requires_confirmation(), "Should ask for confirmation on deploy with failing tests");
    if let Some(reason) = decision2.reason() {
        assert!(reason.contains("Deploy with failing tests"));
        assert!(reason.contains("85.5%")); // Should include coverage from structured signal
    }
    
    // Test 3: Array access from structured signal (failed_tests count > 3)
    let event3 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash", 
        "tool_input": {
            "command": "release v1.0.0"
        },
        "session_id": "test-3",
        "cwd": "/tmp"
    });
    
    let decision3 = engine.evaluate(&event3).await.unwrap();
    assert!(decision3.is_blocking(), "Should deny release with too many failing tests");
    if let Some(reason) = decision3.reason() {
        assert!(reason.contains("Cannot release with 4 failing tests")); // 4 tests in our mock data
    }
    
    // Test 4: Allow case with context injection
    let event4 = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "ls -la"
        },
        "session_id": "test-4", 
        "cwd": "/tmp"
    });
    
    let decision4 = engine.evaluate(&event4).await.unwrap();
    assert!(!decision4.is_blocking(), "Should allow safe command");
    
    // Check context injection using both signal types
    if let cupcake_rego::engine::decision::FinalDecision::Allow { context } = decision4 {
        let context_str = context.join(" ");
        assert!(context_str.contains("Branch: main")); // String signal
        assert!(context_str.contains("Test Coverage: 85.5%")); // Structured signal numeric field
    } else {
        panic!("Expected Allow decision with context");
    }
}

#[tokio::test]
async fn test_signal_json_parsing_fallback() {
    // Create a temporary test project
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    
    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
    let signals_dir = cupcake_dir.join("signals");
    
    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&signals_dir).unwrap();
    
    // Create minimal system policy
    let system_policy = r#"package cupcake.system

import rego.v1

# METADATA
# scope: rule
# title: System Aggregation Policy
# authors: ["Cupcake Engine"]

halts := collect_verbs("halt")
denies := collect_verbs("deny") 
blocks := collect_verbs("block")
asks := collect_verbs("ask")
allow_overrides := collect_verbs("allow_override")
add_context := collect_verbs("add_context")

evaluate := {
    "halts": halts,
    "denies": denies,
    "blocks": blocks,
    "asks": asks,
    "allow_overrides": allow_overrides,
    "add_context": add_context
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
"#;
    fs::write(system_dir.join("evaluate.rego"), system_policy).unwrap();
    
    // Create policy that accesses a signal as string (for invalid JSON fallback test)
    let test_policy = r#"package cupcake.policies.fallback_test

import rego.v1

# METADATA
# scope: rule
# title: Fallback Test Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["invalid_json_signal"]

add_context contains sprintf("Signal output: %s", [input.signals.invalid_json_signal]) if {
    input.signals.invalid_json_signal
}
"#;
    
    fs::write(policies_dir.join("fallback_test.rego"), test_policy).unwrap();
    
    // Create signal that outputs invalid JSON
    let invalid_signal = r#"#!/bin/bash
echo "This is not valid JSON but should still work"
"#;
    
    let signal_path = signals_dir.join("invalid_json_signal.sh");
    fs::write(&signal_path, invalid_signal).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&signal_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&signal_path, perms).unwrap();
    }
    
    // Initialize engine
    let mut engine = Engine::new(&project_path).await.unwrap();
    
    // Test that invalid JSON is handled gracefully
    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo test"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });
    
    let decision = engine.evaluate(&event).await.unwrap();
    
    // Should allow and include context with the invalid JSON stored as string
    if let cupcake_rego::engine::decision::FinalDecision::Allow { context } = decision {
        let context_str = context.join(" ");
        assert!(context_str.contains("This is not valid JSON but should still work"));
    } else {
        panic!("Expected Allow decision with context");
    }
}