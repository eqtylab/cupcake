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
    assert!(
        stderr.contains("No input received from stdin")
            || stderr.contains("No guardrails/cupcake.yaml found")
    );
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
    let temp_dir = tempdir().unwrap();
    
    // First build the binary in the source directory
    let build_output = Command::new("cargo")
        .args(&["build"])
        .output()
        .expect("Failed to build cupcake");
    
    if !build_output.status.success() {
        panic!("Failed to build cupcake: {}", String::from_utf8_lossy(&build_output.stderr));
    }
    
    // Now run the binary from the temp directory
    let cupcake_binary = std::env::current_dir().unwrap().join("target").join("debug").join("cupcake");
    
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
    let output_dir = temp_dir.path().join("verbose-test-guardrails");

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
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
    let temp_dir = tempdir().unwrap();
    
    // Build binary
    let build_output = Command::new("cargo")
        .args(&["build"])
        .output()
        .expect("Failed to build cupcake");
    
    if !build_output.status.success() {
        panic!("Failed to build cupcake: {}", String::from_utf8_lossy(&build_output.stderr));
    }
    
    let cupcake_binary = std::env::current_dir().unwrap().join("target").join("debug").join("cupcake");
    
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
    let temp_dir = tempdir().unwrap();
    
    // Build binary
    let build_output = Command::new("cargo")
        .args(&["build"])
        .output()
        .expect("Failed to build cupcake");
    
    if !build_output.status.success() {
        panic!("Failed to build cupcake: {}", String::from_utf8_lossy(&build_output.stderr));
    }
    
    let cupcake_binary = std::env::current_dir().unwrap().join("target").join("debug").join("cupcake");
    
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
    let expected_commands = vec!["init", "run", "sync", "validate", "audit", "encode"];

    for cmd in expected_commands {
        assert!(
            stdout.contains(cmd),
            "Command '{}' not found in help output",
            cmd
        );
    }
}

// ================================
// Encode Command Integration Tests
// ================================

#[test]
fn test_cli_encode_simple_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "echo hello"])
        .output()
        .expect("Failed to execute cupcake encode");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should produce YAML output by default
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- echo"));
    assert!(stdout.contains("args:"));
    assert!(stdout.contains("- hello"));
}

#[test]
fn test_cli_encode_with_json_format() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "ls -la", "--format", "json"])
        .output()
        .expect("Failed to execute cupcake encode with JSON format");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should produce JSON output
    assert!(stdout.contains("\"command\""));
    assert!(stdout.contains("[\"ls\"]"));
    assert!(stdout.contains("\"args\""));
    assert!(stdout.contains("[\"-la\"]"));
}

#[test]
fn test_cli_encode_with_template() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "npm test", "--template"])
        .output()
        .expect("Failed to execute cupcake encode with template");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should include template comments
    assert!(stdout.contains("# Encoded shell command: npm test"));
    assert!(stdout.contains("# Generated by cupcake encode"));
    assert!(stdout.contains("# This secure array format eliminates shell injection risks"));
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- npm"));
}

#[test]
fn test_cli_encode_piped_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "cat file.txt | grep pattern"])
        .output()
        .expect("Failed to execute cupcake encode with piped command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should parse pipes correctly
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- cat"));
    assert!(stdout.contains("pipe:"));
    assert!(stdout.contains("- cmd:"));
    assert!(stdout.contains("- grep"));
    assert!(stdout.contains("- pattern"));
}

#[test]
fn test_cli_encode_redirected_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "echo hello > output.txt"])
        .output()
        .expect("Failed to execute cupcake encode with redirected command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should parse redirects correctly
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- echo"));
    assert!(stdout.contains("redirect_stdout: output.txt"));
}

#[test]
fn test_cli_encode_append_redirect() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "echo hello >> log.txt"])
        .output()
        .expect("Failed to execute cupcake encode with append redirect");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should parse append redirects correctly
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- echo"));
    assert!(stdout.contains("append_stdout: log.txt"));
}

