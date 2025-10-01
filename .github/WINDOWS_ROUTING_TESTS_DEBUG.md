# Windows Routing Tests Debug Handoff

## Context

We have 10 integration tests in `cupcake-core/tests/claude_code_routing_test.rs` that verify Cupcake's policy routing system works correctly with the Claude CLI. These tests pass on macOS and Linux but fail on Windows in CI.

## What Cupcake Is

Cupcake is a policy engine that intercepts Claude CLI tool calls and evaluates security policies written in Rego (Open Policy Agent). It uses "hooks" - external commands that Claude CLI runs at specific points (like before tool execution).

The routing tests verify that policies are correctly matched to specific events (like "PreToolUse" for the Bash tool).

## Current State

### Tests That Pass (All Platforms)
- ✅ Action execution tests (4 tests) - Fixed with Git Bash path conversion
- ✅ Signal execution tests - Fixed with platform detection + path conversion
- ✅ All core policy evaluation tests

### Tests That Fail (Windows Only)

**Two tests with Error 193 (FIXED, pending CI verification):**
- `test_wildcard_policy_routing`
- `test_multiple_events_routing`

These were trying to execute PowerShell scripts directly. Fix applied: wrapped with `powershell.exe -ExecutionPolicy Bypass -File`.

**Eight tests with missing debug directory:**
- `test_pretooluse_routing`
- `test_posttooluse_routing`
- `test_userpromptsubmit_routing`
- `test_sessionstart_routing`
- `test_notification_routing`
- `test_precompact_routing`
- `test_stop_routing`
- `test_subagentstop_routing`

These all show:
```
[DEBUG] Detected PowerShell script on Windows, using powershell.exe wrapper
[DEBUG] Claude exit status: Some(0)
[DEBUG] Claude stdout length: 33 bytes
[DEBUG] Claude stderr length: 0 bytes
[DEBUG] Waiting 2 seconds for hooks to complete...
[DEBUG] Looking for debug directory: "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpzGmKaA\\.cupcake/debug/routing"
[DEBUG] Debug dir exists: false
[DEBUG] Checking parent directories:
  .cupcake exists: true
  .cupcake/debug exists: false
```

## The Problem

### Expected Behavior
1. Test creates `.claude/settings.json` with a `UserPromptSubmit` hook configured
2. Test executes `claude -p "hello world" --model sonnet`
3. Claude CLI fires the hook, running `cupcake.exe eval` with `CUPCAKE_DEBUG_ROUTING=1`
4. Cupcake creates `.cupcake/debug/routing/routing_map_*.json` files
5. Test reads these files and verifies policy routing worked correctly

### Actual Behavior on Windows
1. ✅ Test creates `.claude/settings.json` correctly
2. ✅ Claude CLI executes successfully (exit code 0)
3. ❓ Hook execution status unknown - no evidence in output
4. ❌ Debug directory `.cupcake/debug/` is NEVER created
5. ❌ Test panics because it can't find the routing files

### Key Observations

**Claude executes successfully:**
```
[DEBUG] Claude exit status: Some(0)
[DEBUG] Claude stdout length: 33 bytes
```

**Hook configuration looks correct:**
```json
{
  "hooks": {
    "UserPromptSubmit": [{
      "hooks": [{
        "type": "command",
        "command": "D:\\a\\cupcake\\cupcake\\target\\release\\cupcake.exe eval",
        "timeout": 120,
        "env": {
          "CUPCAKE_DEBUG_ROUTING": "1",
          "RUST_LOG": "info"
        }
      }]
    }]
  }
}
```

**Directory structure partially exists:**
```
.cupcake exists: true        ✅
.cupcake/debug exists: false ❌
```

## Debugging Tasks

### Task 1: Verify Hooks Actually Run on Windows

**Goal:** Determine if Claude CLI on Windows (installed via npm as `claude.ps1`) actually executes hooks configured in `.claude/settings.json`.

**How to test:**

1. Install Claude CLI on Windows (via npm)
2. Create a minimal test hook that writes to a file:

```json
{
  "hooks": {
    "UserPromptSubmit": [{
      "hooks": [{
        "type": "command",
        "command": "cmd.exe /c echo HOOK_RAN > C:\\temp\\hook_test.txt",
        "timeout": 10
      }]
    }]
  }
}
```

3. Save to `.claude/settings.json` in a test directory
4. Run: `claude -p "hello world" --model sonnet` from that directory
5. Check if `C:\temp\hook_test.txt` was created

**Expected outcome:**
- If file exists: Hooks work, problem is with Cupcake's debug output on Windows
- If file doesn't exist: Hooks don't work with PowerShell-based Claude CLI

### Task 2: Test Cupcake Debug Output Directly

**Goal:** Verify `CUPCAKE_DEBUG_ROUTING=1` works on Windows when calling `cupcake.exe eval` directly.

**How to test:**

1. Build Cupcake: `cargo build --release`
2. Create a minimal test setup:
```powershell
mkdir test-project
cd test-project
mkdir -p .cupcake/policies/system
```

3. Create minimal policy in `.cupcake/policies/test.rego`:
```rego
package cupcake.policies.test

import rego.v1

# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]

deny contains decision if {
    input.tool_name == "Bash"
    decision := {
        "reason": "test",
        "severity": "LOW",
        "rule_id": "TEST-001"
    }
}
```

4. Create system policy in `.cupcake/policies/system/evaluate.rego` (copy from `examples/0_start_here_demo/.cupcake/policies/system/evaluate.rego`)

5. Create test event `event.json`:
```json
{
  "hookEventName": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {"command": "ls"},
  "session_id": "test",
  "cwd": "C:\\test"
}
```

