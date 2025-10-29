//! Adversarial test suite for TOB-EQTY-LAB-CUPCAKE-3
//! Tests defenses against string matching bypass via spacing and obfuscation

use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// Test that extra spaces in dangerous commands are properly detected
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_blocks_rm_with_extra_spaces() -> Result<()> {
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

    // Write the authoritative system evaluation policy
    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Write helper library (CRITICAL for fixing spacing bypass)
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

    // Use the actual rulebook security policy with helper library
    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    // Configure rulebook with security guardrails
    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
    message: "Cupcake files are protected from adversarial commands"
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

    // Test various adversarial spacing patterns
    let adversarial_commands = vec![
        "rm  -rf .cupcake",                 // Double space before flag
        "rm   -rf   .cupcake",              // Triple spaces
        "rm\t-rf .cupcake",                 // Tab character
        "rm -rf  .cupcake/policies",        // Double space in path
        "rm  -rf  .cupcake",                // Multiple double spaces
        "  rm -rf .cupcake",                // Leading spaces
        "rm -rf .cupcake  ",                // Trailing spaces
        "rm  --recursive --force .cupcake", // Long flags with spaces
    ];

    for cmd in adversarial_commands {
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
            _ => panic!(
                "VULNERABILITY: Command '{cmd}' was not blocked! Got: {decision:?}. \
                 The helper library regex should have caught this spacing pattern."
            ),
        }
    }

    Ok(())
}

/// Test that command obfuscation attempts are blocked
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_blocks_obfuscated_commands() -> Result<()> {
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

    // Test various obfuscation techniques
    let obfuscated_commands = vec![
        "echo 'test' && rm -rf .cupcake",     // Command chaining
        "rm -rf ./.cupcake",                  // Dot-slash prefix
        "rm -rf //.cupcake",                  // Double slash
        "rm -rf .cupcake/../.cupcake",        // Path traversal
        "cd / && rm -rf .cupcake",            // Directory change
        "mv .cupcake /tmp/backup",            // Move instead of rm
        "chmod 777 .cupcake",                 // Permission change
        "chown nobody:nobody .cupcake",       // Ownership change
        "find .cupcake -delete",              // Alternative deletion
        "rsync -a --delete empty/ .cupcake/", // rsync deletion
    ];

    for cmd in obfuscated_commands {
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
                // Good - command was blocked
                assert!(
                    reason.contains("Cupcake") || reason.contains("protected"),
                    "Command should mention protection: {reason}"
                );
            }
            _ => panic!(
                "VULNERABILITY: Obfuscated command '{cmd}' was not blocked! Got: {decision:?}"
            ),
        }
    }

    Ok(())
}

/// Test git command obfuscation attempts
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_blocks_git_no_verify_with_spacing() -> Result<()> {
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

    // Write helper library
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    let helpers_paths = include_str!("../../fixtures/helpers/paths.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;
    fs::write(helpers_dir.join("paths.rego"), helpers_paths)?;

    // Use the actual git_block_no_verify policy
    let git_policy = include_str!("../../fixtures/claude/builtins/git_block_no_verify.rego");
    fs::write(builtins_dir.join("git_block_no_verify.rego"), git_policy)?;

    let rulebook_content = r#"
builtins:
  git_block_no_verify:
    enabled: true
    message: "Git hooks must not be bypassed"
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

    // Test git commands with spacing tricks
    let adversarial_git_commands = vec![
        "git  commit --no-verify -m 'test'",  // Extra space after git
        "  git commit --no-verify -m 'test'", // Leading spaces
        "git\tcommit --no-verify -m 'test'",  // Tab after git
        "git commit  --no-verify -m 'test'",  // Double space before flag
        "git commit --no-verify  -m 'test'",  // Double space after flag
        "git   commit   --no-verify   -m   'test'", // Multiple spaces everywhere
    ];

    for cmd in adversarial_git_commands {
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
            // git_block_no_verify uses 'deny' verb, not 'halt'
            cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
                assert!(
                    reason.contains("--no-verify") || reason.contains("hooks"),
                    "Should mention hook bypass: {reason}"
                );
            }
            cupcake_core::engine::decision::FinalDecision::Halt { reason, .. } => {
                assert!(
                    reason.contains("hooks") || reason.contains("bypass"),
                    "Should mention hook bypass: {reason}"
                );
            }
            _ => panic!(
                "VULNERABILITY: Git command with --no-verify '{cmd}' was not blocked! Got: {decision:?}"
            ),
        }
    }

    // Test that normal git commands are allowed
    let allowed_commands = vec![
        "git commit -m 'test'",
        "git add .",
        "git status",
        "git push",
    ];

    for cmd in allowed_commands {
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
                // Good - normal commands are allowed
            }
            _ => panic!("Normal git command '{cmd}' should be allowed, got: {decision:?}"),
        }
    }

    Ok(())
}

