# Pre-Implementation Checklist

**Date**: 2025-10-06
**Purpose**: Final preparation checklist before starting security refactor
**Related**: `SECURITY_REFACTOR_ACTION_PLAN.md`, `BASELINE_CODEBASE_STATE.md`

## Executive Summary

This checklist ensures all preparation work is complete before beginning implementation of the security refactor. Complete all items before starting Phase 1.

## Documentation Review

### Refactor Planning Documents

- [ ] **SECURITY_REFACTOR_ACTION_PLAN.md** reviewed and understood
  - 5 phases clearly defined
  - Timeline understood (4-6 weeks)
  - All 11 Trail of Bits findings addressed
  - No backward compatibility constraints confirmed

- [ ] **PHASE1_IMPLEMENTATION_GUIDE.md** reviewed
  - Day-by-day breakdown understood
  - All 6 tasks identified
  - Code templates reviewed
  - Acceptance criteria clear

- [ ] **IMPLEMENTATION_TRACKER.md** ready for use
  - Task breakdown reviewed
  - Finding-to-task mapping verified
  - Ready to track progress

### Reference Documentation

- [ ] **BASELINE_CODEBASE_STATE.md** reviewed
  - 22,579 lines of code baseline noted
  - 18 env::var occurrences documented
  - 0 bash -c occurrences confirmed
  - File locations for modification identified

- [ ] **TESTING_SETUP_GUIDE.md** reviewed
  - Test pyramid structure understood
  - Unit test templates reviewed
  - Security test scenarios identified
  - CI/CD configuration planned

- [ ] **ENVIRONMENT_VARIABLES.md** reviewed
  - All 30 variables documented
  - 7 deprecated variables identified
  - Migration path understood

### Security Audit Materials

- [ ] **ENV_VAR_VULNERABILITIES.md** reviewed
  - 3 initial findings understood
  - Root causes identified

- [ ] **Trail of Bits PDF Audit** reviewed
  - All 11 findings read and understood
  - Severity levels noted
  - Finding IDs memorized (TOB-EQTY-LAB-CUPCAKE-1 through 11)

## Environment Setup

### Development Tools

- [ ] **Rust toolchain** verified
  ```bash
  rustc --version  # Should be 1.75.0 or later
  cargo --version
  ```

- [ ] **OPA** installed and verified
  ```bash
  opa version  # Should be v0.71.0 or later
  ```

- [ ] **Testing tools** installed
  ```bash
  cargo install cargo-tarpaulin  # Code coverage
  cargo install cargo-watch      # Auto-testing
  cargo install cargo-criterion  # Benchmarking
  ```

- [ ] **Code quality tools** ready
  ```bash
  cargo clippy --version
  cargo fmt --version
  ```

### Repository State

- [ ] **Clean working directory**
  ```bash
  git status  # Should show no uncommitted changes
  ```

- [ ] **Current branch** verified
  - On `tob/config-vul-fixes` branch
  - Or create new feature branch for implementation

- [ ] **Backup branch** created
  ```bash
  git checkout -b backup/pre-refactor-2025-10-06
  git checkout tob/config-vul-fixes
  ```

- [ ] **All tests passing** (baseline)
  ```bash
  CUPCAKE_GLOBAL_CONFIG=/nonexistent cargo test --features deterministic-tests
  ```

### Baseline Metrics

- [ ] **Code coverage** baseline established
  ```bash
  cargo tarpaulin --features deterministic-tests --out Html
  # Document coverage percentage in BASELINE_CODEBASE_STATE.md
  ```

- [ ] **Performance benchmarks** baseline established
  ```bash
  cargo bench --bench evaluation_bench > benchmarks/baseline_2025-10-06.txt
  ```

- [ ] **CLI help output** documented
  ```bash
  cargo run -- eval --help > docs/cli-help-before-refactor.txt
  ```

## Code Audit

### Environment Variable Usage

- [ ] **Run audit script**
  ```bash
  ./scripts/audit_env_vars.sh
  ```

- [ ] **Verify 18 env::var occurrences** match documented locations:
  - cupcake-cli/src/main.rs (2)
  - cupcake-core/src/engine/global_config.rs (3)
  - cupcake-core/src/engine/wasm_runtime.rs (1)
  - cupcake-core/src/debug.rs (1)
  - cupcake-core/src/engine/routing_debug.rs (1)
  - cupcake-core/src/engine/compiler.rs (1)
  - cupcake-core/src/debug/tests.rs (1)
  - cupcake-core/tests/claude_code_routing_test.rs (6)
  - cupcake-core/tests/opa_lookup_test.rs (1)
  - cupcake-core/src/trust/hasher.rs (1)

