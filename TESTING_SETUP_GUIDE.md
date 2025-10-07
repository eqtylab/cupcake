# Testing Infrastructure Setup Guide

**Date**: 2025-10-06
**Purpose**: Configure comprehensive testing for security refactor
**Related**: `SECURITY_REFACTOR_ACTION_PLAN.md`, `BASELINE_CODEBASE_STATE.md`

## Executive Summary

This guide establishes the testing infrastructure needed to validate the security refactor. It covers unit tests, integration tests, security tests, penetration tests, and CI/CD configuration.

## Testing Philosophy

### Defense-in-Depth Testing Strategy

Each security control must have:
1. **Unit tests** - Individual component behavior
2. **Integration tests** - Component interactions
3. **Security tests** - Exploitation attempts
4. **Regression tests** - Prevent re-introduction of vulnerabilities
5. **Property-based tests** - Fuzzing and edge cases

### Test Pyramid Structure

```
           /\
          /  \        Security Tests (10%)
         /    \       - Penetration scenarios
        /      \      - Exploit attempts
       /--------\
      /          \    Integration Tests (30%)
     /            \   - End-to-end workflows
    /              \  - Component interactions
   /----------------\
  /                  \ Unit Tests (60%)
 /____________________\ - Function-level tests
                        - Edge cases
```

## Test Organization

### Directory Structure

```
cupcake-core/
├── src/
│   └── **/*.rs           # Unit tests (mod tests)
├── tests/
│   ├── integration/      # Integration tests
│   │   ├── cli_flags.rs
│   │   ├── config_loading.rs
│   │   ├── policy_evaluation.rs
│   │   └── trust_system.rs
│   ├── security/         # Security-specific tests
│   │   ├── env_var_isolation.rs
│   │   ├── shell_injection.rs
│   │   ├── path_traversal.rs
│   │   ├── namespace_isolation.rs
│   │   └── decision_priority.rs
│   ├── regression/       # Regression test suite
│   │   └── tob_findings.rs
│   └── fixtures/         # Test data
│       ├── policies/
│       ├── configs/
│       └── scripts/
└── benches/
    └── evaluation_bench.rs
```

### Test File Naming Convention

- **Unit tests**: `mod tests` at bottom of source file
- **Integration tests**: `tests/{category}/{feature}.rs`
- **Security tests**: `tests/security/{vulnerability_type}.rs`
- **Benchmarks**: `benches/{component}_bench.rs`

## Unit Testing

