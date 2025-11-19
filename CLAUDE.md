# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Security Principles

Cupcake enforces defense-in-depth security principles based on comprehensive security audits:

1. **No Ambient Authority**: Configuration via explicit CLI flags only, never environment variables
2. **Defense-in-Depth**: Validate security-critical values at parse time and runtime
3. **Explicit Consent**: Debug output requires explicit CLI flags from user
4. **Fail-Safe Defaults**: All security limits enforced with safe minimums (e.g., 1MB WASM memory)
5. **Input Preprocessing**: Automatic normalization at Rust level protects all policies from spacing bypasses

**Rationale**: AI agents can manipulate environment variables through prompts. Explicit CLI flags create an audit trail and prevent bypass attacks.

**Preprocessing Defense**: See `SECURITY_PREPROCESSING.md` for details on how Rust-level preprocessing automatically protects all policies from adversarial spacing patterns.

# Cupcake

Cupcake is a policy engine for AI coding agents. It works by intercepting tool calls from AI
coding agents and evaluating them against user-defined policies written in Open Policy
Agent (OPA) Rego, returning Allow, Block, or Warn decisions. The system integrates with
Claude Code through a hooks mechanism that captures actions like shell commands or file
edits before execution. It compiles policies to WebAssembly (Wasm) for fast evaluation in a
sandboxed environment. Cupcake stores its configuration and trust data in a .cupcake
directory and uses signals to gather contextual information such as Git branch status or file
contents during policy evaluation. Users write policies that can block specific commands,
protect directories, enforce workflow requirements, or inject behavioral guidance prompts
back to the agent.

## Testing with Claude Code

You can use the claude code cli functionality to test Cupcake behavior locally. Configure hooks with `--debug-routing` flag in `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --debug-routing"
          }
        ]
      }
    ]
  }
}
```

Then run: `claude -p "hello world" --model haiku` to create routing map in `.cupcake/debug/`

## Critical Claude Code Hook Integration Requirements

### JSON Response Format Requirements

**CRITICAL**: Claude Code hooks require ONLY valid JSON on stdout. Any other output will cause parsing failure and the hook response will be ignored.

**Key Requirements**:

1. **Field Name Casing**: The response field MUST be `hookSpecificOutput` (camelCase), not `hook_specific_output` (snake_case). See `cupcake-core/src/harness/response/types.rs`.

2. **Stdout Pollution**: ALL logs, debug output, and banners MUST go to stderr, not stdout. Only the JSON response should go to stdout. The tracing subscribers in `cupcake-cli/src/main.rs` use `.with_writer(std::io::stderr)`.

Without proper JSON formatting, Claude Code will ignore deny decisions and execute dangerous commands despite Cupcake returning a proper deny response.

## Hook Event Format Requirements

### Required Common Fields

All Claude Code events MUST include:

```json
{
  "hook_event_name": "EventType",
  "session_id": "string",
  "transcript_path": "/path/to/transcript",
  "cwd": "/working/directory"
}
```

### Event-Specific Fields

- **PreToolUse**: `tool_name`, `tool_input`
- **PostToolUse**: `tool_name`, `tool_input`, `tool_response`
- **UserPromptSubmit**: `prompt`
- **SessionStart**: `source` (Startup/Resume/Clear)

### Context Injection Support

Only these events support `additionalContext`:

- `UserPromptSubmit` - via `hookSpecificOutput.additionalContext`
- `SessionStart` - via `hookSpecificOutput.additionalContext`
- `PreCompact` - joins output with `\n\n` (double newline)

**Note**: PreToolUse does NOT support context injection.

### Response Formats by Event

- **Tool events** (Pre/PostToolUse): `continue: false` with `stopReason` for blocking
- **UserPromptSubmit**: `decision: "block"` OR `hookSpecificOutput` for context
- **Ask verb**: Only works on tool events, ignored on prompt events

## Dependencies and Requirements

### Required Tools

- **OPA (Open Policy Agent)**: v1.71.0 or later (for v1.0 Rego syntax support)
- **Rust**: 1.75.0 or later (edition 2021)
- **Cargo**: Latest stable

### Core Dependencies

- **wasmtime**: WebAssembly runtime for executing compiled policies
- **tokio**: Async runtime with multi-threading
- **serde/serde_json**: JSON serialization/deserialization
- **OPA v1.0 Rego**: Modern syntax with `import rego.v1`

