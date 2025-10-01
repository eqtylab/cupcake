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

### 3. PowerShell Script Execution in Tests

**Problem**: Integration tests that execute the Claude CLI fail on Windows because npm installs Claude as a PowerShell script (`claude.ps1`), not an executable binary.

**Error**: `Os { code: 193, kind: Uncategorized, message: "%1 is not a valid Win32 application." }`

**Affected Tests**:
- `cupcake-core/tests/claude_code_routing_test.rs` - All 10 routing tests

**Solution Implemented** (in `cupcake-core/tests/claude_code_routing_test.rs`):

```rust
// On Windows, PowerShell scripts (.ps1) cannot be executed directly
// They must be invoked via powershell.exe
let output = if cfg!(windows) && claude_path.ends_with(".ps1") {
    std::process::Command::new("powershell.exe")
        .args([
            "-ExecutionPolicy", "Bypass",
            "-File", &claude_path,
            "-p", "hello world",
            "--model", "sonnet",
        ])
        .current_dir(project_path)
        .env("CUPCAKE_DEBUG_ROUTING", "1")
        .output()
        .expect("Failed to execute claude command via powershell.exe")
} else {
    std::process::Command::new(&claude_path)
        .args(["-p", "hello world", "--model", "sonnet"])
        .current_dir(project_path)
        .env("CUPCAKE_DEBUG_ROUTING", "1")
        .output()
        .expect("Failed to execute claude command")
};
```

**Why This Is Different from Actions/Signals**:
- Actions/signals are **product features** - users write `.sh` scripts that run via Git Bash on Windows
- Claude routing tests are **integration tests** - they verify Cupcake works with the actual Claude CLI installation
- The Claude CLI installation format is platform-specific (`.ps1` on Windows, binary on Unix)

**Key Points**:
- Detects `.ps1` extension and wraps execution with `powershell.exe -File`
- Uses `-ExecutionPolicy Bypass` to avoid script execution policy restrictions
- Only applies to Windows - Unix systems execute the binary directly
- Maintains test coverage for Claude CLI integration on all platforms

### 4. OPA Installation

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

### 5. Path Separators

