# Comprehensive Bypass Vulnerability Remediation Plan

**Status**: DRAFT - Comprehensive expansion incorporating generic solutions
**Original Plan**: VUL_BYPASS_PLAN.md (builtin-only fixes)
**This Plan**: Addresses vulnerabilities architecturally for all policies (builtin + user)

---

## Executive Summary

**Problem**: Trail of Bits identified three High-severity bypass vulnerabilities. The original plan fixed only builtin policies, leaving user-written policies vulnerable to the same attacks.

**Solution**: Four-layer defense architecture:
1. **Helper Library** - Secure primitives users can import
2. **Engine Preprocessing** - Canonical path resolution, command tokenization
3. **Declarative Abstractions** - YAML-based policy generation for common patterns
4. **Policy Linting** - Detect vulnerable patterns, recommend fixes

**Timeline**: ~18-20 working days (vs 10 days for builtin-only approach)

**Vulnerabilities Addressed**:
- TOB-EQTY-LAB-CUPCAKE-3: String matching bypass (spacing, functions, substitution)
- TOB-EQTY-LAB-CUPCAKE-2: Cross-tool bypass (tool-specific routing)
- TOB-EQTY-LAB-CUPCAKE-4: Symlink path bypass (string checks on unresolved paths)

---

## Architecture Overview

### Current (Vulnerable)
```
User writes Rego → Uses contains() on commands → Vulnerable to bypasses
```

### After Remediation
```
User writes Rego → Imports helpers → Secure by default
             ↓
   OR: Uses YAML config → Generated Rego (using helpers) → Secure by default
             ↓
   Linting catches anti-patterns → Suggests helper usage
             ↓
   Engine provides canonical inputs → Helpers use resolved paths
```

---

## Phase 1: Helper Library Foundation (2 days)

**Goal**: Provide secure Rego primitives that handle bypasses correctly

### 1.1: Design Helper API (0.5 day)

**Deliverables**:
- Helper library design document
- API specification for two modules:
  - `data.cupcake.helpers.commands` - Shell command analysis
  - `data.cupcake.helpers.paths` - Path normalization and validation

**API Examples**:
```rego
import data.cupcake.helpers.commands
import data.cupcake.helpers.paths

# Instead of: contains(cmd, "rm")  # VULNERABLE
commands.has_verb(cmd, "rm")  # SAFE - regex with anchoring

# Instead of: contains(path, ".cupcake")  # VULNERABLE to symlinks
paths.targets_protected(path, ".cupcake")  # SAFE - normalizes first
```

### 1.2: Implement commands Module (0.75 day)

**File**: `fixtures/helpers/commands.rego`

**Functions**:
```rego
# Detect command verb with proper anchoring
has_verb(command, verb) - Handles (^|\s)verb\s pattern

# Detect dangerous commands
has_dangerous_verb(command, verb_set) - Checks multiple verbs

# Detect output redirection
has_output_redirect(command) - Detects >, >>, tee patterns

# Detect command substitution
has_command_substitution(command) - Detects $(), ``, ${}

# Detect symlink creation
creates_symlink(command) - Detects ln -s patterns

# Check both directions for symlink
symlink_involves_path(command, protected_path) - Checks source AND target
```

**Implementation Notes**:
- All regex patterns anchored to `(^|\s)` to prevent spacing bypass
- Handles variable whitespace between tokens
- Case-insensitive by default (callers should lowercase input)

### 1.3: Implement paths Module (0.75 day)

**File**: `fixtures/helpers/paths.rego`

**Functions**:
```rego
# Check if path targets protected directory
targets_protected(file_path, protected_path) - Normalizes and checks

# Normalize path (remove ./, //, etc.)
normalize(file_path) - Regex-based normalization

# Check multiple protected paths
targets_any_protected(file_path, protected_paths_array)

# Check if path is absolute
is_absolute(file_path)

# Check if path escapes directory
escapes_directory(file_path, base_dir) - Detects ../.. patterns
```

**Limitations Documented**:
- WASM sandbox cannot resolve symlinks (filesystem access blocked)
- Normalization is string-based, not filesystem-based
- Recommend engine preprocessing for true resolution

---

## Phase 2: Engine Preprocessing (2 days)

**Goal**: Rust engine provides canonical/normalized inputs to policies

### 2.1: Design Preprocessing Interface (0.5 day)