- [ ] **No additional env::var calls** found beyond documented ones

### Shell Command Patterns

- [ ] **Verify zero bash -c usage**
  ```bash
  rg 'bash -c' --type rust
  # Should return no results
  ```

- [ ] **Check for other shell patterns**
  ```bash
  rg 'sh -c' --type rust
  rg '\.spawn.*sh' --type rust
  rg 'Command::new.*bash' --type rust
  ```

### Path Validation

- [ ] **Audit trust system path handling**
  - Review `cupcake-core/src/trust/verifier.rs`
  - Confirm no canonicalization currently exists
  - Identify locations needing path traversal checks

- [ ] **Audit config path handling**
  - Review `cupcake-core/src/engine/global_config.rs`
  - Identify validation points for Phase 1

## Dependencies

### Cargo Dependencies

- [ ] **Review Cargo.toml** workspace dependencies
  - clap (needed for CLI parsing) - **NOT YET ADDED**
  - wasmtime 35.0 verified
  - tokio 1.46.1 verified
  - Security deps (sha2, hmac, hex) verified

- [ ] **Plan clap addition** for Phase 1
  ```toml
  # Will need to add to cupcake-cli/Cargo.toml:
  clap = { version = "4.5", features = ["derive", "env"] }
  ```

### External Tools

- [ ] **OPA binary location** verified
  ```bash
  which opa
  # Document default path for OPA discovery
  ```

- [ ] **Git available** for trust system tests
  ```bash
  which git
  ```

## Testing Infrastructure

### Test Directory Structure

- [ ] **Create security test directory**
  ```bash
  mkdir -p cupcake-core/tests/security
  ```

- [ ] **Create integration test directory** (if needed)
  ```bash
  mkdir -p cupcake-core/tests/integration
  ```

- [ ] **Create regression test directory**
  ```bash
  mkdir -p cupcake-core/tests/regression
  ```

- [ ] **Create benchmark directory** (if not exists)
  ```bash
  mkdir -p cupcake-core/benches
  ```

### Test Fixtures

- [ ] **Review existing fixtures**
  - `fixtures/builtins/`
  - `fixtures/global_builtins/`
  - `fixtures/init/`

- [ ] **Plan new fixtures** for Phase 1 tests
  - Config files with various flag combinations
  - Invalid config files for validation tests
  - Test policies for integration tests

## Development Verification Tools

**Note**: Pre-release refactor - no user migration needed. These scripts verify our development environment only.

### Scripts Ready

- [ ] **verify_guidebooks.py** created and executable
  ```bash
  ./scripts/verify_guidebooks.py
  ```

- [ ] **audit_env_vars.sh** created and executable
  ```bash
  ./scripts/audit_env_vars.sh
  ```

- [ ] **verify_migration.sh** created and executable
  ```bash
  # Will be used after Phase 1 implementation
  ./scripts/verify_migration.sh
  ```

## Team Coordination

- [ ] **Team briefed** on security refactor plan
- [ ] **Timeline communicated** (4-6 weeks)
- [ ] **No backward compatibility** constraint confirmed (pre-release)
- [ ] **Review process** established for each phase

**Note**: No external user communication needed - pre-release refactor.

## Risk Assessment

### Rollback Plan

- [ ] **Backup branch** created (documented above)
- [ ] **Rollback procedure** documented:
  ```bash
  # If Phase 1 needs rollback:
  git checkout backup/pre-refactor-2025-10-06
  git checkout -b tob/config-vul-fixes-rollback
  git push origin tob/config-vul-fixes-rollback --force
  ```

- [ ] **Incremental commits** planned
  - Commit after each Phase 1 task (1.1, 1.2, etc.)
  - Tag after each phase completion
  - Enable selective rollback if needed

### Testing Strategy

- [ ] **Test-Driven Development** approach confirmed
  - Write tests first for each task
  - Implement feature to pass tests
  - Verify security properties

- [ ] **Continuous testing** enabled
  ```bash
  # Keep running during development:
  cargo watch -x 'test --features deterministic-tests'
  ```

### Known Risks

- [ ] **Risk: Breaking existing integrations** - Mitigated by comprehensive testing
- [ ] **Risk: Performance regression** - Mitigated by benchmarking
- [ ] **Risk: Incomplete env var removal** - Mitigated by audit script
- [ ] **Risk: Test flakiness** - Mitigated by deterministic test mode
- [ ] **Risk: WASM compilation issues** - Mitigated by OPA version verification

## Phase 1 Specific Preparation

### Task 1.1: CLI Flag Definitions

