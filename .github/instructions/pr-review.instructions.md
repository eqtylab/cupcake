# Cupcake PR Review Guide

## Project Overview

**Cupcake** is a policy enforcement and governance layer for AI coding agents. It intercepts tool calls from AI agents (Claude Code, Cursor) and evaluates them against user-defined policies written in OPA Rego, returning Allow/Block/Warn/Ask decisions before actions execute.

**Core Value Proposition:**
- Deterministic rule-following for AI agents
- Early-warning alerts when agents violate rules repeatedly
- Performance boost by moving rules out of context into guaranteed enforcement layer

**Architecture Pattern:** Hybrid Model
- **Rego (WASM):** Declares policies, evaluates rules, returns decision verbs
- **Rust (Engine):** Routes events, gathers signals, synthesizes decisions

---

## High-Level Architecture

### Core Flow

```
AI Agent → Hook Event → Cupcake Engine → Policy Evaluation → Decision → AI Agent
                            ↓
                    1. Preprocess (normalize input)
                    2. Route (O(1) metadata lookup for signal gating)
                    3. Early Exit if no policies match
                    4. Gather Signals (shell commands)
                    5. Evaluate (ALL policies via WASM)
                    6. Synthesize (apply priority)
                    7. Format Response
```

### Key Components

**Engine** (`cupcake-core/src/engine/`)
- Scanner: Discovers policy files
- Metadata Parser: Extracts routing requirements
- Router: O(1) signal gating and early exit (does NOT control which Rego rules execute)
- Compiler: Converts Rego → WASM (single entrypoint, all policies)
- Evaluator: Executes WASM in sandbox (all compiled policies run via `walk()`)
- Synthesis: Applies decision priority hierarchy

**Preprocessing** (`cupcake-core/src/preprocessing/`)
- Whitespace normalization (defense against spacing bypasses)
- Symlink resolution (defense against path traversal)
- Script inspection (defense against hidden commands in scripts)
- Automatic, always-on defense-in-depth

**Harnesses** (`cupcake-core/src/harness/`)
- Claude Code: PreToolUse, PostToolUse, UserPromptSubmit events
- Cursor: beforeShellExecution, beforeFileEdit, etc.
- Native event formats (no normalization layer)
- Harness-specific response builders

**Policies** (`.cupcake/policies/{claude,cursor}/`)
- OPA Rego v1.0 syntax
- Compiled to WASM for sandboxed execution
- Metadata-driven routing
- Decision verbs: halt, deny, block, ask, add_context

---

## Code Organization

### Directory Structure

```
cupcake-rewrite/
├── cupcake-cli/          # CLI binary (main.rs)
│   ├── src/
│   └── tests/            # CLI integration tests
├── cupcake-core/         # Core library
│   ├── src/
│   │   ├── engine/       # Policy engine
│   │   ├── harness/      # Harness-specific logic
│   │   ├── preprocessing/ # Input normalization
│   │   └── lib.rs
│   └── tests/            # Integration tests
├── fixtures/             # Builtin policies and templates
│   ├── claude/
│   │   └── builtins/     # Claude Code builtin policies
│   ├── cursor/
│   │   └── builtins/     # Cursor builtin policies
│   ├── helpers/          # Shared Rego helpers
│   ├── init/             # Init command templates
│   └── global_builtins/  # Global (org-level) policies
└── docs/                 # User and developer documentation
    ├── reference/
    ├── user-guide/
    └── development/
```

### File Naming Conventions

- **Rust:** `snake_case.rs`
- **Rego:** `snake_case.rego`
- **Config:** `kebab-case.yml`, `kebab-case.json`
- **Tests:** `{feature}_test.rs` or `test_{feature}.rs`

---

## General PR Review Guidelines

### 1. Purpose and Scope

**Check:**
- [ ] PR has clear title describing what it does
- [ ] Description explains *why* the change is needed
- [ ] Scope is focused (one logical change)
- [ ] Breaking changes are clearly marked
- [ ] Related issues/discussions are linked

