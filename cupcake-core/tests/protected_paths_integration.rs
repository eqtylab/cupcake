//! Integration test for protected_paths builtin
//!
//! Tests that protected paths allow reads but block writes

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that protected_paths blocks writes but allows reads
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_read_write_distinction() -> Result<()> {
    // Create a temporary directory for test with harness-specific structure
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
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

    // Use the actual protected_paths policy from Claude fixtures
    let protected_policy = include_str!("../../fixtures/claude/builtins/protected_paths.rego");
    fs::write(builtins_dir.join("protected_paths.rego"), protected_policy)?;

    // Create rulebook with protected_paths configuration
    // The builtin generates its own signals from the config
    let rulebook_content = r#"
builtins:
  protected_paths:
    enabled: true
    message: "This file is protected"
    paths:
      - "production.env"
      - "src/legacy/"
      - "*.secret"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create the engine without global config to avoid interference from global builtins
    // Use an empty temp dir as sentinel to disable global config discovery
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test 1: BLOCK Write operation on protected file
    let write_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "production.env",
            "content": "malicious content"
        }
    });

    let decision = engine.evaluate(&write_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("protected"),
                "Should mention protected: {reason}"
            );
        }
        _ => panic!("Expected Halt for write to protected file, got: {decision:?}"),
    }

    // Test 2: ALLOW Read operation on protected file
    let read_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool_name": "Read",
        "tool_input": {
            "file_path": "production.env"
        }
    });

    let decision = engine.evaluate(&read_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - reads are allowed
        }
        _ => panic!("Expected Allow for read of protected file, got: {decision:?}"),
    }

    // Test 3: BLOCK Edit operation on directory contents
    let edit_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "src/legacy/old_code.rs",
            "old_string": "old",
            "new_string": "new"
        }
    });

    let decision = engine.evaluate(&edit_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("protected"),
                "Should mention protected: {reason}"
            );
        }
        _ => panic!("Expected Halt for edit in protected directory, got: {decision:?}"),
    }

    // Test 4: BLOCK write to glob pattern match
    let secret_write = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "config.secret",
            "content": "secrets"
        }
    });

    let decision = engine.evaluate(&secret_write, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("protected"),
                "Should mention protected: {reason}"
            );
        }
        _ => panic!("Expected Halt for write to .secret file, got: {decision:?}"),
    }

    // Test 5: ALLOW write to non-protected file
    let normal_write = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_str().unwrap(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "src/main.rs",
            "content": "normal content"
        }
    });

    let decision = engine.evaluate(&normal_write, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - writes to non-protected files are allowed
        }
        _ => panic!("Expected Allow for write to non-protected file, got: {decision:?}"),
    }

    Ok(())
}

/// Test Bash command whitelisting for protected paths
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_bash_whitelist() -> Result<()> {
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
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

    let protected_policy = include_str!("../../fixtures/claude/builtins/protected_paths.rego");
    fs::write(builtins_dir.join("protected_paths.rego"), protected_policy)?;

    let rulebook_content = r#"
builtins:
  protected_paths:
    enabled: true
    paths:
      - "secure.txt"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create the engine without global config to avoid interference from global builtins
    // Use an empty temp dir as sentinel to disable global config discovery
    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test whitelisted read commands are ALLOWED
    let read_commands = vec![
        "cat secure.txt",
        "less secure.txt",
        "grep pattern secure.txt",
        "head -n 10 secure.txt",
        "wc -l secure.txt",
    ];

    for cmd in read_commands {
        let bash_event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_str().unwrap(),
            "tool_name": "Bash",
            "tool_input": {
                "command": cmd
            }
        });

        let decision = engine.evaluate(&bash_event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
                // Good - read commands are allowed
            }
            _ => panic!("Expected Allow for read command '{cmd}', got: {decision:?}"),
        }
    }

    // Test non-whitelisted commands are BLOCKED
    let write_commands = vec![
        "echo 'data' > secure.txt",
        "mv secure.txt backup.txt",
        "rm secure.txt",
        "sed -i 's/old/new/g' secure.txt",
        "vim secure.txt", // Not in whitelist
    ];

    for cmd in write_commands {
        let bash_event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_str().unwrap(),
            "tool_name": "Bash",
            "tool_input": {
                "command": cmd
            }
        });

        let decision = engine.evaluate(&bash_event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
                assert!(
                    reason.contains("read operations allowed"),
                    "Should mention only read allowed for '{cmd}': {reason}"
                );
            }
            _ => panic!("Expected Halt for write command '{cmd}', got: {decision:?}"),
        }
    }

    Ok(())
}
