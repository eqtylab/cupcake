//! Test for Grep/Glob symlink bypass vulnerability (GitHub Copilot review finding)
//!
//! This test validates the fix for the reviewer's concern about Grep/Glob tools
//! bypassing TOB-4 symlink defenses by using raw paths instead of canonical paths.

use anyhow::Result;
use cupcake_core::engine::decision::FinalDecision;
use cupcake_core::engine::{Engine, EngineConfig};
use cupcake_core::harness::types::HarnessType;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

mod common;
use common::init_test_logging;

/// Setup test project with rulebook_security_guardrails builtin enabled
async fn setup_test_project_with_guardrails(
    project_dir: &std::path::Path,
) -> Result<(Engine, TempDir)> {
    // Create basic project structure
    common::create_test_project_for_harness(project_dir, HarnessType::ClaudeCode)?;

    let cupcake_dir = project_dir.join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    let claude_dir = policies_dir.join("claude");
    let builtins_dir = claude_dir.join("builtins");
    let helpers_dir = policies_dir.join("helpers");

    fs::create_dir_all(&builtins_dir)?;
    fs::create_dir_all(&helpers_dir)?;

    // Add the rulebook_security_guardrails builtin policy
    let rulebook_policy =
        include_str!("../../fixtures/claude/builtins/rulebook_security_guardrails.rego");
    fs::write(
        builtins_dir.join("rulebook_security_guardrails.rego"),
        rulebook_policy,
    )?;

    // Add helper library that rulebook policy uses
    let helpers_commands = include_str!("../../fixtures/helpers/commands.rego");
    fs::write(helpers_dir.join("commands.rego"), helpers_commands)?;

    // Enable the builtin in rulebook.yml
    let rulebook_content = r#"
builtins:
  rulebook_security_guardrails:
    enabled: true
"#;
    fs::write(cupcake_dir.join("rulebook.yml"), rulebook_content)?;

    // Create empty global config to prevent interference
    let empty_global = TempDir::new()?;
    let config = EngineConfig {
        governance_bundle_path: None,
        governance_service_url: None,
        governance_rulebook_id: None,
        global_config: Some(empty_global.path().to_path_buf()),
        harness: HarnessType::ClaudeCode,
        wasm_max_memory: None,
        opa_path: None,
        debug_routing: false,
    };

    // Create engine with explicit config
    let engine = Engine::new_with_config(project_dir, config).await?;

    Ok((engine, empty_global))
}

/// Test Grep tool with symlink bypass attempt
///
/// Attack: Create symlink with innocent name pointing to .cupcake, then use Grep
///
/// BEFORE FIX: Policy uses raw path "/tmp/backup" - NOT BLOCKED
/// AFTER FIX: Policy uses resolved path "/project/.cupcake" - BLOCKED
#[cfg(unix)]
#[cfg(feature = "deterministic-tests")]
#[tokio::test]
async fn test_grep_symlink_bypass_should_block() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    // Setup test project with rulebook_security_guardrails builtin and get engine
    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    // Create .cupcake directory with a secret file
    let cupcake_dir = project_dir.join(".cupcake");
    fs::create_dir_all(&cupcake_dir)?;
    fs::write(
        cupcake_dir.join("secret.rego"),
        "package secret\nsecret := true",
    )?;

    // Create symlink with innocent name pointing to .cupcake
    let symlink_path = temp_dir.path().join("backup");
    symlink(&cupcake_dir, &symlink_path)?;

    // Attempt Grep through the symlink
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Grep",
        "tool_input": {
            "pattern": "secret",
            "path": symlink_path.to_str().unwrap()  // Symlink with innocent name
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // Should be BLOCKED because resolved path points to .cupcake
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Grep through symlink should be blocked (got: {decision:?})"
    );

    Ok(())
}

/// Test Glob with pattern containing ".cupcake" - ALREADY WORKS
///
/// This scenario is already blocked because the pattern itself contains ".cupcake"
#[cfg(unix)]
#[tokio::test]
async fn test_glob_pattern_with_cupcake_already_blocked() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    // Attempt Glob with pattern that includes ".cupcake"
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Glob",
        "tool_input": {
            "pattern": ".cupcake*"  // Pattern contains ".cupcake"
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // Should be BLOCKED because pattern contains ".cupcake"
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Glob with .cupcake* pattern should be blocked"
    );

    Ok(())
}