6. Run:
```powershell
$env:CUPCAKE_DEBUG_ROUTING = "1"
..\..\target\release\cupcake.exe eval < event.json
```

7. Check if `.cupcake/debug/routing/` directory was created with files

**Expected outcome:**
- If debug files exist: Cupcake works, problem is hook execution
- If no debug files: There's a Windows-specific bug in debug file creation

### Task 3: Path Separator Investigation

**Goal:** Check if mixed path separators cause issues.

**Observation:** Error shows mixed separators:
```
"C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpzGmKaA\\.cupcake/debug/routing"
```

Notice: backslashes until `.cupcake`, then forward slashes for `/debug/routing`.

**How to test:**

Look at `cupcake-core/src/engine/routing_debug.rs` around line 73:
```rust
if env::var("CUPCAKE_DEBUG_ROUTING").is_err() {
    return;
}
```

Check where the debug path is constructed. Search for `.cupcake/debug/routing` in the codebase.

**Potential fix locations:**
- `cupcake-core/src/engine/routing_debug.rs` - Debug output implementation
- Anywhere that constructs paths using string concatenation instead of `Path::join()`

### Task 4: Hook Working Directory

**Goal:** Verify hooks run in the correct working directory.

**Theory:** The hook might be running in a different directory than expected, creating `.cupcake/debug/` somewhere else.

**How to test:**

Modify the hook command to output its working directory:
```json
{
  "hooks": {
    "UserPromptSubmit": [{
      "hooks": [{
        "type": "command",
        "command": "cmd.exe /c cd > C:\\temp\\hook_cwd.txt && D:\\path\\to\\cupcake.exe eval",
        "timeout": 120,
        "env": {
          "CUPCAKE_DEBUG_ROUTING": "1"
        }
      }]
    }]
  }
}
```

Check `C:\temp\hook_cwd.txt` to see where the hook actually ran.

### Task 5: Check Cupcake Stderr/Stdout

**Goal:** See if Cupcake is producing error output that's being swallowed.

**How to test:**

Modify hook to capture Cupcake's output:
```json
{
  "command": "D:\\a\\cupcake\\cupcake\\target\\release\\cupcake.exe eval > C:\\temp\\cupcake_out.txt 2>&1"
}
```

Check `C:\temp\cupcake_out.txt` for any error messages.

## Files to Examine

### Test File
`cupcake-core/tests/claude_code_routing_test.rs`
- Lines 144-310: `verify_routing()` helper function (has PowerShell wrapper)
- Lines 352-460: Tests that use `verify_routing()` (8 failing tests)
- Lines 463-654: `test_wildcard_policy_routing` (direct execution, fixed)
- Lines 656-805: `test_multiple_events_routing` (direct execution, fixed)

### Debug Output Implementation
`cupcake-core/src/engine/routing_debug.rs`
- Line 73: Check for `CUPCAKE_DEBUG_ROUTING` env var
- Where debug directory is created
- Path construction logic

### Engine Initialization
`cupcake-core/src/engine/mod.rs`
- How Engine initializes on Windows
- Working directory handling

## Quick Reference: Running Tests

### Run all routing tests locally (if you have Claude CLI):
```bash
cargo test --test claude_code_routing_test --features deterministic-tests -- --nocapture
```

### Run specific test:
```bash
cargo test --test claude_code_routing_test test_pretooluse_routing --features deterministic-tests -- --nocapture
```

### Set environment for Claude CLI path:
```powershell
$env:CLAUDE_CLI_PATH = "C:\npm\prefix\claude.ps1"
cargo test --test claude_code_routing_test --features deterministic-tests -- --nocapture
```

## Success Criteria

The tests should:
1. Execute Claude CLI via PowerShell wrapper successfully ✅ (already works)
2. Claude CLI should execute the hook from settings.json ❓ (unknown)
3. Hook should run `cupcake.exe eval` with debug env var ❓ (unknown)
4. Cupcake should create `.cupcake/debug/routing/*.json` files ❌ (failing)
5. Test should read and verify routing data ❌ (can't get this far)

## Possible Solutions (Ranked by Likelihood)

### 1. Hooks Don't Work with npm-installed Claude CLI on Windows
**If true:** Skip these tests on Windows with `#[cfg(not(windows))]`
**How to verify:** Task 1

### 2. Path Separator Bug in Debug Output Code
**If true:** Fix path construction in `routing_debug.rs` to use `Path::join()`
**How to verify:** Task 3

### 3. Environment Variables Not Passed Through PowerShell
**If true:** Find alternative way to signal debug mode (config file instead of env var?)
**How to verify:** Task 5 with modified hook command

### 4. Working Directory Mismatch
**If true:** Fix hook configuration to set working directory explicitly
**How to verify:** Task 4

### 5. Cupcake Bug on Windows
**If true:** Debug and fix `routing_debug.rs` Windows-specific issues
**How to verify:** Task 2

## Questions to Answer

1. **Do hooks work at all on Windows with PowerShell-based Claude CLI?**
   - Test with simple echo command to file

2. **Does `CUPCAKE_DEBUG_ROUTING=1` work when calling cupcake.exe directly on Windows?**
   - Run cupcake eval manually with env var set

3. **Is there a path separator bug in the debug output code?**
   - Check routing_debug.rs for string concatenation vs Path::join

4. **Where does the hook actually execute from?**
   - Capture working directory in hook output

5. **Is Cupcake producing errors that are being swallowed?**
   - Redirect hook stdout/stderr to files

## Contact

If you find the root cause or need clarification, please document your findings and update this file with:
- What you tested
- What you found
- Proposed fix (if any)

All previous Windows fixes are documented in `.github/CLAUDE.md` - refer to the "Lessons Learned" section for context on the debugging approach we've been using.