**Questions to ask:**
- Does this PR try to do too much?
- Is the motivation clear?
- Are there alternative approaches worth discussing?

### 2. Code Quality

**Rust Code:**
- [ ] Follows Rust idioms and conventions
- [ ] Uses appropriate error handling (Result, anyhow)
- [ ] Avoids unnecessary allocations/clones
- [ ] Uses appropriate visibility modifiers (pub, pub(crate), private)
- [ ] Includes doc comments for public items
- [ ] Passes `cargo clippy` with no warnings
- [ ] Formatted with `cargo fmt`

**Rego Code:**
- [ ] Uses Rego v1.0 syntax (`import rego.v1`)
- [ ] Metadata is properly placed (package metadata first in file)
- [ ] Decision verbs use `contains` syntax
- [ ] Uses helpers from `data.cupcake.helpers` for security-sensitive operations
- [ ] Passes `opa fmt --diff` with no changes needed

**General:**
- [ ] Names are descriptive and consistent
- [ ] Comments explain *why* not *what*
- [ ] No commented-out code (unless with clear justification)
- [ ] No debug print statements left in
- [ ] TODOs have context and tracking issues

### 3. Security Considerations

**Input Validation:**
- [ ] User input is validated and sanitized
- [ ] File paths use canonical/resolved paths from preprocessing
- [ ] Command matching uses helper functions (not raw `contains()`)
- [ ] No ambient authority (env vars for security config)

**Path Handling:**
- [ ] Use `input.resolved_file_path` for file operations
- [ ] Check `input.is_symlink` for sensitive operations
- [ ] Handle path traversal (`../`) correctly
- [ ] Cross-platform compatible (Windows, Unix, macOS)

**Command Parsing:**
- [ ] Use `data.cupcake.helpers.commands` for verb matching
- [ ] Handle quoted arguments correctly
- [ ] Whitespace normalization is applied (automatic via preprocessing)

**Policy Coverage:**
- [ ] All relevant tools covered in metadata `required_tools`
- [ ] No bypass paths (consider cross-tool attacks)
- [ ] Symlink operations are blocked where appropriate

### 4. Testing

**Review test coverage:**
- [ ] Check that unit tests exist for new functions/modules
- [ ] Check that integration tests exist for new features
- [ ] Verify test code uses `--features deterministic-tests` pattern
- [ ] Review test code to ensure tests are deterministic (no race conditions)
- [ ] Check that tests clean up resources (temp files, directories)

**Inspect test quality:**
- [ ] Verify descriptive test names (what they validate)
- [ ] Check that tests cover both success and failure cases
- [ ] Check that edge cases are covered
- [ ] Verify error messages are validated in assertions
- [ ] Check tests use `create_test_project_for_harness()` for harness-specific tests

**Coverage expectations (review test files):**
- New Rust code: Look for >80% path coverage in test code
- New Rego policies: Check that all decision paths have tests
- Security-critical code: Expect 100% coverage (review thoroughly)

### 5. Documentation

**Code Documentation:**
- [ ] Public functions have doc comments
- [ ] Complex logic has inline comments
- [ ] Security rationale is documented
- [ ] Examples are provided where helpful

**User Documentation:**
- [ ] New features documented in `docs/user-guide/`
- [ ] Breaking changes noted in docs
- [ ] Examples show correct usage patterns
- [ ] Migration guides for breaking changes

**Developer Documentation:**
- [ ] Architecture changes documented in `docs/reference/`
- [ ] Design decisions explained
- [ ] CLAUDE.md updated if relevant

### 6. Performance

**No Regressions:**
- [ ] No unnecessary allocations in hot paths
- [ ] Clones are justified (see preprocessing for security example)
- [ ] Async operations don't block unnecessarily
- [ ] Signal execution is parallel where possible

