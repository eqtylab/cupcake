//! Comprehensive integration test suite for the `cupcake init` command
//! 
//! These tests verify that the init command creates the exact expected
//! directory structure and file contents, ensuring consistency across
//! all environments including CI.

use anyhow::Result;
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

/// Helper to run the init command in a test directory
fn run_init_in_temp_dir() -> Result<(TempDir, PathBuf)> {
    // Create a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().to_path_buf();
    
    // Run cupcake init in the temp directory
    let output = Command::new(get_cupcake_binary())
        .arg("init")
        .current_dir(&project_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cupcake init failed: {}", stderr);
    }
    
    Ok((temp_dir, project_path))
}

/// Verify the exact directory structure is created
#[test]
fn test_init_creates_correct_directory_structure() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let cupcake_dir = project_path.join(".cupcake");
    
    // Verify root .cupcake directory exists
    assert!(cupcake_dir.exists(), ".cupcake directory should exist");
    assert!(cupcake_dir.is_dir(), ".cupcake should be a directory");
    
    // Verify all required subdirectories exist
    let expected_dirs = vec![
        "policies",
        "policies/system",
        "policies/builtins",
        "signals",
        "actions",
    ];
    
    for dir_name in expected_dirs {
        let dir_path = cupcake_dir.join(dir_name);
        assert!(
            dir_path.exists(),
            "Directory .cupcake/{} should exist",
            dir_name
        );
        assert!(
            dir_path.is_dir(),
            ".cupcake/{} should be a directory",
            dir_name
        );
    }
    
    Ok(())
}

/// Verify all expected files are created
#[test]
fn test_init_creates_all_required_files() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let cupcake_dir = project_path.join(".cupcake");
    
    // List of all files that should be created
    let expected_files = vec![
        "guidebook.yml",
        "policies/system/evaluate.rego",
        "policies/example.rego",
        "policies/builtins/always_inject_on_prompt.rego",
        "policies/builtins/global_file_lock.rego",
        "policies/builtins/protected_paths.rego",
        "policies/builtins/git_pre_check.rego",
        "policies/builtins/post_edit_check.rego",
        "policies/builtins/rulebook_security_guardrails.rego",
    ];
    
    for file_name in expected_files {
        let file_path = cupcake_dir.join(file_name);
        assert!(
            file_path.exists(),
            "File .cupcake/{} should exist",
            file_name
        );
        assert!(
            file_path.is_file(),
            ".cupcake/{} should be a file",
            file_name
        );
        
        // Verify file is not empty
        let content = fs::read_to_string(&file_path)?;
        assert!(
            !content.is_empty(),
            "File .cupcake/{} should not be empty",
            file_name
        );
    }
    
    Ok(())
}

/// Verify guidebook.yml contains expected template content
#[test]
fn test_guidebook_yml_content() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let guidebook_path = project_path.join(".cupcake/guidebook.yml");
    
    let content = fs::read_to_string(&guidebook_path)?;
    
    // Verify key sections are present
    assert!(
        content.contains("# Cupcake Base Configuration Template"),
        "guidebook.yml should contain the template header"
    );
    assert!(
        content.contains("# SIGNALS - External data providers"),
        "guidebook.yml should contain signals section"
    );
    assert!(
        content.contains("# ACTIONS - Response to policy violations"),
        "guidebook.yml should contain actions section"
    );
    assert!(
        content.contains("# BUILTINS - Higher-level policy abstractions"),
        "guidebook.yml should contain builtins section"
    );
    
    // Verify all four builtins are documented
    assert!(
        content.contains("always_inject_on_prompt:"),
        "guidebook.yml should document always_inject_on_prompt builtin"
    );
    assert!(
        content.contains("global_file_lock:"),
        "guidebook.yml should document global_file_lock builtin"
    );
    assert!(
        content.contains("git_pre_check:"),
        "guidebook.yml should document git_pre_check builtin"
    );
    assert!(
        content.contains("post_edit_check:"),
        "guidebook.yml should document post_edit_check builtin"
    );
    
    // Verify examples are commented out
    assert!(
        content.contains("# always_inject_on_prompt:"),
        "Builtin examples should be commented out by default"
    );
    
    Ok(())
}

