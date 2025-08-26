# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Dependencies and Requirements

### Required Tools
- **OPA (Open Policy Agent)**: v0.70.0 or later (for v1.0 Rego syntax support)
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

# Compile OPA policies to WASM (from project root)
opa build -t wasm -e cupcake/system/evaluate examples/policies/

# Run cupcake with example policies
cargo run -- examples/policies/
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
**IMPORTANT**: Tests MUST be run with the `--features deterministic-tests` flag. This is NOT optional - the trust system tests will fail intermittently without it due to non-deterministic HMAC key derivation in production mode. The feature flag ensures deterministic key generation for reliable test execution. Use `cargo t` (alias) or `cargo test --features deterministic-tests`.

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
- `src/harness/` - Claude Code response formatting
- `examples/policies/system/evaluate.rego` - Mandatory aggregation entrypoint

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

See `docs/policies/POLICIES.md` for the complete policy authoring guide.