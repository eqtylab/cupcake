//! Comprehensive integration test suite for the `cupcake init` command
//!
//! These tests verify that the init command creates the exact expected
//! directory structure and file contents, ensuring consistency across
//! all environments including CI.

use anyhow::Result;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the path to the cupcake binary
fn get_cupcake_binary() -> PathBuf {
    // In tests, the binary is in target/debug or target/release
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go up from cupcake-cli to cupcake-rewrite
    path.push("target");

    // When running with --release, use the release binary
    // Otherwise use debug
    if cfg!(debug_assertions) {
        path.join("debug/cupcake")
    } else {
        path.join("release/cupcake")
    }
}

/// Helper to run the init command with a specific harness in a test directory
fn run_init_with_harness(harness: &str) -> Result<(TempDir, PathBuf)> {
    // Create a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().to_path_buf();

    // Run cupcake init --harness <harness> in the temp directory
    let output = Command::new(get_cupcake_binary())
        .args(["init", "--harness", harness])
        .current_dir(&project_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "cupcake init --harness {} failed:\nstderr: {}\nstdout: {}",
            harness,
            stderr,
            stdout
        );
    }

    Ok((temp_dir, project_path))
}

/// Helper to run init with claude harness (most common case)
fn run_init_in_temp_dir() -> Result<(TempDir, PathBuf)> {
    run_init_with_harness("claude")
}

/// Verify the exact directory structure is created for Claude harness
#[test]
fn test_init_creates_correct_directory_structure() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let cupcake_dir = project_path.join(".cupcake");

    // Verify root .cupcake directory exists
    assert!(cupcake_dir.exists(), ".cupcake directory should exist");
    assert!(cupcake_dir.is_dir(), ".cupcake should be a directory");

    // Verify all required subdirectories exist (only claude harness)
    let expected_dirs = vec![
        "policies",
        "policies/claude",
        "policies/claude/system",
        "policies/claude/builtins",
        "policies/helpers",
        "signals",
        "actions",
    ];

    for dir_name in expected_dirs {
        let dir_path = cupcake_dir.join(dir_name);
        assert!(
            dir_path.exists(),
            "Directory .cupcake/{dir_name} should exist"
        );
        assert!(
            dir_path.is_dir(),
            ".cupcake/{dir_name} should be a directory"
        );
    }

    // Verify other harness directories do NOT exist
    assert!(
        !cupcake_dir.join("policies/cursor").exists(),
        "Cursor directory should NOT exist when initializing with Claude"
    );
    assert!(
        !cupcake_dir.join("policies/factory").exists(),
        "Factory directory should NOT exist when initializing with Claude"
    );
    assert!(
        !cupcake_dir.join("policies/opencode").exists(),
        "OpenCode directory should NOT exist when initializing with Claude"
    );

    Ok(())
}

/// Verify all expected files are created for Claude harness
#[test]
fn test_init_creates_all_required_files() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let cupcake_dir = project_path.join(".cupcake");

    // List of all files that should be created (Claude harness only)
    let expected_files = vec![
        "rulebook.yml",
        "policies/example.rego",
        "policies/helpers/commands.rego",
        // Claude harness files
        "policies/claude/system/evaluate.rego",
        "policies/claude/builtins/claude_code_always_inject_on_prompt.rego",
        "policies/claude/builtins/claude_code_enforce_full_file_read.rego",
        "policies/claude/builtins/git_block_no_verify.rego",
        "policies/claude/builtins/git_pre_check.rego",
        "policies/claude/builtins/post_edit_check.rego",
        "policies/claude/builtins/protected_paths.rego",
        "policies/claude/builtins/rulebook_security_guardrails.rego",
    ];

    for file_name in expected_files {
        let file_path = cupcake_dir.join(file_name);
        assert!(file_path.exists(), "File .cupcake/{file_name} should exist");
        assert!(file_path.is_file(), ".cupcake/{file_name} should be a file");

        // Verify file is not empty
        let content = fs::read_to_string(&file_path)?;
        assert!(
            !content.is_empty(),
            "File .cupcake/{file_name} should not be empty"
        );
    }

    Ok(())
}