## Workspace Structure

Cupcake uses a Cargo workspace with multiple crates:

- **cupcake-core**: Core engine library (routing, signals, WASM runtime, synthesis)
- **cupcake-cli**: Command-line interface binary
- **cupcake-py**: Python bindings (optional, requires `maturin`)
- **cupcake-ts**: TypeScript/Node.js bindings (optional, requires `NAPI-RS`)

The workspace is configured to build only `cupcake-core` and `cupcake-cli` by default. Language bindings (Python, TypeScript) require separate build steps.

## Build and Development Commands

### Using Just (Recommended)

The project includes a comprehensive `justfile` with common development tasks:

```bash
# Show all available commands
just

# Build commands
just build              # Build workspace in release mode
just build-debug        # Build in debug mode (faster)
just build-core         # Build only cupcake-core
just build-cli          # Build only cupcake-cli
just install            # Install cupcake binary to ~/.cargo/bin/

# Test commands (REQUIRED: Must use deterministic-tests feature)
just test               # Run all Rust tests with deterministic-tests feature
just test-unit          # Run only unit tests
just test-integration   # Run only integration tests
just test-one TEST_NAME # Run a specific test by name
just test-core          # Test only cupcake-core
just test-cli           # Test only cupcake-cli

# Development commands
just check              # Check code without building
just fmt                # Format all code
just lint               # Run clippy linter
just fix                # Auto-fix common issues

# Benchmarks and performance
just bench              # Run benchmarks
just perf-test          # Run performance validation tests

# Utilities
just stats              # Show project statistics
just test-log           # View recent test results
just watch              # Watch for changes and rebuild
just watch-test         # Watch and run tests on change
```

### Using Cargo Directly

```bash
# Build the project
cargo build --release

# Run tests (REQUIRED: Must use deterministic-tests feature for correct test behavior)
cargo test --workspace --features cupcake-core/deterministic-tests

# Run a specific test
cargo test test_name --workspace --features cupcake-core/deterministic-tests

# Run benchmarks
cargo bench -p cupcake-core

# Run with debug logging
cargo run -p cupcake-cli -- eval --log-level debug [args]

# Create a new release (pushes tag to trigger automated workflow)
git tag v[VERSION] && git push origin v[VERSION]

# Enable extensive debugging with policy evaluation tracing
cargo run -- eval --trace eval [args]      # Shows the main policy evaluation pipeline (routing, signals, WASM, synthesis)
cargo test --workspace --features cupcake-core/deterministic-tests --trace all  # Shows everything (all engine components plus lower-level details)

# Compile OPA policies to WASM (from project root)
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/

# Run cupcake with policies
cargo run -- eval --policy-dir .cupcake/policies
```

### Python Bindings (Optional)

```bash
# Setup Python virtual environment
just venv

# Build and install Python bindings locally
just develop-python

# Run Python tests
just test-python

# Build Python wheel
just build-python
```

### TypeScript Bindings (Optional)

```bash
# Navigate to TypeScript bindings directory
cd cupcake-ts

# Install dependencies
npm install

# Build native module (release mode)
npm run build

# Build in debug mode (faster for development)
npm run build:debug

# Run tests
npm test

# Format code
npm run format
```

## Architecture Overview

Cupcake implements a **Hybrid Model**:

- **Rego (WASM)**: Declares policies, evaluates rules, aggregates decision verbs
- **Rust (Engine)**: Routes events, gathers signals, synthesizes final decisions

### Core Flow

```
Event Input → Route (O(1) lookup) → Gather Signals → Evaluate (WASM) → Synthesize → Response
```

### Key Architectural Principles

1. **Intelligence in the Engine**: The Rust engine handles routing, synthesis, and optimization. Policies focus purely on business logic.

2. **Metadata-Driven Routing**: Policies declare their requirements via OPA metadata, not code:

   ```yaml
   # METADATA
   # custom:
   #   routing:
   #     required_events: ["PreToolUse"]
   #     required_tools: ["Bash"]
   ```

3. **Decision Verbs**: Modern Rego v1 syntax with set-based verbs:

   ```rego
   deny contains decision if { ... }
   halt contains decision if { ... }
   ```

4. **Single Aggregation Entrypoint**: All policies are evaluated through `cupcake.system.evaluate` which uses `walk()` for automatic policy discovery.

## Critical Implementation Details

### Test Execution Requirements

**IMPORTANT**: Tests MUST be run with:

1. The `--features deterministic-tests` flag for deterministic HMAC key generation
2. Tests explicitly disable global config via `EngineConfig::global_config` to prevent interference

```bash
# Correct way to run tests
cargo test --features deterministic-tests

# Or use the Just commands
just test
```

Without the `deterministic-tests` feature, tests will fail due to non-deterministic key derivation in the trust system.

### Test Policy Requirements

When tests use `include_str!` to embed policy files at compile time, changes to those policies require recompilation. Run `cargo clean -p cupcake-core` if policy changes aren't being picked up in tests.

### Harness-Specific Testing Requirements

**CRITICAL**: When testing a specific harness (Claude Code or Cursor), you MUST use `create_test_project_for_harness()` instead of `create_test_project()`.

```rust
// WRONG - Creates both claude/ and cursor/ directories causing duplicate package errors
test_helpers::create_test_project(project_dir.path())?;

// CORRECT - Creates only the directory for the specific harness
test_helpers::create_test_project_for_harness(
    project_dir.path(),
    HarnessType::Cursor
)?;
```

**Why this matters**: The compiler copies all policies preserving their relative paths from `.cupcake/policies/`. If both `claude/` and `cursor/` directories exist with files like `minimal.rego` and `system/evaluate.rego`, OPA compilation fails with "package annotation redeclared" errors because both harness directories get compiled together despite the engine only loading from one.

### Field Name Mismatch

Claude Code sends events with `hook_event_name` (snake_case) but expects responses with `hookEventName` (camelCase). The engine accepts both formats for compatibility.

### Policy Trust Model

Policies trust the engine's routing - they don't need to verify event types or tool names. If a policy is evaluating, its routing requirements are already met.

### Decision Priority

The synthesis layer enforces strict priority: Halt > Deny/Block > Ask > Allow

### Two-Phase Evaluation Model

Cupcake evaluates policies in two phases for optimal security and performance:

**Phase 1: Global Policies** (from `~/.cupcake/rulebook.yml`)
- Evaluated first with early termination on Halt/Deny/Block
- Provides organization-wide governance that cannot be overridden by project configs
- Uses separate WASM module with namespace `cupcake.global.policies.builtins.*`
- If Phase 1 blocks, Phase 2 never executes

**Phase 2: Project Policies** (from `.cupcake/rulebook.yml`)
- Evaluated only if Phase 1 allows or adds context
- Project-specific rules and customizations
- Uses namespace `cupcake.policies.*`

This two-phase model enables centralized security controls while allowing project flexibility.

## Module Organization

### cupcake-core/src/