#[test]
fn test_cli_encode_complex_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "git status --porcelain | wc -l", "--format", "json"])
        .output()
        .expect("Failed to execute cupcake encode with complex command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should handle complex pipes in JSON format
    assert!(stdout.contains("\"command\""));
    assert!(stdout.contains("[\"git\"]"));
    assert!(stdout.contains("\"args\""));
    assert!(stdout.contains("\"status\""));
    assert!(stdout.contains("\"--porcelain\""));
    assert!(stdout.contains("\"pipe\""));
    assert!(stdout.contains("\"wc\""));
    assert!(stdout.contains("\"-l\""));
}

#[test]
fn test_cli_encode_quoted_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "echo 'hello world with spaces'"])
        .output()
        .expect("Failed to execute cupcake encode with quoted command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should handle quotes correctly
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- echo"));
    assert!(stdout.contains("args:"));
    assert!(stdout.contains("- hello world with spaces"));
}

#[test]
fn test_cli_encode_json_template() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "curl -X POST https://api.example.com", "--format", "json", "--template"])
        .output()
        .expect("Failed to execute cupcake encode with JSON template");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should include JSON template metadata
    assert!(stdout.contains("\"_metadata\""));
    assert!(stdout.contains("\"original_command\""));
    assert!(stdout.contains("curl -X POST https://api.example.com"));
    assert!(stdout.contains("\"generated_by\""));
    assert!(stdout.contains("cupcake encode"));
    assert!(stdout.contains("\"command_spec\""));
}

#[test]
fn test_cli_encode_invalid_format() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "echo test", "--format", "xml"])
        .output()
        .expect("Failed to execute cupcake encode with invalid format");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    // Should error on unsupported format
    assert!(stderr.contains("Unsupported format: xml") || stderr.contains("Use 'yaml' or 'json'"));
}

#[test]
fn test_cli_encode_empty_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", ""])
        .output()
        .expect("Failed to execute cupcake encode with empty command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    // Should error on empty command
    assert!(stderr.contains("Empty command cannot be encoded") || stderr.contains("cannot be empty"));
}

#[test]
fn test_cli_encode_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "--help"])
        .output()
        .expect("Failed to execute cupcake encode --help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should show encode command help
    assert!(stdout.contains("Convert shell commands to secure array format"));
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("--template"));
    assert!(stdout.contains("yaml"));
    assert!(stdout.contains("json"));
}

#[test]
fn test_cli_encode_default_format() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "pwd"])
        .output()
        .expect("Failed to execute cupcake encode with default format");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should default to YAML format
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- pwd"));
    // Should not contain JSON-specific formatting
    assert!(!stdout.contains("\"command\""));
}

#[test]
fn test_cli_encode_dangerous_command_safety() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "rm -rf /tmp/*; echo done"])
        .output()
        .expect("Failed to execute cupcake encode with dangerous command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should safely parse dangerous commands as literal arguments
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- rm"));
    assert!(stdout.contains("args:"));
    assert!(stdout.contains("- \"-rf\""));
    assert!(stdout.contains("- \"/tmp/*;\""));
    assert!(stdout.contains("- echo"));
    assert!(stdout.contains("- done"));
}

#[test]
fn test_cli_encode_multiple_pipes() {
    let output = Command::new("cargo")
        .args(&["run", "--", "encode", "ps aux | grep nginx | wc -l"])
        .output()
        .expect("Failed to execute cupcake encode with multiple pipes");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should handle multiple pipes correctly
    assert!(stdout.contains("command:"));
    assert!(stdout.contains("- ps"));
    assert!(stdout.contains("pipe:"));
    assert!(stdout.contains("- grep"));
    assert!(stdout.contains("- nginx"));
    assert!(stdout.contains("- wc"));
    assert!(stdout.contains("- \"-l\""));
}
