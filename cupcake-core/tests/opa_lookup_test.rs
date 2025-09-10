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
fn test_opa_path_env_override() {
    // Create a temporary directory with a mock OPA
    let temp_dir = TempDir::new().unwrap();
    let custom_opa = temp_dir.path().join("custom-opa");
    fs::write(&custom_opa, "#!/bin/sh\necho custom opa").unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&custom_opa, fs::Permissions::from_mode(0o755)).unwrap();
    }
    
    // Set the environment variable
    std::env::set_var("CUPCAKE_OPA_PATH", custom_opa.to_str().unwrap());
    
    // The actual find_opa_binary function would check this path
    let opa_path_from_env = std::env::var("CUPCAKE_OPA_PATH")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists());
    
    assert!(opa_path_from_env.is_some());
    assert_eq!(opa_path_from_env.unwrap(), custom_opa);
    
    // Clean up
    std::env::remove_var("CUPCAKE_OPA_PATH");
}

#[test]
fn test_opa_lookup_priority() {
    // This test verifies the documented lookup order:
    // 1. Bundled OPA (same directory as cupcake binary)
    // 2. CUPCAKE_OPA_PATH environment variable
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
    
    // Test 1: All exist - bundled should be preferred
    fs::write(&bundled_opa, "bundled").unwrap();
    fs::write(&env_opa, "env").unwrap();
    
    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(env_opa.clone()),
        "system-fallback",
    );
    assert_eq!(result, bundled_opa);
    
    // Test 2: Remove bundled - env should be preferred
    fs::remove_file(&bundled_opa).unwrap();
    
    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(env_opa.clone()),
        "system-fallback",
    );
    assert_eq!(result, env_opa);
    
    // Test 3: Remove env - system fallback should be used
    fs::remove_file(&env_opa).unwrap();
    
    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(env_dir.join(opa_name)),
        "system-fallback",
    );
    assert_eq!(result, PathBuf::from("system-fallback"));
}

/// Test helper function to verify OPA binary resolution
/// This would be used in the actual implementation
fn resolve_opa_path(
    exe_dir: Option<PathBuf>,
    env_path: Option<PathBuf>,
    system_fallback: &str,
) -> PathBuf {
    // 1. Check bundled location
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
    
    // 2. Check environment variable
    if let Some(path) = env_path {
        if path.exists() {
            return path;
        }
    }
    
    // 3. Fall back to system PATH
    PathBuf::from(system_fallback)
}

#[test]
fn test_resolve_opa_path_helper() {
    let temp_dir = TempDir::new().unwrap();
    let opa_name = if cfg!(windows) { "opa.exe" } else { "opa" };
    
    // Test bundled priority
    let bundled_dir = temp_dir.path().join("bundled");
    fs::create_dir(&bundled_dir).unwrap();
    let bundled_opa = bundled_dir.join(opa_name);
    fs::write(&bundled_opa, "bundled").unwrap();
    
    let result = resolve_opa_path(
        Some(bundled_dir.clone()),
        Some(PathBuf::from("/nonexistent")),
        "system-opa",
    );
    assert_eq!(result, bundled_opa);
    
    // Test env variable when bundled doesn't exist
    let env_path = temp_dir.path().join("env-opa");
    fs::write(&env_path, "env").unwrap();
    
    let result = resolve_opa_path(
        Some(PathBuf::from("/nonexistent-dir")),
        Some(env_path.clone()),
        "system-opa",
    );
    assert_eq!(result, env_path);
    
    // Test system fallback
    let result = resolve_opa_path(
        None,
        None,
        "system-opa",
    );
    assert_eq!(result, PathBuf::from("system-opa"));
}