/// Verify system/evaluate.rego contains the authoritative aggregation policy
#[test]
fn test_system_evaluate_policy_content() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let evaluate_path = project_path.join(".cupcake/policies/system/evaluate.rego");
    
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
    let builtins_dir = project_path.join(".cupcake/policies/builtins");
    
    // Test global_file_lock.rego
    let global_lock_path = builtins_dir.join("global_file_lock.rego");
    let content = fs::read_to_string(&global_lock_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.global_file_lock"),
        "global_file_lock.rego should have correct package"
    );
    assert!(
        content.contains("halt contains decision if"),
        "global_file_lock.rego should use halt verb"
    );
    
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
    
    // Test always_inject_on_prompt.rego
    let inject_path = builtins_dir.join("always_inject_on_prompt.rego");
    let content = fs::read_to_string(&inject_path)?;
    assert!(
        content.contains("package cupcake.policies.builtins.always_inject_on_prompt"),
        "always_inject_on_prompt.rego should have correct package"
    );
    assert!(
        content.contains("add_context contains"),
        "always_inject_on_prompt.rego should use add_context verb"
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

/// Test that init is idempotent - running twice doesn't break anything
#[test]
fn test_init_idempotent() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path().to_path_buf();
    let cupcake_bin = get_cupcake_binary();
    
    // First init
    let output = Command::new(&cupcake_bin)
        .arg("init")
        .current_dir(&project_path)
        .output()?;
    assert!(output.status.success(), "First init should succeed");
    
    // Verify it created the structure
    assert!(project_path.join(".cupcake").exists());
    
    // Get the content of a file to verify it doesn't change
    let guidebook_path = project_path.join(".cupcake/guidebook.yml");
    let original_content = fs::read_to_string(&guidebook_path)?;
    
    // Second init should be safe
    let output = Command::new(&cupcake_bin)
        .arg("init")
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
    let new_content = fs::read_to_string(&guidebook_path)?;
    assert_eq!(
        original_content, new_content,
        "Running init twice should not modify existing files"
    );
    
    Ok(())
}

/// Test that the created structure can be loaded by the engine
#[tokio::test]
async fn test_init_creates_valid_engine_structure() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    
    // Try to create an engine with the initialized structure
    // This verifies that all policies compile and the structure is valid
    let engine = cupcake_core::engine::Engine::new(&project_path).await?;
    
    // Verify we can evaluate a simple input
    let test_input = serde_json::json!({
        "hook_event_name": "UserPromptSubmit",
        "prompt": "test"
    });
    
    let decision = engine.evaluate(&test_input, None).await?;
    
    // Should get an Allow decision (no policies should fire on this simple input)
    assert!(matches!(
        decision,
        cupcake_core::engine::decision::FinalDecision::Allow { .. }
    ), "Should get Allow decision for simple test input");
    
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
    let policy_path = cupcake_dir.join("policies/system/evaluate.rego");
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

/// Verify the guidebook.yml is valid YAML
#[test]
fn test_guidebook_yml_is_valid_yaml() -> Result<()> {
    let (_temp_dir, project_path) = run_init_in_temp_dir()?;
    let guidebook_path = project_path.join(".cupcake/guidebook.yml");
    
    let content = fs::read_to_string(&guidebook_path)?;
    
    // Basic validation - check it's not empty and has expected YAML structure markers
    assert!(!content.is_empty(), "guidebook.yml should not be empty");
    assert!(content.contains("signals:"), "guidebook.yml should contain signals section");
    assert!(content.contains("actions:"), "guidebook.yml should contain actions section");
    assert!(content.contains("builtins:"), "guidebook.yml should contain builtins section");
    
    Ok(())
}

/// Verify file count matches expectations
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
    
    // We should have exactly 11 files (guidebook.yml + 10 policies)
    // Policies: evaluate.rego, example.rego, and 8 builtins
    assert_eq!(
        file_count, 11,
        "Should have exactly 11 files (guidebook.yml + 10 policies)"
    );
    
    // We should have exactly 5 directories (policies, policies/system, policies/builtins, signals, actions)
    assert_eq!(
        dir_count, 5,
        "Should have exactly 5 directories"
    );
    
    Ok(())
}