/// Test Glob with innocent pattern searching symlink directory - STILL VULNERABLE
///
/// This test documents a REMAINING vulnerability that the one-liner does NOT fix.
///
/// Attack: Create symlink named "backup" pointing to .cupcake, use Glob pattern "backup/**"
/// Problem: Pattern "backup/**/*.rego" doesn't contain ".cupcake", so it's not blocked
///
/// This is a KNOWN LIMITATION because Glob patterns can't be canonicalized.
#[cfg(unix)]
#[tokio::test]
#[ignore] // TODO: This vulnerability remains unfixed - requires complex pattern parsing
async fn test_glob_innocent_pattern_symlink_directory_vulnerable() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    // Create .cupcake directory
    let cupcake_dir = project_dir.join(".cupcake");
    fs::create_dir_all(&cupcake_dir)?;
    fs::write(cupcake_dir.join("secret.rego"), "secret")?;

    // Create symlink with innocent name
    let symlink_path = project_dir.join("backup");
    symlink(&cupcake_dir, &symlink_path)?;

    // Attempt Glob with pattern that searches the symlink directory
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Glob",
        "tool_input": {
            "pattern": "backup/**/*.rego"  // Pattern searches symlink dir
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // SHOULD be blocked, but currently isn't because:
    // - Pattern "backup/**/*.rego" doesn't contain ".cupcake"
    // - We can't canonicalize glob patterns
    // - This is a known limitation
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Glob searching symlink directory should be blocked (CURRENTLY FAILS)"
    );

    Ok(())
}

/// Test Glob with wildcard pattern - STILL VULNERABLE
///
/// Attack: Create symlink, use wildcard pattern that searches everything
/// This test documents another REMAINING vulnerability.
#[cfg(unix)]
#[tokio::test]
#[ignore] // TODO: This vulnerability remains unfixed
async fn test_glob_wildcard_pattern_finds_symlinks_vulnerable() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    // Create .cupcake directory
    let cupcake_dir = project_dir.join(".cupcake");
    fs::create_dir_all(&cupcake_dir)?;

    // Create symlink
    let symlink_path = project_dir.join("innocent_name");
    symlink(&cupcake_dir, &symlink_path)?;

    // Wildcard pattern that searches everything
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Glob",
        "tool_input": {
            "pattern": "**/*.rego"  // Wildcard searches everything, including symlinks
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // SHOULD be blocked or at least warned, but currently isn't
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Glob with wildcards that can discover symlinks should be blocked (CURRENTLY FAILS)"
    );

    Ok(())
}

/// Test legitimate Grep usage - should ALLOW
///
/// Grep searching normal directories should work fine
#[tokio::test]
async fn test_grep_legitimate_usage_allowed() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    // Create a normal src directory
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), "fn main() {}")?;

    // Grep in normal directory
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Grep",
        "tool_input": {
            "pattern": "main",
            "path": src_dir.to_str().unwrap()
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // Should be ALLOWED
    assert!(
        matches!(decision, FinalDecision::Allow { .. }),
        "Legitimate Grep usage should be allowed (got: {decision:?})"
    );

    Ok(())
}

/// Test legitimate Glob usage - should ALLOW
///
/// Glob with normal patterns should work fine
#[tokio::test]
async fn test_glob_legitimate_usage_allowed() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    // Create normal project structure
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("lib.rs"), "pub fn test() {}")?;

    // Glob for Rust files
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Glob",
        "tool_input": {
            "pattern": "src/**/*.rs"
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // Should be ALLOWED
    assert!(
        matches!(decision, FinalDecision::Allow { .. }),
        "Legitimate Glob usage should be allowed (got: {decision:?})"
    );

    Ok(())
}

/// Test Grep on .cupcake directly - should BLOCK
///
/// Direct access to .cupcake should always be blocked
#[tokio::test]
async fn test_grep_direct_cupcake_access_blocked() -> Result<()> {
    init_test_logging();

    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let (engine, _global_dir) = setup_test_project_with_guardrails(project_dir).await?;

    let cupcake_dir = project_dir.join(".cupcake");

    // Direct Grep on .cupcake
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Grep",
        "tool_input": {
            "pattern": "policy",
            "path": cupcake_dir.to_str().unwrap()
        },
        "cwd": project_dir.to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // Should be BLOCKED
    assert!(
        matches!(decision, FinalDecision::Halt { .. }),
        "Direct .cupcake access should be blocked (got: {decision:?})"
    );

    Ok(())
}
