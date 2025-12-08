use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_minimal_ask_rule() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir).unwrap();

    // Use the exact same system policy as examples directory
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

    // Minimal test policy that always asks
    let test_policy = r#"package cupcake.policies.minimal

import rego.v1

# METADATA
# scope: rule
# title: Minimal Test
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

ask contains decision if {
    input.tool_name  # Check that tool_name exists (always true for our test input)
    decision := {
        "reason": "Always ask for testing",
        "severity": "LOW",
        "rule_id": "MINIMAL-ASK"
    }
}
"#;

    fs::write(claude_dir.join("minimal.rego"), test_policy).unwrap();

    eprintln!("Test policies created at: {claude_dir:?}");

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
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "echo test"
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
