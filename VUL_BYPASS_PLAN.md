# Vulnerability Bypass Fixes - Implementation Plan

**Date**: 2025-10-20
**Status**: In Progress
**External Review**: Approved with refinements

---

## Executive Summary

This plan addresses three High-severity bypass vulnerabilities identified by Trail of Bits:
- **TOB-EQTY-LAB-CUPCAKE-3**: Bash command string matching bypass
- **TOB-EQTY-LAB-CUPCAKE-2**: Cross-tool bypass
- **TOB-EQTY-LAB-CUPCAKE-4**: Symbolic link path bypass

**Scope**: All three vulnerabilities, comprehensive adversarial testing, both harnesses (Claude Code and Cursor), minimal security documentation.

**Timeline**: 10 working days

---

## Harness Event Mapping

### Claude Code Events
- **PreToolUse**: Before tool execution (Bash, Edit, Write, Read, etc.)
- **PostToolUse**: After tool execution
- **UserPromptSubmit**: Before prompt processing
- Tool names: `Bash`, `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Read`, `Grep`, `Glob`, `Task`

### Cursor Events
- **beforeShellExecution**: Before shell command (equivalent to PreToolUse:Bash)
- **afterFileEdit**: After file modification (equivalent to PostToolUse:Edit/Write)
- **beforeReadFile**: Before file read (no equivalent blocking in Claude)
- **beforeMCPExecution**: Before MCP tool execution
- **beforeSubmitPrompt**: Before prompt submission (no context injection support)
- Command field: `input.command` (direct, not nested in `tool_input`)

**Critical Difference**: Cursor has `afterFileEdit` (post-operation) not `beforeFileEdit` (pre-operation), limiting prevention capabilities.

---

## Phase 1: Regex-Based Command Hardening (Vulnerability #3)

### Task 1.1: Harden Git No-Verify Detection

**Files**:
- `fixtures/claude/builtins/git_block_no_verify.rego`
- `fixtures/cursor/builtins/git_block_no_verify.rego`

**Current Vulnerability**:
```rego
# Lines 34-38 (Claude), 32-36 (Cursor)
contains(cmd, "git")
contains(cmd, "commit")
contains(cmd, "--no-verify")  # Fails with extra spaces: git  commit  --no-verify
```

**Fix** (Reviewer refinement: anchor to start):
```rego
contains_git_no_verify(cmd) if {
    # Handles variable whitespace between tokens, anchored to start
    regex.match(`(^|\s)git\s+commit\s+.*--no-verify`, cmd)
}

contains_git_no_verify(cmd) if {
    # git push --no-verify
    regex.match(`(^|\s)git\s+push\s+.*--no-verify`, cmd)
}

contains_git_no_verify(cmd) if {
    # git merge --no-verify
    regex.match(`(^|\s)git\s+merge\s+.*--no-verify`, cmd)
}
```

**Apply to**: Lines 32-60 in both files

---

### Task 1.2: Harden Rulebook Security Guardrails

**Files**:
- `fixtures/claude/builtins/rulebook_security_guardrails.rego`
- `fixtures/cursor/builtins/rulebook_security_guardrails.rego`

**Current Vulnerability**:
```rego
# Lines 91-93
contains_cupcake_modification_pattern(cmd) if {
    contains(cmd, ".cupcake")  # Too broad, need to check for dangerous operations
}
```

**Fix** (Reviewer refinement: anchor to start, add ln -s):
```rego
# New helper rule (add after line 93)
is_dangerous_command(cmd) if {
    dangerous_verbs := {"rm", "mv", "cp", "chmod", "chown", "tee", "ln"}
    some verb in dangerous_verbs
    # Anchored to start or whitespace, followed by word boundary
    regex.match(concat("", [`(^|\s)`, verb, `\s`]), cmd)
}

# Updated main rule
contains_cupcake_modification_pattern(cmd) if {
    contains(cmd, ".cupcake")
    is_dangerous_command(cmd)
}
```

**Keep existing patterns** (lines 98-178) as additional defense layers.

---

### Task 1.3: Harden Protected Paths Whitelist

**Files**:
- `fixtures/claude/builtins/protected_paths.rego`
- `fixtures/cursor/builtins/protected_paths.rego`

**Current Vulnerability**:
```rego
# Lines 128-154 (safe_read_commands)
startswith(cmd, "cat ")  # Fails with: "  cat file.txt" (leading space)
```