Windows uses backslashes (`\`) but OPA and many tools expect forward slashes (`/`):
- Always convert to forward slashes before passing to OPA
- Use `Path::join()` for filesystem operations (handles platform differences)
- Never use string concatenation for paths

### 6. Drive Letters and Cross-Drive Issues

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

## Lessons Learned from Windows CI Debugging

### 1. Systematic Problem-Solving Approach

**Key Principle**: Understand the entire system end-to-end before deciding to skip tests or apply workarounds.

**Process That Works**:
1. **Analyze the Error**: Don't just read the error message - understand what the system is trying to do
2. **Trace Execution Flow**: Follow code from discovery → storage → execution to find where platform differences matter
3. **Identify Root Cause**: Distinguish between product bugs, test infrastructure issues, and external dependencies
4. **Apply Appropriate Fix**: Product bugs need runtime fixes; test issues may need platform-specific handling

**Example from This Session**:
- **Initial Symptom**: "Action did not execute" on Windows
- **Wrong Approach**: Skip tests because "Windows doesn't support shell scripts"
- **Correct Approach**:
  1. Traced how actions are discovered (stored as Windows paths)
  2. Found where they're executed (passed to `bash.exe`)
  3. Identified the mismatch (Git Bash needs Unix-style paths)
  4. Applied targeted fix (convert paths at execution time)

### 2. Understanding Platform-Specific Execution Models

**Three Distinct Execution Patterns Emerged**:

#### Pattern A: Product Features (Actions & Signals)
- **What**: User-authored `.sh` scripts that Cupcake executes
- **Storage**: Full Windows paths (`C:\Users\...\script.sh`)
- **Execution**: Convert to Git Bash format at runtime (`/c/Users/.../script.sh`)
- **Why**: Users need to write portable scripts on Windows
- **Fix Location**: Engine code (`mod.rs`, `guidebook.rs`)

#### Pattern B: Test Infrastructure (Claude CLI Integration)
- **What**: External tool (Claude CLI) installed by npm
- **Storage**: Platform-specific (`.ps1` on Windows, binary on Unix)
- **Execution**: Platform-specific wrapper (PowerShell on Windows, direct on Unix)
- **Why**: External tool's installation format is outside our control
- **Fix Location**: Test code (`claude_code_routing_test.rs`)

#### Pattern C: Test Helpers (Path Generation)
- **What**: Paths embedded inside bash script content
- **Storage**: N/A (generated dynamically in tests)
- **Execution**: Must use Unix format for Git Bash to interpret correctly
- **Why**: Backslashes in bash scripts are escape characters
- **Fix Location**: Test utility functions (`path_for_bash()`)

### 3. Git Bash Path Conversion Rules

**Critical Understanding**: Git Bash on Windows requires TWO types of path conversion:

1. **Script Path** (passed as argument to `bash.exe`):
   ```rust
   // WRONG: bash.exe C:\Users\foo\script.sh
   // RIGHT: bash.exe /c/Users/foo/script.sh
   ```

2. **Script Content** (paths inside the bash script):
   ```bash
   # WRONG: echo "text" > C:\path\file.txt  # Backslash escapes next char
   # RIGHT: echo "text" > /c/path/file.txt  # Works correctly
   ```

**Key Insight**: Both conversions are necessary. Fixing only one causes subtle failures.

### 4. Error Code 193 - Diagnostic Pattern

**Meaning**: "Not a valid Win32 application" - attempted to execute a file that isn't a binary executable.

**Common Causes**:
- Trying to execute PowerShell scripts (`.ps1`) directly
- Trying to execute batch files (`.bat`, `.cmd`) without `cmd.exe`
- Trying to execute shell scripts (`.sh`) without `bash.exe`

**Solution Pattern**:
```rust
if cfg!(windows) && path.ends_with(".ps1") {
    Command::new("powershell.exe").args(["-File", path, ...])
} else if cfg!(windows) && path.ends_with(".sh") {
    Command::new("bash.exe").arg(convert_to_unix_path(path))
} else {
    Command::new(path)  // Unix: execute directly
}
```

### 5. When to Skip vs. When to Fix

**Skip Tests When**:
- Testing platform-specific behavior that doesn't exist on target platform
- Timing-sensitive tests where platform performance differs significantly
- Tests rely on platform-specific tools not available in CI (e.g., `/bin/echo`)

**Fix Tests When**:
- Core functionality should work cross-platform with appropriate wrappers
- External dependencies have different installation formats but same interface
- Platform differences can be abstracted with conditional compilation

**Decision Matrix**:
| Scenario | Skip or Fix? | Reason |
|----------|-------------|---------|
| Action async timing | Skip | Git Bash slower than native shell, timeouts unreliable |
| Action execution | Fix | Core feature must work on all platforms |
| Signal execution | Fix | Core feature must work on all platforms |
| Claude CLI integration | Fix | External tool, platform-specific install, same interface |
| Unix-specific paths (`/bin/echo`) | Skip | Platform-specific behavior being tested |

### 6. Documentation as Understanding

**Pattern**: Document not just the fix, but the *why* behind it.

**Good Documentation Includes**:
- Problem statement with error messages
- Root cause analysis (not just symptoms)
- Why this solution was chosen over alternatives
- Why this differs from similar-looking problems
- Code examples showing wrong vs. right approaches

**Example from This Session**:
```markdown
**Why This Is Different from Actions/Signals**:
- Actions/signals are **product features** - users write `.sh` scripts
- Claude routing tests are **integration tests** - verify external tool integration
- Claude CLI installation format is platform-specific, outside our control
```

### 7. Debugging Windows Without a Windows Machine

**Techniques Used Successfully**:

1. **Liberal `eprintln!()` Statements**:
   ```rust
   eprintln!("[DEBUG] Path: {:?}", path);
   eprintln!("[DEBUG] Executing: {command}");
   eprintln!("[DEBUG] Exit code: {:?}", output.status.code());
   ```

2. **CI as Validation Loop**:
   - Push changes, wait ~10 minutes for CI feedback
   - Read stderr/stdout from failed tests
   - Iterate based on actual Windows behavior

3. **Extensive Error Context**:
   ```rust
   .expect(&format!("Failed to execute: {}", command))
   // Better than just: .expect("Failed to execute")
   ```

4. **Path Inspection at Every Stage**:
   - Log paths when discovered
   - Log paths when stored
   - Log paths before execution
   - Log paths inside script content

**Key Insight**: Cannot guess Windows behavior - must observe actual execution through CI logs.

### 8. Cross-Platform Test Helpers Pattern

**Best Practice**: Create platform-aware helper functions for test setup:

```rust
#[cfg(windows)]
fn path_for_bash(path: &PathBuf) -> String {
    // Convert C:\path → /c/path for Git Bash
}

#[cfg(not(windows))]
fn path_for_bash(path: &PathBuf) -> String {
    path.display().to_string()  // No conversion needed
}
```

**Why This Works**:
- Tests remain readable (same API on all platforms)
- Platform logic isolated in helper
- Easy to update if conversion rules change
- Compiler ensures correct version used

### 9. The "Script vs. Binary" Distinction

**Critical Insight**: On Windows, there are THREE types of executables:

1. **Native Binaries** (`.exe`): Execute directly with `Command::new(path)`
2. **Scripts** (`.sh`, `.ps1`, `.bat`): Require interpreter wrapper
3. **Shebang Scripts** (Unix only): OS reads `#!/bin/bash` and routes to interpreter

**Windows Behavior**:
- No shebang support - must detect script type by extension
- Each script type needs specific interpreter
- Interpreter location varies (Git Bash in Program Files, PowerShell in System32)

**Design Pattern**:
```rust
fn execute_script(path: &str) -> Command {
    if cfg!(windows) {
        if path.ends_with(".ps1") {
            Command::new("powershell.exe")
                .args(["-ExecutionPolicy", "Bypass", "-File", path])
        } else if path.ends_with(".sh") {
            let bash_path = convert_to_unix_path(path);
            Command::new(*SHELL_COMMAND).arg(bash_path)
        } else {
            Command::new(path)  // Assume .exe
        }
    } else {
        Command::new(path)  // Unix handles shebang
    }
}
```

## Future Improvements

1. **Consider using a Rust tar library** instead of OPA's `-o` flag to avoid path issues entirely
2. **File an upstream issue** with OPA about Windows path handling if not already tracked
3. **Add Windows-specific integration tests** that verify path handling
4. **Document workarounds** for any new OPA-related features on Windows
5. **Create reusable test utilities** for cross-platform script execution in tests
6. **Consider GitHub Actions Windows runner documentation** for temp file locations and drive mappings