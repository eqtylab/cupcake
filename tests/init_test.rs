//! Integration tests for the cupcake init command

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run cupcake init in a directory
fn run_init(dir: &Path) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("init")
        .current_dir(dir)
        .output()
}

#[test]
fn test_init_creates_structure() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run init
    let output = run_init(temp_dir.path()).unwrap();
    assert!(output.status.success(), "Init command failed");
    
    // Verify structure
    assert!(temp_dir.path().join(".cupcake").exists());
    assert!(temp_dir.path().join(".cupcake/policies").exists());
    assert!(temp_dir.path().join(".cupcake/policies/system").exists());
    assert!(temp_dir.path().join(".cupcake/policies/system/evaluate.rego").exists());
    assert!(temp_dir.path().join(".cupcake/policies/example.rego").exists());
    assert!(temp_dir.path().join(".cupcake/signals").exists());
    assert!(temp_dir.path().join(".cupcake/actions").exists());
    
    // Verify output contains success message
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✅ Initialized Cupcake project"));
}

#[test]
fn test_init_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run init first time
    let output1 = run_init(temp_dir.path()).unwrap();
    assert!(output1.status.success());
    
    // Run init second time
    let output2 = run_init(temp_dir.path()).unwrap();
    assert!(output2.status.success());
    
    // Should say it already exists
    let stdout = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout.contains("already initialized"));
}

#[test]
fn test_evaluate_file_valid() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run init
    run_init(temp_dir.path()).unwrap();
    
    // Read the evaluate.rego file
    let evaluate_path = temp_dir.path()
        .join(".cupcake/policies/system/evaluate.rego");
    let content = fs::read_to_string(evaluate_path).unwrap();
    
    // Check critical content
    assert!(content.contains("package cupcake.system"));
    assert!(content.contains("import rego.v1"));
    assert!(content.contains("evaluate :="));
    assert!(content.contains("collect_verbs"));
    assert!(content.contains("halts"));
    assert!(content.contains("denies"));
}

#[test]
fn test_example_policy_valid() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run init
    run_init(temp_dir.path()).unwrap();
    
    // Read the example.rego file
    let example_path = temp_dir.path()
        .join(".cupcake/policies/example.rego");
    let content = fs::read_to_string(example_path).unwrap();
    
    // Check critical content
    assert!(content.contains("package cupcake.policies.example"));
    assert!(content.contains("import rego.v1"));
    assert!(content.contains("# METADATA"));
    assert!(content.contains("required_events"));
    // Should have a rule that never fires
    assert!(content.contains("deny contains decision"));
    assert!(content.contains("CUPCAKE_EXAMPLE_RULE_THAT_NEVER_FIRES"));
}

#[test]
fn test_engine_accepts_init_structure() {
    let temp_dir = TempDir::new().unwrap();
    
    // Run init
    run_init(temp_dir.path()).unwrap();
    
    // Try to verify with the engine
    let output = Command::new(env!("CARGO_BIN_EXE_cupcake"))
        .arg("verify")
        .arg("--policy-dir")
        .arg(temp_dir.path())
        .output()
        .unwrap();
    
    assert!(output.status.success(), "Verify command failed on init structure");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verification complete"));
}