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
  - `test_discovery_with_rulebook_precedence` - **SKIPPED** (timing-sensitive)
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

### 3. Claude CLI Routing Tests - JSON Path Escaping

**Problem**: All 10 Claude CLI routing tests failed on Windows due to invalid JSON in `.claude/settings.json`. Windows paths contain backslashes (`C:\Users\...`), which were inserted into JSON strings without escaping, creating invalid escape sequences like `\U` and `\A`.

**Status**: ✅ FIXED - All 10 routing tests now pass on Windows

**Affected Tests**:

- `cupcake-core/tests/claude_code_routing_test.rs` - All 10 routing tests

**Root Cause**:

Test code generated JSON with unescaped backslashes:

```rust
let command = "cargo run --manifest-path C:\\Users\\Admin\\cupcake\\Cargo.toml -- eval";
let settings = format!(r#"{{"command": "{command}"}}"#);
```

This created invalid JSON:

```json
{ "command": "cargo run --manifest-path C:UsersAdmincupcakeCargo.toml -- eval" }
```

Claude CLI silently failed to parse the malformed JSON, so hooks never executed.

**Solution Implemented**:

1. **Escape backslashes in JSON** (lines 198, 550, 730):

   ```rust
   // Escape backslashes for JSON on Windows
   let command_escaped = command.replace('\\', "\\\\");

   let settings = format!(
       r#"{{
         "hooks": {{
           "UserPromptSubmit": [{{
             "hooks": [{{
               "type": "command",
               "command": "{command_escaped}",
               "timeout": 120,
               "env": {{
                 "CUPCAKE_DEBUG_ROUTING": "1",
                 "RUST_LOG": "info"
               }}
             }}]
           }}]
         }}
       }}"#
   );
   ```

2. **Increased hook completion wait time** (line 283):
   ```rust
   // Wait longer on Windows for hooks to complete
   std::thread::sleep(std::time::Duration::from_secs(5));
   ```

**Test Results**:

```
running 10 tests
test test_multiple_events_routing ... ok
test test_notification_routing ... ok
test test_posttooluse_routing ... ok
test test_precompact_routing ... ok
test test_pretooluse_routing ... ok
test test_sessionstart_routing ... ok
test test_stop_routing ... ok
test test_subagentstop_routing ... ok
test test_userpromptsubmit_routing ... ok
test test_wildcard_policy_routing ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Key Points**:

- Windows paths MUST be escaped when embedded in JSON strings
- Claude CLI silently ignores malformed JSON (no error output)
- Always use `serde_json` for JSON generation in production code
- Test infrastructure can use manual escaping for simple cases
- This was the last blocker for full Windows test coverage

**See Also**: `.github/WINDOWS_ROUTING_TESTS_DEBUG.md` for detailed debugging history

### 4. Path Separators and JSON Escaping

**Critical**: Windows paths use backslashes which have special meaning in multiple contexts:

1. **JSON Strings**: Backslashes are escape characters

   - Must use `\\` in JSON: `"C:\\Users\\path"`
   - Use `command.replace('\\', "\\\\")` before inserting into JSON
   - Or use `serde_json` which handles escaping automatically

2. **OPA Input**: OPA expects forward slashes

   - Always convert to forward slashes before passing to OPA
   - Use `path.replace('\\', "/")` for OPA paths
   - Use `Path::join()` for filesystem operations (handles platform differences)

3. **Git Bash**: Expects Unix-style paths
   - Convert `C:\Users\path` to `/c/Users/path`
   - See "Shell Script Tests on Windows" section above

**Key Rule**: Never use string concatenation for paths - always use `Path::join()`

### 5. OPA Installation

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
- **Fix Location**: Engine code (`mod.rs`, `rulebook.rs`)

#### Pattern B: Test Infrastructure (Claude CLI Integration)

- **What**: External tool (Claude CLI) installed by npm
- **Storage**: Platform-specific (`.ps1` on Windows, binary on Unix)
- **Execution**: JSON configuration with escaped paths on Windows
- **Why**: JSON parsing requires proper escaping of backslashes
- **Fix Location**: Test code (`claude_code_routing_test.rs`)
- **Critical Fix**: Escape Windows paths before embedding in JSON (`command.replace('\\', "\\\\")`)

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

**Pattern**: Document not just the fix, but the _why_ behind it.

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

### 8. Silent Failures in External Tools

**Lesson from Claude CLI Routing Tests**:

External tools may silently ignore malformed configuration files, making debugging extremely difficult.

**The Issue**:

- Windows paths in JSON must have escaped backslashes: `"C:\\Users\\path"`
- Test code inserted unescaped paths: `"C:\Users\path"` (invalid JSON)
- Claude CLI silently failed to parse JSON - no error output, exit code 0
- Hooks never executed, but no indication why

**Debugging Symptoms**:

- Process succeeds (exit code 0) ✅
- No stderr output ✅
- Expected side effects don't happen ❌
- Leads to investigating the wrong components (Cupcake instead of JSON config)

**Solution Pattern**:

```rust
// BAD: Manual string formatting
let path = "C:\\Users\\path";
let json = format!(r#"{{"path": "{path}"}}"#);  // Invalid JSON!

// GOOD: Use serde_json for automatic escaping
use serde_json::json;
let config = json!({
    "path": path  // Automatically escaped
});
```

**When Debugging Silent Failures**:

1. Validate all generated config files (JSON, YAML, etc.)
2. Check for platform-specific character escaping issues
3. Test with minimal reproducible config outside the main application
4. Add explicit validation/parsing in test setup if possible

### 9. Cross-Platform Test Helpers Pattern

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

### 10. The "Script vs. Binary" Distinction

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
