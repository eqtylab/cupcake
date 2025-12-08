//! Integration tests for builtin abstractions
//!
//! These tests verify the complete signal flow:
//! 1. Builtin configuration generates signals
//! 2. Signals are executed and results injected
//! 3. Policies can access signals at input.signals
//! 4. Validation actually works with real commands

use anyhow::Result;
use cupcake_core::engine::builtins::*;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

/// Test that builtin configurations generate the expected signals
#[test]
fn test_builtin_signal_generation() {
    // Build the by_extension map for post_edit_check
    let mut by_extension = HashMap::new();
    by_extension.insert(
        "rs".to_string(),
        CheckConfig {
            command: "cargo check".to_string(),
            message: "Rust code must compile".to_string(),
        },
    );
    by_extension.insert(
        "py".to_string(),
        CheckConfig {
            command: "python -m py_compile".to_string(),
            message: "Python syntax must be valid".to_string(),
        },
    );

    // Create the full configuration in one go
    let config = BuiltinsConfig {
        claude_code_always_inject_on_prompt: Some(AlwaysInjectConfig {
            enabled: true,
            context: vec![
                ContextSource::String("Test context".to_string()),
                ContextSource::Dynamic {
                    file: Some("/tmp/test.txt".to_string()),
                    command: None,
                },
                ContextSource::Dynamic {
                    file: None,
                    command: Some("echo 'dynamic'".to_string()),
                },
            ],
        }),
        git_pre_check: Some(GitPreCheckConfig {
            enabled: true,
            checks: vec![
                CheckConfig {
                    command: "cargo test".to_string(),
                    message: "Tests must pass".to_string(),
                },
                CheckConfig {
                    command: "cargo fmt --check".to_string(),
                    message: "Code must be formatted".to_string(),
                },
            ],
        }),
        post_edit_check: Some(PostEditCheckConfig {
            enabled: true,
            by_extension,
        }),
        rulebook_security_guardrails: Some(RulebookSecurityConfig {
            enabled: true,
            message: "Test protection message".to_string(),
            protected_paths: vec![".cupcake/".to_string(), ".config/".to_string()],
        }),
        protected_paths: None,
        git_block_no_verify: None,
        system_protection: None,
        sensitive_data_protection: None,
        cupcake_exec_protection: None,
        claude_code_enforce_full_file_read: None,
    };

    // Generate signals
    let signals = config.generate_signals();

    // Verify always_inject signals - only dynamic sources generate signals now
    // Static strings are injected directly via builtin_config
    // Index 0 was a static string (no signal)
    // Index 1 and 2 are dynamic sources and should generate signals
    assert!(signals.contains_key("__builtin_prompt_context_1")); // file source
    assert!(signals.contains_key("__builtin_prompt_context_2")); // command source

    // Verify git_pre_check signals
    assert!(signals.contains_key("__builtin_git_check_0"));
    assert!(signals.contains_key("__builtin_git_check_1"));
    assert_eq!(signals["__builtin_git_check_0"].command, "cargo test");
    assert_eq!(
        signals["__builtin_git_check_1"].command,
        "cargo fmt --check"
    );

    // Verify post_edit_check signals
    assert!(signals.contains_key("__builtin_post_edit_rs"));
    assert!(signals.contains_key("__builtin_post_edit_py"));
    assert_eq!(signals["__builtin_post_edit_rs"].command, "cargo check");

    // rulebook_security_guardrails no longer generates signals - uses builtin_config instead
    // The message and paths are injected directly via builtin_config
}