**Benchmarks:**
- [ ] Performance-critical code has benchmarks
- [ ] Regressions are identified and justified
- [ ] Preprocessing overhead remains <0.1%

### 7. Compatibility

**Backward Compatibility:**
- [ ] Existing policies continue to work
- [ ] Config format changes are backward compatible
- [ ] CLI flag changes don't break existing scripts

**Cross-Platform:**
- [ ] Code has platform-specific handling where needed (`#[cfg(windows)]`, `#[cfg(unix)]`)
- [ ] Platform-agnostic paths used (`std::env::temp_dir()`, not `/tmp/`)
- [ ] Windows path escaping for JSON/Git Bash if applicable

**Harness Compatibility:**
- [ ] Changes work for both Claude Code and Cursor
- [ ] Harness-specific logic is properly isolated
- [ ] Native event formats preserved (no normalization)

---

## Common Patterns to Follow

### 1. Rust Error Handling

**✅ GOOD - Use Result and anyhow:**
```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .context("Failed to read config file")?;

    serde_json::from_str(&content)
        .context("Failed to parse config JSON")
}
```

**❌ BAD - Unwrap in library code:**
```rust
fn load_config(path: &Path) -> Config {
    let content = fs::read_to_string(path).unwrap();  // Will panic!
    serde_json::from_str(&content).unwrap()
}
```

### 2. Rego Policy Structure

**✅ GOOD - Proper metadata and structure:**
```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.example

import data.cupcake.helpers.commands
import rego.v1

deny contains decision if {
    commands.has_verb(input.tool_input.command, "rm")
    decision := {
        "rule_id": "BLOCK-RM",
        "reason": "Dangerous rm command",
        "severity": "HIGH"
    }
}
```

**❌ BAD - Missing metadata, unsafe matching:**
```rego
package cupcake.policies.example

# No metadata - won't be routed!

deny[decision] {  # Old syntax
    contains(input.tool_input.command, "rm -rf")  # Spacing bypass!
    decision := {"reason": "bad"}  # Missing fields
}
```

### 3. File Path Handling

**✅ GOOD - Use preprocessed canonical paths:**
```rust
// Preprocessing happens automatically in engine
let file_path = event.get("resolved_file_path")
    .and_then(|v| v.as_str())
    .context("Missing resolved_file_path")?;

// Check if it's a symlink
let is_symlink = event.get("is_symlink")
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
```

```rego
# In policy
file_path := input.resolved_file_path
is_symlink := input.is_symlink
```

**❌ BAD - Use raw file paths:**
```rust
// Vulnerable to symlinks and path traversal!
let file_path = event["tool_input"]["file_path"].as_str()?;
```

### 4. Test Structure

**✅ GOOD - Clear, focused test:**
```rust
#[tokio::test]
async fn test_symlink_bypass_is_blocked() -> Result<()> {
    // Setup
    let temp_dir = TempDir::new()?;
    let project_dir = create_test_project_for_harness(
        temp_dir.path(),
        HarnessType::ClaudeCode
    )?;

    // Create symlink to protected path
    let protected = project_dir.join(".cupcake/rulebook.yml");
    let symlink = temp_dir.join("innocent.yml");
    symlink_file(&protected, &symlink)?;

    // Test
    let engine = Engine::new(&project_dir, HarnessType::ClaudeCode).await?;
    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {"file_path": symlink.to_str().unwrap()},
        "cwd": temp_dir.path().to_str().unwrap()
    });

    let decision = engine.evaluate(&event, None).await?;

    // Assert
    assert!(matches!(decision, FinalDecision::Halt { .. }));

    Ok(())
}
```

**❌ BAD - Unclear, multiple assertions:**
```rust
#[test]
fn test_stuff() {
    let engine = Engine::new(...).unwrap();  // No context on error
    let result = engine.evaluate(...).unwrap();
    assert!(result.is_some());  // What are we testing?
    // No cleanup!
}
```