**Deliverables**:
- Design document for preprocessing system
- Input schema additions

**Proposed Schema Addition**:
```json
{
  "hook_event_name": "PreToolUse",
  "tool_input": {
    "file_path": "/project/.secret-link"  // Original
  },
  "canonical_inputs": {
    "file_path": "/project/.cupcake/policies/secret.rego",  // Resolved symlink
    "file_path_normalized": "/project/.cupcake/policies/secret.rego"  // Cleaned
  },
  "parsed_inputs": {
    "command_tokens": ["git", "commit", "-m", "foo", "--no-verify"]  // Tokenized
  }
}
```

**Scope**:
- Symlink resolution (Unix realpath, Windows GetFinalPathNameByHandle)
- Path normalization (remove ./, //, etc.)
- Command tokenization (shell-aware splitting)

### 2.2: Implement Symlink Resolution (0.75 day)

**Files**: `cupcake-core/src/engine/preprocessing.rs`

**Implementation**:
```rust
use std::fs;

pub struct Preprocessor;

impl Preprocessor {
    pub fn resolve_paths(input: &mut Value) -> Result<()> {
        // Extract file_path fields from tool_input
        if let Some(file_path) = input.get("tool_input")
            .and_then(|t| t.get("file_path"))
            .and_then(|p| p.as_str()) {

            // Resolve symlink
            if let Ok(canonical) = fs::canonicalize(file_path) {
                input["canonical_inputs"]["file_path"] =
                    canonical.to_string_lossy().into();
            }
        }
        Ok(())
    }
}
```

**Error Handling**:
- If path doesn't exist, skip resolution (don't fail evaluation)
- Log resolution failures at DEBUG level
- Policies check both `tool_input.file_path` and `canonical_inputs.file_path`

### 2.3: Implement Command Tokenization (0.75 day)

**Implementation**:
```rust
// Use shell-words crate for proper tokenization
use shell_words;

pub fn tokenize_command(command: &str) -> Vec<String> {
    shell_words::split(command).unwrap_or_else(|_| vec![])
}
```

**Integration**:
- Applied to Bash tool commands before policy evaluation
- Exposed as `input.parsed_inputs.command_tokens`
- Handles quoted strings, escapes, etc.

---

## Phase 3: Refactor Builtins to Use Helpers (1.5 days)

**Goal**: Prove helper library works, DRY up builtin code

### 3.1: Refactor git_block_no_verify (0.25 day)

**Before** (fixtures/claude/builtins/git_block_no_verify.rego):
```rego
contains_git_no_verify(cmd) if {
    regex.match(`(^|\s)git\s+commit\s+.*--no-verify`, cmd)
}
```

**After**:
```rego
import data.cupcake.helpers.commands

contains_git_no_verify(cmd) if {
    commands.has_verb(cmd, "git")
    commands.has_verb(cmd, "commit")
    contains(cmd, "--no-verify")
}
```

**Benefits**:
- Simpler logic
- Reusable patterns
- Centralized maintenance

### 3.2: Refactor rulebook_security_guardrails (0.5 day)

**Current** (fixtures/claude/builtins/rulebook_security_guardrails.rego:97-108):
```rego
is_dangerous_command(cmd) if {
    dangerous_verbs := {"rm", "rmdir", "mv", "cp", ...}
    some verb in dangerous_verbs
    regex.match(concat("", [`(^|\s)`, verb, `\s`]), cmd)
}
```

**After**:
```rego
import data.cupcake.helpers.commands
import data.cupcake.helpers.paths

contains_cupcake_modification_pattern(cmd) if {
    # Use canonical path if available (from engine preprocessing)
    file_path := object.get(input, ["canonical_inputs", "file_path"],
                            input.tool_input.file_path)

    paths.targets_protected(file_path, ".cupcake")
    commands.has_dangerous_verb(cmd, {"rm", "mv", "cp", "chmod", ...})
}
```

### 3.3: Refactor protected_paths (0.5 day)

**Apply same pattern** to fixtures/claude/builtins/protected_paths.rego

### 3.4: Apply to Cursor Versions (0.25 day)

**Refactor** all three Cursor builtin versions to use helpers

---

## Phase 4: Cross-Tool Metadata Expansion (1 day)

**Goal**: Address TOB-EQTY-LAB-CUPCAKE-2 (cross-tool bypass)

### 4.1: Audit Policy Metadata (0.25 day)

