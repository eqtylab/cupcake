use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_ask_with_concat_workaround() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    let cupcake_dir = project_path.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let system_dir = policies_dir.join("system");
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

    // Test policy using concat instead of sprintf
    let test_policy = r#"package cupcake.policies.test_concat

import rego.v1

# METADATA
# scope: rule
# title: Test Concat
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["test_status"]

# Using concat as a workaround for sprintf not working in WASM
ask contains decision if {
    input.signals.test_status.passing == false
    input.signals.test_status.coverage < 90
    contains(input.tool_input.command, "deploy")
    
    # Convert coverage to integer for display (since we can't use %.1f)
    coverage_int := floor(input.signals.test_status.coverage)
    coverage_str := format_int(coverage_int, 10)
    
    # Build message using concat
    reason_parts := [
        "Deploy with failing tests and ",
        coverage_str,
        "% coverage?"
    ]
    
    decision := {
        "reason": concat("", reason_parts),
        "severity": "MEDIUM",
        "rule_id": "TEST-002"
    }
}
"#;

    fs::write(policies_dir.join("test_concat.rego"), test_policy).unwrap();

    // Create test_status signal
    let signal_script = r#"#!/bin/bash
echo '{"passing": false, "coverage": 85.5}'
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

    let engine = Engine::new(&project_path).await.unwrap();

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
    eprintln!("Concat workaround decision: {decision:?}");

    assert!(
        decision.requires_confirmation(),
        "Expected ASK decision but got: {decision:?}"
    );

    if let Some(reason) = decision.reason() {
        assert!(
            reason.contains("Deploy with failing tests"),
            "Expected deploy message"
        );
        assert!(
            reason.contains("85% coverage"),
            "Expected coverage percentage"
        );
    }
}
