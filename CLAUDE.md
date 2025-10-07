# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Security Principles

Following Trail of Bits audit (2025-09), Cupcake enforces these security principles:

1. **No Ambient Authority**: Configuration via explicit CLI flags only, never environment variables
2. **Defense-in-Depth**: Validate security-critical values at parse time and runtime
3. **Explicit Consent**: Debug output requires explicit CLI flags from user
4. **Fail-Safe Defaults**: All security limits enforced with safe minimums (e.g., 1MB WASM memory)

**Rationale**: AI agents can manipulate environment variables through prompts. Explicit CLI flags create an audit trail and prevent bypass attacks.

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

You can use the claude code cli functionality to test Cupcake behavior locally:

```bash
CUPCAKE_DEBUG_ROUTING=1 claude -p "hello world" --model haiku # This will create the routing map in .cupcake/debug/
```

## Critical Claude Code Hook Integration Issues (FIXED)

### JSON Response Format Requirements

**CRITICAL**: Claude Code hooks require ONLY valid JSON on stdout. Any other output will cause parsing failure and the hook response will be ignored.

**Fixed Issues (2025-09-03)**:

1. **Field Name Casing**: The response field MUST be `hookSpecificOutput` (camelCase), not `hook_specific_output` (snake_case). Fixed in `cupcake-core/src/harness/response/types.rs`.

2. **Stdout Pollution**: ALL logs, debug output, and banners MUST go to stderr, not stdout. Only the JSON response should go to stdout. Fixed in `cupcake-cli/src/main.rs` by adding `.with_writer(std::io::stderr)` to all tracing subscribers.

Without these fixes, Claude Code will ignore deny decisions and execute dangerous commands despite Cupcake returning a proper deny response.

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

- **wasmtime**: 35.0 - WebAssembly runtime for executing compiled policies
- **tokio**: 1.46.1 - Async runtime with multi-threading
- **serde/serde_json**: 1.0 - JSON serialization/deserialization
- **OPA v1.0 Rego**: Modern syntax with `import rego.v1`

## Build and Development Commands

```bash
# Build the project
cargo build --release

# Run tests (REQUIRED: Must use deterministic-tests feature for correct test behavior)
cargo test --features deterministic-tests
# Or use the provided alias
cargo t

# Run a specific test
cargo test test_name --features deterministic-tests

# Run benchmarks
cargo bench

# Run with debug logging
RUST_LOG=debug cargo run -- [args]

# Create a new release (pushes tag to trigger automated workflow)
git tag v0.1.8 && git push origin v0.1.8

# Enable extensive debugging with policy evaluation tracing
CUPCAKE_TRACE=eval cargo run -- [args]      # Shows the main policy evaluation pipeline (routing, signals, WASM, synthesis)
CUPCAKE_TRACE=all cargo test --features deterministic-tests  # Shows everything (all engine components plus lower-level details)

# Compile OPA policies to WASM (from project root)
opa build -t wasm -e cupcake/system/evaluate .cupcake/policies/

# Run cupcake with policies
cargo run -- eval --policy-dir .cupcake/policies
```

## Architecture Overview

Cupcake implements the **Hybrid Model** from `NEW_GUIDING_FINAL.md`:

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
2. `CUPCAKE_GLOBAL_CONFIG=/nonexistent` to prevent developer's global config from interfering

```bash
# Correct way to run tests
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests

# Or use the Just commands which handle both automatically
just test
```

Without these, tests will fail due to either non-deterministic key derivation or global config override issues.

### Test Policy Requirements

When tests use `include_str!` to embed policy files at compile time, changes to those policies require recompilation. Run `cargo clean -p cupcake-core` if policy changes aren't being picked up in tests.

### Field Name Mismatch

Claude Code sends events with `hook_event_name` (snake_case) but expects responses with `hookEventName` (camelCase). The engine accepts both formats for compatibility.

### Policy Trust Model

Policies trust the engine's routing - they don't need to verify event types or tool names. If a policy is evaluating, its routing requirements are already met.

### Decision Priority

The synthesis layer enforces strict priority: Halt > Deny/Block > Ask > Allow

## Key Files and Modules

- `src/engine/mod.rs` - Core engine with routing and evaluation
- `src/engine/metadata.rs` - OPA metadata parser
- `src/engine/synthesis.rs` - Decision synthesis (Intelligence Layer)
- `src/engine/builtins.rs` - Builtin abstractions configuration
- `src/harness/` - Claude Code response formatting
- `cupcake-core/tests/fixtures/system_evaluate.rego` - Reference system aggregation entrypoint
- `fixtures/init/base-config.yml` - Template for builtin configuration