/// Verify rulebook.yml contains expected template content
#[test]
fn test_rulebook_yml_content() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let rulebook_path = project_path.join(".cupcake/rulebook.yml");

    let content = fs::read_to_string(&rulebook_path)?;

    // Verify key sections are present
    assert!(
        content.contains("# Cupcake Base Configuration Template"),
        "rulebook.yml should contain the template header"
    );
    assert!(
        content.contains("# SIGNALS - External data providers"),
        "rulebook.yml should contain signals section"
    );
    assert!(
        content.contains("# ACTIONS - Response to policy violations"),
        "rulebook.yml should contain actions section"
    );
    assert!(
        content.contains("# BUILTINS - Higher-level policy abstractions"),
        "rulebook.yml should contain builtins section"
    );

    // Verify builtins are documented
    assert!(
        content.contains("claude_code_always_inject_on_prompt:"),
        "rulebook.yml should document claude_code_always_inject_on_prompt builtin"
    );
    assert!(
        content.contains("git_pre_check:"),
        "rulebook.yml should document git_pre_check builtin"
    );
    assert!(
        content.contains("post_edit_check:"),
        "rulebook.yml should document post_edit_check builtin"
    );

    // Verify examples are commented out
    assert!(
        content.contains("# claude_code_always_inject_on_prompt:"),
        "Builtin examples should be commented out by default"
    );

    Ok(())
}

/// Verify system/evaluate.rego contains the authoritative aggregation policy
#[test]
fn test_system_evaluate_policy_content() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    // Check Claude harness evaluate.rego
    let evaluate_path = project_path.join(".cupcake/policies/claude/system/evaluate.rego");

    let content = fs::read_to_string(&evaluate_path)?;

    // Verify package declaration
    assert!(
        content.starts_with("package cupcake.system"),
        "evaluate.rego should start with package cupcake.system"
    );

    // Verify it has the Rego v1 import
    assert!(
        content.contains("import rego.v1"),
        "evaluate.rego should import rego.v1"
    );

    // Verify critical metadata
    assert!(
        content.contains("# METADATA"),
        "evaluate.rego should contain metadata"
    );
    assert!(
        content.contains("System Aggregation Entrypoint for Hybrid Model"),
        "evaluate.rego should have correct title"
    );
    assert!(
        content.contains("entrypoint: true"),
        "evaluate.rego should be marked as entrypoint"
    );

    // Verify the evaluate rule with all decision verbs
    assert!(
        content.contains("evaluate := decision_set if"),
        "evaluate.rego should define evaluate rule"
    );
    assert!(
        content.contains(r#""halts": collect_verbs("halt")"#),
        "evaluate.rego should collect halt verbs"
    );
    assert!(
        content.contains(r#""denials": collect_verbs("deny")"#),
        "evaluate.rego should collect deny verbs"
    );
    assert!(
        content.contains(r#""blocks": collect_verbs("block")"#),
        "evaluate.rego should collect block verbs"
    );
    assert!(
        content.contains(r#""asks": collect_verbs("ask")"#),
        "evaluate.rego should collect ask verbs"
    );

    // Verify the walk-based collection
    assert!(
        content.contains("walk(data.cupcake.policies"),
        "evaluate.rego should use walk() for policy discovery"
    );

    // Verify default empty array handling
    assert!(
        content.contains("default collect_verbs(_) := []"),
        "evaluate.rego should have default empty array for collect_verbs"
    );

    Ok(())
}

/// Verify builtin policies are properly copied
#[test]
fn test_builtin_policies_content() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    // Check Claude harness builtins
    let builtins_dir = project_path.join(".cupcake/policies/claude/builtins");

    // Test git_pre_check.rego
    let git_check_path = builtins_dir.join("git_pre_check.rego");
    let content = fs::read_to_string(&git_check_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.git_pre_check"),
        "git_pre_check.rego should have correct package"
    );
    assert!(
        content.contains("halt contains decision if"),
        "git_pre_check.rego should use halt verb"
    );

    // Test post_edit_check.rego
    let post_edit_path = builtins_dir.join("post_edit_check.rego");
    let content = fs::read_to_string(&post_edit_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.post_edit_check"),
        "post_edit_check.rego should have correct package"
    );

    // Test claude_code_always_inject_on_prompt.rego
    let inject_path = builtins_dir.join("claude_code_always_inject_on_prompt.rego");
    let content = fs::read_to_string(&inject_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.claude_code_always_inject_on_prompt"),
        "claude_code_always_inject_on_prompt.rego should have correct package"
    );
    assert!(
        content.contains("add_context contains"),
        "claude_code_always_inject_on_prompt.rego should use add_context verb"
    );

    // Test protected_paths.rego
    let protected_path = builtins_dir.join("protected_paths.rego");
    let content = fs::read_to_string(&protected_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.protected_paths"),
        "protected_paths.rego should have correct package"
    );
    assert!(
        content.contains("halt contains decision if"),
        "protected_paths.rego should use halt verb"
    );

    // Test rulebook_security_guardrails.rego
    let rulebook_path = builtins_dir.join("rulebook_security_guardrails.rego");
    let content = fs::read_to_string(&rulebook_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.rulebook_security_guardrails"),
        "rulebook_security_guardrails.rego should have correct package"
    );
    assert!(
        content.contains("halt contains decision if"),
        "rulebook_security_guardrails.rego should use halt verb"
    );

    Ok(())
}

/// Test that init is idempotent - running twice with same harness doesn't break anything
#[test]
fn test_init_idempotent() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().to_path_buf();
    let cupcake_bin = get_cupcake_binary();

    // First init
    let output = Command::new(&cupcake_bin)
        .args(["init", "--harness", "claude"])
        .current_dir(&project_path)
        .output()?;
    assert!(output.status.success(), "First init should succeed");

    // Verify it created the structure
    assert!(project_path.join(".cupcake").exists());

    // Get the content of a file to verify it doesn't change
    let rulebook_path = project_path.join(".cupcake/rulebook.yml");
    let original_content = fs::read_to_string(&rulebook_path)?;

    // Second init with same harness should be safe
    let output = Command::new(&cupcake_bin)
        .args(["init", "--harness", "claude"])
        .current_dir(&project_path)
        .output()?;
    assert!(output.status.success(), "Second init should succeed");

    // Should indicate it already exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("already initialized"),
        "Second init should indicate project already exists"
    );

    // Content should be unchanged
    let new_content = fs::read_to_string(&rulebook_path)?;
    assert_eq!(
        original_content, new_content,
        "Running init twice should not modify existing files"
    );

    Ok(())
}

