# Custom Review Instructions for GitHub Copilot AI Reviewer

## Branch Overview

**Branch:** `tob/fix-bypasses`
**Target:** `main`
**Commits:** 45 commits
**Purpose:** Address external security audit findings from Trail of Bits (TOB) and implement comprehensive defense-in-depth security fixes

This branch represents a major security hardening effort implementing fixes for critical vulnerabilities identified in a professional security audit of the Cupcake policy engine.

---

## What is Cupcake?

Cupcake is a **policy enforcement engine** for AI coding agents (like Claude Code and Cursor). It acts as a security and governance layer that:

1. **Intercepts tool calls** from AI agents before they execute
2. **Evaluates them against policies** written in OPA Rego (compiled to WebAssembly)
3. **Returns decisions** (Allow/Block/Warn/Ask) to enforce rules

**Core Architecture:**
```
AI Agent → Hook Event → Cupcake Engine → Policy Evaluation (WASM) → Decision → AI Agent
```

### Key Concepts

#### 1. Harnesses
A **harness** is the AI coding agent platform (Claude Code, Cursor, etc.). Each harness:
- Has unique event formats (different JSON structures)
- Has unique response formats
- Has different capabilities (context injection, file access)

**Critical Design Decision:** Cupcake uses a **harness-specific architecture** with NO normalization layer. Events flow through in their native format, and policies are physically separated by harness:

```
.cupcake/policies/
├── claude/          # Claude Code policies
└── cursor/          # Cursor policies
```

#### 2. Hooks
**Hooks** are integration points where AI agents call external commands during their execution. Examples:
- `PreToolUse` - Before a tool executes (e.g., before running a shell command)
- `PostToolUse` - After a tool completes
- `UserPromptSubmit` - Before processing user input
- `SessionStart` - When a session begins

Cupcake integrates via hooks in `.claude/settings.json` or `.cursor/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [{
      "hooks": [{
        "type": "command",
        "command": "cupcake eval --harness claude"
      }]
    }]
  }
}
```

#### 3. Preprocessing (NEW - Core of this Branch)
**Input preprocessing** is a Rust-level defense layer that automatically normalizes and enriches input **before** policy evaluation. This provides universal protection against bypass attacks.

**Critical Architecture:** Preprocessing happens **inside** `Engine.evaluate()`, not at the CLI level. This ensures:
- All entry points (CLI, FFI, tests) are protected
- Impossible to bypass by calling the engine directly
- Future integrations automatically secure

**Preprocessing Pipeline:**
1. **Whitespace normalization** (TOB-3 defense) - Collapses spaces, preserves quotes
2. **Symlink resolution** (TOB-4 defense) - Resolves canonical paths, detects symlinks
3. **Script inspection** (TOB-2 defense, opt-in) - Loads script content for analysis

#### 4. Policies
Policies are written in **OPA Rego v1.0** and compiled to **WebAssembly** for sandboxed execution. They declare:
- **Routing metadata** - Which events/tools they care about
- **Decision verbs** - What actions to take (`deny`, `halt`, `ask`, `add_context`)
- **Structured decisions** - `rule_id`, `reason`, `severity`

Example policy structure:
```rego
# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.example

deny contains decision if {
    input.tool_input.command contains "rm -rf"
    decision := {
        "rule_id": "BLOCK-RM",
        "reason": "Dangerous command",
        "severity": "HIGH"
    }
}
```

---

## Security Audit Context

### Trail of Bits Findings (2025-09)

This branch addresses **4 critical/high severity vulnerabilities** identified in a professional security audit:

1. **TOB-3 (High):** Spacing bypass - `rm  -rf` bypasses `"rm -rf"` string match
2. **TOB-2 (High):** Cross-tool script execution - Write malicious script, Bash executes it
3. **TOB-4 (Critical):** Symlink bypass - Create symlink to protected path, write through it
4. **Multiple comments:** Documentation inaccuracies and clarity issues

### Security Model