**Fix**:
```rego
is_whitelisted_read_command(cmd) if {
    # Exclude dangerous sed variants FIRST (keep this)
    startswith(cmd, "sed -i")
    false
}

is_whitelisted_read_command(cmd) if {
    safe_read_commands := {
        "cat", "less", "more", "head", "tail",
        "grep", "egrep", "fgrep", "zgrep",
        "wc", "file", "stat", "ls", "find",
        "awk", "sed", "sort", "uniq", "diff",
        "cmp", "md5sum", "sha256sum", "hexdump",
        "strings", "od"
    }

    some cmd_name in safe_read_commands
    # Handle leading/trailing whitespace and tabs
    regex.match(concat("", [`^\s*`, cmd_name, `\s+`]), cmd)

    # Additional safety: explicitly reject sed -i
    not regex.match(`^\s*sed\s+.*-i`, cmd)
}
```

**Apply to**: Lines 119-161 in both files

---

## Phase 2: Cross-Tool Coverage Expansion (Vulnerability #2)

### Task 2.1: Expand Policy Metadata

**Audit all builtins** for proper `required_events` and `required_tools`:

#### Claude Code Policies

**protected_paths.rego**:
```rego
# Current (line 9):
required_events: ["PreToolUse"]

# Updated:
required_events: ["PreToolUse"]
required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"]
```

**global_file_lock.rego**:
```rego
# Current:
required_events: ["PreToolUse"]

# Updated:
required_events: ["PreToolUse"]
required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash", "Task"]
```

**rulebook_security_guardrails.rego**:
```rego
# Current (line 9):
required_events: ["PreToolUse"]

# Updated:
required_events: ["PreToolUse"]
required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Read", "Grep", "Glob", "Bash", "Task", "WebFetch"]
```

#### Cursor Policies

**protected_paths.rego**:
```rego
# Current (line 9):
required_events: ["beforeShellExecution"]

# Updated:
required_events: ["beforeShellExecution", "afterFileEdit"]
# Note: Cursor only has POST-edit hook, cannot prevent, only validate
```

**rulebook_security_guardrails.rego**:
```rego
# Current (line 9):
required_events: ["beforeShellExecution"]

# Updated:
required_events: ["beforeShellExecution", "beforeReadFile", "afterFileEdit", "beforeMCPExecution"]
```

**Files to modify**: 14 builtin policies (7 Claude + 7 Cursor)

---

### Task 2.2: Ensure Bash Redirection Detection

**Files**:
- `fixtures/claude/builtins/protected_paths.rego`
- `fixtures/cursor/builtins/protected_paths.rego`

**Verify existing** lines 40-62 block:
- `echo ... > protected/file`
- `echo ... >> protected/file`
- `tee protected/file`

**Add if missing** (already present in current implementation as dangerous patterns).

---

## Phase 3: Filesystem Hardening (Vulnerability #4)

### Task 3.1: Enforce Directory Permissions (Unix)

**File**: `cupcake-cli/src/main.rs`
**Function**: `init_project_config` (line ~1151)

**Add after** `.cupcake` directory creation (around line 1180):

```rust
// Set strict permissions on .cupcake directory (Unix only)
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let cupcake_path = Path::new(".cupcake");
    let mut perms = fs::metadata(&cupcake_path)
        .context("Failed to read .cupcake metadata")?
        .permissions();
    perms.set_mode(0o700); // rwx for owner, no permissions for group/other
    fs::set_permissions(&cupcake_path, perms)
        .context("Failed to set .cupcake permissions")?;
    info!("Set strict permissions (0700) on .cupcake directory");
}

#[cfg(not(unix))]
{
    warn!("Unable to set strict filesystem permissions on .cupcake directory on this OS. Please secure this directory manually.");
}
```

---

### Task 3.2: Block Symlink Creation

**Files**:
- `fixtures/claude/builtins/rulebook_security_guardrails.rego`
- `fixtures/cursor/builtins/rulebook_security_guardrails.rego`

**Add new rule** (after existing `halt` rules, around line 60):

```rego
# Block symlink creation targeting protected paths
halt contains decision if {
    input.hook_event_name == "PreToolUse"  # or "beforeShellExecution" for Cursor
    input.tool_name == "Bash"  # or check input.command for Cursor

    # Get command (harness-specific)
    command := lower(input.tool_input.command)  # Claude
    # command := lower(input.command)  # Cursor

    # Detect ln -s command
    regex.match(`(^|\s)ln\s+.*-s`, command)

    # Check if any protected path is source OR target (reviewer refinement)
    some protected_path in get_protected_paths
    contains(command, protected_path)

    message := get_configured_message

    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY-SYMLINK",
        "reason": concat("", [message, " (creating symlinks to protected directories is not permitted)"]),
        "severity": "CRITICAL"
    }
}
```

**Note**: Implement separate versions for Claude (line ~60) and Cursor (adjust `input` path).

---

## Phase 4: Comprehensive Adversarial Testing

**Timeline Adjustment** (Reviewer refinement): Allocate **4-5 days** for testing (not 2).

### Task 4.1: String Matching Bypass Tests

**New File**: `cupcake-core/tests/adversarial_string_matching.rs`

