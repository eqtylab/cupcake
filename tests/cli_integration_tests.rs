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
    assert!(stdout.contains("transforms natural language rules"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("sync"));
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("audit"));
}

#[test]
fn test_cli_init_command() {
    let temp_dir = tempdir().unwrap();
    let output_dir = temp_dir.path().join("test-guardrails");

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "init",
            "--output",
            output_dir.to_str().unwrap(),
            "--yes",
        ])
        .output()
        .expect("Failed to execute cupcake init");

    // Should succeed and create the guardrails structure
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Cupcake guardrails initialized successfully"));
    assert!(stdout.contains("Created structure"));
}

#[test]
fn test_cli_run_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "run", "--event", "PreToolUse", "--debug"])
        .output()
        .expect("Failed to execute cupcake run");

    // Will fail due to no stdin input
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No input received from stdin") || stderr.contains("No guardrails/cupcake.yaml found"));
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
        .args(&["run", "--", "validate", "--strict"])
        .output()
        .expect("Failed to execute cupcake validate");

    // Will fail due to no YAML config present for validate command
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No guardrails/cupcake.yaml found"));
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
        .args(&[
            "run",
            "--",
            "init",
            "--output",
            output_file.to_str().unwrap(),
            "--verbose",
            "--yes",
        ])
        .output()
        .expect("Failed to execute cupcake init --verbose");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Output directory:"));
    assert!(stdout.contains("Initializing Cupcake guardrails"));
}

#[test]
fn test_cli_run_with_debug() {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "run",
            "--event",
            "PostToolUse",
            "--debug",
            "--timeout",
            "30",
        ])
        .output()
        .expect("Failed to execute cupcake run --debug");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Event: PostToolUse"));
    assert!(stderr.contains("Timeout: 30s"));
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
        .args(&["run", "--", "validate", "--format", "json"])
        .output()
        .expect("Failed to execute cupcake validate with format");

    // Will fail due to no YAML config present for validate command
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No guardrails/cupcake.yaml found"));
}

#[test]
fn test_cli_audit_with_filters() {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "audit",
            "--session",
            "test-session",
            "--event",
            "PreToolUse",
            "--follow",
        ])
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
        .args(&["run", "--", "run", "--event", "PreToolUse", "--debug"])
        .output()
        .expect("Failed to execute cupcake run with defaults");

    let stderr = String::from_utf8(output.stderr).unwrap();
    // Will fail due to no YAML config, but debug should show timeout  
    assert!(stderr.contains("Timeout: 60s"));
}

#[test]
fn test_cli_init_default_output() {
    let output = Command::new("cargo")
        .args(&["run", "--", "init", "--yes"])
        .output()
        .expect("Failed to execute cupcake init with default output");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("guardrails initialized successfully"));
}

#[test]
fn test_cli_validate_default_file() {
    let output = Command::new("cargo")
        .args(&["run", "--", "validate"])
        .output()
        .expect("Failed to execute cupcake validate with default file");

    // Will fail due to no YAML config present for validate command
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("No guardrails/cupcake.yaml found"));
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
        assert!(
            stdout.contains(cmd),
            "Command '{}' not found in help output",
            cmd
        );
    }
}
