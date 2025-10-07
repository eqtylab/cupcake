# Environment Variable Removal - Implementation Plan

**Date**: 2025-10-06
**Scope**: Remove all behavioral environment variables, replace with CLI flags
**Context**: Pre-release refactor - no backward compatibility constraints

---

## Goal

**Remove 7 environment variables that control Cupcake behavior** and replace them with validated CLI flags.

### Why We're Doing This

Environment variables in AI agent contexts are **untrusted input** - agents can manipulate them via user prompts to bypass security controls.

### Trail of Bits Findings Addressed

- **TOB-EQTY-LAB-CUPCAKE-11** (HIGH): Global config override via `CUPCAKE_GLOBAL_CONFIG`
- **TOB-EQTY-LAB-CUPCAKE-1** (MEDIUM): WASM memory bypass via `CUPCAKE_WASM_MAX_MEMORY`
- **TOB-EQTY-LAB-CUPCAKE-9** (LOW): Log exposure via `RUST_LOG`/`CUPCAKE_TRACE`

---

## Environment Variables to Remove

| Variable | Current Usage | Replacement | Risk |
|----------|---------------|-------------|------|
| `CUPCAKE_TRACE` | Enable eval tracing | `--trace` flag | HIGH |
| `RUST_LOG` | Log level control | `--log-level` flag | MEDIUM |
| `CUPCAKE_GLOBAL_CONFIG` | Override global config path | `--global-config` flag | HIGH |
| `CUPCAKE_WASM_MAX_MEMORY` | Set WASM memory limit | `--wasm-max-memory` flag | MEDIUM |
| `CUPCAKE_DEBUG_FILES` | Enable debug file output | `--debug-files` flag | LOW |
| `CUPCAKE_DEBUG_ROUTING` | Enable routing debug | `--debug-routing` flag | LOW |
| `CUPCAKE_OPA_PATH` | Override OPA binary path | `--opa-path` flag | MEDIUM |

---

## Current State (Baseline)

From `BASELINE_CODEBASE_STATE.md`:

- **18 `env::var()` occurrences** across 10 files
- **7 deprecated variables** controlling behavior
- **0 `bash -c` patterns** (good - no shell injection to fix)
- **22,579 lines of Rust code**

### Files Containing Deprecated Env Vars

1. `cupcake-cli/src/main.rs` (2) - `CUPCAKE_TRACE`, `RUST_LOG`
2. `cupcake-core/src/engine/global_config.rs` (3) - `CUPCAKE_GLOBAL_CONFIG`
3. `cupcake-core/src/engine/wasm_runtime.rs` (1) - `CUPCAKE_WASM_MAX_MEMORY`
4. `cupcake-core/src/debug.rs` (1) - `CUPCAKE_DEBUG_FILES`
5. `cupcake-core/src/engine/routing_debug.rs` (1) - `CUPCAKE_DEBUG_ROUTING`
6. `cupcake-core/src/engine/compiler.rs` (1) - `CUPCAKE_OPA_PATH`

Test files (will be updated but not removed):
- `cupcake-core/src/debug/tests.rs` (1)
- `cupcake-core/tests/claude_code_routing_test.rs` (6)
- `cupcake-core/tests/opa_lookup_test.rs` (1)
- `cupcake-core/src/trust/hasher.rs` (1) - deterministic test mode (keep)

---

## Implementation Tasks

### Task 1: Add CLI Flag Parsing (cupcake-cli)

**File**: `cupcake-cli/src/main.rs`

**Add dependency** to `cupcake-cli/Cargo.toml`:
```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

**Define CLI structure**:
```rust
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
enum TraceModule {
    Eval,
    Signals,
    Wasm,
    Synthesis,
    Routing,
    All,
}

#[derive(Debug, Clone, ValueEnum)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone)]
struct MemorySize {
    bytes: usize,
}

impl FromStr for MemorySize {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const MIN_MEMORY: usize = 1024 * 1024; // 1MB
        const MAX_MEMORY: usize = 100 * 1024 * 1024; // 100MB

        let parsed = if s.ends_with("MB") {
            s.trim_end_matches("MB").parse::<usize>()
                .map(|n| n * 1024 * 1024)
        } else if s.ends_with("KB") {
            s.trim_end_matches("KB").parse::<usize>()
                .map(|n| n * 1024)
        } else {
            s.parse::<usize>()
        };

        let bytes = parsed.map_err(|_| format!("Invalid memory size: {}", s))?;

        if bytes < MIN_MEMORY {
            return Err(format!("Memory size too small: {}. Minimum is 1MB", s));
        }
        if bytes > MAX_MEMORY {
            return Err(format!("Memory size too large: {}. Maximum is 100MB", s));
        }