**Test Structure**:
```rust
use anyhow::Result;
use cupcake_core::engine::Engine;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// Helper: Setup engine with git_block_no_verify policy
async fn setup_git_no_verify_engine() -> Result<(TempDir, Engine)> { ... }

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_git_no_verify_extra_spaces() -> Result<()> {
    let (_temp, engine) = setup_git_no_verify_engine().await?;

    // Test variations with extra spaces
    let commands = vec![
        "git  commit  --no-verify",     // Double spaces
        "git   commit   --no-verify",   // Triple spaces
        "git commit  --no-verify",      // Space before flag
        "git  commit --no-verify",      // Space after git
    ];

    for cmd in commands {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": cmd},
            // ... other fields
        });

        let decision = engine.evaluate(&event, None).await?;
        assert!(
            matches!(decision, FinalDecision::Deny { .. }),
            "Should deny: {}", cmd
        );
    }
    Ok(())
}

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_git_no_verify_tabs() -> Result<()> { ... }

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_git_no_verify_leading_trailing_spaces() -> Result<()> { ... }

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_paths_whitelist_spacing() -> Result<()> {
    // Test:  cat, cat, \tcat variations
}

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_rulebook_security_rm_spacing() -> Result<()> {
    // Test: rm  .cupcake, rm   .cupcake variations
}
```

**Total Tests**: ~15 tests covering:
- git_block_no_verify (6 tests)
- protected_paths whitelist (5 tests)
- rulebook_security_guardrails (4 tests)

---

### Task 4.2: Cross-Tool Bypass Tests

**New File**: `cupcake-core/tests/adversarial_cross_tool.rs`

**Test Structure**:
```rust
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_protected_path_write_vs_bash() -> Result<()> {
    // Setup: protected_paths with "production.env"

    // Test 1: Write tool should be blocked
    let write_event = json!({
        "tool_name": "Write",
        "tool_input": {"file_path": "production.env", "content": "hack"}
    });
    assert!(matches!(decision, FinalDecision::Halt { .. }));

    // Test 2: Bash echo > should also be blocked
    let bash_event = json!({
        "tool_name": "Bash",
        "tool_input": {"command": "echo hack > production.env"}
    });
    assert!(matches!(decision, FinalDecision::Halt { .. }));

    // Test 3: Bash >> should also be blocked
    let bash_append = json!({
        "tool_name": "Bash",
        "tool_input": {"command": "echo hack >> production.env"}
    });
    assert!(matches!(decision, FinalDecision::Halt { .. }));
}

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_script_creation_execution_bypass() -> Result<()> {
    // Test multi-step bypass:
    // 1. Write creates script.sh with "rm production.env"
    // 2. Bash executes "chmod +x script.sh"
    // 3. Bash executes "./script.sh"

    // After fixes, step 1 or 3 should be blocked
}

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_metadata_expansion_routing() -> Result<()> {
    // Verify routing map shows all tools for expanded metadata
    let engine = setup_engine().await?;

    let routing = engine.routing_map();
    let protected_paths_routes = routing.get("PreToolUse:Write")
        .expect("Should route Write tool");

    assert!(protected_paths_routes.iter()
        .any(|p| p.package_name.contains("protected_paths")));
}
```

**Total Tests**: ~12 tests covering:
- Cross-tool equivalence (5 tests)
- Multi-step attacks (3 tests)
- Routing verification (4 tests)

---

### Task 4.3: Symlink Attack Tests

**New File**: `cupcake-core/tests/adversarial_symlink.rs`

**Test Structure**:
```rust
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_symlink_creation_blocked() -> Result<()> {
    // Test: ln -s .cupcake foo
    let event = json!({
        "tool_name": "Bash",
        "tool_input": {"command": "ln -s .cupcake foo"}
    });

    assert!(matches!(decision, FinalDecision::Halt { .. }));
}

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_symlink_creation_reverse() -> Result<()> {
    // Test: ln -s /other/path .cupcake (reviewer refinement)
    let event = json!({
        "tool_name": "Bash",
        "tool_input": {"command": "ln -s /tmp/fake .cupcake"}
    });

    assert!(matches!(decision, FinalDecision::Halt { .. }));
}

#[tokio::test]
#[cfg(feature = "deterministic-tests")]
#[cfg(unix)]
async fn test_directory_permissions() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Run cupcake init
    // Verify .cupcake has 0o700 permissions
    let metadata = fs::metadata(".cupcake")?;
    assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
}
```

**Total Tests**: ~10 tests covering:
- Symlink creation blocking (4 tests)
- Hard link attempts (2 tests)
- Permissions verification (2 tests Unix)
- Path traversal + symlink (2 tests)

---

### Task 4.4: Extend Existing Integration Tests

**Files**:
- `cupcake-core/tests/protected_paths_integration.rs`
- `cupcake-core/tests/rulebook_security_integration.rs`

