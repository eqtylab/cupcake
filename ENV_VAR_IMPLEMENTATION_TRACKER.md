# Environment Variable Removal - Implementation Tracker

**Date Started**: 2025-10-06
**Goal**: Remove all 7 behavioral environment variables, replace with CLI flags
**Status**: Not Started

---

## Progress Overview

- [ ] **Task 1**: Add CLI flag parsing (4 hours)
- [ ] **Task 2**: Update global config loading (2 hours)
- [ ] **Task 3**: Update WASM memory configuration (2 hours)
- [ ] **Task 4**: Update debug file output (1 hour)
- [ ] **Task 5**: Update routing debug (1 hour)
- [ ] **Task 6**: Update OPA path discovery (2 hours)
- [ ] **Task 7**: Update test files (2 hours)

**Total**: 0/7 tasks complete (0/14 hours estimated)

---

## Task Details

### ✅ Task 1: Add CLI Flag Parsing

**File**: `cupcake-cli/src/main.rs`, `cupcake-cli/Cargo.toml`
**Status**: ⏳ Not Started
**Estimated**: 4 hours

**Checklist**:
- [ ] Add `clap` dependency to Cargo.toml
- [ ] Define `TraceModule` enum
- [ ] Define `LogLevel` enum
- [ ] Define `MemorySize` struct with validation
- [ ] Define `Cli` struct with all 7 flags
- [ ] Remove `CUPCAKE_TRACE` env var check
- [ ] Remove `RUST_LOG` env var check
- [ ] Pass CLI flags to engine
- [ ] Write unit tests for flag parsing
- [ ] Test manually: `cargo run -- eval --help`

**Verification**:
```bash
cargo run -- eval --help | grep -E "(--trace|--log-level|--global-config|--wasm-max-memory|--debug-files|--debug-routing|--opa-path)"
```

---

### ✅ Task 2: Update Global Config Loading

**File**: `cupcake-core/src/engine/global_config.rs`
**Status**: ⏳ Not Started
**Estimated**: 2 hours

**Checklist**:
- [ ] Add `validate_config_path()` function
- [ ] Update `load_global_config()` signature to accept `Option<PathBuf>`
- [ ] Remove env var check (lines 36-46)
- [ ] Add path validation (absolute, exists, is file, .yml extension)
- [ ] Write unit tests for validation
- [ ] Update all callers to pass CLI flag

**Addresses**: TOB-EQTY-LAB-CUPCAKE-11 (HIGH)

**Verification**:
```bash
rg "CUPCAKE_GLOBAL_CONFIG" cupcake-core/src/engine/global_config.rs
# Should return 0 results
```

---

### ✅ Task 3: Update WASM Memory Configuration

**File**: `cupcake-core/src/engine/wasm_runtime.rs`
**Status**: ⏳ Not Started
**Estimated**: 2 hours

**Checklist**:
- [ ] Update `create_runtime()` signature to accept `MemorySize`
- [ ] Remove env var check
- [ ] Add defense-in-depth 1MB minimum enforcement
- [ ] Update all callers to pass CLI flag
- [ ] Write unit tests for memory clamping
- [ ] Test with values below 1MB (should reject)

**Addresses**: TOB-EQTY-LAB-CUPCAKE-1 (MEDIUM)

**Verification**:
```bash
rg "CUPCAKE_WASM_MAX_MEMORY" cupcake-core/src/engine/wasm_runtime.rs
# Should return 0 results
```

---

### ✅ Task 4: Update Debug File Output

**File**: `cupcake-core/src/debug.rs`
**Status**: ⏳ Not Started
**Estimated**: 1 hour

**Checklist**:
- [ ] Update `write_debug_files()` signature to accept `enabled: bool`
- [ ] Remove env var check
- [ ] Update all callers to pass CLI flag
- [ ] Write unit tests
- [ ] Test manually with `--debug-files` flag