        Ok(MemorySize { bytes })
    }
}

#[derive(Parser)]
#[command(name = "cupcake")]
#[command(about = "Policy-driven security for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Enable evaluation tracing (eval, signals, wasm, synthesis, routing, all)
    #[arg(long, value_delimiter = ',', global = true)]
    trace: Vec<TraceModule>,

    /// Set log level
    #[arg(long, default_value = "info", global = true)]
    log_level: LogLevel,

    /// Override global configuration file path
    #[arg(long, global = true)]
    global_config: Option<PathBuf>,

    /// Maximum WASM memory allocation
    #[arg(long, default_value = "10MB", global = true)]
    wasm_max_memory: MemorySize,

    /// Enable debug file output
    #[arg(long, global = true)]
    debug_files: bool,

    /// Enable routing debug output
    #[arg(long, global = true)]
    debug_routing: bool,

    /// Override OPA binary path
    #[arg(long, global = true)]
    opa_path: Option<PathBuf>,
}
```

**Remove env var checks**:
```rust
// DELETE these lines:
let cupcake_trace = env::var("CUPCAKE_TRACE").ok();
let rust_log = env::var("RUST_LOG").ok();
```

**Estimated time**: 4 hours

---

### Task 2: Update Global Config Loading

**File**: `cupcake-core/src/engine/global_config.rs`

**Add validation function**:
```rust
fn validate_config_path(path: &Path) -> Result<()> {
    // Must be absolute
    if !path.is_absolute() {
        bail!("Global config path must be absolute (got: {})", path.display());
    }

    // Must exist
    if !path.exists() {
        bail!("Global config path does not exist: {}", path.display());
    }

    // Must be regular file
    if !path.metadata()?.is_file() {
        bail!("Global config path must be a regular file: {}", path.display());
    }

    // Must have .yml/.yaml extension
    match path.extension().and_then(|s| s.to_str()) {
        Some("yml") | Some("yaml") => Ok(()),
        _ => bail!("Global config must be a YAML file (.yml or .yaml)"),
    }
}
```

**Update function signature**:
```rust
// OLD
pub fn load_global_config() -> Result<Option<CupcakeConfig>> {
    if let Ok(env_path) = std::env::var("CUPCAKE_GLOBAL_CONFIG") {
        // ...
    }
}

// NEW
pub fn load_global_config(cli_override: Option<PathBuf>) -> Result<Option<CupcakeConfig>> {
    if let Some(path) = cli_override {
        validate_config_path(&path)?;
        return load_config_from_path(&path).map(Some);
    }
    load_default_global_config()
}
```

**Remove env var check** (lines 36-46):
```rust
// DELETE this entire block:
if let Ok(env_path) = std::env::var("CUPCAKE_GLOBAL_CONFIG") {
    let path = PathBuf::from(env_path);
    if path.exists() {
        return load_config_from_path(&path).map(Some);
    }
}
```

**Estimated time**: 2 hours

---

### Task 3: Update WASM Memory Configuration

**File**: `cupcake-core/src/engine/wasm_runtime.rs`

**Update function signature**:
```rust
// OLD
pub fn create_runtime() -> Result<WasmRuntime> {
    let max_memory_str = env::var("CUPCAKE_WASM_MAX_MEMORY")
        .unwrap_or_else(|_| DEFAULT_MAX_MEMORY.to_string());
}

// NEW
pub fn create_runtime(max_memory: MemorySize) -> Result<WasmRuntime> {
    const ABSOLUTE_MIN_BYTES: usize = 1024 * 1024; // 1MB - defense in depth
    let memory_bytes = max_memory.bytes.max(ABSOLUTE_MIN_BYTES);

    // Convert to WASM pages (64KB each)
    let max_pages = (memory_bytes / (64 * 1024)) as u32;
}
```

**Remove env var check**:
```rust
// DELETE:
let max_memory_str = env::var("CUPCAKE_WASM_MAX_MEMORY")
    .unwrap_or_else(|_| DEFAULT_MAX_MEMORY.to_string());
```

**Estimated time**: 2 hours

---

### Task 4: Update Debug File Output

**File**: `cupcake-core/src/debug.rs`

**Update function signature**:
```rust
// OLD
pub fn write_debug_files(data: &DebugData) -> Result<()> {
    if env::var("CUPCAKE_DEBUG_FILES").is_ok() {
        // Write files
    }
}