/// Test that the created structure can be loaded by the engine
#[tokio::test]
#[serial(home_env)]
async fn test_init_creates_valid_engine_structure() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;

    // Try to create an engine with the initialized structure
    // This verifies that all policies compile and the structure is valid
    let engine = cupcake_core::engine::Engine::new(
        &project_path,
        cupcake_core::harness::types::HarnessType::ClaudeCode,
    )
    .await?;

    // Verify we can evaluate a simple input
    let test_input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test"
    });

    let decision = engine.evaluate(&test_input, None).await?;

    // Should get an Allow decision (no policies should fire on this simple input)
    assert!(
        matches!(
            decision,
            cupcake_core::engine::decision::FinalDecision::Allow { .. }
        ),
        "Should get Allow decision for simple test input"
    );

    Ok(())
}

/// Test file permissions are reasonable (not a security issue)
#[test]
#[cfg(unix)]
fn test_init_file_permissions() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let cupcake_dir = project_path.join(".cupcake");

    // Check directory permissions (should be readable/executable by owner)
    let dir_metadata = fs::metadata(&cupcake_dir)?;
    let dir_perms = dir_metadata.permissions();
    let dir_mode = dir_perms.mode() & 0o777;

    // Directory should be at least 0o700 (owner rwx)
    assert!(
        dir_mode & 0o700 == 0o700,
        ".cupcake directory should be readable/writable/executable by owner"
    );

    // Check a policy file permissions
    let policy_path = cupcake_dir.join("policies/claude/system/evaluate.rego");
    let file_metadata = fs::metadata(&policy_path)?;
    let file_perms = file_metadata.permissions();
    let file_mode = file_perms.mode() & 0o777;

    // File should be at least 0o600 (owner rw)
    assert!(
        file_mode & 0o600 == 0o600,
        "Policy files should be readable/writable by owner"
    );

    Ok(())
}

/// Test that empty signal/action directories don't cause issues
#[test]
fn test_empty_directories_are_valid() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;

    let signals_dir = project_path.join(".cupcake/signals");
    let actions_dir = project_path.join(".cupcake/actions");

    // Verify directories exist but are empty
    assert!(signals_dir.exists() && signals_dir.is_dir());
    assert!(actions_dir.exists() && actions_dir.is_dir());

    let signals_contents: Vec<_> = fs::read_dir(&signals_dir)?.collect();
    let actions_contents: Vec<_> = fs::read_dir(&actions_dir)?.collect();

    assert!(
        signals_contents.is_empty(),
        "signals directory should be empty initially"
    );
    assert!(
        actions_contents.is_empty(),
        "actions directory should be empty initially"
    );

    Ok(())
}

/// Verify the rulebook.yml is valid YAML
#[test]
fn test_rulebook_yml_is_valid_yaml() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let rulebook_path = project_path.join(".cupcake/rulebook.yml");

    let content = fs::read_to_string(&rulebook_path)?;

    // Basic validation - check it's not empty and has expected YAML structure markers
    assert!(!content.is_empty(), "rulebook.yml should not be empty");
    assert!(
        content.contains("signals:"),
        "rulebook.yml should contain signals section"
    );
    assert!(
        content.contains("actions:"),
        "rulebook.yml should contain actions section"
    );
    assert!(
        content.contains("builtins:"),
        "rulebook.yml should contain builtins section"
    );

    Ok(())
}

