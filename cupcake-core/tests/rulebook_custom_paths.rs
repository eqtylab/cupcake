//! Tests for rulebook_security_guardrails with user-configured protected paths
//!
//! Verifies that custom paths (not just .cupcake) get total lockdown protection

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that rulebook_security_guardrails protects user-configured custom paths
/// with total lockdown (blocks both reads AND writes, unlike protected_paths builtin)
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_rulebook_security_protects_custom_paths() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    // Configure with CUSTOM protected paths (not just .cupcake)
    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
    message: "Critical files are locked down"
    protected_paths:
      - ".cupcake/"
      - "secrets/"
      - ".env.production"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        governance_bundle_path: None,
        governance_service_url: None,
        governance_rulebook_id: None,
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // TEST 1: Block WRITE operations to custom paths
    let write_secrets = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "secrets/api-key.txt",
            "content": "sk-1234567890"
        }
    });

    let decision = engine.evaluate(&write_secrets, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("locked down") || reason.contains("protected"),
                "Should block write to secrets/: {reason}"
            );
        }
        _ => panic!("Expected Halt for write to secrets/, got: {decision:?}"),
    }

    // TEST 2: Block EDIT operations to custom paths
    let edit_env = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Edit",
        "tool_input": {
            "file_path": ".env.production",
            "old_string": "OLD_VALUE",
            "new_string": "NEW_VALUE"
        }
    });

    let decision = engine.evaluate(&edit_env, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("locked down") || reason.contains("protected"),
                "Should block edit to .env.production: {reason}"
            );
        }
        _ => panic!("Expected Halt for edit to .env.production, got: {decision:?}"),
    }

    // TEST 3: Block READ operations to custom paths (total lockdown, not read-only)
    let read_secrets = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Read",
        "tool_input": {
            "file_path": "secrets/api-key.txt"
        }
    });

    let decision = engine.evaluate(&read_secrets, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("locked down") || reason.contains("protected"),
                "Should block READ to secrets/ (total lockdown): {reason}"
            );
        }
        _ => panic!("Expected Halt for READ to secrets/ (total lockdown), got: {decision:?}"),
    }

    // TEST 4: Block Grep operations to custom paths
    let grep_secrets = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Grep",
        "tool_input": {
            "pattern": "password",
            "path": "secrets/"
        }
    });

    // Debug: Print input to understand what's being sent
    eprintln!(
        "DEBUG: Grep input: {}",
        serde_json::to_string_pretty(&grep_secrets)?
    );

    let decision = engine.evaluate(&grep_secrets, None).await?;

    // Debug: Check if preprocessing is happening
    eprintln!("DEBUG: Final decision: {decision:?}");

    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("locked down") || reason.contains("protected"),
                "Should block Grep on secrets/: {reason}"
            );
        }
        _ => panic!("Expected Halt for Grep on secrets/, got: {decision:?}"),
    }

    // TEST 5: Block Bash commands mentioning custom paths
    let bash_cat_secrets = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "cat secrets/api-key.txt"
        }
    });

    let decision = engine.evaluate(&bash_cat_secrets, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("locked down") || reason.contains("protected"),
                "Should block bash command on secrets/: {reason}"
            );
        }
        _ => panic!("Expected Halt for bash command on secrets/, got: {decision:?}"),
    }

    let bash_rm_env = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm .env.production"
        }
    });

    let decision = engine.evaluate(&bash_rm_env, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
            assert!(
                reason.contains("locked down") || reason.contains("protected"),
                "Should block bash rm on .env.production: {reason}"
            );
        }
        _ => panic!("Expected Halt for bash rm on .env.production, got: {decision:?}"),
    }

    // TEST 6: Verify .cupcake still protected (baseline check)
    let write_cupcake = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": ".cupcake/evil.rego",
            "content": "malicious"
        }
    });

    let decision = engine.evaluate(&write_cupcake, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Halt { .. } => {
            // Good - .cupcake still protected
        }
        _ => panic!("Expected Halt for .cupcake (should still work), got: {decision:?}"),
    }

    // TEST 7: Allow operations on NON-protected paths
    let write_normal = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": "src/main.rs",
            "content": "fn main() {}"
        }
    });

    let decision = engine.evaluate(&write_normal, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - normal files still allowed
        }
        _ => panic!("Expected Allow for non-protected file src/main.rs, got: {decision:?}"),
    }

    let read_normal = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Read",
        "tool_input": {
            "file_path": "README.md"
        }
    });

    let decision = engine.evaluate(&read_normal, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
            // Good - normal files still allowed
        }
        _ => panic!("Expected Allow for README.md, got: {decision:?}"),
    }

    Ok(())
}