// NEW
pub fn write_debug_files(data: &DebugData, enabled: bool) -> Result<()> {
    if enabled {
        // Write files
    }
    Ok(())
}
```

**Remove env var check**:
```rust
// DELETE:
if env::var("CUPCAKE_DEBUG_FILES").is_ok() {
```

**Estimated time**: 1 hour

---

### Task 5: Update Routing Debug

**File**: `cupcake-core/src/engine/routing_debug.rs`

**Update function signature**:
```rust
// OLD
pub fn write_routing_debug(data: &RoutingData) -> Result<()> {
    if env::var("CUPCAKE_DEBUG_ROUTING").is_ok() {
        // Write files
    }
}

// NEW
pub fn write_routing_debug(data: &RoutingData, enabled: bool) -> Result<()> {
    if enabled {
        // Write files
    }
    Ok(())
}
```

**Remove env var check**:
```rust
// DELETE:
if env::var("CUPCAKE_DEBUG_ROUTING").is_ok() {
```

**Estimated time**: 1 hour

---

### Task 6: Update OPA Path Discovery

**File**: `cupcake-core/src/engine/compiler.rs`

**Add validation function**:
```rust
fn validate_opa_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("OPA binary not found at: {}", path.display());
    }

    if !path.is_file() {
        bail!("OPA path must be a file: {}", path.display());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = path.metadata()?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            bail!("OPA binary is not executable: {}", path.display());
        }
    }

    Ok(())
}
```

**Update function signature**:
```rust
// OLD
pub fn find_opa() -> Result<PathBuf> {
    if let Ok(env_path) = env::var("CUPCAKE_OPA_PATH") {
        return Ok(PathBuf::from(env_path));
    }
    // Fallback to which opa
}

// NEW
pub fn find_opa(cli_override: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = cli_override {
        validate_opa_path(&path)?;
        return Ok(path);
    }

    // Try `which opa`
    find_opa_in_path()
}
```

**Remove env var check**:
```rust
// DELETE:
if let Ok(env_path) = env::var("CUPCAKE_OPA_PATH") {
    return Ok(PathBuf::from(env_path));
}
```

**Estimated time**: 2 hours

---

### Task 7: Update Test Files

**Files**:
- `cupcake-core/src/debug/tests.rs`
- `cupcake-core/tests/claude_code_routing_test.rs`
- `cupcake-core/tests/opa_lookup_test.rs`

**Update all test code** to pass CLI flags instead of setting env vars:

```rust
// OLD
env::set_var("CUPCAKE_DEBUG_FILES", "1");
let result = write_debug_files(&data);

// NEW
let result = write_debug_files(&data, true);
```

**Estimated time**: 2 hours

---

## Testing Strategy

### Unit Tests

Create unit tests for each task:

```rust
#[test]
fn test_memory_size_parsing() {
    assert!(MemorySize::from_str("1MB").is_ok());
    assert!(MemorySize::from_str("512KB").is_err()); // Below minimum
}

#[test]
fn test_global_config_validation() {
    assert!(validate_config_path(Path::new("relative.yml")).is_err());
}
```

### Integration Tests

**File**: `cupcake-core/tests/cli_flags.rs`

```rust
#[test]
fn test_env_vars_ignored() {
    env::set_var("CUPCAKE_TRACE", "all");

    // CLI without --trace flag should NOT trace
    let result = run_cupcake_eval(&[]);

    // Verify tracing is NOT active

    env::remove_var("CUPCAKE_TRACE");
}
```

### Verification Scripts

Run after implementation:

```bash
# Should show 0 deprecated env var usages
./scripts/audit_env_vars.sh

# Should pass all verification tests
./scripts/verify_migration.sh

# Should find no env var references in configs
./scripts/verify_guidebooks.py
```

---

## Timeline

**Total estimated time**: 14 hours (~2 days)

| Task | Time | Running Total |
|------|------|---------------|
| 1. CLI flag parsing | 4h | 4h |
| 2. Global config | 2h | 6h |
| 3. WASM memory | 2h | 8h |
| 4. Debug files | 1h | 9h |
| 5. Routing debug | 1h | 10h |
| 6. OPA path | 2h | 12h |
| 7. Test updates | 2h | 14h |

---

## Success Criteria

✅ All 7 environment variables removed from code
✅ All CLI flags implemented and validated
✅ `./scripts/audit_env_vars.sh` shows 0 deprecated usages
✅ `./scripts/verify_migration.sh` passes all tests
✅ Full test suite passes
✅ No clippy warnings
✅ Code formatted with `cargo fmt`

---

## Rollback Plan

If needed, rollback to backup branch:

```bash
git checkout backup/pre-refactor-2025-10-06
```

Each task will be committed separately for granular rollback.

---

## References

- `ENV_VAR_VULNERABILITIES.md` - Security findings
- `ENVIRONMENT_VARIABLES.md` - Complete variable documentation
- `BASELINE_CODEBASE_STATE.md` - Pre-refactor snapshot
- `TESTING_SETUP_GUIDE.md` - Testing infrastructure
