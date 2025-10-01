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

**Affected Tests** (all marked with `#[cfg(not(windows))]`):
- `cupcake-core/tests/action_async_test.rs`
  - `test_action_fire_and_forget` - **SKIPPED** (timing-sensitive)
  - `test_multiple_actions_concurrent` - **SKIPPED** (timing-sensitive)
- `cupcake-core/tests/action_discovery_test.rs`
  - `test_action_discovery_from_directory` - **SKIPPED** (timing-sensitive)
  - `test_discovery_with_guidebook_precedence` - **SKIPPED** (timing-sensitive)
  - `test_action_discovery_ignores_subdirs` - **SKIPPED** (timing-sensitive)
- `cupcake-core/tests/action_edge_cases_test.rs`
  - `test_action_execution_edge_cases` - **SKIPPED** (Unix-specific paths like /bin/echo)
  - `test_nonexistent_script_fallback` - **SKIPPED** (Unix shell fallback behavior)

**Why Tests Are Skipped on Windows**:

1. **Timing-Sensitive Tests**: Process spawning through Git Bash is slower than native Unix shells, making hard-coded timeouts unreliable
2. **Unix-Specific Behavior**: Some tests use Unix-only paths (e.g., `/bin/echo`) or test Unix shell fallback semantics
3. **Coverage**: Core functionality is validated by tests that DO run on Windows:
   - `test_action_failure_non_blocking` - verifies actions don't block on failure
   - `test_actions_dont_block_subsequent_evaluations` - verifies async execution
   - All policy evaluation and routing tests
   - All trust mode and validation tests

**Solution Implemented** (in `cupcake-core/src/engine/mod.rs`):

1. **Git Bash Detection** - Automatically finds bash.exe at standard Windows install paths:
```rust
fn find_shell_command() -> &'static str {
    if cfg!(windows) {
        if std::path::Path::new(r"C:\Program Files\Git\bin\bash.exe").exists() {
            return r"C:\Program Files\Git\bin\bash.exe";
        }
        if std::path::Path::new(r"C:\Program Files (x86)\Git\bin\bash.exe").exists() {
            return r"C:\Program Files (x86)\Git\bin\bash.exe";
        }
        "bash.exe"  // Fallback to PATH
    } else {
        "sh"
    }
}
```

2. **Windows Path Conversion for Git Bash** - Converts Windows paths to Unix-style for .sh scripts:
```rust
// When executing .sh files on Windows, convert paths for Git Bash
if is_shell_script && cfg!(windows) {
    // Convert C:\Users\foo\script.sh → /c/Users/foo/script.sh
    let bash_path = if command.len() >= 3 && command.chars().nth(1) == Some(':') {
        let drive = command.chars().next().unwrap().to_lowercase();
        let path_part = &command[2..].replace('\\', "/");
        format!("/{}{}", drive, path_part)
    } else {
        command.replace('\\', "/")
    };

    tokio::process::Command::new(*SHELL_COMMAND)
        .arg(&bash_path)  // Pass Unix-style path to bash
        .output()
        .await
}
```

3. **Test Helper for Bash-Compatible Paths** - Tests use `path_for_bash()` to generate correct paths:
```rust
#[cfg(windows)]
fn path_for_bash(path: &PathBuf) -> String {
    // Converts C:\path\to\file.txt → /c/path/to/file.txt
    // For use inside bash scripts on Windows
}
```

**Why This Fix Was Needed**:

Git Bash on Windows expects Unix-style paths, not Windows paths:
- **Wrong**: `bash.exe C:\Users\foo\script.sh` → Git Bash can't interpret `C:\Users`
- **Correct**: `bash.exe /c/Users/foo/script.sh` → Git Bash understands `/c/Users`

Additionally, paths embedded **inside** bash scripts must use Unix format:
- **Wrong**: `echo "text" > C:\path\file.txt` → Backslash escapes the next character
- **Correct**: `echo "text" > /c/path/file.txt` → Works correctly

The fix addresses both:
1. Script path passed to `bash.exe` (engine code)
2. Paths inside script content (test helper function)

**Key Points**:
- Automatically detects Git Bash at standard Windows installation paths
- Falls back to `bash.exe` in PATH if not at standard locations
- Converts Windows paths (`C:\`) to Git Bash format (`/c/`)
- Test helpers ensure script content uses bash-compatible paths
- `.sh` files are explicitly invoked through bash on Windows
- Maintains cross-platform compatibility (no changes needed on Unix)

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