/// Verify file count matches expectations for Claude harness
#[test]
fn test_correct_number_of_files_created() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let cupcake_dir = project_path.join(".cupcake");

    // Count all files recursively
    let mut file_count = 0;
    let mut dir_count = 0;

    fn count_entries(path: &std::path::Path, files: &mut u32, dirs: &mut u32) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                *dirs += 1;
                count_entries(&path, files, dirs)?;
            } else {
                *files += 1;
            }
        }
        Ok(())
    }

    count_entries(&cupcake_dir, &mut file_count, &mut dir_count)?;

    // For Claude harness only:
    // - 1 rulebook.yml
    // - 1 example.rego
    // - 1 helper (commands.rego)
    // - Claude: 1 evaluate.rego + 7 builtins = 8 files
    // Total: 1 + 1 + 1 + 8 = 11 files
    assert_eq!(
        file_count, 11,
        "Should have exactly 11 files (1 rulebook + 1 example + 1 helper + 1 evaluate + 7 builtins)"
    );

    // We should have exactly 7 directories:
    // actions, signals, policies, policies/helpers,
    // policies/claude, policies/claude/system, policies/claude/builtins
    assert_eq!(dir_count, 7, "Should have exactly 7 directories");

    Ok(())
}

/// Test that Cursor harness creates correct structure
#[test]
fn test_init_cursor_creates_cursor_only() -> Result<()> {
    let (_temp_dir, project_path) = run_init_with_harness("cursor")?;
    let cupcake_dir = project_path.join(".cupcake");

    // Cursor directory should exist
    assert!(cupcake_dir.join("policies/cursor/system").exists());
    assert!(cupcake_dir.join("policies/cursor/builtins").exists());

    // Other harness directories should NOT exist
    assert!(
        !cupcake_dir.join("policies/claude").exists(),
        "Claude directory should NOT exist when initializing with Cursor"
    );
    assert!(
        !cupcake_dir.join("policies/factory").exists(),
        "Factory directory should NOT exist when initializing with Cursor"
    );
    assert!(
        !cupcake_dir.join("policies/opencode").exists(),
        "OpenCode directory should NOT exist when initializing with Cursor"
    );

    // Cursor should have 5 builtins (no always_inject_on_prompt or enforce_full_file_read)
    let builtins_dir = cupcake_dir.join("policies/cursor/builtins");
    let builtin_count = fs::read_dir(&builtins_dir)?.count();
    assert_eq!(builtin_count, 5, "Cursor should have 5 builtins");

    Ok(())
}

/// Test that OpenCode harness creates correct structure
#[test]
fn test_init_opencode_creates_opencode_only() -> Result<()> {
    let (_temp_dir, project_path) = run_init_with_harness("opencode")?;
    let cupcake_dir = project_path.join(".cupcake");

    // OpenCode directory should exist
    assert!(cupcake_dir.join("policies/opencode/system").exists());
    assert!(cupcake_dir.join("policies/opencode/builtins").exists());

    // Other harness directories should NOT exist
    assert!(
        !cupcake_dir.join("policies/claude").exists(),
        "Claude directory should NOT exist when initializing with OpenCode"
    );
    assert!(
        !cupcake_dir.join("policies/cursor").exists(),
        "Cursor directory should NOT exist when initializing with OpenCode"
    );
    assert!(
        !cupcake_dir.join("policies/factory").exists(),
        "Factory directory should NOT exist when initializing with OpenCode"
    );

    // OpenCode should have 7 builtins (same as Claude)
    let builtins_dir = cupcake_dir.join("policies/opencode/builtins");
    let builtin_count = fs::read_dir(&builtins_dir)?.count();
    assert_eq!(builtin_count, 7, "OpenCode should have 7 builtins");

    Ok(())
}

/// Test that Factory harness creates correct structure
#[test]
fn test_init_factory_creates_factory_only() -> Result<()> {
    let (_temp_dir, project_path) = run_init_with_harness("factory")?;
    let cupcake_dir = project_path.join(".cupcake");

    // Factory directory should exist
    assert!(cupcake_dir.join("policies/factory/system").exists());
    assert!(cupcake_dir.join("policies/factory/builtins").exists());

    // Other harness directories should NOT exist
    assert!(
        !cupcake_dir.join("policies/claude").exists(),
        "Claude directory should NOT exist when initializing with Factory"
    );
    assert!(
        !cupcake_dir.join("policies/cursor").exists(),
        "Cursor directory should NOT exist when initializing with Factory"
    );
    assert!(
        !cupcake_dir.join("policies/opencode").exists(),
        "OpenCode directory should NOT exist when initializing with Factory"
    );

    // Factory should have 7 builtins (same as Claude)
    let builtins_dir = cupcake_dir.join("policies/factory/builtins");
    let builtin_count = fs::read_dir(&builtins_dir)?.count();
    assert_eq!(builtin_count, 7, "Factory should have 7 builtins");

    Ok(())
}