**Defense-in-Depth Principles:**
1. **No Ambient Authority** - Config via CLI flags only, never environment variables
2. **Automatic Preprocessing** - Rust-level input normalization protects all policies
3. **Sandboxed Evaluation** - Policies run in WASM with memory isolation
4. **Explicit Consent** - Debug output requires explicit CLI flags
5. **Fail-Safe Defaults** - All security limits enforced with safe minimums

**Performance:**
- Preprocessing overhead: ~30-100μs (<0.1% of evaluation time)
- Whitespace normalization: <1μs
- Path canonicalization: ~15μs
- Symlink detection: ~15μs

---

## What This Branch Changes

### 1. Input Preprocessing System (NEW)

**Files Added:**
- `cupcake-core/src/preprocessing/mod.rs` (612 lines) - Main preprocessing pipeline
- `cupcake-core/src/preprocessing/config.rs` (181 lines) - Configuration
- `cupcake-core/src/preprocessing/normalizers.rs` (331 lines) - Whitespace normalization
- `cupcake-core/src/preprocessing/symlink_resolver.rs` (299 lines) - Path canonicalization
- `cupcake-core/src/preprocessing/script_inspector.rs` (317 lines) - Script content inspection

**Integration Point:**
```rust
// cupcake-core/src/engine/mod.rs (lines 889-903)
pub async fn evaluate(&self, input: &Value, ...) -> Result<FinalDecision> {
    // STEP 0: ALWAYS PREPROCESS (Self-Defending Engine)
    let mut safe_input = input.clone();
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut safe_input, &preprocess_config, self.config.harness);

    // Continue with SAFE input...
}
```

**What Gets Added to Events:**

For file operations (Edit, Write, Read, NotebookEdit, MultiEdit):
```json
{
  "tool_input": {"file_path": "config.json"},
  // ADDED BY PREPROCESSING:
  "resolved_file_path": "/home/user/project/config.json",  // Canonical
  "original_file_path": "config.json",                     // Original
  "is_symlink": false                                       // Detection flag
}
```

For Bash commands (if whitespace normalization needed):
```json
{
  "tool_input": {
    "command": "rm -rf .cupcake"  // Normalized from "rm  -rf  .cupcake"
  }
}
```

### 2. Policy Updates

**All builtin policies updated to use preprocessing-enriched input:**

**Claude Code Policies:**
- `fixtures/claude/builtins/protected_paths.rego` - Now uses `input.resolved_file_path`
- `fixtures/claude/builtins/rulebook_security_guardrails.rego` - Symlink-aware
- `fixtures/claude/builtins/git_block_no_verify.rego` - Uses command helpers

**Cursor Policies:**
- `fixtures/cursor/builtins/protected_paths.rego` - Uses `input.resolved_file_path`
- `fixtures/cursor/builtins/rulebook_security_guardrails.rego` - Symlink-aware
- `fixtures/cursor/builtins/git_block_no_verify.rego` - Uses command helpers

**Global Builtins:**
- `fixtures/global_builtins/claude/system_protection.rego` - Uses resolved paths
- `fixtures/global_builtins/claude/sensitive_data_protection.rego` - Uses resolved paths
- `fixtures/global_builtins/cursor/*` - Mirror changes

**Key Pattern:**
```rego
# OLD (vulnerable to symlinks)
file_path := input.tool_input.file_path

# NEW (secure)
file_path := input.resolved_file_path  # Always canonical, symlinks resolved
```

### 3. Helper Libraries

**New helper modules for secure string matching:**

`fixtures/helpers/commands.rego` (49 lines):
- `has_verb(command, verb)` - Regex-based word boundary matching
- `has_dangerous_verb(command, verb_set)` - Set-based matching
- `creates_symlink(command)` - Detects `ln -s`
- `symlink_involves_path(command, protected_path)` - Symlink protection

**Usage in policies:**
```rego
# OLD (vulnerable to spacing)
contains(cmd, "rm -rf")

# NEW (secure)
import data.cupcake.helpers.commands
commands.has_verb(cmd, "rm")
```