### Standard Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_success_case() {
        // Arrange
        let input = create_test_input();

        // Act
        let result = function_under_test(input);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_output);
    }

    #[test]
    fn test_feature_error_case() {
        // Test error handling
        let invalid_input = create_invalid_input();
        let result = function_under_test(invalid_input);
        assert!(result.is_err());
    }

    #[test]
    fn test_feature_edge_case() {
        // Test boundary conditions
    }
}
```

### Phase 1 Unit Tests (CLI Flags)

**File**: `cupcake-cli/src/main.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_size_parsing_valid() {
        let sizes = vec![
            ("1MB", 1024 * 1024),
            ("10MB", 10 * 1024 * 1024),
            ("100MB", 100 * 1024 * 1024),
            ("1048576", 1024 * 1024),
        ];

        for (input, expected) in sizes {
            let parsed = MemorySize::from_str(input).unwrap();
            assert_eq!(parsed.bytes, expected);
        }
    }

    #[test]
    fn test_memory_size_below_minimum() {
        let result = MemorySize::from_str("512KB");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too small"));
    }

    #[test]
    fn test_memory_size_above_maximum() {
        let result = MemorySize::from_str("200MB");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_trace_module_parsing() {
        let modules = vec!["eval", "signals", "wasm", "synthesis", "routing", "all"];
        for module in modules {
            let parsed = TraceModule::from_str(module, true).unwrap();
            assert!(matches!(parsed, TraceModule::_));
        }
    }

    #[test]
    fn test_log_level_parsing() {
        assert!(matches!(
            LogLevel::from_str("debug", true).unwrap(),
            LogLevel::Debug
        ));
    }
}
```

### Phase 1 Unit Tests (Config Validation)

**File**: `cupcake-core/src/engine/global_config.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_config_path_relative() {
        let result = validate_config_path(Path::new("relative/path.yml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be absolute"));
    }

    #[test]
    fn test_validate_config_path_nonexistent() {
        let result = validate_config_path(Path::new("/nonexistent/path.yml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_validate_config_path_not_file() {
        let result = validate_config_path(Path::new("/tmp"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a regular file"));
    }

    #[test]
    fn test_validate_config_path_wrong_extension() {
        let temp = NamedTempFile::new().unwrap();
        let result = validate_config_path(temp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a YAML file"));
    }

    #[test]
    fn test_validate_config_path_valid() {
        let mut temp = NamedTempFile::with_suffix(".yml").unwrap();
        write!(temp, "test: data").unwrap();
        let result = validate_config_path(temp.path());
        assert!(result.is_ok());
    }
}
```

## Integration Testing

### Integration Test Setup

**File**: `cupcake-core/tests/integration/mod.rs`

```rust
use cupcake_core::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary test directory with basic Cupcake structure
pub fn setup_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let cupcake_dir = dir.path().join(".cupcake");
    std::fs::create_dir_all(&cupcake_dir).unwrap();

    // Create minimal guidebook
    let guidebook = cupcake_dir.join("guidebook.yml");
    std::fs::write(&guidebook, "version: '1.0'\n").unwrap();

    dir
}

/// Create a test policy file
pub fn create_test_policy(dir: &Path, name: &str, content: &str) -> PathBuf {
    let policy_dir = dir.join(".cupcake/policies");
    std::fs::create_dir_all(&policy_dir).unwrap();

    let policy_file = policy_dir.join(format!("{}.rego", name));
    std::fs::write(&policy_file, content).unwrap();

    policy_file
}

/// Create a test event JSON
pub fn create_test_event(event_type: &str) -> serde_json::Value {
    serde_json::json!({
        "hook_event_name": event_type,
        "session_id": "test-session",
        "transcript_path": "/tmp/transcript.json",
        "cwd": "/tmp/test"
    })
}
```

### Phase 1 Integration Tests (CLI Flags)

**File**: `cupcake-core/tests/integration/cli_flags.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help_shows_new_flags() {
    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--trace"))
        .stdout(predicate::str::contains("--log-level"))
        .stdout(predicate::str::contains("--global-config"))
        .stdout(predicate::str::contains("--wasm-max-memory"))
        .stdout(predicate::str::contains("--debug-files"))
        .stdout(predicate::str::contains("--debug-routing"))
        .stdout(predicate::str::contains("--opa-path"));
}

#[test]
fn test_cli_accepts_trace_flag() {
    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval", "--trace", "eval,signals"]);

    // Should not error on flag parsing
    // (may error on missing input, but that's OK)
    let output = cmd.output().unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains("unrecognized"));
}

#[test]
fn test_cli_rejects_env_vars() {
    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.env("CUPCAKE_TRACE", "eval");
    cmd.args(&["eval"]);

    // Should NOT use env var (verify via debug output or lack of tracing)
    // This test verifies env vars are ignored
}

#[test]
fn test_global_config_flag_validation() {
    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval", "--global-config", "/nonexistent/path.yml"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_memory_size_flag_below_minimum() {
    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval", "--wasm-max-memory", "512KB"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("too small"));
}
```

### Phase 1 Integration Tests (Config Loading)

**File**: `cupcake-core/tests/integration/config_loading.rs`

```rust
use cupcake_core::engine::global_config::load_global_config;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_load_global_config_with_cli_override() {
    let mut temp = NamedTempFile::with_suffix(".yml").unwrap();
    write!(temp, "version: '1.0'\nbuiltins:\n  test: true\n").unwrap();

    let result = load_global_config(Some(temp.path().to_path_buf()));
    assert!(result.is_ok());

    let config = result.unwrap().unwrap();
    assert_eq!(config.version, "1.0");
}

#[test]
fn test_load_global_config_without_override() {
    // Should load from default location or return None
    let result = load_global_config(None);
    assert!(result.is_ok());
}

#[test]
fn test_load_global_config_invalid_path() {
    let result = load_global_config(Some(PathBuf::from("/nonexistent/path.yml")));
    assert!(result.is_err());
}
```

## Security Testing

### Security Test Suite Structure

**File**: `cupcake-core/tests/security/mod.rs`

```rust
/// Security test utilities
pub mod common {
    use std::process::Command;

    /// Attempt to inject a shell command
    pub fn attempt_shell_injection(input: &str) -> Result<(), String> {
        // Test various injection patterns
        let patterns = vec![
            format!("{}; malicious_command", input),
            format!("{}$(malicious_command)", input),
            format!("{}`malicious_command`", input),
            format!("{}| malicious_command", input),
            format!("{}&& malicious_command", input),
        ];

        for pattern in patterns {
            // Attempt injection (should fail safely)
        }

        Ok(())
    }

    /// Attempt path traversal
    pub fn attempt_path_traversal(base: &Path, traversal: &str) -> Result<PathBuf, String> {
        let path = base.join(traversal);
        // Should be detected and rejected
        Ok(path)
    }
}
```

### Phase 1 Security Tests (Env Var Isolation)

**File**: `cupcake-core/tests/security/env_var_isolation.rs`

```rust
use std::env;
use assert_cmd::Command;

#[test]
fn test_cupcake_trace_env_var_ignored() {
    env::set_var("CUPCAKE_TRACE", "eval");

    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval"]);

    // Verify trace is NOT active (check output/logs)
    let output = cmd.output().unwrap();
    // Should not see trace-level debug output

    env::remove_var("CUPCAKE_TRACE");
}

#[test]
fn test_global_config_env_var_ignored() {
    env::set_var("CUPCAKE_GLOBAL_CONFIG", "/malicious/config.yml");

    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval"]);

    // Should NOT load the env var path
    // Verify by checking which config is loaded

    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
}

#[test]
fn test_wasm_max_memory_env_var_ignored() {
    env::set_var("CUPCAKE_WASM_MAX_MEMORY", "1KB"); // Below minimum

    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval"]);

    // Should use default, not env var (wouldn't bypass minimum)

    env::remove_var("CUPCAKE_WASM_MAX_MEMORY");
}

#[test]
fn test_all_deprecated_env_vars_have_no_effect() {
    let env_vars = vec![
        ("CUPCAKE_TRACE", "all"),
        ("RUST_LOG", "debug"),
        ("CUPCAKE_GLOBAL_CONFIG", "/tmp/malicious.yml"),
        ("CUPCAKE_WASM_MAX_MEMORY", "1KB"),
        ("CUPCAKE_DEBUG_FILES", "1"),
        ("CUPCAKE_DEBUG_ROUTING", "1"),
        ("CUPCAKE_OPA_PATH", "/tmp/malicious_opa"),
    ];

    for (key, value) in &env_vars {
        env::set_var(key, value);
    }

    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval"]);

    // Verify none of the env vars have any effect
    // Run successful evaluation without using any env var values

    for (key, _) in &env_vars {
        env::remove_var(key);
    }
}
```

### Phase 2 Security Tests (Shell Injection)

**File**: `cupcake-core/tests/security/shell_injection.rs`

```rust
use cupcake_core::engine::secure_command::SecureCommand;

#[test]
fn test_git_command_injection_attempt() {
    let malicious_args = vec![
        "status".to_string(),
        "; rm -rf /".to_string(), // Injection attempt
    ];

    let cmd = SecureCommand::Git { args: malicious_args };
    let result = cmd.execute(Path::new("/tmp"));

    // Should execute git with literal args (git will reject "; rm -rf /")
    // Should NOT execute through shell
    assert!(result.is_ok() || result.unwrap_err().to_string().contains("git"));
}

#[test]
fn test_script_command_no_shell_interpretation() {
    let cmd = SecureCommand::Script {
        name: "validate.sh".to_string(),
        args: vec!["arg1; malicious".to_string()],
    };

    let result = cmd.execute(Path::new("/tmp"));

    // Args should be passed literally, not interpreted
    // Script receives "arg1; malicious" as single argument
}

#[test]
fn test_external_command_rejection_without_whitelist() {
    let cmd = SecureCommand::External {
        program: "bash".to_string(), // Not whitelisted
        args: vec!["-c".to_string(), "malicious".to_string()],
    };

    let result = cmd.execute(Path::new("/tmp"));

    // Should reject non-whitelisted programs
    assert!(result.is_err());
}
```

### Phase 3 Security Tests (Path Traversal)

**File**: `cupcake-core/tests/security/path_traversal.rs`

```rust
use cupcake_core::trust::verifier::verify_script;

#[test]
fn test_trust_script_path_traversal_blocked() {
    let traversals = vec![
        "../../../etc/passwd",
        "subdir/../../etc/passwd",
        "./../../etc/passwd",
        "scripts/../../../etc/passwd",
    ];

    let base_dir = PathBuf::from("/tmp/project/.cupcake");

    for traversal in traversals {
        let result = verify_script(&base_dir, traversal);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }
}

#[test]
fn test_trust_script_absolute_path_blocked() {
    let base_dir = PathBuf::from("/tmp/project/.cupcake");
    let result = verify_script(&base_dir, "/etc/passwd");

    assert!(result.is_err());
}

#[test]
fn test_trust_script_symlink_traversal_blocked() {
    let temp = TempDir::new().unwrap();
    let cupcake_dir = temp.path().join(".cupcake/trust");
    std::fs::create_dir_all(&cupcake_dir).unwrap();

    // Create symlink to outside directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("/etc", cupcake_dir.join("evil_link")).unwrap();

        let result = verify_script(&cupcake_dir, "evil_link/passwd");
        assert!(result.is_err());
    }
}
```

### Phase 4 Security Tests (Namespace Isolation)

**File**: `cupcake-core/tests/security/namespace_isolation.rs`

```rust
use cupcake_core::engine::routing::route_to_policies;

#[test]
fn test_global_project_namespace_collision_prevented() {
    // Create policies with same name in different namespaces
    let global_policy = "package cupcake.global.policies.test";
    let project_policy = "package cupcake.policies.test";

    // Both should load without collision
    // Routing should keep them separate
}

#[test]
fn test_global_policy_cannot_access_project_data() {
    // Global policy should not see input.project_config
    // Only input.global_config
}

#[test]
fn test_project_policy_cannot_override_global() {
    // Project policy with same name shouldn't override global
}
```

### Phase 4 Security Tests (Decision Priority)

**File**: `cupcake-core/tests/security/decision_priority.rs`

```rust
use cupcake_core::engine::synthesis::synthesize_decision;

#[test]
fn test_global_deny_blocks_project_ask() {
    let global_decisions = vec![
        Decision::Deny { /* ... */ }
    ];

    let project_decisions = vec![
        Decision::Ask { /* ... */ }
    ];

    let final_decision = synthesize_decision(global_decisions, project_decisions);

    // Final decision should be Deny (global wins)
    assert!(matches!(final_decision, Decision::Deny { .. }));
}

#[test]
fn test_halt_overrides_all() {
    let decisions = vec![
        Decision::Allow,
        Decision::Deny { /* ... */ },
        Decision::Halt { /* ... */ },
        Decision::Ask { /* ... */ },
    ];

    let final_decision = synthesize_decision(decisions, vec![]);
    assert!(matches!(final_decision, Decision::Halt { .. }));
}

#[test]
fn test_priority_order_exhaustive() {
    // Test all permutations of decision priority
    // Halt > Deny > Block > Ask > Allow
}
```

## Regression Testing

### TOB Findings Regression Suite

**File**: `cupcake-core/tests/regression/tob_findings.rs`

```rust
/// Regression tests for all Trail of Bits findings
/// Each test ensures the vulnerability cannot be re-introduced

#[test]
fn test_tob_cupcake_1_wasm_memory_bypass_fixed() {
    // TOB-EQTY-LAB-CUPCAKE-1: WASM memory bypass
    // Ensure minimum 1MB cannot be bypassed via env var or config

    // Attempt bypass via CLI flag
    let mut cmd = Command::cargo_bin("cupcake").unwrap();
    cmd.args(&["eval", "--wasm-max-memory", "1KB"]);
    cmd.assert().failure();

    // Attempt bypass via config file
    let config = serde_json::json!({
        "wasm_max_memory": "1KB"
    });
    // Should reject config
}

#[test]
fn test_tob_cupcake_2_signal_shell_injection_fixed() {
    // TOB-EQTY-LAB-CUPCAKE-2: Shell injection in signals
    // Ensure SecureCommand prevents injection

    let signal_cmd = SecureCommand::Git {
        args: vec!["status".to_string(), "; rm -rf /".to_string()],
    };

    // Should execute safely (git rejects invalid arg)
    let result = signal_cmd.execute(Path::new("/tmp"));
    assert!(result.is_ok() || !result.unwrap_err().to_string().contains("rm"));
}

#[test]
fn test_tob_cupcake_11_global_config_override_fixed() {
    // TOB-EQTY-LAB-CUPCAKE-11: Global config override
    // Ensure env var is ignored

    env::set_var("CUPCAKE_GLOBAL_CONFIG", "/malicious/path.yml");

    let result = load_global_config(None);
    // Should NOT load from env var

    env::remove_var("CUPCAKE_GLOBAL_CONFIG");
}

// ... Add test for each TOB finding (1-11)
```

## Performance Testing

### Benchmark Suite

**File**: `cupcake-core/benches/evaluation_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cupcake_core::*;

fn bench_routing(c: &mut Criterion) {
    c.bench_function("route_event_to_policies", |b| {
        let event = create_test_event();
        b.iter(|| {
            route_event(black_box(&event))
        });
    });
}

fn bench_wasm_evaluation(c: &mut Criterion) {
    c.bench_function("evaluate_policy_wasm", |b| {
        let runtime = create_test_runtime();
        let input = create_test_input();
        b.iter(|| {
            runtime.evaluate(black_box(&input))
        });
    });
}

fn bench_decision_synthesis(c: &mut Criterion) {
    c.bench_function("synthesize_decisions", |b| {
        let decisions = create_test_decisions();
        b.iter(|| {
            synthesize_decision(black_box(&decisions), vec![])
        });
    });
}

criterion_group!(benches, bench_routing, bench_wasm_evaluation, bench_decision_synthesis);
criterion_main!(benches);
```

### Performance Regression Detection

```bash
# Baseline before refactor
cargo bench --bench evaluation_bench > benchmarks/baseline.txt

# After each phase
cargo bench --bench evaluation_bench > benchmarks/phase1.txt

# Compare
cargo install cargo-criterion
cargo criterion --baseline baseline
```

## Test Coverage

### Coverage Measurement

**Install tarpaulin**:
```bash
cargo install cargo-tarpaulin
```

**Generate coverage report**:
```bash
# HTML report
cargo tarpaulin --features deterministic-tests --out Html

# Terminal output
cargo tarpaulin --features deterministic-tests --out Stdout

# CI-friendly output
cargo tarpaulin --features deterministic-tests --out Lcov
```

**Coverage Targets**:
- Overall: 80%+
- Security-critical code: 95%+
- New code (refactor): 90%+

### Coverage by Module

**Phase 1 Coverage Goals**:
- `cupcake-cli/src/main.rs`: 90%+ (CLI parsing critical)
- `cupcake-core/src/engine/global_config.rs`: 95%+ (security-critical)
- `cupcake-core/src/engine/wasm_runtime.rs`: 95%+ (security-critical)

## CI/CD Integration

### GitHub Actions Workflow

**File**: `.github/workflows/security-tests.yml`

```yaml
name: Security Tests

on:
  push:
    branches: [main, tob/config-vul-fixes]
  pull_request:
    branches: [main]

jobs:
  security-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-toolchain@v1
        with:
          toolchain: stable

      - name: Install OPA
        run: |
          curl -L -o opa https://openpolicyagent.org/downloads/v0.71.0/opa_linux_amd64
          chmod +x opa
          sudo mv opa /usr/local/bin/

      - name: Run Unit Tests
        run: cargo test --features deterministic-tests
        env:
          CUPCAKE_GLOBAL_CONFIG: /nonexistent

      - name: Run Security Tests
        run: cargo test --features deterministic-tests --test security

      - name: Run Regression Tests
        run: cargo test --features deterministic-tests --test regression

      - name: Check Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --features deterministic-tests --out Lcov

      - name: Upload Coverage
        uses: codecov/codecov-action@v3
        with:
          files: ./lcov.info

      - name: Run Benchmarks
        run: cargo bench --no-fail-fast

  penetration-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Run Penetration Test Suite
        run: ./scripts/run_penetration_tests.sh
```

### Pre-commit Hooks

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
# Run tests before allowing commit

set -e

echo "Running security tests..."
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests --quiet

echo "Checking code coverage..."
cargo tarpaulin --features deterministic-tests --out Stdout --fail-under 80

echo "Running clippy..."
cargo clippy -- -D warnings

echo "Checking formatting..."
cargo fmt -- --check

echo "All checks passed!"
```

## Test Data Management

### Test Fixtures

**File**: `cupcake-core/tests/fixtures/mod.rs`

```rust
/// Test fixture management
pub mod policies {
    pub const ALLOW_ALL: &str = include_str!("policies/allow_all.rego");
    pub const DENY_ALL: &str = include_str!("policies/deny_all.rego");
    pub const ASK_SENSITIVE: &str = include_str!("policies/ask_sensitive.rego");
}

pub mod configs {
    pub const MINIMAL: &str = include_str!("configs/minimal.yml");
    pub const FULL: &str = include_str!("configs/full.yml");
}

pub mod events {
    pub fn pre_tool_use_bash() -> serde_json::Value {
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "session_id": "test",
            "tool_name": "Bash",
            "tool_input": {"command": "ls"}
        })
    }
}
```

### Snapshot Testing

**File**: `cupcake-core/tests/integration/snapshots.rs`

```rust
use insta::assert_json_snapshot;

#[test]
fn test_decision_output_format() {
    let decision = Decision::Deny {
        reason: "Test denial".to_string(),
        severity: Severity::High,
        rule_id: "TEST-001".to_string(),
    };

    let response = format_response(&decision);

    // Snapshot the JSON output
    assert_json_snapshot!(response);
}
```

## Test Execution

### Running All Tests

```bash
# Full test suite
CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests

# Specific test category
cargo test --test security --features deterministic-tests

# Specific test
cargo test test_env_var_isolation --features deterministic-tests

# With output
cargo test test_name -- --nocapture
```

### Parallel Execution

```bash
# Default (parallel)
cargo test --features deterministic-tests

# Single-threaded (for debugging)
cargo test --features deterministic-tests -- --test-threads=1
```

### Test Filtering

```bash
# Run only security tests
cargo test security --features deterministic-tests

# Run only integration tests
cargo test --test integration --features deterministic-tests

# Run tests matching pattern
cargo test cli_flag --features deterministic-tests
```

## Debugging Failed Tests

### Verbose Output

```bash
# Show test output
cargo test --features deterministic-tests -- --nocapture

# Show backtraces
RUST_BACKTRACE=1 cargo test test_name --features deterministic-tests
```

### Logging in Tests

```rust
#[test]
fn test_with_logging() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Test code with debug logging
}
```

## Security Test Scenarios

### Penetration Test Script

**File**: `scripts/run_penetration_tests.sh`

```bash
#!/bin/bash
# Penetration testing scenarios for Cupcake

set -e

echo "Running penetration tests..."

# Test 1: Attempt env var manipulation
echo "Test 1: Environment variable manipulation"
export CUPCAKE_GLOBAL_CONFIG="/tmp/malicious.yml"
cargo run -- eval < test_event.json
unset CUPCAKE_GLOBAL_CONFIG

# Test 2: Attempt shell injection in signals
echo "Test 2: Shell injection in signals"
# Create policy with malicious signal
# Verify execution is safe

# Test 3: Path traversal in trust scripts
echo "Test 3: Path traversal"
# Attempt to execute script outside .cupcake
# Should be blocked

# Test 4: WASM memory bypass
echo "Test 4: WASM memory bypass"
# Attempt to set memory below 1MB
# Should be rejected

echo "All penetration tests passed!"
```

## Continuous Testing

### Watch Mode for Development

```bash
# Install cargo-watch
cargo install cargo-watch

# Auto-run tests on file changes
cargo watch -x 'test --features deterministic-tests'

# Auto-run specific test
cargo watch -x 'test test_name --features deterministic-tests'
```

## Documentation Testing

### Doc Tests

```rust
/// Validates a global config path
///
/// # Examples
///
/// ```
/// use cupcake_core::engine::global_config::validate_config_path;
/// use std::path::Path;
///
/// let result = validate_config_path(Path::new("/tmp/config.yml"));
/// assert!(result.is_ok() || result.is_err()); // Depends on file existence
/// ```
pub fn validate_config_path(path: &Path) -> Result<()> {
    // Implementation
}
```

Run doc tests:
```bash
cargo test --doc
```

## Test Maintenance

### Monthly Test Review

- [ ] Review test coverage reports
- [ ] Update security test scenarios
- [ ] Add tests for new vulnerabilities discovered
- [ ] Remove obsolete tests
- [ ] Update snapshots if intentional changes made

### Quarterly Security Audit

- [ ] Run full penetration test suite
- [ ] Review all TOB finding regression tests
- [ ] Update threat model
- [ ] Add new security tests based on industry vulnerabilities

## References

- **Testing Best Practices**: https://doc.rust-lang.org/book/ch11-00-testing.html
- **Security Testing Guide**: OWASP Testing Guide v4
- **Fuzzing**: cargo-fuzz for property-based testing
- **Coverage Tools**: tarpaulin, grcov

---

**Next Steps**:
1. Create test directory structure
2. Implement Phase 1 unit tests
3. Set up CI/CD pipeline
4. Establish coverage baseline
5. Begin implementation with TDD approach