---

## Rego-Specific Guidelines

### Metadata Placement (CRITICAL)

**✅ CORRECT - Package metadata FIRST in file:**
```rego
# METADATA
# scope: package
# custom:
#   routing: {...}
package cupcake.policies.example

# Rest of policy...
```

**❌ WRONG - Package declared before metadata:**
```rego
package cupcake.policies.example

# METADATA  <-- ERROR: too late!
# scope: package
```

### Decision Verb Syntax

**Modern v1 syntax:**
```rego
deny contains decision if { ... }
halt contains decision if { ... }
ask contains decision if { ... }
add_context contains decision if { ... }
```

**Legacy syntax (still works but discouraged):**
```rego
deny[decision] { ... }
```

### Object Key Membership (Common Bug!)

**✅ CORRECT:**
```rego
# Check if key exists in object
"key" in object.keys(my_object)

# Iterate over key-value pairs
some key, value in my_object
```

**❌ WRONG - Silently returns false in Rego v1:**
```rego
"key" in my_object  # ALWAYS FALSE!
```

### Helper Function Usage

**Security-critical operations MUST use helpers:**

```rego
import data.cupcake.helpers.commands
import data.cupcake.helpers.paths

# Command matching
commands.has_verb(cmd, "rm")
commands.has_dangerous_verb(cmd, {"rm", "dd", "mkfs"})

# Path checking
paths.targets_protected(file_path, ".cupcake/")
```

---

## Testing Requirements

### Test Execution Pattern

**Check that tests document the correct execution pattern:**
```bash
cargo test --features deterministic-tests

# Or use the alias
cargo t
```

**Why this matters:** The trust system uses HMAC key derivation which is non-deterministic in production mode. Without this flag, tests will fail intermittently. When reviewing, look for:
- Test documentation mentions this requirement (in comments or README)
- Test code structure suggests deterministic execution
- No randomness or timing dependencies in test logic

### Test Categories

**Unit Tests** (`src/*.rs` with `#[cfg(test)]`)
- Test individual functions in isolation
- Fast, no I/O when possible
- Use `#[test]` for sync, `#[tokio::test]` for async

**Integration Tests** (`tests/*.rs`)
- Test full workflows end-to-end
- Use real file system, temp directories
- Clean up resources (use `TempDir`)

**Adversarial Tests** (`tests/adversarial_*.rs`)
- Validate security defenses
- Test bypass attempts
- Document attack vectors in comments

### Test Helpers

**Use harness-specific helpers:**
```rust
// CORRECT - Creates only claude/ directory
create_test_project_for_harness(dir, HarnessType::ClaudeCode)?;

// WRONG - Creates both claude/ and cursor/ (duplicate packages!)
create_test_project(dir)?;
```

**Disable global config in tests:**
```rust
let config = EngineConfig {
    global_config: false,  // Prevent interference from ~/.config/cupcake
    ..Default::default()
};
```

### Test Patterns

**Fixture Policies:**
```rust
// Embed policy at compile time
const POLICY: &str = include_str!("fixtures/test_policy.rego");

// Or use authoritative system evaluate
const SYSTEM_EVAL: &str = include_str!("fixtures/system_evaluate.rego");
```

**Policy changes require recompilation:**
```bash
# If policy changes don't appear in tests
cargo clean -p cupcake-core
cargo test --features deterministic-tests
```

---

## Performance Considerations

### Hot Paths (Optimize)

1. **Signal gating lookup** - O(1) HashMap lookups (determines which signals to run)
2. **Policy evaluation** - WASM execution (all compiled policies run via `walk()`)
3. **Signal gathering** - Parallel shell command execution

**In these paths:**
- Minimize allocations
- Use references where possible
- Clone only when necessary
- Profile before optimizing

### Cold Paths (Prefer Clarity)

1. **Engine initialization** - Once per session
2. **Policy compilation** - Once at startup
3. **Config loading** - Infrequent