- [ ] **Clap dependency** plan confirmed
- [ ] **ValueEnum types** reviewed in PHASE1_IMPLEMENTATION_GUIDE.md
  - TraceModule enum
  - LogLevel enum
  - MemorySize struct with FromStr

- [ ] **Global flag strategy** understood
  - All flags available to all subcommands
  - Flag precedence documented

### Task 1.2: Global Config Validation

- [ ] **Validation requirements** documented
  - Must be absolute path
  - Must exist
  - Must be regular file
  - Must have .yml/.yaml extension

- [ ] **Error messages** planned
  - Clear, actionable error messages
  - Include suggested fixes

### Task 1.3: WASM Memory Configuration

- [ ] **1MB minimum** enforcement understood (TOB-EQTY-LAB-CUPCAKE-1)
- [ ] **100MB maximum** enforcement understood
- [ ] **Defense-in-depth** strategy confirmed (CLI validation + runtime clamping)

### Task 1.4: Tracing Initialization

- [ ] **Module-specific tracing** approach understood
- [ ] **EnvFilter construction** without env vars reviewed

### Task 1.5: Debug Flags

- [ ] **Boolean flags** for debug_files and debug_routing confirmed
- [ ] **Default false** behavior documented

### Task 1.6: OPA Path Discovery

- [ ] **Search strategy** defined:
  1. CLI flag override
  2. which opa
  3. Common installation paths
  4. Error if not found

- [ ] **Path validation** requirements identified

## CI/CD Preparation

### GitHub Actions

- [ ] **Existing CI** reviewed
- [ ] **Security test workflow** planned (from TESTING_SETUP_GUIDE.md)
- [ ] **Coverage reporting** integration planned

### Pre-commit Hooks

- [ ] **Pre-commit script** planned (from TESTING_SETUP_GUIDE.md)
- [ ] **Local git hooks** ready to install

## Final Checks

### Documentation Complete

- [x] SECURITY_REFACTOR_ACTION_PLAN.md
- [x] IMPLEMENTATION_TRACKER.md
- [x] PHASE1_IMPLEMENTATION_GUIDE.md
- [x] BASELINE_CODEBASE_STATE.md
- [x] TESTING_SETUP_GUIDE.md
- [x] PRE_IMPLEMENTATION_CHECKLIST.md (this file)

### Scripts Complete

- [x] scripts/verify_guidebooks.py
- [x] scripts/audit_env_vars.sh
- [x] scripts/verify_migration.sh

### Team Ready

- [ ] All documentation reviewed by team
- [ ] Questions answered
- [ ] Implementation approach approved
- [ ] Ready to begin Phase 1, Task 1.1

## Sign-off

Before proceeding to implementation:

- [ ] **Lead Developer** sign-off: _______________________
- [ ] **Security Reviewer** sign-off: _______________________
- [ ] **Project Manager** sign-off: _______________________

**Date implementation begins**: _______________________

## Next Steps

Once all items are checked:

1. **Run audit script one final time**:
   ```bash
   ./scripts/audit_env_vars.sh
   ```

2. **Commit all preparation documents**:
   ```bash
   git add SECURITY_REFACTOR_ACTION_PLAN.md \
           IMPLEMENTATION_TRACKER.md \
           PHASE1_IMPLEMENTATION_GUIDE.md \
           BASELINE_CODEBASE_STATE.md \
           TESTING_SETUP_GUIDE.md \
           PRE_IMPLEMENTATION_CHECKLIST.md \
           scripts/

   git commit -m "docs: Add security refactor preparation documents

   - Comprehensive action plan for 5-phase security refactor
   - Detailed Phase 1 implementation guide with code templates
   - Testing infrastructure setup guide
   - Baseline codebase state documentation
   - Development verification scripts (audit, verify)
   - Pre-implementation checklist

   Pre-release refactor - no backward compatibility constraints.
   Addresses TOB-EQTY-LAB-CUPCAKE-1 through 11.

   🤖 Generated with [Claude Code](https://claude.com/claude-code)

   Co-Authored-By: Claude <noreply@anthropic.com>"
   ```

3. **Create implementation branch** (if not already on one):
   ```bash
   git checkout -b feature/phase1-env-var-elimination
   ```

4. **Begin Phase 1, Task 1.1** per PHASE1_IMPLEMENTATION_GUIDE.md

5. **Update IMPLEMENTATION_TRACKER.md** as you complete tasks

---

**Preparation Status**: ⏳ In Progress

**Ready for Implementation**: ⬜ Not Yet / ✅ Ready

**Estimated Start Date**: _______________________
