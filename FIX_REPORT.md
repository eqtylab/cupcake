# Security Fix Report: Trust System Parser Unification

## Executive Summary
Fixed a critical security vulnerability where the trust system used a different parser than the runtime engine, causing it to miss auto-discovered scripts that would be executed.

## The Issue

### Root Cause
Two separate parser implementations existed:
1. **Engine Parser** (`engine/guidebook.rs`) - Full-featured with auto-discovery
2. **Trust Parser** (`trust/guidebook.rs`) - Basic YAML-only parser

### Security Impact
- Trust system only saw scripts explicitly listed in `guidebook.yml`
- Engine executed additional scripts from `signals/` and `actions/` directories
- `on_any_denial` actions were invisible to trust system
- **Result**: Unverified scripts could execute at runtime

## The Fix

### Code Changes
**File**: `cupcake-cli/src/trust_cli.rs` (79 additions, 15 deletions)

**Key Changes**:
1. Replaced import: `trust::guidebook::Guidebook` → `engine::guidebook::Guidebook`
2. Changed from `load()` to `load_with_conventions()` 
3. Added handling for `on_any_denial` actions
4. Added handling for `by_rule_id` action structure

### Before vs After

**Before** (line 111):
```rust
let guidebook = crate::trust::guidebook::Guidebook::load(project_dir)
```

**After** (line 116):
```rust
let guidebook = Guidebook::load_with_conventions(
    &guidebook_path,
    &signals_dir,    // Now scans this directory
    &actions_dir     // Now scans this directory
).await
```

## Verification

### Test Setup Created
- Directory: `test-trust/` with mixed explicit and auto-discovered scripts
- Explicit in YAML: 1 signal, 2 actions
- Auto-discovered: 1 signal, 1 action in directories

### Test Results
✅ **Before Fix**: Trust saw only 1 script
✅ **After Fix**: Trust sees all 5 scripts including:
- Auto-discovered `auto_signal` from `signals/`
- Auto-discovered `RULE-002` from `actions/`
- `on_any_denial` global action

## Artifacts Created

1. **PARSER_ANALYSIS.md** - Detailed vulnerability analysis
2. **TEST_TRUST_FIX.md** - Test procedures and verification steps
3. **test-trust/** - Test environment with sample scripts
4. **FIX_REPORT.md** - This report

## Recommendation

**IMMEDIATE**: This fix should be deployed as it closes a critical security gap.

**FUTURE**: Consider removing the unused `trust/guidebook.rs` parser entirely to prevent regression.