**In these paths:**
- Clarity over performance
- Helpful error messages
- Validation and safety checks

### Clone Justification

**When to clone:**
- Security requirements (preprocessing must not modify original)
- Ownership needed across async boundaries
- Simplifies code significantly

**Document in comments:**
```rust
// IMPORTANT: This clone is intentional and required for security.
// DO NOT OPTIMIZE: The preprocessing defends against adversarial attacks
// and must never modify the original input.
let mut safe_input = input.clone();
```

---

## Documentation Standards

### Rust Doc Comments

**Public items require documentation:**
```rust
/// Preprocesses input JSON to normalize adversarial patterns
///
/// This is the main entry point for input preprocessing. It:
/// 1. Identifies tool-specific fields that need normalization
/// 2. Applies appropriate normalizers based on configuration
/// 3. Logs all transformations for auditability
///
/// # Arguments
/// * `input` - Mutable reference to the input JSON
/// * `config` - Configuration controlling what normalizations to apply
///
/// # Example
/// ```
/// use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
/// let mut input = json!({"command": "rm  -rf"});
/// preprocess_input(&mut input, &PreprocessConfig::default());
/// ```
pub fn preprocess_input(input: &mut Value, config: &PreprocessConfig) { ... }
```

### Inline Comments

**Explain why, not what:**
```rust
// GOOD
// Check if path is a symlink to prevent bypass attacks (TOB-4 defense)
if is_symlink { ... }

// BAD
// Check if symlink
if is_symlink { ... }
```

### CLAUDE.md Updates

**Update when:**
- New architectural patterns introduced
- Security principles change
- Testing requirements evolve
- Common pitfalls identified

---

## Common Anti-Patterns to Avoid

### 1. Environment Variable Config for Security

**❌ WRONG:**
```rust
let max_memory = env::var("CUPCAKE_WASM_MAX_MEMORY")
    .unwrap_or("10MB");