/// Test that enabled builtins are correctly identified
#[test]
fn test_enabled_builtins_list() {
    let mut config = BuiltinsConfig::default();
    assert_eq!(config.enabled_builtins().len(), 0);

    // Test rulebook_security_guardrails
    config.rulebook_security_guardrails = Some(RulebookSecurityConfig {
        enabled: true,
        message: "Protected".to_string(),
        protected_paths: vec![".cupcake/".to_string()],
    });
    let enabled = config.enabled_builtins();
    assert!(enabled.contains(&"rulebook_security_guardrails".to_string()));
    assert_eq!(enabled.len(), 1);

    config.git_pre_check = Some(GitPreCheckConfig {
        enabled: true,
        checks: vec![],
    });
    assert_eq!(config.enabled_builtins().len(), 2);
    let enabled = config.enabled_builtins();
    assert!(enabled.contains(&"git_pre_check".to_string()));
    assert!(enabled.contains(&"rulebook_security_guardrails".to_string()));
}

/// Integration test: verify policies can access builtin signals
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_builtin_policy_signal_access() -> Result<()> {
    // Create a temporary directory for test policies with harness-specific structure
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    std::fs::create_dir_all(&system_dir)?;

    // Use the authoritative system evaluation policy
    let evaluate_policy = r#"package cupcake.system

import rego.v1

# METADATA
# scope: rule
# title: System Aggregation Policy
# authors: ["Cupcake Engine"]
# custom:
#   description: "Aggregates all decision verbs from policies into a DecisionSet"
#   entrypoint: true
#   routing:
#     required_events: []
#     required_tools: []

# The single entrypoint for the Hybrid Model.
# This uses the `walk()` built-in to recursively traverse data.cupcake.policies,
# automatically discovering and aggregating all decision verbs from all loaded
# policies, regardless of their package name or nesting depth.
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

# Helper function to collect all decisions for a specific verb type.
# Uses walk() to recursively find all instances of the verb across
# the entire policy hierarchy under data.cupcake.policies.
collect_verbs(verb_name) := result if {
    # Collect all matching verb sets from the policy tree
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    
    # Flatten all sets into a single array
    # Since Rego v1 decision verbs are sets, we need to convert to arrays
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    
    result := all_decisions
}

# Default to empty arrays if no decisions found
default collect_verbs(_) := []"#;
    std::fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Write a test policy that uses signals (in Claude directory)
    let test_policy = r#"package cupcake.policies.test_signal_access

import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
#     required_signals: ["__builtin_test_signal"]

# Test that we can access builtin signals
add_context contains context_msg if {
    input.hook_event_name == "UserPromptSubmit"

    # Access the signal - it should be injected by the engine
    test_signal := input.signals["__builtin_test_signal"]

    # add_context expects strings, not decision objects
    context_msg := concat(" ", ["Signal value:", test_signal])
}"#;
    std::fs::write(claude_dir.join("test_signal_access.rego"), test_policy)?;

    // Create a rulebook with test signal
    let rulebook_path = cupcake_dir.join("rulebook.yml");
    let rulebook_content = r#"signals:
  __builtin_test_signal:
    command: "echo 'test-value-123'"
    timeout_seconds: 1

# No builtins configured - just using manual signals for this test"#;
    std::fs::write(&rulebook_path, rulebook_content)?;

    // Initialize engine from the temp directory
    // Disable global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Create test input
    let input = json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test prompt"
    });

    // Evaluate with engine
    let decision = engine.evaluate(&input, None).await?;

    // The engine should have:
    // 1. Executed the __builtin_test_signal
    // 2. Injected result at input.signals.__builtin_test_signal
    // 3. Policy should access it and add context

    // Check that we got an Allow decision with context
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { context } => {
            assert!(!context.is_empty(), "Should have context");
            let combined = context.join(" ");
            assert!(
                combined.contains("test-value-123"),
                "Context should contain signal value, got: {combined}"
            );
        }
        _ => panic!("Expected Allow decision with context, got: {decision:?}"),
    }

    Ok(())
}

