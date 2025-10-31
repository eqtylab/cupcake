//! Integration tests for Rust-level input preprocessing
//!
//! These tests demonstrate that preprocessing protects ALL policies
//! (including naive user policies) from spacing/whitespace bypasses.

use anyhow::Result;
use cupcake_core::engine::Engine;
use cupcake_core::harness::types::HarnessType;
use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that preprocessing protects naive user policies from spacing bypasses
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_preprocessing_protects_naive_user_policy() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Create a NAIVE user policy that uses basic contains() - vulnerable without preprocessing
    let naive_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.claude.user_naive

import rego.v1

# This policy is NAIVE - it uses basic contains() without regex
# Without preprocessing, it would be bypassed by extra spaces
deny contains decision if {
    input.tool_name == "Bash"
    cmd := input.tool_input.command

    # VULNERABLE pattern matching (without preprocessing)
    contains(cmd, "rm -rf /important")

    decision := {
        "rule_id": "USER-NAIVE-001",
        "reason": "Cannot delete important directory",
        "severity": "HIGH"
    }
}
"#;
    fs::write(claude_dir.join("user_naive.rego"), naive_policy)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test various adversarial spacing patterns
    let adversarial_commands = vec![
        "rm  -rf /important",    // Double space after rm
        "rm -rf  /important",    // Double space after -rf
        "rm   -rf   /important", // Triple spaces
        "rm\t-rf /important",    // Tab instead of space
        "  rm -rf /important",   // Leading spaces
        "rm -rf /important  ",   // Trailing spaces
    ];

    for original_cmd in adversarial_commands {
        // Create event with adversarial command
        let mut bash_event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": "Bash",
            "tool_input": {
                "command": original_cmd
            }
        });

        // Apply preprocessing (simulating what CLI does)
        let preprocess_config = PreprocessConfig::default();
        preprocess_input(&mut bash_event, &preprocess_config, HarnessType::ClaudeCode);

        // Verify command was normalized
        let normalized_cmd = bash_event["tool_input"]["command"].as_str().unwrap();
        assert_eq!(
            normalized_cmd, "rm -rf /important",
            "Command '{original_cmd}' should be normalized to 'rm -rf /important'"
        );

        // Evaluate with preprocessing applied
        let decision = engine.evaluate(&bash_event, None).await?;

        match decision {
            cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
                assert_eq!(
                    reason, "Cannot delete important directory",
                    "Naive policy should now catch normalized command"
                );
            }
            _ => panic!(
                "PREPROCESSING FAILURE: Naive policy failed to block '{original_cmd}' even after normalization! Got: {decision:?}"
            ),
        }
    }

    // Also verify exact match still works
    let mut exact_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /important"
        }
    });

    preprocess_input(
        &mut exact_event,
        &PreprocessConfig::default(),
        HarnessType::ClaudeCode,
    );
    let decision = engine.evaluate(&exact_event, None).await?;

    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Cannot delete important directory");
        }
        _ => panic!("Exact command should still be blocked"),
    }

    Ok(())
}

/// Test that preprocessing preserves quotes correctly
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_preprocessing_preserves_quotes() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Policy that checks for a specific commit message
    let quote_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.claude.quote_test

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    cmd := input.tool_input.command

    # Check for specific commit message with spaces
    contains(cmd, "'Fix  spacing  issue'")  # Note: double spaces in message

    decision := {
        "rule_id": "QUOTE-TEST-001",
        "reason": "Found specific commit message",
        "severity": "LOW"
    }
}
"#;
    fs::write(claude_dir.join("quote_test.rego"), quote_policy)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Command with spaces inside quotes (should be preserved)
    let mut quote_event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {
            "command": "git  commit  -m  'Fix  spacing  issue'"  // Spaces outside and inside quotes
        }
    });

    // Apply preprocessing
    preprocess_input(
        &mut quote_event,
        &PreprocessConfig::default(),
        HarnessType::ClaudeCode,
    );

    // Check normalization: outside quotes normalized, inside preserved
    let normalized = quote_event["tool_input"]["command"].as_str().unwrap();
    assert_eq!(
        normalized, "git commit -m 'Fix  spacing  issue'",
        "Spaces outside quotes normalized, inside quotes preserved"
    );

    // Policy should match because spaces in quotes were preserved
    let decision = engine.evaluate(&quote_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Found specific commit message");
        }
        _ => panic!("Policy should match the preserved spaces in quotes"),
    }

    Ok(())
}