### 4. Test Infrastructure

**New adversarial test suites (12,880 total test lines):**

- `cupcake-core/tests/adversarial_string_matching.rs` (571 lines) - Spacing bypass tests
- `cupcake-core/tests/adversarial_cross_tool.rs` (643 lines) - Cross-tool bypass tests
- `cupcake-core/tests/adversarial_symlink.rs` (526 lines) - Symlink bypass tests
- `cupcake-core/tests/adversarial_script_execution.rs` (268 lines) - Script execution tests
- `cupcake-core/tests/tob4_path_traversal_test.rs` (226 lines) - Path traversal validation
- `cupcake-core/tests/tob4_symlink_integration.rs` (365 lines) - Symlink integration tests
- `cupcake-core/tests/tob2_script_integration.rs` (342 lines) - Script inspection tests
- `cupcake-core/tests/preprocessing_integration.rs` (383 lines) - Preprocessing validation
- `cupcake-core/tests/preprocessing_cursor_test.rs` (201 lines) - Cursor preprocessing

**Test Execution Requirements:**
```bash
# CRITICAL: Must use deterministic-tests feature
cargo test --features deterministic-tests

# Or use alias
cargo t
```

**Why:** The trust system uses HMAC key derivation which is non-deterministic in production mode. The feature flag ensures deterministic keys for reliable test execution.

**Test Structure Changes:**
- Moved `tests/test_helpers.rs` → `tests/common/mod.rs` (proper Rust test pattern)
- Added `create_test_project_for_harness()` to prevent duplicate package errors
- All tests now disable global config via `EngineConfig::global_config(false)`

### 5. Documentation

**New comprehensive documentation:**

- `docs/reference/security.md` (543 lines) - Security model, TOB fixes, defenses
- `docs/reference/input-preprocessing.md` (586 lines) - Preprocessing architecture
- `docs/user-guide/policies/writing-policies.md` (94 lines additions) - Secure policy patterns

**Updates to CLAUDE.md:**
- Preprocessing defense section
- Testing requirements
- Policy development guidance
- Rego v1 migration checklist

### 6. Comment Clarifications

**External reviewer feedback addressed:**

1. **engine/mod.rs:889-896** - Clarified when preprocessing modifies input
2. **script_inspector.rs:47-54** - Added regression test for -c flag handling
3. **protected_paths.rego:227-232** - Explained why sed -i check is NOT redundant
4. **protected_paths.rego:229** - Changed "fails on" to "explicitly rejects"

---

## Critical Review Focus Areas

### 1. Security Correctness

**Preprocessing Integration:**
- ✅ Verify preprocessing happens **inside** `Engine.evaluate()` (lines 889-904)
- ✅ Verify clone is intentional and documented (security requirement)
- ✅ Verify preprocessing is idempotent (safe to call multiple times)
- ✅ Verify no bypass paths exist (CLI, FFI, tests all go through engine)

**Path Canonicalization:**
- ✅ Verify `std::fs::canonicalize()` is used correctly
- ✅ Verify symlink detection logic (metadata check + resolve)
- ✅ Verify dangling symlink handling (parent directory fallback)
- ✅ Verify cross-platform compatibility (Unix, Windows, macOS)

**Whitespace Normalization:**
- ✅ Verify quoted content is preserved exactly
- ✅ Verify consecutive whitespace collapsed to single space
- ✅ Verify tabs/newlines converted to spaces
- ✅ Verify leading/trailing whitespace trimmed

**Policy Updates:**
- ✅ Verify all builtin policies use `input.resolved_file_path`
- ✅ Verify policies check `input.is_symlink` where appropriate
- ✅ Verify policies use helper functions for command matching
- ✅ Verify no raw `contains()` for command matching

### 2. OPA/Rego Correctness

