# Baseline Codebase State

**Date**: 2025-10-06
**Version**: v0.1.0
**Branch**: tob/config-vul-fixes
**Purpose**: Pre-implementation baseline for security refactor

## Executive Summary

This document captures the current state of the Cupcake codebase before implementing the security refactor outlined in `SECURITY_REFACTOR_ACTION_PLAN.md`. It serves as a reference point for measuring progress and verifying changes.

## Codebase Metrics

### Repository Statistics
- **Total Rust Source Files**: 121
- **Total Lines of Code**: 22,579 (excluding target/)
- **Repository Size**: 6.1 GB (includes build artifacts)
- **Workspace Version**: 0.1.0
- **Rust Edition**: 2021

### Workspace Structure

```
cupcake-rewrite/
├── cupcake-core/       # Core engine (wasmtime, routing, synthesis)
├── cupcake-cli/        # CLI interface
├── cupcake-py/         # Python bindings (optional)
├── fixtures/           # Test fixtures and base configs
├── docs/               # Documentation
├── eval/               # Evaluation test cases
└── scripts/            # Utility scripts
```

### Core Module Breakdown

**cupcake-core** (44 source files):
- `src/engine/` - 11 files - Core evaluation engine
  - `mod.rs` - Main engine with routing & evaluation
  - `routing.rs` - Event routing via metadata
  - `synthesis.rs` - Decision synthesis (Intelligence Layer)
  - `wasm_runtime.rs` - WASM policy execution
  - `compiler.rs` - OPA compilation to WASM
  - `metadata.rs` - Policy metadata parser
  - `builtins.rs` - Builtin policy configuration
  - `guidebook.rs` - Config file parsing
  - `global_config.rs` - Global config loading
  - `trace.rs` - Evaluation tracing
  - `routing_debug.rs` - Routing debug output

- `src/harness/` - 16 files - Claude Code integration
  - `events/` - Event parsing (PreToolUse, PostToolUse, etc.)
  - `response/` - Response formatting (JSON output)

- `src/trust/` - 5 files - Trust system
  - `manifest.rs` - Policy manifest handling
  - `verifier.rs` - Script verification
  - `hasher.rs` - HMAC computation

- `src/validator/` - 4 files - Decision validation
- `src/debug.rs` - Debug file output
- `src/bindings.rs` - Python FFI bindings

**cupcake-cli** (2 source files):
- `src/main.rs` - CLI entry point
- Tests

## Security-Relevant Code Locations

### Environment Variable Usage (18 occurrences across 10 files)

**Critical Files** (will be modified in Phase 1):

1. **cupcake-cli/src/main.rs** (2 occurrences)
   - `CUPCAKE_TRACE` - Evaluation tracing
   - `RUST_LOG` - Log level configuration

2. **cupcake-core/src/engine/global_config.rs** (3 occurrences)
   - `CUPCAKE_GLOBAL_CONFIG` - Global config path override
   - Used in `load_global_config()` function (lines 36-46)

3. **cupcake-core/src/engine/wasm_runtime.rs** (1 occurrence)
   - `CUPCAKE_WASM_MAX_MEMORY` - Memory limit configuration

4. **cupcake-core/src/debug.rs** (1 occurrence)
   - `CUPCAKE_DEBUG_FILES` - Debug file output toggle

5. **cupcake-core/src/engine/routing_debug.rs** (1 occurrence)
   - `CUPCAKE_DEBUG_ROUTING` - Routing debug output

6. **cupcake-core/src/engine/compiler.rs** (1 occurrence)
   - `CUPCAKE_OPA_PATH` - OPA binary location override

**Test Files** (8 occurrences - will be updated):
- `cupcake-core/src/debug/tests.rs`
- `cupcake-core/tests/claude_code_routing_test.rs` (6 occurrences)
- `cupcake-core/tests/opa_lookup_test.rs`
- `cupcake-core/src/trust/hasher.rs` (deterministic test mode)

### Shell Command Usage

**Finding**: 0 occurrences of `bash -c` in Rust code

This is positive - the main shell execution vulnerability (TOB-EQTY-LAB-CUPCAKE-2, 4, 8) is not present in the current Rust codebase. However, the refactor plan still includes creating `SecureCommand` infrastructure to:
1. Prevent future introduction of shell injection
2. Handle signal/action execution securely
3. Support trust script validation

### Trust System Files

**Current Implementation**:
- `cupcake-core/src/trust/manifest.rs` - Manifest parsing
- `cupcake-core/src/trust/verifier.rs` - Script verification logic
- `cupcake-core/src/trust/hasher.rs` - HMAC key derivation
- `cupcake-core/src/trust/error.rs` - Trust errors

**Known Issues** (TOB-EQTY-LAB-CUPCAKE-3, 6):
- Path traversal not prevented in script execution
- Deterministic test mode weakens HMAC (`deterministic-tests` feature)
- No canonicalization of trust script paths

### Policy System Files

**Current Implementation**:
- `cupcake-core/src/engine/routing.rs` - Routes events to policies
- `cupcake-core/src/engine/synthesis.rs` - Aggregates decisions
- `cupcake-core/src/engine/metadata.rs` - Parses policy metadata
- `cupcake-core/src/engine/builtins.rs` - Builtin policy loading