**Add to protected_paths_integration.rs**:
```rust
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_bash_spacing_variations() -> Result<()> {
    // Test read whitelist with spacing
    let commands = vec![
        "cat secure.txt",
        "  cat secure.txt",    // Leading space
        "cat  secure.txt",     // Extra space
        "\tcat secure.txt",    // Tab
    ];
    // All should be allowed
}
```

**Add to rulebook_security_integration.rs**:
```rust
#[tokio::test]
#[cfg(feature = "deterministic-tests")]
async fn test_cupcake_modification_spacing() -> Result<()> {
    let commands = vec![
        "rm -rf .cupcake",
        "rm  -rf  .cupcake",   // Extra spaces
        "rm   .cupcake",       // Multiple spaces
    ];
    // All should be blocked
}
```

---

## Phase 5: Documentation

### Task 5.1: Update Security Reference

**File**: `docs/reference/security.md`

**Add section** after existing "Vulnerabilities Fixed" section (after line 35):

```markdown
### Bypass Vulnerabilities Fixed (2025-10-20)

Following the Trail of Bits security audit, three High-severity bypass vulnerabilities were addressed:

**TOB-EQTY-LAB-CUPCAKE-3: Bash Command String Matching Bypass**
- **Issue**: Policies using `contains()` could be bypassed with extra whitespace or command substitution
- **Fix**: Replaced string matching with regex patterns tolerant to whitespace variations
- **Affected**: `git_block_no_verify`, `rulebook_security_guardrails`, `protected_paths` (all builtins)

**TOB-EQTY-LAB-CUPCAKE-2: Cross-Tool Bypass**
- **Issue**: Policies were tool-specific, allowing equivalent operations via different tools
- **Fix**: Expanded policy metadata to cover all file modification tools
- **Affected**: `protected_paths`, `global_file_lock`, `rulebook_security_guardrails`

**TOB-EQTY-LAB-CUPCAKE-4: Symbolic Link Path Bypass**
- **Issue**: Path protection could be bypassed by creating symlinks to protected directories
- **Fix**: Added symlink creation blocking + OS-level directory permissions (Unix)
- **Affected**: `rulebook_security_guardrails`, directory initialization

**Testing**: 37+ new adversarial tests validate fixes against all known bypass vectors.

**Details**: See `VUL_BYPASS_UNDERSTANDING.md` for technical analysis.
```

---

## Execution Order

### Day 1-3: Regex Hardening
1. Task 1.1: Git no-verify (both harnesses)
2. Task 1.2: Rulebook security (both harnesses)
3. Task 1.3: Protected paths (both harnesses)

### Day 4-5: Cross-Tool Coverage
1. Task 2.1: Expand metadata (14 policies)
2. Task 2.2: Verify Bash redirection

### Day 6-7: Filesystem Hardening
1. Task 3.1: Directory permissions
2. Task 3.2: Symlink blocking

### Day 8-12: Testing (5 days - reviewer adjustment)
1. Task 4.1: String matching tests
2. Task 4.2: Cross-tool tests
3. Task 4.3: Symlink tests
4. Task 4.4: Integration tests

### Day 13: Documentation
1. Task 5.1: Update security.md
2. Update VUL_BYPASS_FIX_LOG.md

---

## Verification Checklist

- [ ] All regex patterns anchored to start: `(^|\s)`
- [ ] Symlink blocking checks both source AND target
- [ ] Cursor policies use correct event names and input paths
- [ ] Claude-specific builtins not deployed to Cursor
- [ ] Unix permission code has Windows warning
- [ ] All 37+ adversarial tests pass
- [ ] Existing tests still pass
- [ ] `cargo test --features deterministic-tests` passes
- [ ] `cargo fmt` and `cargo clippy` pass

---

## Open Risks & Mitigations

### Risk 1: Cursor afterFileEdit Limitation
**Issue**: Cursor only has post-edit hook, cannot prevent modifications before they happen.

**Mitigation**: Document limitation; focus on beforeShellExecution for prevention.

### Risk 2: Regex Performance
**Issue**: Multiple regex checks per command could impact performance.

**Mitigation**: Keep blanket `contains()` checks as first pass; regex only when necessary.

### Risk 3: Test Suite Complexity
**Issue**: 37+ tests with tempdir setup is complex to maintain.

**Mitigation**: Create shared test helpers (`test_helpers.rs`) for engine setup.

---

## Team Review Feedback Incorporated

✅ Regex patterns anchored to start: `(^|\s)verb\s`
✅ Symlink blocking checks source AND target
✅ Testing timeline increased to 4-5 days
✅ Windows permissions warning added
✅ Harness event mapping completed
✅ Cross-harness tool coverage clarified

**Approval**: Plan approved by team member with refinements.