**Review** 14 builtin policies for missing cross-tool coverage

**Example Issue**:
```rego
# Current: Only blocks Bash
# METADATA
# routing:
#   required_events: ["PreToolUse"]
#   required_tools: ["Bash"]

# Should: Block all file writes
# METADATA
# routing:
#   required_events: ["PreToolUse"]
#   required_tools: ["Bash", "Write", "Edit", "MultiEdit"]
```

### 4.2: Expand Metadata (0.5 day)

**Update** routing metadata to cover attack vectors identified in VUL_BYPASS_UNDERSTANDING.md

### 4.3: Add Redirection Detection (0.25 day)

**Ensure** protected_paths detects `bash command > .cupcake/file`

**Implementation**:
```rego
import data.cupcake.helpers.commands

deny contains decision if {
    input.tool_name == "Bash"
    cmd := lower(input.tool_input.command)

    # Check if command has output redirection
    commands.has_output_redirect(cmd)

    # Check if redirecting to protected path
    paths.targets_any_protected(cmd, get_protected_paths())
}
```

---

## Phase 5: Symlink Defenses (1 day)

**Goal**: Address TOB-EQTY-LAB-CUPCAKE-4 (symlink bypass)

### 5.1: Unix Directory Permissions (0.5 day)

**File**: `cupcake-cli/src/main.rs:1151` (init_project_config)

**Implementation**:
```rust
#[cfg(unix)]
fn set_cupcake_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o700);  // Owner: rwx, Group: ---, Other: ---
    fs::set_permissions(path, perms)?;
    Ok(())
}

// In init_project_config():
#[cfg(unix)]
set_cupcake_permissions(&cupcake_dir)?;
```

**Warning for Windows**:
```rust
#[cfg(windows)]
eprintln!("Warning: .cupcake directory permissions should be restricted manually on Windows");
```

### 5.2: Block Symlink Creation (0.5 day)

**Add to rulebook_security_guardrails** (both harnesses):
```rego
import data.cupcake.helpers.commands

halt contains decision if {
    input.tool_name == "Bash"
    cmd := lower(input.tool_input.command)

    # Detect symlink creation
    commands.creates_symlink(cmd)

    # Check if EITHER source OR target is protected path
    commands.symlink_involves_path(cmd, ".cupcake")

    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY",
        "reason": "Creating symlinks involving .cupcake directory is not permitted",
        "severity": "HIGH"
    }
}
```

---

## Phase 6: Declarative Abstractions (2 days)

**Goal**: Simplify common patterns - users write YAML, generate Rego using helpers

### 6.1: Design YAML Schema (0.5 day)

**Deliverables**:
- YAML schema for common policy patterns
- Code generation design

**Example YAML** (.cupcake/policies/protect-config.yml):
```yaml
# Declarative policy - generates Rego using helpers
protected_paths:
  - path: ".cupcake/"
    severity: CRITICAL
    tools: [Bash, Edit, Write, MultiEdit]
    message: "Cupcake configuration is protected"

  - path: ".env"
    severity: HIGH
    tools: [Bash, Read, Write, Edit]
    message: "Environment files contain secrets"

block_commands:
  - verbs: ["rm", "mv", "chmod"]
    when_targeting: [".git/hooks/"]
    severity: HIGH
    message: "Git hooks cannot be modified"
```

### 6.2: Implement Code Generator (1 day)

**Binary**: `cupcake generate` command

**Implementation**:
```bash
$ cupcake generate .cupcake/policies/protect-config.yml

Generated: .cupcake/policies/protect-config.rego
```

**Generated Rego**:
```rego
# Auto-generated from protect-config.yml
# DO NOT EDIT - regenerate with: cupcake generate

import data.cupcake.helpers.commands
import data.cupcake.helpers.paths

halt contains decision if {
    input.tool_name in {"Bash", "Edit", "Write", "MultiEdit"}
    file_path := object.get(input, ["canonical_inputs", "file_path"],
                            get_file_path_from_tool_input())
    paths.targets_protected(file_path, ".cupcake/")

    decision := {
        "rule_id": "GENERATED-PROTECT-CUPCAKE",
        "reason": "Cupcake configuration is protected",
        "severity": "CRITICAL"
    }
}
```

### 6.3: Update Init Command (0.5 day)

**Modify** `cupcake init` to offer choice:

