//! Test for global_file_lock pattern matching
//!
//! This test validates:
//! 1. Preprocessing protects against spacing bypass attempts
//! 2. Substring matching has false positive issues (needs helper functions)

use anyhow::Result;
use cupcake_core::engine::decision::FinalDecision;
use cupcake_core::engine::{Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

mod common;
use common::init_test_logging;

/// Setup test project with global_file_lock builtin
async fn setup_test_project(project_dir: &std::path::Path) -> Result<(Engine, TempDir)> {
    common::create_test_project_for_harness(project_dir, HarnessType::Cursor)?;

    let cupcake_dir = project_dir.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let cursor_dir = policies_dir.join("cursor");
    let builtins_dir = cursor_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    // Add the helper library that global_file_lock depends on
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    // Add the global_file_lock builtin policy
    let lock_policy = include_str!("../../fixtures/cursor/builtins/global_file_lock.rego");
    fs::write(builtins_dir.join("global_file_lock.rego"), lock_policy)?;

    // Enable the builtin in rulebook.yml
    let rulebook_content = r#"
builtins:
  global_file_lock:
    enabled: true
    message: "All file modifications are locked"
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create empty global config
    let empty_global = TempDir::new()?;
    let config = EngineConfig {
        governance_bundle_path: None,
        governance_service_url: None,
        governance_rulebook_id: None,
        global_config: Some(empty_global.path().to_path_buf()),
        harness: HarnessType::Cursor,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };

    let engine = Engine::new_with_config(project_dir, config).await?;
    Ok((engine, empty_global))
}

/// Test that preprocessing protects against spacing bypass attempts
#[tokio::test]
async fn test_spacing_bypass_attempts_blocked_by_preprocessing() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();
    let (engine, _global_dir) = setup_test_project(project_dir).await?;

    // Reviewer's concern: spacing variations could bypass detection
    let spacing_variants = vec![
        "cp  file1  file2",    // Double spaces
        "cp\tfile1\tfile2",    // Tabs
        "mv    old    new",    // Multiple spaces
        "echo hello > file",   // Redirect with spacing
        "echo hello  >  file", // Extra spaces around redirect
    ];

    for command in spacing_variants {
        let event = json!({
            "hook_event_name": "beforeShellExecution",
            "command": command
        });

        let decision = engine.evaluate(&event, None).await?;

        // All should be BLOCKED because preprocessing normalizes whitespace
        assert!(
            matches!(decision, FinalDecision::Deny { .. }),
            "Command with spacing variant should be blocked (preprocessing normalizes): {command}"
        );
    }

    Ok(())
}

/// Test that helper functions eliminate false positives from substring matching
#[tokio::test]
async fn test_helper_functions_eliminate_false_positives() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();
    let (engine, _global_dir) = setup_test_project(project_dir).await?;

    // These commands should NOT be blocked (after fix with helper functions)
    let legitimate_commands = vec![
        "scp file user@host:/path", // "scp" is NOT "cp" (word boundary check)
        "grep '>' logfile",         // grep argument ">" is NOT a redirect
    ];

    for command in legitimate_commands {
        let event = json!({
            "hook_event_name": "beforeShellExecution",
            "command": command
        });

        let decision = engine.evaluate(&event, None).await?;

        // FIXED: Helper functions eliminate major false positives with proper word boundaries
        println!("Command '{command}' decision: {decision:?}");

        // Verify false positives are eliminated for common cases
        assert!(
            matches!(decision, FinalDecision::Allow { .. }),
            "Command '{command}' should be ALLOWED (not a file write operation)"
        );
    }

    // Known limitation: Redirects inside quotes still trigger detection
    // This is acceptable - better to be overly cautious (false positive) than miss real writes
    // Example: "echo 'use >> for append'" is blocked because >> pattern matches even in quotes
    // Fixing this would require a full shell parser, which is beyond simple pattern matching

    Ok(())
}

/// Test that commands with redirects in quoted strings are blocked (known limitation)
#[tokio::test]
async fn test_redirect_in_quotes_blocked_conservatively() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();
    let (engine, _global_dir) = setup_test_project(project_dir).await?;

    // This command won't actually redirect (>> is in quotes), but our pattern matching
    // doesn't parse shell quoting, so it conservatively blocks it
    let command = "echo 'use >> for append'";
    let event = json!({
        "hook_event_name": "beforeShellExecution",
        "command": command
    });

    let decision = engine.evaluate(&event, None).await?;

    // Known limitation: False positive is acceptable for security
    // Better to block innocent commands than allow malicious ones
    assert!(
        matches!(decision, FinalDecision::Deny { .. }),
        "Command with redirect in quotes is conservatively blocked (acceptable false positive)"
    );

    Ok(())
}

/// Test actual write commands that SHOULD be blocked
#[tokio::test]
async fn test_legitimate_write_commands_blocked() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();
    let (engine, _global_dir) = setup_test_project(project_dir).await?;

    let write_commands = vec![
        "cp file1 file2",
        "mv old new",
        "echo data > file",
        "cat file >> log",
        "tee output.txt",
    ];

    for command in write_commands {
        let event = json!({
            "hook_event_name": "beforeShellExecution",
            "command": command
        });

        let decision = engine.evaluate(&event, None).await?;

        assert!(
            matches!(decision, FinalDecision::Deny { .. }),
            "Write command should be blocked: {command}"
        );
    }

    Ok(())
}