/// Test that preprocessing handles edge cases gracefully
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_preprocessing_edge_cases() -> Result<()> {
    let config = PreprocessConfig::default();

    // Empty command
    let mut empty = json!({
        "tool_name": "Bash",
        "tool_input": { "command": "" }
    });
    preprocess_input(&mut empty, &config, HarnessType::ClaudeCode);
    assert_eq!(empty["tool_input"]["command"].as_str().unwrap(), "");

    // Command with only spaces
    let mut spaces = json!({
        "tool_name": "Bash",
        "tool_input": { "command": "   " }
    });
    preprocess_input(&mut spaces, &config, HarnessType::ClaudeCode);
    assert_eq!(spaces["tool_input"]["command"].as_str().unwrap(), "");

    // Non-Bash tool (whitespace not normalized, but symlink resolution applied)
    let mut read_tool = json!({
        "tool_name": "Read",
        "tool_input": { "file_path": "file  with  spaces.txt" }
    });
    preprocess_input(&mut read_tool, &config, HarnessType::ClaudeCode);
    // File path should preserve spaces (no whitespace normalization)
    assert_eq!(
        read_tool["tool_input"]["file_path"].as_str().unwrap(),
        "file  with  spaces.txt",
        "Read tool paths should preserve spaces"
    );
    // But symlink resolution metadata should be added
    assert!(
        read_tool.get("resolved_file_path").is_some(),
        "Should have resolved_file_path"
    );
    assert!(
        read_tool.get("original_file_path").is_some(),
        "Should have original_file_path"
    );
    assert!(
        read_tool.get("is_symlink").is_some(),
        "Should have is_symlink flag"
    );

    // Missing tool_input
    let mut missing = json!({
        "tool_name": "Bash"
    });
    preprocess_input(&mut missing, &config, HarnessType::ClaudeCode); // Should not panic

    // tool_input is not an object
    let mut invalid = json!({
        "tool_name": "Bash",
        "tool_input": "not an object"
    });
    preprocess_input(&mut invalid, &config, HarnessType::ClaudeCode); // Should not panic

    // command is not a string
    let mut non_string = json!({
        "tool_name": "Bash",
        "tool_input": { "command": 123 }
    });
    preprocess_input(&mut non_string, &config, HarnessType::ClaudeCode); // Should not panic

    Ok(())
}

/// Test that preprocessing can be disabled via configuration
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_preprocessing_can_be_disabled() -> Result<()> {
    let mut event = json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm  -rf  test"
        }
    });

    let original = event.clone();

    // Preprocessing disabled
    let disabled_config = PreprocessConfig::disabled();
    preprocess_input(&mut event, &disabled_config, HarnessType::ClaudeCode);

    assert_eq!(
        event, original,
        "With disabled config, no normalization should occur"
    );

    // Now with enabled config
    let mut event2 = original.clone();
    let enabled_config = PreprocessConfig::default();
    preprocess_input(&mut event2, &enabled_config, HarnessType::ClaudeCode);

    assert_ne!(
        event2, original,
        "With enabled config, normalization should occur"
    );
    assert_eq!(
        event2["tool_input"]["command"].as_str().unwrap(),
        "rm -rf test"
    );

    Ok(())
}

/// Benchmark: measure preprocessing performance impact
#[test]
fn test_preprocessing_performance() {
    use std::time::Instant;

    let event = json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "ls  -la  |  grep  test  |  wc  -l"
        }
    });

    let config = PreprocessConfig::default();

    // Warm up
    for _ in 0..100 {
        let mut test_event = event.clone();
        preprocess_input(&mut test_event, &config, HarnessType::ClaudeCode);
    }

    // Measure
    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        let mut test_event = event.clone();
        preprocess_input(&mut test_event, &config, HarnessType::ClaudeCode);
    }

    let duration = start.elapsed();
    let per_iteration = duration.as_micros() as f64 / iterations as f64;

    println!("Preprocessing performance: {per_iteration:.2}μs per command");

    // Assert reasonable performance (< 100 microseconds per command)
    assert!(
        per_iteration < 100.0,
        "Preprocessing too slow: {per_iteration:.2}μs per iteration"
    );
}
