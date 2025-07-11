use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_cli_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute cupcake --help");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Cupcake: Deterministic policy enforcement for Claude Code"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("sync"));
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("audit"));
}

#[test]
fn test_cli_init_command() {
    let temp_dir = tempdir().unwrap();
    let output_file = temp_dir.path().join("test-cupcake.toml");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "init", "--output", output_file.to_str().unwrap(), "--yes"])
        .output()
        .expect("Failed to execute cupcake init");
    
    // Should not panic and should indicate implementation is pending
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("init command"));
    assert!(stdout.contains("implementation pending"));
}

#[test]
fn test_cli_run_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "run", "--event", "PreToolUse", "--debug"])
        .output()
        .expect("Failed to execute cupcake run");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("run command"));
    assert!(stdout.contains("implementation pending"));
}

#[test]
fn test_cli_sync_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "sync", "--dry-run"])
        .output()
        .expect("Failed to execute cupcake sync");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("sync command"));
    assert!(stdout.contains("implementation pending"));
}

#[test]
fn test_cli_validate_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "validate", "test-policy.toml", "--strict"])
        .output()
        .expect("Failed to execute cupcake validate");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("validate command"));
    assert!(stdout.contains("implementation pending"));
}

#[test]
fn test_cli_audit_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "audit", "--tail", "10", "--format", "json"])
        .output()
        .expect("Failed to execute cupcake audit");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("audit command"));
    assert!(stdout.contains("implementation pending"));
}

#[test]
fn test_cli_init_with_verbose() {
    let temp_dir = tempdir().unwrap();
    let output_file = temp_dir.path().join("verbose-test.toml");
    
    let output = Command::new("cargo")
        .args(&["run", "--", "init", "--output", output_file.to_str().unwrap(), "--verbose", "--yes"])
        .output()
        .expect("Failed to execute cupcake init --verbose");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Output file:"));
    assert!(stdout.contains("Auto-confirm:"));
}

#[test]
fn test_cli_run_with_debug() {
    let output = Command::new("cargo")
        .args(&["run", "--", "run", "--event", "PostToolUse", "--debug", "--timeout", "30"])
        .output()
        .expect("Failed to execute cupcake run --debug");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Event: PostToolUse"));
    assert!(stdout.contains("Timeout: 30s"));
}

#[test]
fn test_cli_sync_with_force() {
    let output = Command::new("cargo")
        .args(&["run", "--", "sync", "--force", "--dry-run"])
        .output()
        .expect("Failed to execute cupcake sync --force");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Force: true"));
    assert!(stdout.contains("Dry run: true"));
}

#[test]
fn test_cli_validate_with_format() {
    let output = Command::new("cargo")
        .args(&["run", "--", "validate", "policy.toml", "--format", "json"])
        .output()
        .expect("Failed to execute cupcake validate with format");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Format: json"));
}

#[test]
fn test_cli_audit_with_filters() {
    let output = Command::new("cargo")
        .args(&["run", "--", "audit", "--session", "test-session", "--event", "PreToolUse", "--follow"])
        .output()
        .expect("Failed to execute cupcake audit with filters");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Session filter: Some(\"test-session\")"));
    assert!(stdout.contains("Event filter: Some(\"PreToolUse\")"));
    assert!(stdout.contains("Follow: true"));
}

#[test]
fn test_cli_invalid_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "invalid-command"])
        .output()
        .expect("Failed to execute cupcake with invalid command");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error") || stderr.contains("unrecognized"));
}

#[test]
fn test_cli_missing_required_args() {
    let output = Command::new("cargo")
        .args(&["run", "--", "run"])
        .output()
        .expect("Failed to execute cupcake run without required args");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("required") || stderr.contains("error"));
}

#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute cupcake --version");
    
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("cupcake") && stdout.contains("0.1.0"));
}

#[test]
fn test_cli_default_values() {
    let output = Command::new("cargo")
        .args(&["run", "--", "run", "--event", "PreToolUse"])
        .output()
        .expect("Failed to execute cupcake run with defaults");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Timeout: 60s"));
    assert!(stdout.contains("Policy file: cupcake.toml"));
}

#[test]
fn test_cli_init_default_output() {
    let output = Command::new("cargo")
        .args(&["run", "--", "init", "--yes"])
        .output()
        .expect("Failed to execute cupcake init with default output");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Output file: cupcake.toml"));
}

#[test]
fn test_cli_validate_default_file() {
    let output = Command::new("cargo")
        .args(&["run", "--", "validate"])
        .output()
        .expect("Failed to execute cupcake validate with default file");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Policy file: cupcake.toml"));
}

#[test]
fn test_cli_audit_default_format() {
    let output = Command::new("cargo")
        .args(&["run", "--", "audit", "--tail", "5"])
        .output()
        .expect("Failed to execute cupcake audit with default format");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Format: text"));
}

#[test]
fn test_cli_all_subcommands_exist() {
    // Test that all expected subcommands are available
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute cupcake --help");
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected_commands = vec!["init", "run", "sync", "validate", "audit"];
    
    for cmd in expected_commands {
        assert!(stdout.contains(cmd), "Command '{}' not found in help output", cmd);
    }
}