**Rego v1 Syntax:**
- ✅ All policies use `import rego.v1` (or omit - default in OPA 1.0+)
- ✅ Decision verbs use `contains` syntax: `deny contains decision if { ... }`
- ✅ Rules use `if` keyword: `rule_name := value if { ... }`
- ✅ Object membership uses `in object.keys(obj)` NOT `"key" in obj`

**Metadata Placement (CRITICAL):**
```rego
# CORRECT - Package metadata must be FIRST in file
# METADATA
# scope: package
# custom: ...
package cupcake.policies.example

# WRONG - Will cause "annotation scope 'package' must be applied to package" error
package cupcake.policies.example
# METADATA
# scope: package
```

**Common Rego Gotchas:**
- Object key membership: `"key" in obj` returns `false` in v1 (silent bug!)
- Use `"key" in object.keys(obj)` instead
- `sprintf()` can be inconsistent in WASM, prefer `concat()`
- Ask decisions MUST have both `reason` and `question` fields

### 3. Test Coverage

**Review that adversarial tests exist for:**
- ✅ Spacing variations: double spaces, tabs, leading/trailing
- ✅ Cross-tool bypasses: Write script, Bash execute
- ✅ Symlink bypasses: direct symlinks, symlink chains, dangling symlinks
- ✅ Path traversal: `../` patterns, relative paths
- ✅ Script inspection: bash scripts, python scripts, interpreter detection

**Inspect test code to validate patterns:**
- ✅ All integration tests use `create_test_project_for_harness()`
- ✅ No tests use `create_test_project()` directly (causes duplicate packages)
- ✅ Tests disable global config: `EngineConfig::global_config(false)`
- ✅ Tests use `include_str!()` for policy embedding where appropriate

**Check Windows compatibility in test code:**
- ✅ Use `std::env::temp_dir()` instead of hardcoded `/tmp/`
- ✅ Use platform-agnostic path operations
- ✅ Symlink tests have proper Windows support (`#[cfg(windows)]`)

### 4. Performance

**No Performance Regressions:**
- ✅ Preprocessing overhead <0.1% of evaluation time
- ✅ No unnecessary clones outside preprocessing
- ✅ Symlink resolution uses lazy evaluation
- ✅ Script inspection is opt-in (performance cost ~1ms)

**Documented Tradeoffs:**
- Clone in `Engine.evaluate()` is intentional (security over micro-optimization)
- Preprocessing overhead is acceptable for universal protection
- Script inspection disabled by default (opt-in for projects that need it)

### 5. Documentation Quality

**Comments Should:**
- ✅ Explain *why* not just *what*
- ✅ Reference TOB findings where applicable (e.g., "TOB-4 defense")
- ✅ Document security rationale for non-obvious decisions
- ✅ Include examples where helpful

**Examples from this branch:**
```rust
// GOOD - Explains why and references security requirement
// IMPORTANT: This clone is intentional and required for security.
// DO NOT OPTIMIZE: The preprocessing defends against adversarial input attacks
// (TOB findings) and must never modify the original input.

// BAD - Just states what the code does
// Clone the input
```

---

## Common Anti-Patterns to Watch For

### 1. String Matching Vulnerabilities

**❌ WRONG - Vulnerable to spacing:**
```rego
contains(input.tool_input.command, "rm -rf")
```

**✅ CORRECT - Uses helper with regex anchoring:**
```rego
import data.cupcake.helpers.commands
commands.has_verb(input.tool_input.command, "rm")
```

### 2. Path Protection Without Canonicalization

**❌ WRONG - Bypassed by symlinks and ../ traversal:**
```rego
file_path := input.tool_input.file_path
startswith(file_path, ".cupcake/")
```

**✅ CORRECT - Uses resolved canonical path:**
```rego
file_path := input.resolved_file_path  # Preprocessing guarantees this exists
startswith(file_path, ".cupcake/")
```

### 3. Incomplete Tool Coverage

**❌ WRONG - Only blocks Bash, can be bypassed via Write:**
```rego
# METADATA
# custom:
#   routing:
#     required_tools: ["Bash"]
```