/// Test protected paths with obfuscated file operations
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_blocks_obfuscated_writes() -> Result<()> {
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
    message: "This path is protected"
    paths:
      - "production.env"
      - "secrets/"
      - "*.key"
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

    // Test obfuscated write attempts to protected files
    let obfuscated_writes = vec![
        "echo  'data' > production.env",      // Extra space in echo
        "echo 'data'  >  production.env",     // Spaces around redirect
        "echo 'data' >production.env",        // No space before file
        "cat > production.env << EOF",        // Here document
        "printf 'data' > production.env",     // Different command
        "tee production.env < input.txt",     // tee command
        "dd if=input of=production.env",      // dd command
        "sed -i 's/old/new/' production.env", // In-place edit
        "echo 'test' > ./production.env",     // Dot-slash prefix
        "echo 'test' > secrets/api.key",      // Protected directory
        "mv temp.txt production.env",         // Move to protected file
    ];

    for cmd in obfuscated_writes {
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
                    reason.contains("protected") || reason.contains("read operations allowed"),
                    "Should mention protection for '{cmd}': {reason}"
                );
            }
            _ => panic!(
                "VULNERABILITY: Write to protected path '{cmd}' was not blocked! Got: {decision:?}"
            ),
        }
    }

    // Verify reads are still allowed
    let read_commands = vec![
        "cat production.env",
        "grep pattern production.env",
        "less secrets/api.key",
    ];

    for cmd in read_commands {
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
                // Good - reads are allowed
            }
            _ => panic!("Read command '{cmd}' should be allowed, got: {decision:?}"),
        }
    }

    Ok(())
}

/// Validate that preprocessing now protects even naive user policies without helpers
/// This demonstrates the effectiveness of our Rust-level preprocessing defense
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_preprocessing_protects_naive_policies() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let system_dir = claude_dir.join("system");

    fs::create_dir_all(&system_dir)?;
    fs::create_dir_all(&claude_dir)?;

    let evaluate_policy = include_str!("fixtures/system_evaluate.rego");
    fs::write(system_dir.join("evaluate.rego"), evaluate_policy)?;

    // Create a naive user policy that uses basic contains() - previously vulnerable
    let naive_policy = r#"
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.user_naive

import rego.v1

# This policy WAS vulnerable to spacing bypass, but preprocessing now protects it
deny contains decision if {
    input.tool_name == "Bash"
    cmd := input.tool_input.command

    # Previously VULNERABLE: Basic string matching without regex
    # Now PROTECTED by Rust preprocessing
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

    // Test various spacing patterns that would bypass naive policies
    let adversarial_commands = vec![
        "rm  -rf /important",    // Double space after rm
        "rm -rf  /important",    // Double space after -rf
        "rm   -rf   /important", // Triple spaces
        "rm\t-rf /important",    // Tab instead of space
        "  rm -rf /important",   // Leading spaces
        "rm -rf /important  ",   // Trailing spaces
    ];

    for bypass_command in adversarial_commands {
        // Apply preprocessing (simulating what CLI does)
        let mut bash_event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "transcript_path": "/tmp/transcript.md",
            "cwd": temp_dir.path().to_string_lossy(),
            "tool_name": "Bash",
            "tool_input": {
                "command": bypass_command
            }
        });

        // Apply preprocessing to normalize the command
        let preprocess_config = cupcake_core::preprocessing::PreprocessConfig::default();
        cupcake_core::preprocessing::preprocess_input(
            &mut bash_event,
            &preprocess_config,
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        );

        // Verify command was normalized
        let normalized_cmd = bash_event["tool_input"]["command"].as_str().unwrap();
        assert_eq!(
            normalized_cmd, "rm -rf /important",
            "Command '{bypass_command}' should be normalized to 'rm -rf /important'"
        );

        // Evaluate with preprocessing applied
        let decision = engine.evaluate(&bash_event, None).await?;
        match decision {
            cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
                // SUCCESS: Preprocessing protected the naive policy!
                assert_eq!(
                    reason, "Cannot delete important directory",
                    "Naive policy should now catch normalized command"
                );
            }
            _ => {
                panic!(
                    "PREPROCESSING FAILURE: Naive policy failed to block '{bypass_command}' even after normalization! Got: {decision:?}"
                );
            }
        }
    }

    // Also verify exact match still works
    let mut exact_event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "test",
        "transcript_path": "/tmp/transcript.md",
        "cwd": temp_dir.path().to_string_lossy(),
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /important"
        }
    });

    cupcake_core::preprocessing::preprocess_input(
        &mut exact_event,
        &cupcake_core::preprocessing::PreprocessConfig::default(),
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    );
    let decision = engine.evaluate(&exact_event, None).await?;
    match decision {
        cupcake_core::engine::decision::FinalDecision::Deny { reason, .. } => {
            assert_eq!(reason, "Cannot delete important directory");
        }
        _ => panic!("Exact command should still be blocked, got: {decision:?}"),
    }

    Ok(())
}
