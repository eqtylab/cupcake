use std::process::Command;
use tempfile::tempdir;

fn get_cupcake_binary() -> String {
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join("debug")
        .join("cupcake")
        .to_string_lossy()
        .to_string()
}

#[test]
fn test_cli_help_command() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .arg("--help")
        .output()
        .expect("Failed to execute cupcake --help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("transforms natural language rules"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("sync"));
    assert!(stdout.contains("validate"));
}


#[test]
fn test_cli_run_command() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["run", "--event", "PreToolUse", "--debug"])
        .output()
        .expect("Failed to execute cupcake run");

    // Will fail due to no stdin input
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("No input received from stdin")
            || stderr.contains("No guardrails/cupcake.yaml found")
    );
}

#[test]
fn test_cli_sync_command() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["sync", "--dry-run"])
        .output()
        .expect("Failed to execute cupcake sync");

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Sync command shows syncing message and dry run mode
    assert!(stdout.contains("Syncing Cupcake hooks") || stdout.contains("🔄"));
    assert!(stdout.contains("Dry run mode") || stdout.contains("🔍"));
}

#[test]
fn test_cli_validate_command() {
    let temp_dir = tempdir().unwrap();
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["validate", "--strict"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute cupcake validate");

    // Check that validation fails with proper error message
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should fail with config not found error
    assert!(
        stderr.contains("No guardrails/cupcake.yaml found") || 
        stdout.contains("No guardrails/cupcake.yaml found")
    );
}



#[test]
fn test_cli_run_with_debug() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&[
            "run",
            "--event",
            "PostToolUse",
            "--debug",
        ])
        .output()
        .expect("Failed to execute cupcake run --debug");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Event: PostToolUse"));
}

#[test]
fn test_cli_sync_with_force() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["sync", "--force", "--dry-run"])
        .output()
        .expect("Failed to execute cupcake sync --force");

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Sync command with force and dry-run shows appropriate output
    assert!(stdout.contains("Dry run mode") || stdout.contains("🔍") || stdout.contains("would write"));
    // The JSON output should be present in dry run mode
    assert!(stdout.contains("hooks") || stdout.contains("PreToolUse"));
}

#[test]
fn test_cli_validate_with_format() {
    let temp_dir = tempdir().unwrap();
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["validate", "--format", "json"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute cupcake validate with format");

    // Will fail due to no YAML config present for validate command
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stderr.contains("No guardrails/cupcake.yaml found") || 
        stdout.contains("No guardrails/cupcake.yaml found")
    );
}


#[test]
fn test_cli_invalid_command() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .arg("invalid-command")
        .output()
        .expect("Failed to execute cupcake with invalid command");

    assert!(!output.status.success(), "Command should have failed");
    
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // More robust check - error message might be in stdout or stderr
    let combined = format!("{}{}", stderr, stdout);
    assert!(
        combined.contains("error") || combined.contains("unrecognized"),
        "Expected 'error' or 'unrecognized' in output.\nStderr: {}\nStdout: {}",
        stderr,
        stdout
    );
}

#[test]
fn test_cli_missing_required_args() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["run"])
        .output()
        .expect("Failed to execute cupcake run without required args");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("required") || stderr.contains("error"));
}

#[test]
fn test_cli_version() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .arg("--version")
        .output()
        .expect("Failed to execute cupcake --version");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("cupcake") && stdout.contains("0.1.0"));
}

#[test]
fn test_cli_default_values() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["run", "--event", "PreToolUse", "--debug"])
        .output()
        .expect("Failed to execute cupcake run with defaults");

    let stderr = String::from_utf8(output.stderr).unwrap();
    // Will fail due to no YAML config, but debug should show event
    assert!(stderr.contains("Event: PreToolUse"));
}


#[test]
fn test_cli_validate_default_file() {
    let temp_dir = tempdir().unwrap();
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["validate"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute cupcake validate with default file");

    // Will fail due to no YAML config present for validate command
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stderr.contains("No guardrails/cupcake.yaml found") || 
        stdout.contains("No guardrails/cupcake.yaml found")
    );
}


#[test]
fn test_cli_all_subcommands_exist() {
    // Test that all expected subcommands are available
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .arg("--help")
        .output()
        .expect("Failed to execute cupcake --help");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected_commands = vec!["init", "run", "sync", "validate"];

    for cmd in expected_commands {
        assert!(
            stdout.contains(cmd),
            "Command '{}' not found in help output",
            cmd
        );
    }
}