**✅ CORRECT - Blocks all file modification tools:**
```rego
# METADATA
# custom:
#   routing:
#     required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"]
```

### 4. Object Key Membership (Rego v1)

**❌ WRONG - Silently returns false in Rego v1:**
```rego
"key" in my_object  # Always false!
```

**✅ CORRECT - Use object.keys():**
```rego
"key" in object.keys(my_object)
```

### 5. Preprocessing Bypass Attempts

**❌ WRONG - Preprocessing at CLI level only:**
```rust
// CLI preprocesses, then calls engine
let preprocessed = preprocess(input);
engine.evaluate(&preprocessed)  // FFI can bypass!
```

**✅ CORRECT - Preprocessing inside engine:**
```rust
// Engine preprocesses automatically
pub async fn evaluate(&self, input: &Value) {
    let mut safe_input = input.clone();
    preprocess_input(&mut safe_input, ...);
    // Continue with safe_input
}
```

---

## File-by-File Review Guide

### Core Engine Changes

**cupcake-core/src/engine/mod.rs** (lines 889-904)
- **Purpose:** Integrate preprocessing into evaluation pipeline
- **Check:** Preprocessing happens before routing
- **Check:** Clone is documented and justified
- **Check:** Uses `PreprocessConfig::default()`

**cupcake-core/src/preprocessing/mod.rs**
- **Purpose:** Main preprocessing entry point
- **Check:** Handles both Claude Code and Cursor harnesses
- **Check:** Tool-specific preprocessing logic
- **Check:** Symlink resolution always enabled by default
- **Check:** Script inspection is opt-in

**cupcake-core/src/preprocessing/symlink_resolver.rs**
- **Purpose:** Path canonicalization and symlink detection (TOB-4)
- **Check:** Uses `std::fs::canonicalize()` correctly
- **Check:** Handles dangling symlinks (parent directory fallback)
- **Check:** Cross-platform support (Unix/Windows)
- **Check:** Returns None when path is invalid

**cupcake-core/src/preprocessing/normalizers.rs**
- **Purpose:** Whitespace normalization (TOB-3)
- **Check:** Preserves quoted content exactly
- **Check:** Handles edge cases (empty strings, all spaces)
- **Check:** Trims leading/trailing whitespace
- **Check:** Collapses consecutive whitespace

**cupcake-core/src/preprocessing/script_inspector.rs**
- **Purpose:** Script content inspection (TOB-2)
- **Check:** Detects bash/sh/python/node/ruby scripts
- **Check:** Handles `-c` flag correctly (immediately returns None)
- **Check:** Loads script content when found
- **Check:** Returns None for inline commands

### Policy Changes

**fixtures/claude/builtins/protected_paths.rego**
- **Purpose:** Protect user-configured paths from modification
- **Check:** Uses `input.resolved_file_path` for single-file tools
- **Check:** Handles `MultiEdit` specially (array of edits)
- **Check:** Uses command helpers for Bash verb matching
- **Check:** Comprehensive tool coverage in metadata

**fixtures/claude/builtins/rulebook_security_guardrails.rego**
- **Purpose:** Protect `.cupcake/` directory from tampering
- **Check:** Uses `input.resolved_file_path`
- **Check:** Blocks symlink creation to `.cupcake/`
- **Check:** Uses command helpers for verb matching
- **Check:** Blocks all file modification tools

**fixtures/helpers/commands.rego**
- **Purpose:** Secure command parsing helpers
- **Check:** `has_verb()` uses regex word boundaries
- **Check:** `creates_symlink()` detects `ln -s`
- **Check:** `symlink_involves_path()` checks both arguments

### Test Files

**cupcake-core/tests/adversarial_*.rs**
- **Purpose:** Validate defenses against bypass techniques
- **Check:** Comprehensive coverage of attack vectors
- **Check:** Tests both positive (should block) and negative (should allow) cases
- **Check:** Uses `create_test_project_for_harness()`
- **Check:** Verifies error messages are helpful

