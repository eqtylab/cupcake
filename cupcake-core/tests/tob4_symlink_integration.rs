//! Integration tests demonstrating TOB-4 (symlink bypass) fix
//!
//! These tests demonstrate how Rust-level symlink resolution in preprocessing
//! prevents bypass attacks where attackers create symlinks to protected directories.
//!
//! The key insight: By resolving symlinks in Rust BEFORE policy evaluation,
//! we provide universal protection that works for ALL policies without requiring
//! policy authors to implement secure symlink handling.

use cupcake_core::harness::types::HarnessType;
use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
use serde_json::json;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

/// Test that symlink to .cupcake/ file is detected and resolved
#[test]
fn test_tob4_cupcake_symlink_bypass_blocked() {
    let temp_dir = TempDir::new().unwrap();

    // Create .cupcake directory with a policy file
    let cupcake_dir = temp_dir.path().join(".cupcake");
    let policies_dir = cupcake_dir.join("policies");
    fs::create_dir_all(&policies_dir).unwrap();

    let policy_file = policies_dir.join("test.rego");
    fs::write(&policy_file, "package test").unwrap();

    // Create symlink directly to the policy FILE (not directory)
    // This is the attack: symlink to a protected file with an innocent name
    let symlink_path = temp_dir.path().join("innocent.rego");
    symlink(&policy_file, &symlink_path).unwrap();

    // Try to write through the symlink
    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap(),
            "content": "package evil\n\nhalt contains {}"
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing (TOB-4 fix)
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify symlink was detected and resolved
    assert_eq!(event["is_symlink"], json!(true), "Symlink should be detected");
    assert!(event.get("resolved_file_path").is_some(), "Should have resolved path");

    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(
        resolved.contains(".cupcake"),
        "Resolved path should contain .cupcake: {}",
        resolved
    );
}

/// Test that symlink to protected path is blocked by protected_paths builtin
#[test]
fn test_tob4_protected_paths_symlink_bypass_blocked() {
    let temp_dir = TempDir::new().unwrap();

    // Create a protected directory
    let protected_dir = temp_dir.path().join("protected");
    fs::create_dir_all(&protected_dir).unwrap();

    let protected_file = protected_dir.join("important.txt");
    fs::write(&protected_file, "important data").unwrap();

    // Create symlink to protected file
    let symlink_path = temp_dir.path().join("safe.txt");
    symlink(&protected_file, &symlink_path).unwrap();

    // Try to write through the symlink
    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap(),
            "old_string": "important",
            "new_string": "HACKED"
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify symlink was detected and resolved
    assert_eq!(event["is_symlink"], json!(true));

    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(
        resolved.contains("protected"),
        "Resolved path should contain protected directory: {}",
        resolved
    );
}

/// Test that symlink to system path is detected
#[test]
fn test_tob4_system_protection_symlink_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a fake system file (we can't actually create /etc/passwd symlink in tests)
    let fake_etc = temp_dir.path().join("fake_etc");
    fs::create_dir_all(&fake_etc).unwrap();

    let fake_passwd = fake_etc.join("passwd");
    fs::write(&fake_passwd, "root:x:0:0:root:/root:/bin/bash").unwrap();

    // Create symlink to fake system file
    let symlink_path = temp_dir.path().join("myfile.txt");
    symlink(&fake_passwd, &symlink_path).unwrap();

    // Try to read through the symlink
    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap()
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify symlink was detected and resolved
    assert_eq!(event["is_symlink"], json!(true));
    assert!(event.get("resolved_file_path").is_some());

    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(
        resolved.contains("passwd"),
        "Resolved path should contain passwd: {}",
        resolved
    );
}

/// Test that symlink to sensitive file is detected
#[test]
fn test_tob4_sensitive_data_symlink_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create a fake .env file
    let env_file = temp_dir.path().join(".env");
    fs::write(&env_file, "API_KEY=secret123\nDB_PASSWORD=password456").unwrap();

    // Create symlink to .env file
    let symlink_path = temp_dir.path().join("config.txt");
    symlink(&env_file, &symlink_path).unwrap();

    // Try to read through the symlink
    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap()
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify symlink was detected and resolved
    assert_eq!(event["is_symlink"], json!(true));

    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(
        resolved.ends_with(".env"),
        "Resolved path should end with .env: {}",
        resolved
    );
}