- **engine/**: Core policy evaluation engine
  - `mod.rs` - Main engine with routing and evaluation orchestration
  - `metadata.rs` - OPA metadata parser for routing declarations
  - `routing.rs` - O(1) policy routing based on event/tool metadata
  - `synthesis.rs` - Decision synthesis layer (Halt > Deny > Ask > Allow priority)
  - `compiler.rs` - OPA policy compilation to WASM
  - `wasm_runtime.rs` - Wasmtime execution environment
  - `builtins.rs` - Builtin policy abstractions configuration
  - `rulebook.rs` - Configuration file parsing (.cupcake/rulebook.yml)
  - `global_config.rs` - Global (user-level) configuration discovery
  - `decision.rs` - Decision types and validation
  - `trace.rs` - Evaluation tracing for debugging

- **harness/**: Harness-specific integrations
  - `events/claude_code/` - Claude Code event parsers (PreToolUse, PostToolUse, UserPromptSubmit, etc.)
  - `events/cursor/` - Cursor event parsers (BeforeShellExecution, AfterFileEdit, etc.)
  - `response/claude_code/` - Claude Code response formatters
  - `response/cursor/` - Cursor response formatters
  - `types.rs` - Common harness types and enums

- **preprocessing/**: Input normalization and security
  - `normalizers.rs` - Text normalization (whitespace, unicode) to prevent bypass attacks
  - `script_inspector.rs` - Shell script content extraction from `-c` arguments
  - `symlink_resolver.rs` - Symlink resolution for file path security
  - `config.rs` - Preprocessing configuration

- **trust/**: Policy integrity verification
  - `mod.rs` - Trust system orchestration
  - `manifest.rs` - Policy manifest generation and verification
  - `hasher.rs` - Content hashing with HMAC
  - `verifier.rs` - Trust verification logic

- **validator/**: Decision and event validation
  - `decision_event_matrix.rs` - Matrix of valid decision types per event
  - `rules.rs` - Validation rules for decisions
  - `mod.rs` - Validation orchestration

- **debug/**: Debug output and routing visualization
  - `mod.rs` - Debug file generation for `.cupcake/debug/`

### Key Files

- `cupcake-core/tests/fixtures/system_evaluate.rego` - Reference system aggregation entrypoint
- `fixtures/init/base-config.yml` - Template for builtin configuration
- `justfile` - Development task runner with all common commands

## Language Bindings

Cupcake supports embedding in multiple languages through the `BindingEngine` abstraction:

### Python Bindings (`cupcake-py/`)
- **Build System**: PyO3 with maturin
- **Package**: `cupcake` on PyPI
- **Key Feature**: GIL release during evaluation for true Python concurrency
- **API**: Both sync and async (via `asyncio.to_thread`)
- **Use Case**: Embed Cupcake in Python applications, agents, automation

### TypeScript Bindings (`cupcake-ts/`)
- **Build System**: NAPI-RS with cross-platform pre-built binaries
- **Package**: `@eqtylab/cupcake` on NPM
- **Key Feature**: Native async/await integration, non-blocking evaluation
- **API**: Async-first with sync alternatives
- **Use Case**: Embed in Node.js apps, Vercel AI SDK agents, custom tools
- **OPA Management**: Auto-downloads and verifies OPA binary (SHA256)

### Architecture

Both bindings wrap `cupcake_core::bindings::BindingEngine`:
- **Thread-safe** (Arc-based, Send + Sync)
- **Single-threaded Tokio runtime** for FFI compatibility
- **String-based contract** (JSON in, JSON out)
- **No core Engine changes** needed for new language bindings

## Reference Documents

Key documentation files in the repository:

- `README.md` - Project overview, quick start, and feature highlights
- `SECURITY_PREPROCESSING.md` - Details on adversarial input protection
- `cupcake-py/README.md` - Python bindings documentation
- `cupcake-ts/README.md` - TypeScript bindings documentation
- `docs/` - New documentation site (in development, using Astro)
- `docs-old/` - Legacy documentation (being migrated)
  - `docs-old/user-guide/policies/writing-policies.md` - Complete policy authoring guide
  - `docs-old/user-guide/policies/metadata-system.md` - Routing metadata documentation
  - `docs-old/user-guide/policies/builtin-policies-reference.md` - All builtin policies
  - `docs-old/user-guide/harnesses/` - Harness-specific integration guides
  - `docs-old/development/DEBUGGING.md` - Advanced debugging techniques
- `examples/` - Example policies and event payloads

## Policy Development

Policies follow a metadata-driven format:

1. Declare routing requirements in metadata
2. Use decision verbs (`deny contains`, `halt contains`)
3. Trust the engine's routing (no redundant checks)
4. Return structured decision objects with reason, severity, rule_id
5. Use `concat` instead of `sprintf` (WASM limitation)
6. The `add_context` verb in Cupcake expects just strings, not objects

See `docs/policies/POLICIES.md` for the complete policy authoring guide.

## Routing and Wildcard Policies

Policies that declare `required_events` without `required_tools` act as wildcards - they match ANY tool for that event. The engine automatically routes both:

- Specific tool events to their exact matches
- Tool events to wildcard policies (those with only the event declared)

Example: A policy with `required_events: ["PostToolUse"]` will match all PostToolUse events regardless of tool.

## Builtin Abstractions

Eleven builtins provide common patterns without writing Rego:

- **claude_code_always_inject_on_prompt** - Adds context to every user prompt
- **global_file_lock** - Blocks all file modifications globally
- **git_pre_check** - Validates before git operations
- **post_edit_check** - Runs validation after file edits
- **rulebook_security_guardrails** - Protects .cupcake files from modification
- **protected_paths** - Blocks modifications to specified paths
- **git_block_no_verify** - Prevents --no-verify flag in git commits
- **system_protection** - Protects system directories from access
- **sensitive_data_protection** - Blocks access to sensitive files (SSH keys, etc.)
- **cupcake_exec_protection** - Prevents execution of cupcake commands
- **claude_code_enforce_full_file_read** - Enforces reading entire files under configurable line limit

Configure in `.cupcake/rulebook.yml` under `builtins:` section. See `fixtures/init/base-config.yml` for template.

### Builtin Configuration Notes

1. **Default Enabled**: Builtins default to `enabled: true` when configured. You don't need to explicitly set `enabled: true` - just configuring a builtin enables it.

2. **Config Access**: Builtin policies access their configuration via `input.builtin_config.<builtin_name>` directly, NOT through signals. Static configuration values are injected without spawning shell processes.

3. **Signal Usage**: Signals are only used for dynamic data gathering (e.g., running validation commands). Static config like messages and paths come through `builtin_config`.

4. **Global Override**: Global builtin configurations override project configurations. This is intentional for organizational policy enforcement.

5. **Grep/Glob Path Handling (Claude Code only)**: In Claude Code harness, Grep and Glob tools operate on directory patterns (`"secrets/"`) not file paths. Policies must use raw `tool_input.path` for these tools instead of waiting for `resolved_file_path` from preprocessing. Cursor harness doesn't have these tools - it uses different events.

## Rego v1 Migration Checklist

**CRITICAL**: Cupcake uses OPA v1.71.0+ where Rego v1 syntax is the DEFAULT (no import needed).

### 🚨 Breaking Changes - Silent Failures

#### 1. Object Key Membership (CRITICAL - Silent Bug)

```rego
# WRONG - Always returns false in Rego v1 (silent failure!)
"key" in my_object

# CORRECT - Use object.keys() to get the set of keys
"key" in object.keys(my_object)

# ALSO CORRECT - For key-value iteration
some key, value in my_object
```

**Why Critical**: The old syntax compiles but silently returns `false`, making policies ineffective.

#### 2. Import Policy (OPA v1.71.0+)

```rego
package cupcake.policies.example

import rego.v1  # OPTIONAL - No-op in OPA v1.0+, but good for compatibility

# Since OPA v1.0, v1 syntax is DEFAULT - no import required
# Keep import only for backward compatibility with pre-v1.0 OPA
```

### ✅ Still Valid Syntax

The `in` operator works correctly for:

- **Sets**: `"value" in {"a", "b", "c"}`
- **Arrays**: `"value" in ["foo", "bar"]`
- **Iteration**: `some key, value in object` (key-value pairs)

### 🔧 Common Gotchas

#### 3. Decision Verb Syntax

```rego
# MODERN - Use contains with decision verbs
deny contains decision if { ... }
halt contains decision if { ... }
ask contains decision if { ... }

# LEGACY - Still works but prefer modern
deny[decision] { ... }
```

#### 4. Ask Decision Fields

```rego
decision := {
    "rule_id": "RULE-ID",
    "reason": "Why we're asking",      # REQUIRED
    "question": "What to ask user",    # REQUIRED
    "severity": "MEDIUM"
}
```

#### 5. String Functions

```rego
# PREFERRED - More consistent
concat(" ", ["hello", "world"])

# WORKS - But can be inconsistent in WASM
sprintf("%s %s", ["hello", "world"])
```

### 🧪 Testing & Validation

#### 6. Policy Compilation

```bash
# Test policies compile correctly
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/

# Validate specific policy syntax
opa fmt --diff .cupcake/policies/your-policy.rego
```

#### 7. Audit Commands

```bash
# Find potential object membership issues
rg '".*" in [^o]' --type rego .cupcake/policies/
rg 'in [^o].*\{' --type rego .cupcake/policies/

# Check for missing import rego.v1
rg -L 'import rego\.v1' --type rego .cupcake/policies/
```

### 🎯 Metadata Best Practices

#### Scope Options and Placement

```rego
# CORRECT - Package scope (recommended for Cupcake routing metadata)
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.example

# WRONG - Detached metadata (lint error)
package cupcake.policies.example

# METADATA  <-- Separated from target, won't work!
# scope: rule
deny contains decision if { ... }

# CORRECT - Rule scope (immediately before rule)
# METADATA
# scope: rule
# title: Specific Rule Title
deny contains decision if { ... }
```

#### Scope Rules

- **`package`**: Applies to entire package - use for routing metadata ⚠️ **MUST be first in file**
- **`rule`**: Applies to single rule - must immediately precede the rule
- **`document`**: Applies to all rules with same name across files
- **No scope**: Auto-detects based on position (package if before `package`, rule if before rule)

#### ⚠️ Critical Rule: Package Metadata Placement

```rego
# METADATA must be THE FIRST THING in the file for scope: package
# METADATA
# scope: package
# ... other metadata ...
package cupcake.policies.example

# ❌ WRONG - This causes "annotation scope 'package' must be applied to package" error
package cupcake.policies.example
# METADATA
# scope: package  <-- Too late! Package already declared
```

### 📋 Migration Checklist

- [ ] ~~Add `import rego.v1` to all policies~~ (Optional with OPA v1.0+)
- [ ] Replace `"key" in object` with `"key" in object.keys(object)`
- [ ] Use modern `contains` syntax for decision verbs (required in v1)
- [ ] Add `if` keyword before rule bodies (required in v1)
- [ ] **METADATA PLACEMENT**: `scope: package` metadata must be FIRST in file, before `package` declaration
- [ ] Ensure ask decisions have both `reason` and `question`
- [ ] Test policy compilation with `opa build`
- [ ] Verify no silent failures in tests

**OPA Version**: v1.71.0+ (Rego v1 is default, no import needed)
**Metadata Fix**: Policies should use `scope: package` to avoid detached metadata lint errors

## Signal Access in Policies

- Signals are accessed via `input.signals.*` NOT `data.*`
- Example: `input.signals.__builtin_system_protection_paths`
- Builtin policies auto-discover signals matching their pattern
- Both project and global builtins support signal auto-discovery

## Global Builtins

- Global policies use namespace `cupcake.global.policies.builtins.*`
- Compile to separate WASM module with different entrypoint
- Evaluated in Phase 1 with early termination (halt/deny/block)
- Signals must be defined in global rulebook.yml
- Test policies should NOT use `data.*` for signal access

## Testing Claude Code Integration

### Running Claude in Tests

When testing Cupcake with Claude Code CLI, ensure proper environment inheritance:

```rust
// CORRECT - inherits environment and sets additional vars
Command::new(claude_path)
    .args(&["-p", "hello world", "--model", "haiku"])
    .current_dir(test_dir)
    .env("SOME_TEST_VAR", "value")  // Adds to inherited environment
```

### Hook Configuration for Tests

For integration tests, configure `.claude/settings.json` with UserPromptSubmit hook:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cargo run --manifest-path /path/to/Cargo.toml -- eval --log-level info --debug-routing"
          }
        ]
      }
    ]
  }
}
```

Key points:

- UserPromptSubmit always fires on `claude -p "hello world"`
- Use CLI flags (`--log-level`, `--debug-routing`) instead of environment variables
- Debug files write to `.cupcake/debug/routing/` in the working directory
- Use `std::thread::sleep(Duration::from_secs(2))` after Claude command to ensure hooks complete

## Debugging Best Practices

### Debug File Output

Enable detailed debug capture to `.cupcake/debug/` directory:

```bash
# Use --debug-files flag to enable
cupcake eval --debug-files --policy-dir .cupcake/policies < event.json
```

This creates human-readable debug files with complete evaluation flow including routing, signals, WASM results, and final decisions.

### Trace Levels

The `--trace` flag enables different levels of evaluation tracing to stderr:

```bash
# Trace main evaluation pipeline only (routing, signals, WASM, synthesis)
cargo run -- eval --trace eval < event.json

# Trace everything (all engine components plus lower-level details)
cargo run -- eval --trace all < event.json

# Trace specific components
cargo run -- eval --trace routing      # Routing decisions
cargo run -- eval --trace signals      # Signal gathering
cargo run -- eval --trace wasm         # WASM execution
cargo run -- eval --trace synthesis    # Decision synthesis
```

Trace output goes to stderr, so it doesn't interfere with JSON responses on stdout.

### Policy Changes in Tests

When tests use `include_str!()` to embed policy content at compile time, changes to those policies require recompilation:

```bash
# Clean and rebuild to pick up policy changes
cargo clean -p cupcake-core
cargo test test_name --features deterministic-tests
```

### Signal Result Format

Signals that execute commands return structured results when they fail:

```json
{
  "exit_code": 1,
  "output": "stdout content",
  "error": "stderr content",
  "success": false
}
```

Policies should check `exit_code == 0` to determine success for validation signals.

### Rego Print Statements

`print()` statements in Rego policies may not show in test output by default. Use `opa test -v` for debugging isolated policy logic outside of the engine.

### Wildcard Policy Routing

The engine routes events to wildcard policies (those with only `required_events`) even when a specific tool is involved. Wildcard policies correctly receive all matching events.