/// Test that symlinks cannot be created to/from user-configured protected paths
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_rulebook_security_blocks_symlinks_to_custom_paths() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");
    let builtins_dir = claude_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    // Configure with custom protected paths for symlink testing
    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
    message: "Symlinks to critical paths forbidden"
    protected_paths:
      - ".cupcake/"
      - "secrets/"
      - ".env.production"
      - "config/database.yml"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        governance_bundle_path: None,
        governance_service_url: None,
        governance_rulebook_id: None,
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // TEST 1: Block symlink creation with custom path as SOURCE
    let symlink_attacks_source = vec![
        "ln -s secrets/ /tmp/leak",
        "ln -s secrets/api-key.txt /tmp/key",
        "ln -sf .env.production /tmp/env",
        "ln -s config/database.yml /tmp/db.yml",
        "ln  -s  secrets/  /tmp/leak",                 // Extra spaces
        "ln -s secrets/ link && cat link/api-key.txt", // Chained attack
    ];

    for cmd in symlink_attacks_source {
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
                    reason.contains("symlink")
                        || reason.contains("Symlink")
                        || reason.contains("forbidden")
                        || reason.contains("protected"),
                    "Command '{cmd}' should mention symlink/protection: {reason}"
                );
            }
            _ => {
                panic!(
                    "VULNERABILITY: Symlink to custom path '{cmd}' was not blocked! Got: {decision:?}"
                )
            }
        }
    }

    // TEST 2: Block symlink creation with custom path as TARGET
    let symlink_attacks_target = vec![
        "ln -s /tmp/malicious secrets/injected",
        "ln -s /tmp/evil .env.production",
        "ln -s /etc/passwd config/database.yml",
        "ln -s ./malicious.txt secrets/link.txt",
    ];

    for cmd in symlink_attacks_target {
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
                    reason.contains("symlink")
                        || reason.contains("Symlink")
                        || reason.contains("forbidden")
                        || reason.contains("protected"),
                    "Command '{cmd}' (target protected) should be blocked: {reason}"
                );
            }
            _ => {
                panic!(
                    "VULNERABILITY: Symlink with protected target '{cmd}' was not blocked! Got: {decision:?}"
                )
            }
        }
    }

    // TEST 3: Verify .cupcake symlink blocking still works
    let cupcake_symlinks = vec![
        "ln -s .cupcake /tmp/cupcake-leak",
        "ln -s .cupcake/rulebook.yml /tmp/config",
    ];

    for cmd in cupcake_symlinks {
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
            cupcake_core::engine::decision::FinalDecision::Halt { .. } => {
                // Good - .cupcake still protected
            }
            _ => panic!("Expected Halt for .cupcake symlink (baseline), got: {decision:?}"),
        }
    }

    // TEST 4: Allow symlinks for NON-protected paths
    let allowed_symlinks = vec![
        "ln -s src/lib.rs link.rs",
        "ln -s /usr/bin/python3 python",
        "ln -s README.md readme",
        "ln -sf node_modules/.bin/tsc tsc",
    ];

    for cmd in allowed_symlinks {
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
            cupcake_core::engine::decision::FinalDecision::Allow { .. } => {
                // Good - normal symlinks allowed
            }
            _ => panic!("Normal symlink '{cmd}' should be allowed! Got: {decision:?}"),
        }
    }

    Ok(())
}