## Reference Documents

Critical references in parent directory:

- `../NEW_GUIDING_FINAL.md` - The authoritative architecture specification
- `../claude-code-docs/` - Claude Code hooks documentation
- `../cupcake-deprecated/spec/spec_hook_mapping.md` - Decision to JSON mapping

## Policy Development

Policies follow the new metadata-driven format:

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

- **always_inject_on_prompt** - Adds context to every user prompt
- **global_file_lock** - Blocks all file modifications globally
- **git_pre_check** - Validates before git operations
- **post_edit_check** - Runs validation after file edits
- **rulebook_security_guardrails** - Protects .cupcake files from modification
- **protected_paths** - Blocks modifications to specified paths
- **git_block_no_verify** - Prevents --no-verify flag in git commits
- **system_protection** - Protects system directories from access
- **sensitive_data_protection** - Blocks access to sensitive files (SSH keys, etc.)
- **cupcake_exec_protection** - Prevents execution of cupcake commands
- **enforce_full_file_read** - Enforces reading entire files under configurable line limit

Configure in `.cupcake/guidebook.yml` under `builtins:` section. See `fixtures/init/base-config.yml` for template.

### Builtin Configuration Notes

1. **Default Enabled**: Builtins default to `enabled: true` when configured. You don't need to explicitly set `enabled: true` - just configuring a builtin enables it.

2. **Config Access**: Builtin policies access their configuration via `input.builtin_config.<builtin_name>` directly, NOT through signals. Static configuration values are injected without spawning shell processes.

3. **Signal Usage**: Signals are only used for dynamic data gathering (e.g., running validation commands). Static config like messages and paths come through `builtin_config`.

4. **Global Override**: Global builtin configurations override project configurations. This is intentional for organizational policy enforcement.

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
**Metadata Fix**: ✅ All policies use `scope: package` to avoid detached metadata lint errors  
**Audit Status**: ✅ All Cupcake policies audited and fixed (last check: post-metadata fixes)

## Signal Access in Policies

- Signals are accessed via `input.signals.*` NOT `data.*`
- Example: `input.signals.__builtin_system_protection_paths`
- Builtin policies auto-discover signals matching their pattern
- Both project and global builtins support signal auto-discovery

## Global Builtins

- Global policies use namespace `cupcake.global.policies.builtins.*`
- Compile to separate WASM module with different entrypoint
- Evaluated in Phase 1 with early termination (halt/deny/block)
- Signals must be defined in global guidebook.yml
- Test policies should NOT use `data.*` for signal access

## Testing Claude Code Integration

### Running Claude in Tests

When testing Cupcake with Claude Code CLI, environment variables must be properly inherited:

```rust
// WRONG - clears all env vars
Command::new(claude_path)
    .env("CUPCAKE_DEBUG_ROUTING", "1")  // Only this var exists

// CORRECT - adds to inherited environment
Command::new(claude_path)
    .args(&["-p", "hello world", "--model", "haiku"])
    .current_dir(test_dir)
    .env("CUPCAKE_DEBUG_ROUTING", "1")  // Adds to existing env
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
            "command": "cargo run --manifest-path /path/to/Cargo.toml -- eval",
            "env": {
              "CUPCAKE_DEBUG_ROUTING": "1",
              "RUST_LOG": "info"
            }
          }
        ]
      }
    ]
  }
}
```

Key points:

- UserPromptSubmit always fires on `claude -p "hello world"`
- Hook env vars apply to the subprocess (cargo/cupcake), not Claude itself
- Debug files write to `.cupcake/debug/routing/` in the working directory
- Use `std::thread::sleep(Duration::from_secs(2))` after Claude command to ensure hooks complete

## Debugging Best Practices

### Debug File Output

Enable detailed debug capture to `.cupcake/debug/` directory:

```bash
# Set CUPCAKE_DEBUG_FILES to any value to enable
CUPCAKE_DEBUG_FILES=1 cupcake eval --policy-dir .cupcake/policies < event.json
```

This creates human-readable debug files with complete evaluation flow including routing, signals, WASM results, and final decisions.

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

The engine routes events to wildcard policies (those with only `required_events`) even when a specific tool is involved. This was a bug that was fixed - wildcard policies now correctly receive all matching events.
