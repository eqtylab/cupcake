use std::process::Command;
use std::sync::Once;
use tempfile::tempdir;

// Ensure we only build the binary once for all tests
static BUILD_ONCE: Once = Once::new();
static mut BINARY_PATH: Option<String> = None;

fn get_cupcake_binary() -> String {
    unsafe {
        BUILD_ONCE.call_once(|| {
            // Build the binary
            let output = Command::new("cargo")
                .args(&["build"])
                .output()
                .expect("Failed to build cupcake");
            
            if !output.status.success() {
                panic!("Failed to build cupcake binary: {}", String::from_utf8_lossy(&output.stderr));
            }
            
            let path = std::env::current_dir()
                .unwrap()
                .join("target")
                .join("debug")
                .join("cupcake");
            
            BINARY_PATH = Some(path.to_string_lossy().to_string());
        });
        
        BINARY_PATH.clone().unwrap()
    }
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
    assert!(stdout.contains("audit"));
}

#[test]
fn test_cli_init_command() {
    let cupcake_binary = get_cupcake_binary();
    let temp_dir = tempdir().unwrap();
    let output_dir = temp_dir.path().join("test-guardrails");

    let output = Command::new(&cupcake_binary)
        .args(&[
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
    assert!(stdout.contains("sync command"));
    assert!(stdout.contains("implementation pending"));
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
fn test_cli_audit_command() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["audit", "--tail", "10", "--format", "json"])
        .output()
        .expect("Failed to execute cupcake audit");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("audit command"));
    assert!(stdout.contains("implementation pending"));
}

#[test]
fn test_cli_init_with_verbose() {
    let cupcake_binary = get_cupcake_binary();
    let temp_dir = tempdir().unwrap();
    let output_dir = temp_dir.path().join("verbose-test-guardrails");

    let output = Command::new(&cupcake_binary)
        .args(&[
            "init",
            "--output",
            output_dir.to_str().unwrap(),
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
    assert!(stdout.contains("Force: true"));
    assert!(stdout.contains("Dry run: true"));
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
fn test_cli_audit_with_filters() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&[
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
fn test_cli_init_default_output() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["init", "--yes"])
        .output()
        .expect("Failed to execute cupcake init with default output");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("guardrails initialized successfully"));
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
fn test_cli_audit_default_format() {
    let cupcake_binary = get_cupcake_binary();
    
    let output = Command::new(&cupcake_binary)
        .args(&["audit", "--tail", "5"])
        .output()
        .expect("Failed to execute cupcake audit with default format");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Format: text"));
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
    let expected_commands = vec!["init", "run", "sync", "validate", "audit"];

    for cmd in expected_commands {
        assert!(
            stdout.contains(cmd),
            "Command '{}' not found in help output",
            cmd
        );
    }
}