**cupcake-core/tests/tob4_*.rs**
- **Purpose:** Validate specific TOB-4 finding remediation
- **Check:** Tests direct paths, symlinks, symlink chains
- **Check:** Tests dangling symlinks
- **Check:** Tests path traversal (`../` patterns)
- **Check:** Verifies policies correctly use resolved paths

**cupcake-core/tests/common/mod.rs** (formerly test_helpers.rs)
- **Purpose:** Shared test utilities
- **Check:** `create_test_project_for_harness()` creates only harness-specific dir
- **Check:** No `#[allow(dead_code)]` attributes (all functions should be used)
- **Check:** Proper visibility modifiers (`pub` for shared functions)

### Documentation

**docs/reference/security.md**
- **Purpose:** Security model and TOB fix documentation
- **Check:** Accurate description of vulnerabilities
- **Check:** Clear explanation of fixes
- **Check:** Performance characteristics documented
- **Check:** Testing approach explained

**docs/reference/input-preprocessing.md**
- **Purpose:** Preprocessing architecture and usage guide
- **Check:** Architecture diagram accurate
- **Check:** Examples show correct usage
- **Check:** Performance benchmarks documented
- **Check:** Policy patterns demonstrate best practices

---

## Regression Risk Areas

### High Risk

1. **Breaking existing policies** - All policies must update to use `resolved_file_path`
2. **Preprocessing bugs** - Could break legitimate commands (e.g., preserve quoted spaces)
3. **Cross-platform issues** - Path handling differs on Windows vs Unix
4. **OPA compilation errors** - Metadata changes could break existing policies

### Medium Risk

1. **Performance degradation** - Preprocessing adds overhead (should be <0.1%)
2. **Test flakiness** - Symlink tests can be flaky on some filesystems
3. **Helper library compatibility** - Existing policies using old patterns

### Low Risk

1. **Documentation drift** - Docs must stay in sync with code
2. **Comment accuracy** - Clarifications must be technically correct

---

## Testing Strategy Validation

### Required Test Coverage

**Review that unit tests exist for:**
- ✅ Whitespace normalization edge cases
- ✅ Symlink resolution edge cases
- ✅ Script inspection detection patterns
- ✅ Helper function correctness

**Review that integration tests exist for:**
- ✅ End-to-end preprocessing in engine
- ✅ Policy evaluation with enriched input
- ✅ Harness-specific event handling
- ✅ Multi-tool coverage

**Review that adversarial tests exist for:**
- ✅ Spacing bypass attempts
- ✅ Cross-tool bypass attempts
- ✅ Symlink bypass attempts
- ✅ Path traversal attempts
- ✅ Script execution hiding

### Test Quality Checks

**Inspect test code to ensure:**
- ✅ Tests use descriptive names explaining what they validate
- ✅ Tests have clear comments explaining attack vectors
- ✅ Tests verify both blocking behavior AND error messages
- ✅ Tests clean up resources (temp directories, symlinks)
- ✅ Tests use `--features deterministic-tests` in documentation/CI config

---

## Final Checklist for Reviewers

### Security Review

- [ ] Review code paths to ensure no preprocessing bypass exists
- [ ] Verify all builtin policies use canonical paths (`resolved_file_path`)
- [ ] Verify all builtin policies use command helpers
- [ ] Check that symlink detection covers all file tools
- [ ] Confirm path traversal patterns are canonicalized
- [ ] Check that quoted content is preserved in normalization

### Code Quality Review

- [ ] Check that comments explain security rationale
- [ ] Flag any `#[allow(dead_code)]` without justification
- [ ] Verify all public functions are documented
- [ ] Check that error messages are helpful and actionable
- [ ] Confirm code follows Rust idioms and conventions

### Test Review

- [ ] Review test structure to ensure they use `--features deterministic-tests`
- [ ] Check that tests cover positive and negative cases
- [ ] Verify tests use proper harness-specific helpers
- [ ] Check that tests clean up resources (TempDir, etc.)
- [ ] Confirm test names clearly describe what they validate