```bash
$ cupcake init

How would you like to write policies?
1) Rego (full control, requires learning OPA)
2) YAML (declarative, limited to common patterns)
3) Both (YAML for common cases, Rego for complex logic)

Choice: 2

Created: .cupcake/policies/protected_paths.yml
Run 'cupcake generate' to build policies
```

---

## Phase 7: Policy Linting (2 days)

**Goal**: Detect vulnerable patterns, guide users to helpers

### 7.1: Design Lint Rules (0.5 day)

**Deliverables**:
- Anti-pattern catalog
- Lint rule specifications

**Lint Rules**:

| Rule ID | Pattern | Issue | Fix |
|---------|---------|-------|-----|
| `LINT-001` | `contains(cmd, "verb")` | Vulnerable to spacing bypass | Use `commands.has_verb(cmd, "verb")` |
| `LINT-002` | `contains(path, ".cupcake")` | Vulnerable to symlink bypass | Use `paths.targets_protected(path, ".cupcake")` |
| `LINT-003` | `input.tool_input.file_path` only | Missing canonical path check | Check `input.canonical_inputs.file_path` too |
| `LINT-004` | No `import data.cupcake.helpers` | Not using secure primitives | Import helper library |
| `LINT-005` | Regex without `(^|\s)` anchor | Vulnerable to prefix bypass | Anchor to start or whitespace |
| `LINT-006` | Symlink check missing target | Only checks `ln -s .cupcake X` | Check both source and target |

### 7.2: Implement Linter (1 day)

**Binary**: `cupcake lint` command

**Implementation**:
```bash
$ cupcake lint .cupcake/policies/

Linting 5 policies...

❌ .cupcake/policies/my-policy.rego:42
   Rule: LINT-001 (String matching bypass vulnerability)

   contains(cmd, "rm")
   ^^^^^^^^^^^^^^^^^^

   Issue: Substring matching on shell commands is vulnerable to spacing bypasses.
          An attacker could use 'git  rm' or '$(echo rm)' to bypass this check.

   Fix: Use the helper library for secure command detection:

   import data.cupcake.helpers.commands

   commands.has_verb(cmd, "rm")

⚠️  .cupcake/policies/my-policy.rego:67
   Rule: LINT-004 (Not using helper library)

   Issue: This policy doesn't import data.cupcake.helpers.
          Helper functions provide secure primitives that handle bypass patterns.

   Recommendation: Add to top of file:
   import data.cupcake.helpers.commands
   import data.cupcake.helpers.paths

Summary: 1 error, 1 warning in 5 files
```

### 7.3: Integration with CI (0.5 day)

**GitHub Action Template**:
```yaml
name: Policy Linting
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Cupcake
        run: cargo install cupcake-cli
      - name: Lint Policies
        run: cupcake lint .cupcake/policies/ --strict
```

---

## Phase 8: Comprehensive Testing (5 days)

**Goal**: Validate all components with adversarial tests

### 8.1: Helper Library Tests (0.5 day)

**File**: `cupcake-core/tests/helpers_test.rs`

**Test Cases**:
```rust
#[test]
fn test_has_verb_with_spacing_bypass() {
    // Test that helpers handle spacing correctly
    assert!(commands::has_verb("rm  -rf  /", "rm"));  // Extra spaces
    assert!(commands::has_verb("  rm -rf /", "rm"));  // Leading space
    assert!(!commands::has_verb("harmful", "rm"));     // Not a verb
}

#[test]
fn test_paths_normalization() {
    assert!(paths::targets_protected("././.cupcake/file", ".cupcake"));
    assert!(paths::targets_protected("./.cupcake/../.cupcake/file", ".cupcake"));
}
```

### 8.2: Engine Preprocessing Tests (0.5 day)

**File**: `cupcake-core/tests/preprocessing_test.rs`

**Test Cases**:
```rust
#[test]
fn test_symlink_resolution() {
    // Create: .secret-link -> .cupcake/policies/secret.rego
    // Verify: canonical_inputs.file_path contains ".cupcake"
}

#[test]
fn test_command_tokenization() {
    let tokens = tokenize("git commit -m \"fix bug\" --no-verify");
    assert_eq!(tokens, vec!["git", "commit", "-m", "fix bug", "--no-verify"]);
}
```

### 8.3: Adversarial String Matching (1 day)

**File**: `cupcake-core/tests/adversarial_string_matching.rs`