**Known Issues** (TOB-EQTY-LAB-CUPCAKE-5, 10):
- No namespace isolation (global vs project policies)
- Decision priority documented but not enforced in all paths
- Global deny blocking project ask needs explicit testing

## Dependencies

### Core Runtime Dependencies

```toml
wasmtime = "35.0"                    # WASM runtime
tokio = "1.46.1"                     # Async runtime
serde = "1.0.219"                    # Serialization
serde_json = "1.0.140"               # JSON handling
anyhow = "1.0.98"                    # Error handling
```

### Security-Relevant Dependencies

```toml
sha2 = "0.10"                        # SHA-256 hashing
hmac = "0.12"                        # HMAC computation
hex = "0.4"                          # Hex encoding
```

### Testing Dependencies

```toml
pretty_assertions = "1.4"            # Better test output
tempfile = "3.0"                     # Temp directories
insta = "1.35"                       # Snapshot testing
criterion = "0.5"                    # Benchmarking
```

## Test Infrastructure

### Test Execution Requirements

**CRITICAL**: Tests must be run with:
```bash
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests
```

Or using Just commands:
```bash
just test
```

### Current Test Files

**Integration Tests** (`cupcake-core/tests/`):
- `claude_code_routing_test.rs` - Routing verification
- `opa_lookup_test.rs` - OPA binary discovery
- Additional integration tests (need enumeration)

**Unit Tests**:
- Embedded in module files (need count)

### Test Coverage

**Status**: Not measured - need baseline metrics