/// Test that dangling symlinks are still detected and resolved
#[test]
fn test_tob4_dangling_symlink_to_cupcake() {
    let temp_dir = TempDir::new().unwrap();

    // Create symlink to .cupcake directory that doesn't exist yet
    let nonexistent_cupcake = temp_dir.path().join(".cupcake").join("policies").join("evil.rego");
    let symlink_path = temp_dir.path().join("safe.rego");

    // Create dangling symlink
    symlink(&nonexistent_cupcake, &symlink_path).unwrap();

    // Verify the symlink exists but target doesn't
    assert!(symlink_path.symlink_metadata().is_ok(), "Symlink should exist");
    assert!(!nonexistent_cupcake.exists(), "Target should not exist");

    // Try to write through the dangling symlink
    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap(),
            "content": "package evil"
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify symlink was detected
    assert_eq!(event["is_symlink"], json!(true), "Dangling symlink should be detected");

    // Verify resolved path contains .cupcake (even though target doesn't exist)
    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(
        resolved.contains(".cupcake"),
        "Resolved path should contain .cupcake: {}",
        resolved
    );
}

/// Test that regular files ARE canonicalized (always-on approach)
#[test]
fn test_tob4_regular_file_not_flagged() {
    let temp_dir = TempDir::new().unwrap();

    // Create a regular file (not a symlink)
    let regular_file = temp_dir.path().join("regular.txt");
    fs::write(&regular_file, "test content").unwrap();

    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": regular_file.to_str().unwrap(),
            "content": "new content"
        },
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // TOB-4 always-on: Verify regular files ARE canonicalized
    assert_eq!(event["is_symlink"], json!(false), "Regular file should be marked as NOT a symlink");
    assert!(event.get("resolved_file_path").is_some(), "Should ALWAYS have canonical path");

    // Verify the canonical path is absolute
    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(resolved.starts_with("/") || resolved.contains(":\\"), "Canonical path should be absolute");
}

/// Test that symlink resolution can be disabled
#[test]
fn test_tob4_can_be_disabled() {
    let temp_dir = TempDir::new().unwrap();

    // Create symlink
    let target = temp_dir.path().join("target.txt");
    fs::write(&target, "test").unwrap();

    let symlink_path = temp_dir.path().join("link.txt");
    symlink(&target, &symlink_path).unwrap();

    let mut event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {
            "file_path": symlink_path.to_str().unwrap()
        }
    });

    // Disable symlink resolution
    let preprocess_config = PreprocessConfig {
        enable_symlink_resolution: false,
        ..Default::default()
    };
    preprocess_input(&mut event, &preprocess_config, HarnessType::ClaudeCode);

    // Verify symlink was NOT resolved
    assert!(event.get("is_symlink").is_none());
    assert!(event.get("resolved_file_path").is_none());
}

/// Test Cursor harness symlink detection
#[test]
fn test_tob4_cursor_harness() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file inside .cupcake
    let cupcake_dir = temp_dir.path().join(".cupcake");
    fs::create_dir_all(&cupcake_dir).unwrap();
    let target_file = cupcake_dir.join("test.txt");
    fs::write(&target_file, "test").unwrap();

    // Create symlink directly to the FILE
    let symlink_path = temp_dir.path().join("safe.txt");
    symlink(&target_file, &symlink_path).unwrap();

    // Cursor uses different event structure
    let mut event = json!({
        "hook_event_name": "beforeFileWrite",
        "file_path": symlink_path.to_str().unwrap(),
        "cwd": temp_dir.path().to_str().unwrap()
    });

    // Apply preprocessing with Cursor harness
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut event, &preprocess_config, HarnessType::Cursor);

    // Verify symlink was detected
    assert_eq!(event["is_symlink"], json!(true));

    let resolved = event["resolved_file_path"].as_str().unwrap();
    assert!(resolved.contains(".cupcake"));
}