**From VUL_BYPASS_PLAN.md Phase 4.1** - tests for:
- Spacing variations (`git  commit`, `git    commit`)
- Shell functions (`myrm() { rm "$@"; }; myrm -rf .cupcake`)
- Command substitution (`$(echo git) commit --no-verify`)
- Variable expansion (`CMD=commit; git $CMD --no-verify`)

**All tests should FAIL before fixes, PASS after**

### 8.4: Adversarial Cross-Tool (1 day)

**File**: `cupcake-core/tests/adversarial_cross_tool.rs`

**From VUL_BYPASS_PLAN.md Phase 4.2** - tests for:
- Bash redirection to protected paths
- Write tool to protected paths when Bash-only policy exists
- Edit tool when only Write is blocked

### 8.5: Adversarial Symlink (1 day)

**File**: `cupcake-core/tests/adversarial_symlink.rs`

**From VUL_BYPASS_PLAN.md Phase 4.3** - tests for:
- Symlink source → protected (block)
- Symlink protected → target (block)
- Symlink both directions
- Engine canonical path resolution

### 8.6: Linting Tests (0.5 day)

**File**: `cupcake-cli/tests/lint_test.rs`

**Test Cases**:
```rust
#[test]
fn test_lint_detects_contains_on_command() {
    let result = run_lint(r#"
        deny contains decision if {
            contains(input.tool_input.command, "rm")
        }
    "#);
    assert!(result.errors.contains("LINT-001"));
}
```

### 8.7: Integration Tests Extension (0.5 day)

**Extend existing**:
- `cupcake-core/tests/protected_paths_integration.rs`
- `cupcake-core/tests/rulebook_security_integration.rs`

**Add**: Tests for refactored builtins using helpers

---

## Phase 9: Documentation (1.5 days)

**Goal**: Comprehensive security guide for policy authors

### 9.1: Security Anti-Patterns Guide (0.5 day)

**File**: `docs/reference/security-anti-patterns.md`

**Structure**:
```markdown
# Security Anti-Patterns in Policy Writing

## ❌ Anti-Pattern 1: Substring Matching on Commands

**Vulnerable**:
```rego
deny contains decision if {
    contains(input.tool_input.command, "rm")  # BYPASS: "harmful" contains "rm"
}
```

**Secure**:
```rego
import data.cupcake.helpers.commands

deny contains decision if {
    commands.has_verb(cmd, "rm")  # Properly anchored regex
}
```

**Why**: Substring matching fails with spacing, functions, substitution...

[More anti-patterns...]
```

### 9.2: Helper Library Documentation (0.5 day)

**File**: `docs/reference/helper-library.md`

**API Reference** for all helper functions with examples

### 9.3: Update Main Security Docs (0.25 day)

**File**: `docs/reference/security.md`

**Add sections**:
- Link to anti-patterns guide
- Recommend helper library for all policies
- Mention linting tool
- Explain engine preprocessing

### 9.4: Update Example Policies (0.25 day)

**File**: `examples/fixtures/security_policy.rego`

**Refactor** to use helpers, add comments explaining patterns

---

## Phase 10: Verification & Closeout (0.5 day)

### 10.1: Verification Checklist

**Run through**:
- [ ] All 3 vulnerabilities have test cases that previously failed
- [ ] All adversarial tests now pass
- [ ] Linter detects all known anti-patterns
- [ ] Helper library has >90% test coverage
- [ ] Engine preprocessing handles symlinks on Unix
- [ ] Documentation covers all new features
- [ ] Example policies use helpers
- [ ] Builtins refactored to use helpers

### 10.2: Update Fix Log

**File**: VUL_BYPASS_FIX_LOG.md

**Final entry** documenting completion

### 10.3: Release Notes

**Draft release notes** for comprehensive remediation:

