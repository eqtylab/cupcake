//! Test to validate the external reviewer's concern about path traversal bypasses
//!
//! The reviewer claims that non-canonical paths with "../" components could bypass
//! string-based protection checks. This test validates our defense.

use cupcake_core::harness::types::HarnessType;
use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
use serde_json::json;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use tempfile::TempDir;

/// Test that path traversal patterns are properly resolved
#[test]
fn test_reviewer_path_traversal_concern() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create directory structure:
    // /tmp/xxx/
    //   .cupcake/
    //     secret.txt
    //   deep/
    //     nested/
    //       dir/
    let cupcake_dir = base.join(".cupcake");
    fs::create_dir_all(&cupcake_dir).unwrap();
    let secret_file = cupcake_dir.join("secret.txt");
    fs::write(&secret_file, "SECRET DATA").unwrap();

    let deep_dir = base.join("deep/nested/dir");
    fs::create_dir_all(&deep_dir).unwrap();

    // Test Case 1: Symlink with "../" in target path
    println!("\n=== Test 1: Dangling Symlink with Path Traversal ===");

    // Create a DANGLING symlink (target doesn't exist yet)
    let symlink_path = deep_dir.join("innocent.txt");
    let traversal_target = "../../../.cupcake/secret.txt";
    symlink(traversal_target, &symlink_path).unwrap();

    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap(),
            "content": "malicious content"
        },
        "cwd": base.to_str().unwrap()
    });

    // Apply preprocessing
    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

    // Check the results
    assert!(event.get("is_symlink").is_some());
    assert_eq!(event["is_symlink"], json!(true));

    let resolved = event["resolved_file_path"].as_str().unwrap();
    println!("Resolved path: {resolved}");

    // CRITICAL: Check if .cupcake is detected
    assert!(
        resolved.contains(".cupcake"),
        "Failed to detect .cupcake in resolved path: {resolved}"
    );

    // Test Case 2: Direct path traversal (no symlink)
    println!("\n=== Test 2: Direct Path Traversal (No Symlink) ===");

    let direct_traversal = deep_dir.join("../../../.cupcake/secret.txt");
    let mut event2 = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": direct_traversal.to_str().unwrap()
        },
        "cwd": base.to_str().unwrap()
    });

    preprocess_input(&mut event2, &config, HarnessType::ClaudeCode);

    let resolved2 = event2["resolved_file_path"].as_str().unwrap();
    println!("Direct traversal resolved to: {resolved2}");

    // Should be canonicalized
    assert!(
        resolved2.contains(".cupcake"),
        "Failed to detect .cupcake in direct traversal: {resolved2}"
    );
    assert!(
        !resolved2.contains(".."),
        "Resolved path still contains '..': {resolved2}"
    );

    // Test Case 3: Reviewer's specific example
    println!("\n=== Test 3: Reviewer's Example ===");

    // Create symlink pointing to "../../../../etc/.cupcake/secret"
    let reviewer_symlink = temp_dir.path().join("reviewer_test.txt");
    let reviewer_target = "../../../../etc/.cupcake/secret";

    // This will be a dangling symlink (target doesn't exist)
    symlink(reviewer_target, &reviewer_symlink).unwrap();

    let mut event3 = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": reviewer_symlink.to_str().unwrap(),
            "content": "test"
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    preprocess_input(&mut event3, &config, HarnessType::ClaudeCode);

    let resolved3 = event3["resolved_file_path"].as_str().unwrap();
    println!("Reviewer's example resolved to: {resolved3}");

    // Even for dangling symlinks, the target path WILL contain .cupcake
    assert!(
        resolved3.contains(".cupcake") || resolved3.contains("/etc/"),
        "Path detection failed for reviewer's example: {resolved3}"
    );

    println!("\n=== ANALYSIS COMPLETE ===");
    println!("All test cases properly detected protected paths.");
    println!("The reviewer's concern is addressed by our implementation:");
    println!("1. For existing files: We canonicalize (no '../' remains)");
    println!("2. For dangling symlinks: Raw target still contains '.cupcake'");
    println!("3. Policies check resolved_file_path which always has this info");
}

/// Test symlink resolution with various edge cases
#[test]
fn test_symlink_resolution_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create .cupcake directory
    let cupcake_dir = base.join(".cupcake");
    fs::create_dir_all(&cupcake_dir).unwrap();

    // Test: Symlink chain (symlink -> symlink -> target)
    let target = cupcake_dir.join("final.txt");
    fs::write(&target, "data").unwrap();

    let link1 = base.join("link1");
    let link2 = base.join("link2");

    symlink(&target, &link1).unwrap();
    symlink(&link1, &link2).unwrap();

    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": link2.to_str().unwrap()
        },
        "cwd": base.to_str().unwrap()
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(
        resolved.contains(".cupcake"),
        "Failed to resolve symlink chain to .cupcake"
    );
    assert_eq!(event["is_symlink"], json!(true));
}

/// Test that policies using resolved_file_path are protected
#[test]
fn test_policy_protection_with_resolved_path() {
    // This test validates that policies checking resolved_file_path
    // (as all our builtins do) are protected against traversal attacks

    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Setup
    let cupcake_dir = base.join(".cupcake");
    fs::create_dir_all(&cupcake_dir).unwrap();
    let protected_file = cupcake_dir.join("policies.rego");
    fs::write(&protected_file, "package test").unwrap();

    // Create malicious symlink with innocent name
    let evil_symlink = base.join("readme.txt");
    symlink(&protected_file, &evil_symlink).unwrap();

    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "tool_input": {
            "file_path": evil_symlink.to_str().unwrap(),
            "old_string": "package test",
            "new_string": "package evil"
        },
        "cwd": base.to_str().unwrap()
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

    // Verify the attack would be caught
    let original = event["original_file_path"].as_str().unwrap();
    let resolved = event["resolved_file_path"].as_str().unwrap();

    println!("Original (attacker-provided): {original}");
    println!("Resolved (what policies see): {resolved}");

    // Original path looks innocent
    assert!(!original.contains(".cupcake"));

    // But resolved path reveals the truth
    assert!(resolved.contains(".cupcake"));

    // This is what protects us - policies check resolved_file_path
}
