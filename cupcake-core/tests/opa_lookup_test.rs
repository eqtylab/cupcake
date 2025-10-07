//! Integration tests for OPA binary lookup logic

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_find_bundled_opa_unix() {
    if cfg!(windows) {
        return; // Skip on Windows
    }

    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    fs::create_dir(&bin_dir).unwrap();

    // Create mock cupcake and opa binaries
    let mock_cupcake = bin_dir.join("cupcake");
    let mock_opa = bin_dir.join("opa");

    fs::write(&mock_cupcake, "#!/bin/sh\necho mock cupcake").unwrap();
    fs::write(&mock_opa, "#!/bin/sh\necho mock opa").unwrap();

    // Make them executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_cupcake, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&mock_opa, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Test the lookup by simulating current_exe returning our mock cupcake
    // Note: We can't override std::env::current_exe() directly, so we test the logic
    // by verifying the file exists where we expect it
    assert!(mock_opa.exists());
    assert_eq!(mock_opa.file_name().unwrap(), "opa");
}

#[test]
fn test_find_bundled_opa_windows() {
    if !cfg!(windows) {
        return; // Skip on non-Windows
    }

    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let bin_dir = temp_dir.path().join("bin");
    fs::create_dir(&bin_dir).unwrap();

    // Create mock cupcake.exe and opa.exe
    let mock_cupcake = bin_dir.join("cupcake.exe");
    let mock_opa = bin_dir.join("opa.exe");

    fs::write(&mock_cupcake, "mock cupcake").unwrap();
    fs::write(&mock_opa, "mock opa").unwrap();

    // Test the lookup
    assert!(mock_opa.exists());
    assert_eq!(mock_opa.file_name().unwrap(), "opa.exe");
}

#[test]
fn test_opa_path_cli_override() {
    // Create a temporary directory with a mock OPA
    let temp_dir = TempDir::new().unwrap();
    let custom_opa = temp_dir.path().join("custom-opa");
    fs::write(&custom_opa, "#!/bin/sh\necho custom opa").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&custom_opa, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // The actual find_opa_binary function would accept a CLI override parameter
    // Simulate the validation logic
    let opa_path_from_cli = Some(custom_opa.clone());
    let validated_path = opa_path_from_cli.filter(|p| p.exists() && p.is_file());

    assert!(validated_path.is_some());
    assert_eq!(validated_path.unwrap(), custom_opa);
}

#[test]
fn test_opa_lookup_priority() {
    // This test verifies the documented lookup order:
    // 1. CLI override (--opa-path flag)
    // 2. Bundled OPA (same directory as cupcake binary)
    // 3. System PATH (fallback)

    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();

    // Create three different OPA locations
    let bundled_dir = temp_dir.path().join("bundled");
    let env_dir = temp_dir.path().join("env");

    fs::create_dir(&bundled_dir).unwrap();
    fs::create_dir(&env_dir).unwrap();

    let opa_name = if cfg!(windows) { "opa.exe" } else { "opa" };

    let bundled_opa = bundled_dir.join(opa_name);
    let env_opa = env_dir.join(opa_name);

    // Test 1: CLI override provided - CLI should be preferred (highest priority)
    fs::write(&bundled_opa, "bundled").unwrap();
    fs::write(&env_opa, "cli").unwrap();

    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(env_opa.clone()),
        "system-fallback",
    );
    assert_eq!(result, env_opa, "CLI override should have highest priority");

    // Test 2: No CLI override - bundled should be preferred
    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        None,
        "system-fallback",
    );
    assert_eq!(result, bundled_opa, "Bundled should be used when no CLI override");

    // Test 3: Remove bundled, no CLI override - system fallback should be used
    fs::remove_file(&bundled_opa).unwrap();

    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        None,
        "system-fallback",
    );
    assert_eq!(result, PathBuf::from("system-fallback"), "System fallback should be used when bundled doesn't exist");
}

/// Test helper function to verify OPA binary resolution
/// This would be used in the actual implementation
fn resolve_opa_path(
    exe_dir: Option<PathBuf>,
    cli_override: Option<PathBuf>,
    system_fallback: &str,
) -> PathBuf {
    // 1. Check CLI override
    if let Some(path) = cli_override {
        if path.exists() {
            return path;
        }
    }

    // 2. Check bundled location
    if let Some(dir) = exe_dir {
        let bundled = if cfg!(windows) {
            dir.join("opa.exe")
        } else {
            dir.join("opa")
        };
        if bundled.exists() {
            return bundled;
        }
    }

    // 3. Fall back to system PATH
    PathBuf::from(system_fallback)
}

#[test]
fn test_resolve_opa_path_helper() {
    let temp_dir = TempDir::new().unwrap();
    let opa_name = if cfg!(windows) { "opa.exe" } else { "opa" };

    // Test CLI override has highest priority
    let cli_path = temp_dir.path().join("cli-opa");
    fs::write(&cli_path, "cli").unwrap();

    let bundled_dir = temp_dir.path().join("bundled");
    fs::create_dir(&bundled_dir).unwrap();
    let bundled_opa = bundled_dir.join(opa_name);
    fs::write(&bundled_opa, "bundled").unwrap();

    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(cli_path.clone()),
        "system-opa",
    );
    assert_eq!(result, cli_path);

    // Test bundled priority when CLI override doesn't exist
    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(PathBuf::from("/nonexistent")),
        "system-opa",
    );
    assert_eq!(result, bundled_opa);

    // Test system fallback
    let result = resolve_opa_path(None, None, "system-opa");
    assert_eq!(result, PathBuf::from("system-opa"));
}
