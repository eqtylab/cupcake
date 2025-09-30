# GitHub Actions CI - Windows-Specific Notes

This document explains the Windows-specific requirements and fixes needed for the CI pipeline.

## Critical Windows Issues and Solutions

### 1. OPA (Open Policy Agent) Path Handling Bug

**Problem**: OPA v1.7.1 on Windows has known bugs with path handling:
- Strips drive letters from normal paths: `C:\path` becomes `\path`
- Cannot write to `file://` URL paths (syntax error)
- UNC paths (`\\?\C:\`) are also not supported

**Reference**: https://github.com/open-policy-agent/opa/issues/4174

**Solution Implemented** (in `cupcake-core/src/engine/compiler.rs`):
```rust
// INPUT: Use file:// URL format (OPA can read from URLs)
let temp_path_arg = if cfg!(windows) {
    let url_path = temp_path_str.replace('\\', "/");
    format!("file:///{}", url_path)
} else {
    temp_path_str.to_string()
};

// OUTPUT: Use relative path with working directory set
let bundle_path_arg = if cfg!(windows) {
    opa_cmd.current_dir(temp_path);
    "bundle.tar.gz".to_string()
} else {
    bundle_path_str.to_string()
};
```

**Key Points**:
- Do NOT use `canonicalize()` on Windows (produces UNC paths)
- Input directory: `file:///C:/Users/path` format
- Output file: relative path with `.current_dir()` set
- This hybrid approach works around all OPA Windows path bugs

### 2. Shell Script Tests on Windows

**Problem**: Tests that execute `.sh` files fail on Windows because:
- Windows doesn't have native shell script support
- Requires Git Bash, WSL, or similar Unix environment
- GitHub Actions Windows runners have Git Bash but tests may not find it

**Affected Tests**:
- `cupcake-core/tests/action_async_test.rs`
  - `test_action_fire_and_forget` - **SKIPPED ON WINDOWS** (timing-sensitive)
  - `test_multiple_actions_concurrent` - **SKIPPED ON WINDOWS** (timing-sensitive)
- `cupcake-core/tests/action_discovery_test.rs`
  - `test_action_discovery_from_directory` - **SKIPPED ON WINDOWS** (timing-sensitive)
  - `test_discovery_with_guidebook_precedence` - **SKIPPED ON WINDOWS** (timing-sensitive)
  - `test_action_discovery_ignores_subdirs` - **SKIPPED ON WINDOWS** (timing-sensitive)

**Tests Skipped on Windows**:
The timing-sensitive async tests are skipped on Windows using `#[cfg(not(windows))]` because:
- Process spawning through Git Bash is slower than native Unix shells
- Hard-coded timeouts would need to be much longer on Windows (unreliable)
- The async behavior is validated on Unix platforms (macOS, Ubuntu)
- Other action tests (`test_action_failure_non_blocking`, `test_actions_dont_block_subsequent_evaluations`) still run on Windows

**Solution Implemented** (in `cupcake-core/src/engine/mod.rs`):
```rust
// Shell detection function with Git Bash support
fn find_shell_command() -> &'static str {
    if cfg!(windows) {
        // Check standard Git for Windows paths
        if std::path::Path::new(r"C:\Program Files\Git\bin\bash.exe").exists() {
            return r"C:\Program Files\Git\bin\bash.exe";
        }
        if std::path::Path::new(r"C:\Program Files (x86)\Git\bin\bash.exe").exists() {
            return r"C:\Program Files (x86)\Git\bin\bash.exe";
        }
        "bash.exe"  // Try PATH
    } else {
        "sh"
    }
}

// Cached at first use
static SHELL_COMMAND: Lazy<&'static str> = Lazy::new(find_shell_command);

// Script execution logic
if is_shell_script && cfg!(windows) {
    // Invoke .sh files through bash on Windows
    tokio::process::Command::new(*SHELL_COMMAND)
        .arg(&command)
        .current_dir(script_working_dir)
        .output()
        .await
} else {
    // Use detected shell for commands
    tokio::process::Command::new(*SHELL_COMMAND)
        .arg("-c")
        .arg(&command)
        .current_dir(&working_dir)
        .output()
        .await
}
```

**Key Points**:
- Automatically detects Git Bash at standard Windows installation paths
- Falls back to `bash.exe` in PATH if not at standard locations
- `.sh` files are explicitly invoked through bash on Windows
- Uses lazy static initialization for performance (checks paths once)
- Maintains cross-platform compatibility (uses `sh` on Unix)

### 3. OPA Installation

The CI workflow installs OPA v1.7.1 on all platforms:

**Windows PowerShell**:
```powershell
Invoke-WebRequest -Uri "https://github.com/open-policy-agent/opa/releases/download/v1.7.1/opa_windows_amd64.exe" -OutFile "opa.exe"
Move-Item opa.exe "C:\Windows\System32\opa.exe" -Force
opa version
```

**Unix (Linux/macOS)**:
```bash
curl -L -o opa https://github.com/open-policy-agent/opa/releases/download/v1.7.1/opa_linux_amd64_static
chmod +x opa
sudo mv opa /usr/local/bin/
opa version
```

### 4. Path Separators

Windows uses backslashes (`\`) but OPA and many tools expect forward slashes (`/`):
- Always convert to forward slashes before passing to OPA
- Use `Path::join()` for filesystem operations (handles platform differences)
- Never use string concatenation for paths

### 5. Drive Letters and Cross-Drive Issues

**Critical**: GitHub Actions Windows runners use `D:\a\...` for the working directory but temp files are on `C:\Users\...`

Root-relative paths (`\path`) resolve to the current process's drive, NOT the file's drive:
- Process on `D:` accessing `\Users\...` becomes `D:\Users\...` ❌
- This is why we need full `file:///C:/Users/...` URLs for OPA

## Testing Windows Changes

To test Windows-specific changes locally without a Windows machine:
1. The CI must be used for validation (10-minute feedback loop)
2. Use extensive `eprintln!()` debugging (tracing doesn't work in tests)
3. Verify both the happy path AND error messages contain correct paths

## Debugging Tips

1. **Use `eprintln!()` not `debug!()`**: Tracing is not initialized in tests
2. **Check actual paths**: Use `eprintln!("[DEBUG] Path: {:?}", path)` liberally
3. **Verify OPA version**: Ensure v1.7.1+ is installed (has `opa version` in CI)
4. **Check working directory**: `std::env::current_dir()` shows where the process runs
5. **Inspect temp files**: Debug output shows temp directory paths

## Future Improvements

1. **Consider using a Rust tar library** instead of OPA's `-o` flag to avoid path issues entirely
2. **File an upstream issue** with OPA about Windows path handling if not already tracked
3. **Add Windows-specific integration tests** that verify path handling
4. **Document workarounds** for any new OPA-related features on Windows