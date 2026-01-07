#![allow(unused_imports)]

use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_ask_with_signals() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let signals_dir = cupcake_dir.join("signals");

    fs::create_dir_all(&system_dir).unwrap();
    fs::create_dir_all(&signals_dir).unwrap();

    // System policy
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

    // Test policy with the EXACT ASK rule from the failing test
    let test_policy = r#"package cupcake.policies.test_signals

import rego.v1

# METADATA
# scope: rule
# title: Test Signals
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["test_status"]

# Rule using structured signal access - NOW FIXED WITH CONCAT
ask contains decision if {
    input.signals.test_status.passing == false
    input.signals.test_status.coverage < 90
    contains(input.tool_input.command, "deploy")
    
    # WASM workaround: use concat instead of sprintf
    coverage_int := floor(input.signals.test_status.coverage)
    coverage_str := format_int(coverage_int, 10)
    
    decision := {
        "reason": concat("", ["Deploy with failing tests and ", coverage_str, "% coverage?"]),
        "severity": "MEDIUM",
        "rule_id": "TEST-002"
    }
}
"#;

    fs::write(claude_dir.join("test_signals.rego"), test_policy).unwrap();

    // Create test_status signal
    let signal_script = r#"#!/bin/bash
echo '{
  "passing": false,
  "coverage": 85.5,
  "duration": 45.2,
  "failed_tests": ["test_a", "test_b", "test_c", "test_d"],
  "environment": "ci"
}'
"#;

    let signal_path = signals_dir.join("test_status.sh");
    fs::write(&signal_path, signal_script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&signal_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&signal_path, perms).unwrap();
    }

    eprintln!("Test setup complete at: {project_path:?}");

    // Initialize engine - disable global config to avoid interference
    let empty_global = TempDir::new().unwrap();
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(&project_path, config)
        .await
        .unwrap();

    let event = json!({
        "hookEventName": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "deploy --production"
        },
        "session_id": "test",
        "cwd": "/tmp"
    });

    let decision = engine.evaluate(&event, None).await.unwrap();
    eprintln!("Decision: {decision:?}");

    assert!(
        decision.requires_confirmation(),
        "Expected ASK decision but got: {decision:?}"
    );
}
