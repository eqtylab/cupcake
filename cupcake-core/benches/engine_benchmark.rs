//! Performance benchmarks for Cupcake engine
//! Target: <50ms for complete evaluation

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn create_test_policy_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a realistic policy using new metadata format
    fs::write(
        temp_dir.path().join("bash_guard.rego"),
        r#"
# METADATA
# scope: package
# title: Bash Security Benchmark Policy
# custom:
#   severity: HIGH
#   id: BENCH-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.bash_guard

import rego.v1

# Deny dangerous commands
deny contains decision if {
    contains(input.tool_input.command, "rm -rf /")
    
    decision := {
        "reason": "Destructive command blocked",
        "severity": "CRITICAL",
        "rule_id": "BENCH-001-DESTROY"
    }
}

deny contains decision if {
    contains(input.tool_input.command, "sudo")
    
    decision := {
        "reason": "Sudo commands require approval",
        "severity": "HIGH",
        "rule_id": "BENCH-001-SUDO"
    }
}

# Add context for all bash commands
add_context contains "Be careful with bash commands"
"#,
    )
    .unwrap();

    // Create the system aggregation policy
    fs::create_dir(temp_dir.path().join("system")).unwrap();
    fs::write(
        temp_dir.path().join("system/evaluate.rego"),
        r#"
package cupcake.system

import rego.v1

# METADATA
# scope: document
# custom:
#   entrypoint: true
#   routing:
#     required_events: []
#     required_tools: []

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
"#,
    )
    .unwrap();

    temp_dir
}

fn benchmark_single_evaluation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("single_evaluation_safe", |b| {
        let temp_dir = create_test_policy_dir();

        // Debug: Print the policy directory
        eprintln!("Benchmark policy dir: {:?}", temp_dir.path());
        eprintln!("Policy files created:");
        for e in std::fs::read_dir(temp_dir.path()).unwrap().flatten() {
            eprintln!("  - {}", e.path().display());
        }
        if let Ok(system_dir) = std::fs::read_dir(temp_dir.path().join("system")) {
            for e in system_dir.flatten() {
                eprintln!("  - {}", e.path().display());
            }
        }

        let engine = runtime.block_on(Engine::new(temp_dir.path())).unwrap();

        let safe_event = json!({
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "ls -la"},
            "session_id": "test",
            "transcript_path": "/tmp/test.txt",
            "cwd": "/tmp"
        });

        // Debug: Test one evaluation before benchmarking
        eprintln!("Testing single evaluation...");
        let test_result = runtime.block_on(async { engine.evaluate(&safe_event, None).await });
        match &test_result {
            Ok(decision) => eprintln!("Test evaluation succeeded: {decision:?}"),
            Err(e) => eprintln!("Test evaluation FAILED: {e}"),
        }

        b.iter(|| {
            runtime.block_on(async {
                let _decision = engine.evaluate(black_box(&safe_event), None).await.unwrap();
            });
        });
    });

    c.bench_function("single_evaluation_deny", |b| {
        let temp_dir = create_test_policy_dir();
        let engine = runtime.block_on(Engine::new(temp_dir.path())).unwrap();

        let dangerous_event = json!({
            "hookEventName": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "sudo rm -rf /"},
            "session_id": "test",
            "transcript_path": "/tmp/test.txt",
            "cwd": "/tmp"
        });

        // Debug: Test one evaluation before benchmarking
        eprintln!("Testing deny evaluation with dangerous command...");
        let test_result = runtime.block_on(async { engine.evaluate(&dangerous_event, None).await });
        match &test_result {
            Ok(decision) => eprintln!("Deny test evaluation result: {decision:?}"),
            Err(e) => eprintln!("Deny test evaluation FAILED: {e}"),
        }

        b.iter(|| {
            runtime.block_on(async {
                let _decision = engine.evaluate(black_box(&dangerous_event), None).await.unwrap();
            });
        });
    });
}

fn benchmark_complete_pipeline(c: &mut Criterion) {
    use cupcake_core::harness::ClaudeHarness;

    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("complete_pipeline", |b| {
        let temp_dir = create_test_policy_dir();
        let engine = runtime.block_on(Engine::new(temp_dir.path())).unwrap();

        let event_str = r#"{
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "npm test"},
            "session_id": "test",
            "transcript_path": "/tmp/test.txt",
            "cwd": "/tmp"
        }"#;

        b.iter(|| {
            runtime.block_on(async {
                // Parse event
                let event = ClaudeHarness::parse_event(black_box(event_str)).unwrap();

                // Convert to JSON for engine
                let event_json = serde_json::to_value(&event).unwrap();

                // Evaluate
                let decision = engine.evaluate(&event_json, None).await.unwrap();

                // Format response
                let _response = ClaudeHarness::format_response(&event, &decision).unwrap();
            });
        });
    });
}

criterion_group!(
    benches,
    benchmark_single_evaluation,
    benchmark_complete_pipeline
);
criterion_main!(benches);