**Action Required**: Establish coverage baseline before refactor
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Get baseline coverage
cargo tarpaulin --features deterministic-tests --out Html
```

## Build Configuration

### Compiler Profiles

**Release** (production builds):
```toml
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for max optimization
opt-level = 3           # Maximum optimization
```

**Dev** (development builds):
```toml
opt-level = 1           # Minimal optimization for faster builds
```

**Test**:
```toml
opt-level = 2           # Moderate optimization for test performance
```

### Feature Flags

**Current Features**:
- `deterministic-tests` - Deterministic HMAC key derivation for tests
  - **Security Note**: This weakens HMAC security and MUST NOT be enabled in production

**Planned Features** (Phase 5):
- Security test suites as optional features

## File Locations for Modification

### Phase 1: Environment Variable Elimination

**Files to Modify**:
1. `cupcake-cli/src/main.rs`
   - Add CLI flag parsing with `clap`
   - Remove `CUPCAKE_TRACE`, `RUST_LOG` env var checks
   - Pass flags to core engine

2. `cupcake-core/src/engine/global_config.rs`
   - Add `load_global_config(cli_override: Option<PathBuf>)` parameter
   - Remove `CUPCAKE_GLOBAL_CONFIG` env var check
   - Add path validation function

3. `cupcake-core/src/engine/wasm_runtime.rs`
   - Add memory size parameter to runtime initialization
   - Remove `CUPCAKE_WASM_MAX_MEMORY` env var check
   - Add 1MB minimum enforcement (TOB-EQTY-LAB-CUPCAKE-1)

4. `cupcake-core/src/debug.rs`
   - Add `should_write_debug_files` parameter
   - Remove `CUPCAKE_DEBUG_FILES` env var check

5. `cupcake-core/src/engine/routing_debug.rs`
   - Add `should_write_routing_debug` parameter
   - Remove `CUPCAKE_DEBUG_ROUTING` env var check

6. `cupcake-core/src/engine/compiler.rs`
   - Add `opa_path_override` parameter
   - Remove `CUPCAKE_OPA_PATH` env var check
   - Add path validation

**New Files to Create**:
- None for Phase 1 (pure refactor)

### Phase 2: Shell Command Hardening

**New Files to Create**:
1. `cupcake-core/src/engine/secure_command.rs`
   - `SecureCommand` enum
   - Command execution without shell interpretation
   - Whitelisted command types

2. `cupcake-core/src/engine/secure_command/tests.rs`
   - Unit tests for all command types
   - Security tests (injection attempts)

### Phase 3: Trust System Improvements

**Files to Modify**:
1. `cupcake-core/src/trust/verifier.rs`
   - Add path canonicalization
   - Add path traversal checks
   - Prevent `deterministic-tests` bypass

2. `cupcake-core/src/trust/hasher.rs`
   - Add production security checks
   - Document feature flag risk

3. `cupcake-core/src/trust/manifest.rs`
   - Add integrity validation

### Phase 4: Policy System Hardening

**Files to Modify**:
1. `cupcake-core/src/engine/routing.rs`
   - Add namespace isolation
   - Prevent global/project collision

2. `cupcake-core/src/engine/synthesis.rs`
   - Add explicit priority enforcement
   - Document global deny > project ask

### Phase 5: Testing & Validation

**New Files to Create**:
- `cupcake-core/tests/security/` directory
  - `env_var_isolation.rs`
  - `shell_injection.rs`
  - `path_traversal.rs`
  - `namespace_isolation.rs`
  - `decision_priority.rs`

## Environment Variables Documentation

See `ENVIRONMENT_VARIABLES.md` for complete list of 30 variables.

**Variables Targeted for Deprecation** (Phase 1):
1. `CUPCAKE_TRACE` → `--trace` flag
2. `RUST_LOG` → `--log-level` flag
3. `CUPCAKE_GLOBAL_CONFIG` → `--global-config` flag
4. `CUPCAKE_WASM_MAX_MEMORY` → `--wasm-max-memory` flag
5. `CUPCAKE_DEBUG_FILES` → `--debug-files` flag
6. `CUPCAKE_DEBUG_ROUTING` → `--debug-routing` flag
7. `CUPCAKE_OPA_PATH` → `--opa-path` flag

## Git State

**Current Branch**: `tob/config-vul-fixes`
**Main Branch**: `main`

**Untracked Files**:
```
2025.09 - EQTY Lab - Cupcake - Code Review - Summary Report.pdf
ENVIRONMENT_VARIABLES.md
ENV_VAR_DOCUMENTATION_INDEX.md
ENV_VAR_FINAL_SUMMARY.md
ENV_VAR_MISSING_ADDENDUM.md
ENV_VAR_VERIFICATION_REPORT.md
ENV_VAR_VULNERABILITIES.md
SECURITY_REFACTOR_ACTION_PLAN.md
IMPLEMENTATION_TRACKER.md
PHASE1_IMPLEMENTATION_GUIDE.md
BASELINE_CODEBASE_STATE.md (this file)
analyze_markdown_inventory.py
markdown_inventory.json
```

**Recent Commits**:
```
3470bff Doc Overhaul and Hooks Update (#33)
e9bf0ae more readable readme
771109b docs cleanup
10b8d6b docs cleanup
ea1e843 V0.1.0 (#32)
```

## Performance Baselines

**Status**: Not established

**Action Required**:
1. Run benchmarks with `cargo bench`
2. Document baseline performance
3. Compare after each refactor phase

```bash
# Establish baseline
cargo bench --bench evaluation_bench > benchmarks/baseline_2025-10-06.txt
```

## Known Issues Summary

From Trail of Bits audit and internal analysis:

### High Severity
- **TOB-EQTY-LAB-CUPCAKE-2**: Shell injection in signals (not found in Rust, but risk exists)
- **TOB-EQTY-LAB-CUPCAKE-4**: Shell injection in actions (not found in Rust, but risk exists)
- **TOB-EQTY-LAB-CUPCAKE-8**: Shell injection in trust scripts (not found in Rust, but risk exists)
- **TOB-EQTY-LAB-CUPCAKE-11**: Global config override via env var (3 occurrences)

### Medium Severity
- **TOB-EQTY-LAB-CUPCAKE-1**: WASM memory bypass (1 occurrence)
- **TOB-EQTY-LAB-CUPCAKE-3**: Path traversal in trust scripts
- **TOB-EQTY-LAB-CUPCAKE-6**: Deterministic test mode weakens HMAC
- **TOB-EQTY-LAB-CUPCAKE-9**: Log exposure via env var (2 occurrences)

### Low Severity
- **TOB-EQTY-LAB-CUPCAKE-5**: Namespace isolation missing
- **TOB-EQTY-LAB-CUPCAKE-7**: Decision priority not explicitly enforced
- **TOB-EQTY-LAB-CUPCAKE-10**: Global deny blocks project ask (needs testing)

## Pre-Implementation Checklist

Before starting Phase 1 implementation:

- [ ] Establish test coverage baseline (`cargo tarpaulin`)
- [ ] Run benchmark suite (`cargo bench`)
- [ ] Document current CLI help output (`cargo run -- --help`)
- [ ] Create backup branch from current state
- [ ] Review all 30 environment variables for missed usage
- [ ] Verify no additional `bash -c` patterns exist
- [ ] Check for additional shell execution patterns (`sh -c`, `system()`, etc.)
- [ ] Audit signal/action definitions for command execution
- [ ] Create test cases for all 7 deprecated env vars
- [ ] Set up CI/CD pipeline for security tests

## Migration Path

See `SECURITY_REFACTOR_ACTION_PLAN.md` for comprehensive migration strategy.

**Estimated Timeline**:
- Phase 1: 1 week
- Phase 2: 1.5 weeks
- Phase 3: 1 week
- Phase 4: 1 week
- Phase 5: 1 week
- **Total**: 4-6 weeks

## References

- `SECURITY_REFACTOR_ACTION_PLAN.md` - Comprehensive refactor plan
- `IMPLEMENTATION_TRACKER.md` - Progress tracking
- `PHASE1_IMPLEMENTATION_GUIDE.md` - Detailed Phase 1 guide
- `ENVIRONMENT_VARIABLES.md` - Complete env var documentation
- `ENV_VAR_VULNERABILITIES.md` - Security findings
- `2025.09 - EQTY Lab - Cupcake - Code Review - Summary Report.pdf` - Full audit

---

**Baseline Established**: 2025-10-06
**Next Action**: Begin Phase 1 implementation per `PHASE1_IMPLEMENTATION_GUIDE.md`
