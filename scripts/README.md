# Development Verification Scripts

**Purpose**: Verify security refactor implementation in development environment
**Context**: Pre-release refactor - no users to migrate

## Scripts Overview

### 1. audit_env_vars.sh

**Purpose**: Audit codebase for environment variable usage

**When to use**:
- Before starting Phase 1 (establish baseline)
- After completing Phase 1 (verify removal)
- Periodically during development

**What it does**:
- Searches Rust code for `env::var("CUPCAKE_*")` calls
- Identifies 7 deprecated environment variables
- Reports all occurrences with file/line numbers
- Ensures complete removal from codebase

**Usage**:
```bash
./scripts/audit_env_vars.sh
```

**Expected output**:
- Before Phase 1: Shows 18 env::var occurrences
- After Phase 1: Shows 0 occurrences (or only non-behavioral ones)

---

### 2. verify_guidebooks.py

**Purpose**: Verify guidebook.yml files don't reference deprecated env vars

**When to use**:
- Before starting refactor
- After updating any guidebook.yml files
- As part of test suite

**What it does**:
- Scans `.cupcake/guidebook.yml` and fixture files
- Looks for `$CUPCAKE_TRACE`, `${CUPCAKE_GLOBAL_CONFIG}`, etc.
- Reports any deprecated env var references in signals/actions

**Usage**:
```bash
# Scan all common locations
./scripts/verify_guidebooks.py

# Check specific file
./scripts/verify_guidebooks.py .cupcake/guidebook.yml
```

**Why this matters**:
Even after removing env vars from Rust code, guidebook files might
have signal/action commands that reference them. These would silently
stop working after the refactor.

---

### 3. verify_migration.sh

**Purpose**: Automated verification that Phase 1 implementation succeeded

**When to use**: After completing Phase 1 implementation

**What it does**:
- Tests that CLI flags exist (`--trace`, `--global-config`, etc.)
- Tests that environment variables are ignored
- Verifies CLI flag validation works (e.g., memory minimum)
- Runs full test suite
- Checks code quality (clippy, formatting)

**Usage**:
```bash
./scripts/verify_migration.sh
```

**Exit codes**:
- 0: All verification passed
- 1: Some tests failed

---

## Development Workflow

### Before Implementation (Pre-Phase 1)

```bash
# 1. Establish baseline
./scripts/audit_env_vars.sh
# Should show: 18 occurrences in 10 files

# 2. Verify guidebooks
./scripts/verify_guidebooks.py
# Should pass (no deprecated env var refs in configs)
```

### During Implementation (Phase 1)

Work through tasks in `PHASE1_IMPLEMENTATION_GUIDE.md`, running:

```bash
# Frequently check progress
./scripts/audit_env_vars.sh

# Watch for remaining env::var calls
```

### After Implementation (Post-Phase 1)

```bash
# 1. Final audit
./scripts/audit_env_vars.sh
# Should show: 0 occurrences (or only non-behavioral)

# 2. Comprehensive verification
./scripts/verify_migration.sh
# Should pass all tests

# 3. Final guidebook check
./scripts/verify_guidebooks.py
```

---

## What These Scripts Are NOT

❌ **Not for user migration** - There are no users (pre-release)
❌ **Not for migrating user configs** - No external configs exist
❌ **Not for production deployment** - Development tools only

## What These Scripts ARE

✅ **Development verification** - Ensure refactor is complete
✅ **Quality assurance** - Catch missed env var references
✅ **Automated testing** - Verify implementation correctness
✅ **Documentation** - Show what changed

---

## Integration with CI/CD

These scripts can be integrated into GitHub Actions:

```yaml
# .github/workflows/verify-refactor.yml
- name: Audit environment variables
  run: ./scripts/audit_env_vars.sh

- name: Verify guidebooks
  run: ./scripts/verify_guidebooks.py

- name: Full verification suite
  run: ./scripts/verify_migration.sh
```

---

## Questions?

See:
- `SECURITY_REFACTOR_ACTION_PLAN.md` - Overall refactor plan
- `PHASE1_IMPLEMENTATION_GUIDE.md` - Step-by-step Phase 1 guide
- `BASELINE_CODEBASE_STATE.md` - Pre-refactor snapshot