/// Test the complete builtin flow with real validation
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_post_edit_validation_flow() -> Result<()> {
    // Create a temporary directory for test with harness-specific structure
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let builtins_dir = claude_dir.join("builtins");
    let system_dir = claude_dir.join("system");
    std::fs::create_dir_all(&builtins_dir)?;
    std::fs::create_dir_all(&system_dir)?;

    // Copy the real post_edit_check policy content from Claude fixtures
    let post_edit_policy = include_str!("../../fixtures/claude/builtins/post_edit_check.rego");
    std::fs::write(builtins_dir.join("post_edit_check.rego"), post_edit_policy)?;

    // Use the authoritative system evaluation policy
    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    std::fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Create rulebook with post_edit_check configuration
    let rulebook_path = cupcake_dir.join("rulebook.yml");
    let rulebook_content = r#"
signals:
  __builtin_post_edit_txt:
    command: "echo 'Text file validated'"
    timeout_seconds: 5
  __builtin_post_edit_fail:
    command: "exit 1"
    timeout_seconds: 5

builtins:
  post_edit_check:
    enabled: true
    by_extension:
      txt:
        command: "echo 'Text file validated'"
        message: "Text validation"
      fail:
        command: "exit 1"
        message: "This always fails"
"#;
    std::fs::write(&rulebook_path, rulebook_content)?;

    // Create engine
    // Disable global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test 1: Edit a .txt file (should pass validation)
    let input_txt = json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "params": {
            "file_path": "test.txt"
        },
        "tool_response": "File edited successfully"
    });

    let decision_txt = engine.evaluate(&input_txt, None).await?;

    // Should add positive context for successful validation
    match decision_txt {
        cupcake_core::engine::decision::FinalDecision::Allow { context } => {
            assert!(!context.is_empty(), "Should have context");
            let combined = context.join(" ");
            assert!(
                combined.contains("✓ Validation passed"),
                "Should show validation passed, got: {combined}"
            );
        }
        _ => panic!("Expected Allow decision with context for .txt file"),
    }

    // Test 2: Edit a .fail file (should fail validation)
    let input_fail = json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "params": {
            "file_path": "test.fail"
        }
    });

    eprintln!("\n=== TEST 2: .fail file test ===");
    eprintln!("Input: {}", serde_json::to_string_pretty(&input_fail)?);

    let decision_fail = engine.evaluate(&input_fail, None).await?;

    eprintln!("Decision for .fail file: {decision_fail:?}");

    // Should ask for user confirmation on failure
    match decision_fail {
        cupcake_core::engine::decision::FinalDecision::Ask { reason, .. } => {
            assert!(
                reason.contains("validation failed"),
                "Should mention validation failed, got: {reason}"
            );
            assert!(
                reason.contains("Do you want to continue anyway?"),
                "Should ask to continue, got: {reason}"
            );
        }
        _ => panic!("Expected Ask decision for failed validation, got: {decision_fail:?}"),
    }

    Ok(())
}

/// Test that builtin policies are only loaded when enabled
#[test]
fn test_builtin_policy_loading() {
    use cupcake_core::engine::builtins::should_load_builtin_policy;
    use std::path::Path;

    let enabled = vec!["protected_paths".to_string(), "git_pre_check".to_string()];

    // Should load enabled builtins
    assert!(should_load_builtin_policy(
        Path::new("policies/builtins/protected_paths.rego"),
        &enabled
    ));
    assert!(should_load_builtin_policy(
        Path::new("policies/builtins/git_pre_check.rego"),
        &enabled
    ));

    // Should NOT load disabled builtins
    assert!(!should_load_builtin_policy(
        Path::new("policies/builtins/post_edit_check.rego"),
        &enabled
    ));
    assert!(!should_load_builtin_policy(
        Path::new("policies/builtins/claude_code_always_inject_on_prompt.rego"),
        &enabled
    ));

    // Should always load non-builtin policies
    assert!(should_load_builtin_policy(
        Path::new("policies/custom/my_policy.rego"),
        &enabled
    ));
    assert!(should_load_builtin_policy(
        Path::new("policies/system/evaluate.rego"),
        &enabled
    ));
}
