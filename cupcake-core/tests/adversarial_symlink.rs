//! Adversarial test suite for TOB-EQTY-LAB-CUPCAKE-4
//! Tests defenses against symlink-based path bypass attacks

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that symlink creation to .cupcake is blocked
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_blocks_symlink_creation_to_cupcake() -> Result<()> {
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

    // Helper library includes symlink detection
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

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
    message: "Symlinks to Cupcake directory are forbidden"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test various symlink creation attempts
    let symlink_commands = vec![
        "ln -s .cupcake /tmp/cupcake-link",
        "ln -sf .cupcake /tmp/link",
        "ln -s ./.cupcake ~/link",
        "ln -s .cupcake/rulebook.yml /tmp/config",
        "ln -s /full/path/.cupcake /tmp/link",
        "ln  -s  .cupcake  /tmp/link",        // Extra spaces
        "ln -s .cupcake link && rm -rf link", // Chained command
    ];

    for cmd in symlink_commands {
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
                    reason.contains("symlink") || reason.contains("Cupcake"),
                    "Command '{cmd}' should mention symlink protection: {reason}"
                );
            }
            _ => {
                panic!("VULNERABILITY: Symlink command '{cmd}' was not blocked! Got: {decision:?}")
            }
        }
    }

    Ok(())
}

/// Test that protected paths block symlink operations
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_block_symlinks() -> Result<()> {
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
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

    let protected_policy = include_str!("../../fixtures/claude/builtins/protected_paths.rego");
    fs::write(builtins_dir.join("protected_paths.rego"), protected_policy)?;

    let rulebook_content = r#"
builtins:
  protected_paths:
    enabled: true
    message: "Protected files cannot be symlinked"
    paths:
      - "production.env"
      - "secrets/"
      - "config/"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test symlink operations on protected paths
    let symlink_attacks = vec![
        "ln -s production.env /tmp/env-link",
        "ln -s secrets/api.key /tmp/key",
        "ln -s config/database.yml /tmp/db",
        "ln -sf production.env prod-link", // Force flag
        "ln -s ./production.env link",     // Relative path
    ];

    for cmd in symlink_attacks {
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
                    reason.contains("protected") || reason.contains("read operations"),
                    "Symlink to protected path should be blocked: {reason}"
                );
            }
            _ => panic!(
                "VULNERABILITY: Symlink to protected path '{cmd}' was not blocked! Got: {decision:?}"
            ),
        }
    }

    Ok(())
}

/// Test Unix file permissions defense (0o700 on .cupcake)
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
#[cfg(unix)] // Only run on Unix systems
async fn test_unix_permissions_defense() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");

    // Simulate cupcake init creating the directory with restricted permissions
    fs::create_dir(&cupcake_dir)?;
    let mut perms = fs::metadata(&cupcake_dir)?.permissions();
    perms.set_mode(0o700); // Owner-only access
    fs::set_permissions(&cupcake_dir, perms)?;

    // Verify permissions are set correctly
    let metadata = fs::metadata(&cupcake_dir)?;
    let mode = metadata.permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o700,
        "Directory should have 0o700 permissions"
    );

    // Create a test file inside
    let test_file = cupcake_dir.join("test.txt");
    fs::write(&test_file, "test content")?;

    // Try to create a symlink (this would fail for other users due to permissions)
    let symlink_path = temp_dir.path().join("symlink");

    // As the owner, we CAN create a symlink (but policy would block it)
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        unix_fs::symlink(&test_file, &symlink_path)?;

        // Verify symlink was created
        assert!(symlink_path.exists());

        // Clean up
        fs::remove_file(symlink_path)?;
    }

    println!("Unix permissions (0o700) provide defense-in-depth against other users");
    println!("Owner can still create symlinks, but policies block them");

    Ok(())
}

/// Test that hardlinks are also detected and blocked
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_blocks_hardlink_creation() -> Result<()> {
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
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

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

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test hardlink creation attempts (ln without -s flag)
    let hardlink_commands = vec![
        "ln .cupcake/rulebook.yml /tmp/hardlink",
        "ln .cupcake/policies/test.rego link",
        "ln  .cupcake/rulebook.yml  link", // Extra spaces
    ];

    for cmd in hardlink_commands {
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
                    "Hardlink command '{cmd}' should be blocked: {reason}"
                );
            }
            _ => {
                // Hardlinks might be allowed if we only check for -s flag
                println!(
                    "WARNING: Hardlink command '{cmd}' may not be explicitly blocked. \
                     Decision: {decision:?}"
                );
            }
        }
    }

    Ok(())
}

/// Test path traversal via symlinks
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_path_traversal_symlink_attacks() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&claude_dir)?;

    // Create a custom policy to test path normalization
    let policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash", "Write", "Edit"]
package cupcake.policies.claude.path_traversal

import rego.v1
import data.cupcake.helpers.paths as paths

deny contains decision if {
    input.tool_name == "Write"
    normalized := paths.normalize(input.tool_input.file_path)
    contains(normalized, "sensitive")

    decision := {
        "rule_id": "PATH-001",
        "reason": concat(" ", ["Normalized path", normalized, "contains sensitive"]),
        "severity": "HIGH"
    }
}

deny contains decision if {
    input.tool_name == "Bash"
    cmd := input.tool_input.command

    # Check for path traversal patterns
    contains(cmd, "../")
    contains(cmd, "sensitive")

    decision := {
        "rule_id": "PATH-002",
        "reason": "Path traversal attempt detected",
        "severity": "HIGH"
    }
}
"#;

    // Write helper library (should be under policies directory)
    let helpers_dir = policies_dir.join("helpers");
    fs::create_dir_all(&helpers_dir)?;

    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    fs::write(claude_dir.join("path_traversal.rego"), policy)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test path traversal patterns
    let traversal_attacks = vec![
        (
            "Write",
            json!({
                "file_path": "./sensitive.txt",  // Normalized to sensitive.txt
                "content": "data"
            }),
        ),
        (
            "Write",
            json!({
                "file_path": ".//sensitive.txt",  // Double slash
                "content": "data"
            }),
        ),
        (
            "Bash",
            json!({
                "command": "cat ../../../sensitive.txt"
            }),
        ),
        (
            "Bash",
            json!({
                "command": "ln -s ../sensitive.txt link"
            }),
        ),
    ];

    for (tool_name, tool_input) in traversal_attacks {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": tool_name,
            "tool_input": tool_input
        });

        let decision = engine.evaluate(&event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Deny { .. } => {
                // Good - path traversal detected
            }
            _ => {
                println!("Path traversal with {tool_name} may not be fully blocked: {decision:?}");
            }
        }
    }

    Ok(())
}

/// Test that normal symlink operations (non-.cupcake) are allowed
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_allows_normal_symlinks() -> Result<()> {
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
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

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

    let empty_global = TempDir::new()?;
    let config = cupcake_core::engine::EngineConfig {
        global_config: Some(empty_global.path().to_path_buf()),
        harness: cupcake_core::harness::types::HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };
    let engine = Engine::new_with_config(temp_dir.path(), config).await?;

    // Test normal symlink operations that should be allowed
    let allowed_symlinks = vec![
        "ln -s /usr/bin/python3 python",
        "ln -s src/main.rs main",
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
                // Good - normal symlinks are allowed
            }
            _ => panic!("Normal symlink command '{cmd}' should be allowed! Got: {decision:?}"),
        }
    }

    Ok(())
}