### Documentation Review

- [ ] Check if CLAUDE.md updated with new patterns
- [ ] Verify security docs explain TOB fixes
- [ ] Check preprocessing docs show correct usage
- [ ] Verify examples demonstrate secure patterns
- [ ] Confirm performance characteristics documented

### Policy Review

- [ ] Verify all policies use Rego v1 syntax correctly
- [ ] Check metadata placement is correct (package scope first in file)
- [ ] Confirm decision verbs use `contains` syntax
- [ ] Verify ask decisions have `reason` and `question`
- [ ] Check for any raw `contains()` for command matching (security risk)

### Breaking Changes Review

- [ ] Consider existing policies compatibility
- [ ] Check if migration path documented for breaking changes
- [ ] Verify breaking changes clearly communicated
- [ ] Assess if rollback plan exists if issues found

---

## Key Metrics

**Lines Changed:** 8,117 additions, 623 deletions across 53 files

**Test Coverage:**
- Total tests: 182 passing
- New adversarial tests: 5 test suites
- Total test code: 12,880 lines

**Security Fixes:**
- TOB-3 (High): Spacing bypass ✅ Fixed
- TOB-2 (High): Cross-tool script execution ✅ Fixed
- TOB-4 (Critical): Symlink bypass ✅ Fixed
- Documentation clarity ✅ Fixed

**Performance Impact:**
- Preprocessing overhead: ~30-100μs per evaluation
- Percentage of total: <0.1%
- Impact: Negligible

---

## Questions for Code Author

If anything is unclear during review, consider asking:

1. **Preprocessing Architecture:**
   - Why is preprocessing inside the engine vs at CLI level?
   - What happens if preprocessing is called multiple times?
   - How does preprocessing handle invalid/malformed input?

2. **Symlink Handling:**
   - What happens with symlink chains (A → B → C)?
   - How are dangling symlinks handled?
   - Why is the parent directory fallback necessary?

3. **Policy Migration:**
   - Do existing user policies need updates?
   - Is there a migration guide for policy authors?
   - Are there linting tools to catch old patterns?

4. **Testing:**
   - Why is `--features deterministic-tests` required?
   - What would happen without this flag?
   - Are there known flaky tests?

5. **Cross-Platform:**
   - Are there Windows-specific considerations?
   - How is cross-platform testing performed?
   - Are there known platform limitations?

---

## Additional Resources

**Key Documentation Files:**
- `CLAUDE.md` - Project-level guidance for AI assistants
- `docs/reference/security.md` - Security model and audit fixes
- `docs/reference/input-preprocessing.md` - Preprocessing architecture
- `docs/user-guide/architecture/harness-model.md` - Harness-specific design

**Test Reference:**
- `cupcake-core/tests/CLAUDE.md` - Test policy requirements
- `cupcake-core/tests/common/mod.rs` - Shared test utilities

**External References:**
- OPA Rego v1.0 Documentation: https://www.openpolicyagent.org/docs/latest/policy-language/
- Trail of Bits Security Audits: https://www.trailofbits.com/

---

## Review Priorities

**P0 (Critical - Must Review Thoroughly):**
1. Preprocessing integration in engine (security boundary)
2. Symlink resolution correctness (critical vuln fix)
3. Policy updates using canonical paths
4. Adversarial test coverage

**P1 (High - Important to Review):**
1. Command helper functions (bypass prevention)
2. Whitespace normalization correctness
3. Test infrastructure changes
4. Documentation accuracy

**P2 (Medium - Review if Time Permits):**
1. Comment clarifications
2. Code style and conventions
3. Performance optimizations
4. Error message quality

**P3 (Low - Nice to Have):**
1. Documentation formatting
2. Example code style
3. Variable naming consistency

---

**This branch represents a significant security hardening effort addressing findings from a professional security audit. The changes implement defense-in-depth protections that make bypassing policies significantly harder while maintaining performance and usability.**