```

**✅ CORRECT:**
```rust
// Use CLI flags for security-critical config
#[derive(Parser)]
struct Args {
    #[clap(long, default_value = "10MB")]
    wasm_max_memory: String,
}
```

**Why:** AI agents can manipulate environment variables via prompts.

### 2. String Contains for Command Matching

**❌ WRONG:**
```rego
contains(input.tool_input.command, "rm -rf")
```

**✅ CORRECT:**
```rego
import data.cupcake.helpers.commands
commands.has_verb(input.tool_input.command, "rm")
contains(input.tool_input.command, "-rf")
```

**Why:** Spacing variations bypass exact string matching.

### 3. Ignoring Preprocessed Paths

**❌ WRONG:**
```rego
file_path := input.tool_input.file_path  # Raw path
```

**✅ CORRECT:**
```rego
file_path := input.resolved_file_path  # Canonical, symlinks resolved
```

**Why:** Symlinks and `../` can bypass protection.

### 4. Using #[allow(dead_code)]

**❌ WRONG:**
```rust
#[allow(dead_code)]
fn helper_function() { ... }
```

**✅ CORRECT:**
```rust
// Either use the function or remove it
// If truly needed for future use, document why:
/// Used by integration tests via include_str!
fn helper_function() { ... }
```

**Why:** Dead code indicates incomplete refactoring or poor design.

### 5. Unwrap in Library Code

**❌ WRONG:**
```rust
pub fn process(input: &str) -> Result<Output> {
    let parsed = serde_json::from_str(input).unwrap();  // Panic!
    Ok(Output { data: parsed })
}
```

**✅ CORRECT:**
```rust
pub fn process(input: &str) -> Result<Output> {
    let parsed = serde_json::from_str(input)
        .context("Failed to parse input JSON")?;
    Ok(Output { data: parsed })
}
```

**Why:** Libraries should never panic. Return errors to callers.

---

## PR Review Checklist

### Before Submitting PR (Author Checklist)

**Authors should have:**
- [ ] Verified tests locally with `--features deterministic-tests`
- [ ] Formatted code with `cargo fmt`
- [ ] Addressed warnings from `cargo clippy`
- [ ] Updated documentation
- [ ] Updated CLAUDE.md if patterns changed
- [ ] Written clear and descriptive commit messages
- [ ] Explained what and why in PR description

### During Review

**Review for correctness:**
- [ ] Logic is sound and achieves stated goal
- [ ] Edge cases are handled in code
- [ ] Error handling is appropriate
- [ ] Code analysis shows no race conditions or deadlocks

**Review for security:**
- [ ] Input validation is present
- [ ] Path handling uses canonical paths
- [ ] Command matching uses helpers
- [ ] No ambient authority (env vars for security config)

**Review test structure:**
- [ ] Test code covers new functionality
- [ ] Test code appears deterministic (no sleeps, randomness)
- [ ] Test code cleans up resources (TempDir, etc.)
- [ ] Test names are descriptive

**Review for performance:**
- [ ] No obvious performance regressions in code
- [ ] Clones are justified in comments
- [ ] Hot paths remain optimized

**Review documentation:**
- [ ] Public API has doc comments
- [ ] Complex logic is explained
- [ ] Examples provided where helpful
- [ ] Migration guide exists for breaking changes

**Review code quality:**
- [ ] Names are clear and consistent
- [ ] Comments explain why not what
- [ ] No commented-out code
- [ ] Follows Rust/Rego conventions

### Before Approval

**Final code review checks:**
- [ ] All previous review comments addressed in code
- [ ] Code changes are complete and logical
- [ ] No obvious issues remaining
- [ ] Documentation changes included in same PR

---

## Rust-Specific Conventions

### Naming

- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

### Visibility

- Default to private
- Use `pub` only for external API
- Use `pub(crate)` for internal API
- Use `pub(super)` for parent module access

### Error Handling

- Library functions return `Result<T>`
- Use `anyhow::Result` for applications
- Use `thiserror` for library error types
- Provide context with `.context()`

### Async

- Use `tokio` runtime
- Mark async functions with `async fn`
- Use `.await` for async operations
- Spawn tasks with `tokio::spawn` for parallelism

---

## Key Metrics and Expectations

### Performance Targets

- Preprocessing: <100μs per evaluation
- Routing: <1μs (O(1) lookup)
- Policy evaluation: 10-100ms (WASM)
- Signal gathering: <1s total (parallel execution)
- End-to-end: <200ms typical

### Test Coverage

- Core engine: >90%
- Preprocessing: 100% (security critical)
- Policies: All decision paths
- Integration: Happy path + error cases

### Code Quality

- Clippy warnings: 0
- Format deviations: 0
- Dead code: 0 (without #[allow])
- Public items without docs: 0

---

## Getting Help

**Documentation:**
- Architecture: `docs/reference/architecture.md`
- Security: `docs/reference/security.md`
- User Guide: `docs/user-guide/`
- CLAUDE.md: Project-level AI guidance

**Ask Questions:**
- Unclear code? Ask for clarification
- Security concerns? Flag for discussion
- Better approach? Suggest alternatives
- Breaking change? Discuss migration plan

**Review Culture:**
- Reviews are about code quality, not developers
- Constructive feedback with examples
- Approve when confident, request changes when not
- Learn from each other

---

## Summary

**Cupcake is a security-critical policy enforcement engine.** Every PR should:

1. **Maintain security** - No bypass paths, proper validation
2. **Preserve performance** - No regressions, optimize hot paths
3. **Ensure quality** - Tests, docs, clear code
4. **Follow conventions** - Rust idioms, Rego patterns
5. **Communicate clearly** - Why changes matter, what they affect

**When in doubt:** Ask questions, suggest improvements, and err on the side of security and clarity over cleverness.
