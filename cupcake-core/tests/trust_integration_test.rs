//! Integration tests for the trust system with the engine

use anyhow::Result;
use cupcake_core::engine::Engine;
use cupcake_core::trust::TrustManifest;
use std::fs;
use tempfile::TempDir;

/// Create a minimal test project structure
async fn setup_test_project() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");

    // Create directories with harness-specific structure
    fs::create_dir_all(cupcake_dir.join("policies/claude/system"))?;
    fs::create_dir_all(cupcake_dir.join("signals"))?;
    fs::create_dir_all(cupcake_dir.join("actions"))?;

    // Create system evaluate policy that matches the authoritative example
    fs::write(
        cupcake_dir.join("policies/claude/system/evaluate.rego"),
        r#"package cupcake.system

import rego.v1

# METADATA
# scope: document
# title: System Aggregation Entrypoint for Hybrid Model
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
        "allow_overrides": collect_verbs("allow_override"),
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
default collect_verbs(_) := []
"#,
    )?;

    // Create a simple test policy
    fs::write(
        cupcake_dir.join("policies/claude/test.rego"),
        r#"package cupcake.policies.test
import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["TestEvent"]

deny contains decision if {
    input.dangerous == true
    decision := {
        "reason": "Test denial",
        "severity": "HIGH",
        "rule_id": "TEST-001"
    }
}
"#,
    )?;

    // Create rulebook with test signal
    fs::write(
        cupcake_dir.join("rulebook.yml"),
        r#"signals:
  test_signal:
    command: "echo 'test output'"
    timeout_seconds: 2
"#,
    )?;

    Ok(temp_dir)
}

#[tokio::test]
async fn test_engine_without_trust() -> Result<()> {
    let project = setup_test_project().await?;

    // Initialize engine without trust (should work fine)
    // Disable global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project.path(), config).await?;

    // Create a simple test event
    let event = serde_json::json!({
        "hookEventName": "TestEvent",
        "session_id": "test-session",
        "cwd": project.path().to_str().unwrap(),
        "dangerous": false
    });

    // Should evaluate successfully
    let decision = engine.evaluate(&event, None).await?;
    assert!(matches!(
        decision,
        cupcake_core::engine::decision::FinalDecision::Allow { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_engine_with_trust_no_manifest() -> Result<()> {
    let project = setup_test_project().await?;

    // Engine should initialize fine even without trust manifest
    // (trust is optional)
    // Disable global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let _engine = Engine::new_with_config(project.path(), config).await?;

    Ok(())
}

#[tokio::test]
async fn test_engine_with_valid_trust() -> Result<()> {
    let project = setup_test_project().await?;
    let cupcake_dir = project.path().join(".cupcake");

    // Create a trust manifest
    let mut manifest = TrustManifest::new();

    // Add the test signal to trust
    let signal_entry = cupcake_core::trust::manifest::ScriptEntry {
        script_type: "inline".to_string(),
        command: "echo 'test output'".to_string(),
        hash: cupcake_core::trust::hasher::hash_string("echo 'test output'"),
        absolute_path: None,
        size: None,
        modified: None,
        interpreter: None,
        args: None,
    };
    manifest.add_script("signals", "test_signal", signal_entry);

    // Save the manifest
    manifest.save(&cupcake_dir.join(".trust"))?;

    // Initialize engine with trust enabled
    // Disable global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project.path(), config).await?;

    // Create test event that would trigger signal gathering
    let event = serde_json::json!({
        "hookEventName": "TestEvent",
        "session_id": "test-session",
        "cwd": project.path().to_str().unwrap(),
        "dangerous": false
    });

    // Should evaluate successfully with trusted signal
    let decision = engine.evaluate(&event, None).await?;
    assert!(matches!(
        decision,
        cupcake_core::engine::decision::FinalDecision::Allow { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_engine_with_untrusted_signal() -> Result<()> {
    let project = setup_test_project().await?;
    let cupcake_dir = project.path().join(".cupcake");

    // Create an empty trust manifest (no scripts trusted)
    let mut manifest = TrustManifest::new();
    manifest.save(&cupcake_dir.join(".trust"))?;

    // Modify the test policy to require a signal
    fs::write(
        cupcake_dir.join("policies/claude/test_with_signal.rego"),
        r#"package cupcake.policies.test_signal
import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["TestEvent"]
#     required_signals: ["test_signal"]

deny contains decision if {
    input.dangerous == true
    decision := {
        "reason": "Test denial",
        "severity": "HIGH",
        "rule_id": "TEST-002"
    }
}
"#,
    )?;

    // Initialize engine with trust enabled but signal not trusted
    // Disable global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(project.path(), config).await?;

    // Create test event that requires signal
    let event = serde_json::json!({
        "hookEventName": "TestEvent",
        "session_id": "test-session",
        "cwd": project.path().to_str().unwrap(),
        "dangerous": false
    });

    // Should still evaluate (signal execution fails but doesn't crash)
    let decision = engine.evaluate(&event, None).await?;

    // The evaluation should succeed but without the signal data
    assert!(matches!(
        decision,
        cupcake_core::engine::decision::FinalDecision::Allow { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn test_trust_verifier_lifecycle() -> Result<()> {
    let project = setup_test_project().await?;
    let cupcake_dir = project.path().join(".cupcake");

    // Start without trust
    {
        // Disable global config to avoid interference
        let empty_global = TempDir::new()?;
        let config = cupcake_core::engine::EngineConfig {
            global_config: Some(empty_global.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let _engine = Engine::new_with_config(project.path(), config).await?;
        // Should succeed without trust
    }

    // Add trust manifest
    let mut manifest = TrustManifest::new();
    manifest.save(&cupcake_dir.join(".trust"))?;

    // Now with trust
    {
        // Disable global config to avoid interference
        let empty_global = TempDir::new()?;
        let config = cupcake_core::engine::EngineConfig {
            global_config: Some(empty_global.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let _engine = Engine::new_with_config(project.path(), config).await?;
        // Should succeed with trust
    }

    // Remove trust manifest
    fs::remove_file(cupcake_dir.join(".trust"))?;

    // Back to no trust
    {
        // Disable global config to avoid interference
        let empty_global = TempDir::new()?;
        let config = cupcake_core::engine::EngineConfig {
            global_config: Some(empty_global.path().to_path_buf()),
            harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
            wasm_max_memory: None,
            opa_path: None,
            debug_routing: false,
        };
        let _engine = Engine::new_with_config(project.path(), config).await?;
        // Should still succeed
    }

    Ok(())
}
