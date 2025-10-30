//! Integration test for rulebook_security_guardrails builtin
//!
//! Tests the complete flow from configuration to policy enforcement

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Integration test: rulebook security prevents file operations on .cupcake/ files
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_rulebook_security_blocks_cupcake_file_edits() -> Result<()> {
    // Create a temporary directory for test policies with harness-specific structure
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    // Use the authoritative system evaluation policy
    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Write helper library (required by refactored builtins)
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    // Use the actual rulebook security policy
    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    // Create rulebook with rulebook_security_guardrails enabled
    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
    message: "Test: Cupcake files are protected"
    protected_paths:
      - ".cupcake/"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create the engine - use project root, not .cupcake dir
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

    // Test 1: Block Edit operation on .cupcake/policies/example.rego
    let edit_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Edit",
        "tool_input": {
            "file_path": ".cupcake/policies/example.rego",
            "old_string": "old",
            "new_string": "new"
        }
    });

    let decision = engine.evaluate(&edit_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("Cupcake"),
                "Should mention Cupcake protection: {reason}"
            );
        }
        _ => panic!("Expected Halt for .cupcake file edit, got: {decision:?}"),
    }

    // Test 2: Block Write operation on .cupcake/rulebook.yml
    let write_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": ".cupcake/rulebook.yml",
            "content": "malicious: content"
        }
    });

    let decision = engine.evaluate(&write_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("Cupcake") || reason.contains("protected"),
                "Should mention protection: {reason}"
            );
        }
        _ => panic!("Expected Halt for .cupcake file write, got: {decision:?}"),
    }

    // Test 3: Block bash rm command targeting .cupcake/
    let bash_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf .cupcake/policies/*"
        }
    });

    let decision = engine.evaluate(&bash_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("Cupcake") || reason.contains("protected"),
                "Should mention protection: {reason}"
            );
        }
        _ => panic!("Expected Halt for bash command targeting .cupcake, got: {decision:?}"),
    }

    // Test 4: Allow non-.cupcake file operations
    let normal_edit = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "src/main.rs",
            "old_string": "old",
            "new_string": "new"
        }
    });

    let decision = engine.evaluate(&normal_edit, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - non-.cupcake files are allowed
        }
        _ => panic!("Expected Allow for non-.cupcake file, got: {decision:?}"),
    }

    Ok(())
}

/// Test that bash commands containing .cupcake are blocked
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_rulebook_security_blocks_bash_cupcake_commands() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Write helper library (required by refactored builtins)
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create engine without global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test various bash commands that should be blocked
    let test_commands = vec![
        "cat .cupcake/rulebook.yml",
        "echo 'test' > .cupcake/test.txt",
        "grep -r 'secret' .cupcake/",
        "find .cupcake -name '*.rego'",
        "vim .cupcake/policies/example.rego",
    ];

    for cmd in test_commands {
        let bash_event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": "Bash",
            "tool_input": {
                "command": cmd
            }
        });

        let decision = engine.evaluate(&bash_event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
                assert!(
                    reason.contains("Cupcake") || reason.contains("protected"),
                    "Command '{cmd}' should be blocked with protection message: {reason}"
                );
            }
            _ => panic!("Expected Halt for command '{cmd}', got: {decision:?}"),
        }
    }

    Ok(())
}

/// Test Read operations are also blocked (total lockdown)
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_rulebook_security_blocks_read_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    // Use Claude harness-specific directory
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Write helper library (required by refactored builtins)
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
    message: "Cupcake directory is completely protected"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create engine without global config to avoid interference
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test that Read is ALSO blocked (unlike protected_paths which allows reads)
    let read_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Read",
        "tool_input": {
            "file_path": ".cupcake/rulebook.yml"
        }
    });

    let decision = engine.evaluate(&read_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("protected"),
                "Should block read with protection message: {reason}"
            );
        }
        _ => panic!("Expected Halt for .cupcake read, got: {decision:?}"),
    }

    // Test that Grep is also blocked
    let grep_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Grep",
        "tool_input": {
            "pattern": "secret",
            "path": ".cupcake/"
        }
    });

    let decision = engine.evaluate(&grep_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("protected"),
                "Should block grep with protection message: {reason}"
            );
        }
        _ => panic!("Expected Halt for .cupcake grep, got: {decision:?}"),
    }

    Ok(())
}