/// Test adding a second harness to an existing project
#[test]
fn test_init_can_add_second_harness() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().to_path_buf();
    let cupcake_bin = get_cupcake_binary();

    // First init with claude
    let output = Command::new(&cupcake_bin)
        .args(["init", "--harness", "claude"])
        .current_dir(&project_path)
        .output()?;
    assert!(
        output.status.success(),
        "First init with claude should succeed"
    );

    // Verify only claude exists
    assert!(project_path.join(".cupcake/policies/claude").exists());
    assert!(!project_path.join(".cupcake/policies/cursor").exists());

    // Second init with cursor
    let output = Command::new(&cupcake_bin)
        .args(["init", "--harness", "cursor"])
        .current_dir(&project_path)
        .output()?;
    assert!(
        output.status.success(),
        "Second init with cursor should succeed"
    );

    // Verify output indicates adding harness
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Added cursor harness"),
        "Should indicate adding cursor harness to existing project"
    );

    // Both should now exist
    assert!(
        project_path
            .join(".cupcake/policies/claude/system")
            .exists(),
        "Claude should still exist after adding cursor"
    );
    assert!(
        project_path
            .join(".cupcake/policies/cursor/system")
            .exists(),
        "Cursor should exist after being added"
    );

    // Verify cursor has correct builtins
    let cursor_builtins_dir = project_path.join(".cupcake/policies/cursor/builtins");
    let cursor_builtin_count = fs::read_dir(&cursor_builtins_dir)?.count();
    assert_eq!(cursor_builtin_count, 5, "Cursor should have 5 builtins");

    // Verify claude still has its builtins
    let claude_builtins_dir = project_path.join(".cupcake/policies/claude/builtins");
    let claude_builtin_count = fs::read_dir(&claude_builtins_dir)?.count();
    assert_eq!(
        claude_builtin_count, 7,
        "Claude should still have 7 builtins"
    );

    Ok(())
}

/// Test that init without --harness flag prompts for selection (or fails with no stdin)
#[test]
fn test_init_without_harness_requires_selection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cupcake_bin = get_cupcake_binary();

    // Run without --harness flag and with empty stdin
    let output = Command::new(&cupcake_bin)
        .arg("init")
        .current_dir(temp_dir.path())
        .stdin(std::process::Stdio::null()) // No interactive input
        .output()?;

    // Should fail because no harness was selected
    // (stdin is null so the interactive prompt can't get input)
    assert!(
        !output.status.success(),
        "Init without --harness and no stdin should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show the selection menu in stdout or indicate invalid selection
    assert!(
        stdout.contains("Select a harness to initialize")
            || stderr.contains("Invalid selection")
            || stderr.contains("error"),
        "Should show harness selection menu or error. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    Ok(())
}

/// Test that each harness can be loaded by the engine
#[tokio::test]
#[serial(home_env)]
async fn test_all_harnesses_create_valid_engine_structures() -> Result<()> {
    // Test each harness type
    let harnesses = [
        (
            "claude",
            cupcake_core::harness::types::HarnessType::ClaudeCode,
        ),
        ("cursor", cupcake_core::harness::types::HarnessType::Cursor),
        (
            "factory",
            cupcake_core::harness::types::HarnessType::Factory,
        ),
        (
            "opencode",
            cupcake_core::harness::types::HarnessType::OpenCode,
        ),
    ];

    for (harness_name, harness_type) in harnesses {
        let (_temp_dir, project_path) = run_init_with_harness(harness_name)?;

        // Try to create an engine with the initialized structure
        let engine = cupcake_core::engine::Engine::new(&project_path, harness_type)
            .await
            .with_context(|| format!("Engine creation failed for {harness_name} harness"))?;

        // Verify we can evaluate a simple input
        let test_input = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": "test"
        });

        let decision = engine.evaluate(&test_input, None).await?;

        assert!(
            matches!(
                decision,
                cupcake_core::engine::decision::FinalDecision::Allow { .. }
            ),
            "{harness_name} harness should return Allow decision for simple test input"
        );
    }

    Ok(())
}

use anyhow::Context;
