# Windows Routing Tests - RESOLVED ✅

## Status: FIXED (2025-10-01)

All 10 routing tests now pass on Windows. Root cause was invalid JSON in `.claude/settings.json`.

## Previous Context

We had 10 integration tests in `cupcake-core/tests/claude_code_routing_test.rs` that verified Cupcake's policy routing system works correctly with the Claude CLI. These tests passed on macOS and Linux but failed on Windows in CI.

## What Cupcake Is

Cupcake is a policy engine that intercepts Claude CLI tool calls and evaluates security policies written in Rego (Open Policy Agent). It uses "hooks" - external commands that Claude CLI runs at specific points (like before tool execution).

The routing tests verify that policies are correctly matched to specific events (like "PreToolUse" for the Bash tool).

## Final State - All Tests Passing ✅

### All 10 Routing Tests Pass on Windows:
- ✅ `test_pretooluse_routing`
- ✅ `test_posttooluse_routing`
- ✅ `test_userpromptsubmit_routing`
- ✅ `test_sessionstart_routing`
- ✅ `test_notification_routing`
- ✅ `test_precompact_routing`
- ✅ `test_stop_routing`
- ✅ `test_subagentstop_routing`
- ✅ `test_wildcard_policy_routing`
- ✅ `test_multiple_events_routing`

### Previous Symptoms (Before Fix)

Claude CLI executed successfully but hooks never fired:
```
[DEBUG] Claude exit status: Some(0)
[DEBUG] Claude stdout length: 33 bytes
[DEBUG] Claude stderr length: 0 bytes
[DEBUG] Waiting 2 seconds for hooks to complete...
[DEBUG] Looking for debug directory: "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpzGmKaA\\.cupcake/debug/routing"
[DEBUG] Debug dir exists: false
```

## Root Cause: Invalid JSON

### The Problem

Windows file paths contain backslashes: `C:\Users\Administrator\cupcake\Cargo.toml`

The test code inserted these paths directly into JSON strings:

```rust
let command = "cargo run --manifest-path C:\\Users\\Administrator\\cupcake\\Cargo.toml -- eval";

let settings = format!(
    r#"{{
      "command": "{command}"
    }}"#
);
```

This created **invalid JSON**:
```json
{
  "command": "cargo run --manifest-path C:\Users\Administrator\cupcake\Cargo.toml -- eval"
}
```

The sequences `\U` and `\A` are invalid JSON escape sequences. Claude CLI silently failed to parse the malformed JSON, so hooks never executed.

### The Solution

Escape backslashes before inserting paths into JSON:

```rust
// Escape backslashes for JSON on Windows
let command_escaped = command.replace('\\', "\\\\");

let settings = format!(
    r#"{{
      "command": "{command_escaped}"
    }}"#
);
```

This creates **valid JSON**:
```json
{
  "command": "cargo run --manifest-path C:\\Users\\Administrator\\cupcake\\Cargo.toml -- eval"
}
```

Now Claude CLI successfully parses the settings and hooks execute correctly.

## Changes Made

Fixed in `cupcake-core/tests/claude_code_routing_test.rs`:

1. **Line 198** - Added JSON escaping to `run_claude_test()` helper:
   ```rust
   let command_escaped = command.replace('\\', "\\\\");
   ```

2. **Lines 550, 730** - Added JSON escaping to `test_wildcard_policy_routing` and `test_multiple_events_routing`:
   ```rust
   let command_escaped = command.replace('\\', "\\\\");
   ```

3. **Line 283** - Increased wait time from 2s to 5s:
   ```rust
   std::thread::sleep(std::time::Duration::from_secs(5));
   ```
   (Ensures hooks complete on Windows before checking for debug files)

### Test Results

```bash
cd C:/Users/Administrator/cupcake
export CLAUDE_CLI_PATH="/c/Users/Administrator/AppData/Roaming/npm/claude.cmd"
cargo test --features deterministic-tests --test claude_code_routing_test

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

---

## Historical Debugging Tasks (No Longer Needed)

### Task 1: Verify Hooks Actually Run on Windows (RESOLVED)

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

## Lessons Learned

### Key Insight: JSON Validation in External Tools

When generating configuration files (especially JSON) that contain platform-specific paths:

1. **Always escape special characters** - Backslashes in JSON must be `\\`
2. **Silent failures are hard to debug** - Claude CLI didn't report the JSON parse error
3. **Test on the target platform** - This issue only manifests on Windows
4. **Use structured JSON libraries** when possible instead of string formatting

### Why This Was Hard to Find

1. **Claude CLI succeeded** (exit code 0) - hooks fail silently on bad config
2. **No error output** - JSON parsing errors weren't reported to stderr
3. **Hooks had worked before** - Global config had same issue, but we assumed hooks were firing
4. **Misleading symptoms** - "debug files not created" suggested Cupcake bug, not config bug

### Prevention

Consider using `serde_json` to build settings instead of string formatting:

```rust
use serde_json::json;

let settings = json!({
    "hooks": {
        "UserPromptSubmit": [{
            "hooks": [{
                "type": "command",
                "command": command,  // Automatically escaped by serde_json
                "timeout": 120,
                "env": {
                    "CUPCAKE_DEBUG_ROUTING": "1",
                    "RUST_LOG": "info"
                }
            }]
        }]
    }
});

fs::write(claude_dir.join("settings.json"), settings.to_string())?;
```

This automatically handles platform-specific escaping.