```markdown
## v0.2.0 - Comprehensive Bypass Vulnerability Remediation

### Security Fixes (HIGH SEVERITY)

Fixed three High-severity vulnerabilities identified by Trail of Bits:
- TOB-EQTY-LAB-CUPCAKE-3: String matching bypass
- TOB-EQTY-LAB-CUPCAKE-2: Cross-tool bypass
- TOB-EQTY-LAB-CUPCAKE-4: Symlink path bypass

### New Features

**Helper Library** (`data.cupcake.helpers.*`):
- Secure primitives for command and path analysis
- Handles spacing, substitution, symlink bypasses automatically
- Recommended for all policy authors

**Engine Preprocessing**:
- Canonical path resolution (symlinks)
- Command tokenization
- Available via `input.canonical_inputs.*`

**Declarative Policies**:
- YAML-based policy generation
- `cupcake generate` command
- Ideal for common patterns (protected paths, blocked commands)

**Policy Linting**:
- `cupcake lint` command
- Detects vulnerable patterns
- Recommends secure alternatives

### Breaking Changes

- Builtins refactored to use helper library (behavior unchanged)
- `.cupcake/` directory now created with 0o700 permissions on Unix

### Migration Guide

Existing policies should be updated to use helpers:

**Before**:
```rego
contains(cmd, "rm")  # Vulnerable
```

**After**:
```rego
import data.cupcake.helpers.commands
commands.has_verb(cmd, "rm")  # Secure
```

Run `cupcake lint` to identify issues in your policies.
```

---

## Timeline Summary

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| 1. Helper Library | 2 days | None (start immediately) |
| 2. Engine Preprocessing | 2 days | None (parallel with Phase 1) |
| 3. Refactor Builtins | 1.5 days | Phase 1 complete |
| 4. Cross-Tool Metadata | 1 day | Phase 3 complete |
| 5. Symlink Defenses | 1 day | Phase 2 complete |
| 6. Declarative Abstractions | 2 days | Phase 1 complete |
| 7. Policy Linting | 2 days | Phase 1, 3 complete |
| 8. Testing | 5 days | Phases 1-7 complete |
| 9. Documentation | 1.5 days | Phases 1-7 complete |
| 10. Verification | 0.5 day | All phases complete |
| **TOTAL** | **18.5 days** | |

**Critical Path**: Phase 1 → Phase 3 → Phase 7 → Phase 8 → Phase 10

**Parallelization Opportunities**:
- Phases 1 + 2 (helpers + preprocessing)
- Phases 4 + 5 + 6 (metadata + symlinks + abstractions) after Phase 3
- Phase 9 (docs) can start once phases are individually complete

**Realistic Timeline**: 18-20 working days (4 calendar weeks with buffer)

---

## Success Metrics

**Security**:
- [ ] Zero bypasses possible using ToB attack vectors
- [ ] Linter catches 100% of known anti-patterns
- [ ] All adversarial tests pass

**Usability**:
- [ ] Users can write secure policies without knowing bypass patterns
- [ ] YAML abstractions cover 80%+ of common use cases
- [ ] Linter provides actionable fix suggestions

**Maintainability**:
- [ ] Builtins use shared helper library (DRY)
- [ ] Anti-pattern documentation prevents regression
- [ ] Test suite validates defenses comprehensively

---

## Risk Assessment

**High Risk**:
- Engine preprocessing adds complexity (filesystem I/O in hot path)
- Code generation could produce unexpected Rego
- Timeline is 2x original plan

**Mitigation**:
- Preprocessing is optional enhancement (graceful fallback)
- Generated code tested as rigorously as hand-written
- Parallelization can compress timeline
- Phases can ship incrementally (helper library first)

**Fallback Plan**:
- Ship Phase 1-5 (core fixes) as v0.2.0
- Ship Phase 6-7 (abstractions + linting) as v0.3.0

---

## Harness Compatibility

All phases account for both harnesses:

**Claude Code**:
- Events: `PreToolUse`, `PostToolUse`
- Input: `input.tool_input.command`
- Tools: `Bash`, `Edit`, `Write`, `Read`, etc.

**Cursor**:
- Events: `beforeShellExecution`, `afterFileEdit`, `beforeReadFile`
- Input: `input.command` (direct)
- No tool names (event-based routing)

**Approach**:
- Helper library is harness-agnostic (pure Rego)
- Engine preprocessing works for both (unified schema)
- Builtins have separate versions per harness
- Tests run against both harness schemas

---

## References

- **VUL_BYPASS_ISSUES.md** - Trail of Bits vulnerability report
- **VUL_BYPASS_UNDERSTANDING.md** - Comprehensive root cause analysis
- **VUL_BYPASS_PLAN.md** - Original builtin-only remediation plan
- **VUL_BYPASS_FIX_LOG.md** - Work log (ongoing)
- **docs/agents/claude-code/hooks[official][09062025].md** - Claude Code events
- **docs/agents/cursor/hooks[official][10112025].md** - Cursor events