**Verification**:
```bash
rg "CUPCAKE_DEBUG_FILES" cupcake-core/src/debug.rs
# Should return 0 results
```

---

### ✅ Task 5: Update Routing Debug

**File**: `cupcake-core/src/engine/routing_debug.rs`
**Status**: ⏳ Not Started
**Estimated**: 1 hour

**Checklist**:
- [ ] Update `write_routing_debug()` signature to accept `enabled: bool`
- [ ] Remove env var check
- [ ] Update all callers to pass CLI flag
- [ ] Write unit tests
- [ ] Test manually with `--debug-routing` flag

**Verification**:
```bash
rg "CUPCAKE_DEBUG_ROUTING" cupcake-core/src/engine/routing_debug.rs
# Should return 0 results
```

---

### ✅ Task 6: Update OPA Path Discovery

**File**: `cupcake-core/src/engine/compiler.rs`
**Status**: ⏳ Not Started
**Estimated**: 2 hours

**Checklist**:
- [ ] Add `validate_opa_path()` function
- [ ] Update `find_opa()` signature to accept `Option<PathBuf>`
- [ ] Remove env var check
- [ ] Add path validation (exists, is file, is executable)
- [ ] Update all callers to pass CLI flag
- [ ] Write unit tests for validation
- [ ] Test with invalid paths (should reject)

**Verification**:
```bash
rg "CUPCAKE_OPA_PATH" cupcake-core/src/engine/compiler.rs
# Should return 0 results
```

---

### ✅ Task 7: Update Test Files

**Files**:
- `cupcake-core/src/debug/tests.rs`
- `cupcake-core/tests/claude_code_routing_test.rs`
- `cupcake-core/tests/opa_lookup_test.rs`

**Status**: ⏳ Not Started
**Estimated**: 2 hours

**Checklist**:
- [ ] Update `debug/tests.rs` to pass `enabled` flag
- [ ] Update `claude_code_routing_test.rs` (6 env var usages)
- [ ] Update `opa_lookup_test.rs` to pass CLI override
- [ ] Remove all `env::set_var()` calls for deprecated vars
- [ ] Run full test suite
- [ ] All tests pass

**Verification**:
```bash
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests
```

---

## Final Verification

After completing all tasks:

### 1. Audit Environment Variables

```bash
./scripts/audit_env_vars.sh
```

**Expected**: 0 deprecated env var usages (only `trust/hasher.rs` for deterministic tests)

### 2. Verify Guidebooks

```bash
./scripts/verify_guidebooks.py
```

**Expected**: No deprecated env var references

### 3. Run Verification Suite

```bash
./scripts/verify_migration.sh
```

**Expected**: All tests pass

### 4. Code Quality

```bash
cargo clippy --features deterministic-tests -- -D warnings
cargo fmt -- --check
```

**Expected**: No warnings, no formatting issues

### 5. Full Test Suite

```bash
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests
```

**Expected**: All tests pass

---

## Success Criteria

- ✅ All 7 environment variables removed
- ✅ All 7 CLI flags working
- ✅ All validation implemented
- ✅ All tests passing
- ✅ No clippy warnings
- ✅ Verification scripts pass

---

## Notes

**Environment variables removed**:
1. ✅ `CUPCAKE_TRACE`
2. ✅ `RUST_LOG`
3. ✅ `CUPCAKE_GLOBAL_CONFIG`
4. ✅ `CUPCAKE_WASM_MAX_MEMORY`
5. ✅ `CUPCAKE_DEBUG_FILES`
6. ✅ `CUPCAKE_DEBUG_ROUTING`
7. ✅ `CUPCAKE_OPA_PATH`

**CLI flags added**:
1. ✅ `--trace`
2. ✅ `--log-level`
3. ✅ `--global-config`
4. ✅ `--wasm-max-memory`
5. ✅ `--debug-files`
6. ✅ `--debug-routing`
7. ✅ `--opa-path`
