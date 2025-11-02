# Input Preprocessing and Path Enrichment

## Overview

Cupcake automatically **preprocesses all input** before policy evaluation to provide defense-in-depth against adversarial attack patterns. This preprocessing happens transparently inside the Engine—policies receive enriched, normalized input without needing to perform their own validation.

**Key Benefits:**
- 🛡️ **Automatic Security**: All policies protected from TOB-3 and TOB-4 attacks
- 🎯 **Simplified Policies**: No need to handle symlinks, whitespace, or path canonicalization
- 🔒 **Universal Protection**: CLI, FFI bindings, tests—all paths protected
- ⚡ **Fast**: <0.1% overhead (~30-100μs per evaluation)

## Architecture: Self-Defending Engine

```
┌──────────────────────────────────────────────────────────┐
│                   Engine.evaluate()                       │
│                                                           │
│  Step 0: ALWAYS PREPROCESS (Automatic)                  │
│  ├─ Normalize whitespace (TOB-3 defense)                │
│  ├─ Canonicalize file paths (TOB-4 defense)             │
│  ├─ Resolve symlinks                                     │
│  └─ Inspect scripts (if enabled)                         │
│                                                           │
│  Step 1: Route to relevant policies                      │
│  Step 2: Gather signals                                  │
│  Step 3: Evaluate policies (with enriched input)         │
│  Step 4: Synthesize decision                             │
└──────────────────────────────────────────────────────────┘
```

**Critical Design Principle**: The Engine never accepts raw, unpreprocessed input. This provides defense-in-depth by ensuring ALL integration paths (CLI, FFI, tests, future integrations) are automatically protected.

## What Gets Added to Events

### Single-File Tools (Edit, Write, Read, NotebookEdit)

**Original Event:**
```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "config.json"
  },
  "cwd": "/home/user/project"
}
```

**After Preprocessing (Enriched):**
```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "config.json"
  },
  "cwd": "/home/user/project",

  // ⭐ ADDED BY PREPROCESSING:
  "resolved_file_path": "/home/user/project/config.json",  // Canonical absolute path
  "original_file_path": "config.json",                     // Original for reference
  "is_symlink": false                                       // Symlink detection
}
```

### Symlink Detection

**When a file is a symlink:**
```json
{
  "tool_input": {
    "file_path": "safe_link.txt"  // Actually points to .cupcake/secrets.txt
  },
  "cwd": "/tmp",

  // Preprocessing reveals the real target:
  "resolved_file_path": "/home/user/project/.cupcake/secrets.txt",  // ✅ Actual target
  "original_file_path": "safe_link.txt",                            // Link name
  "is_symlink": true                                                 // ⚠️ Warning flag
}
```

### MultiEdit (Array-Based Operations)

**Original:**
```json
{
  "tool_name": "MultiEdit",
  "tool_input": {
    "edits": [
      {"file_path": "file1.txt", "old_string": "foo", "new_string": "bar"},
      {"file_path": "link.txt", "old_string": "x", "new_string": "y"}
    ]
  }
}
```

**After Preprocessing (Each Edit Enriched):**
```json
{
  "tool_name": "MultiEdit",
  "tool_input": {
    "edits": [
      {
        "file_path": "file1.txt",
        "resolved_file_path": "/home/user/project/file1.txt",
        "original_file_path": "file1.txt",
        "is_symlink": false,
        "old_string": "foo",
        "new_string": "bar"
      },
      {
        "file_path": "link.txt",
        "resolved_file_path": "/home/user/project/.cupcake/policies/evil.rego",  // ⚠️
        "original_file_path": "link.txt",
        "is_symlink": true,  // ⚠️ Detected!
        "old_string": "x",
        "new_string": "y"
      }
    ]
  }
}
```

### Pattern-Based Tools (Glob, Grep)

**Important**: Glob and Grep use **patterns**, not file paths. Patterns cannot be canonicalized.

**Glob Example:**
```json
{
  "tool_name": "Glob",
  "tool_input": {
    "pattern": ".cupcake/**/*.rego"  // This is NOT a file path
  },
  // NO resolved_file_path added (pattern can't be canonicalized)
}
```

**Grep Example:**
```json
{
  "tool_name": "Grep",
  "tool_input": {
    "pattern": "secret",
    "path": ".cupcake/"  // Directory to search
  },
  // NO resolved_file_path added
}
```

**Policy Handling**: Policies must fall back to checking raw `pattern`/`path` fields for these tools. See [Policy Patterns](#policy-patterns) below.

## Defense Mechanisms

### TOB-3: Whitespace Normalization

**Attack Vector**: Bypass policies by adding extra spaces

```bash
# Attacker tries:
rm  -rf  .cupcake  # Double spaces

# Policy checks:
input.tool_input.command == "rm -rf .cupcake"  # Won't match!
```

**Defense**: Preprocessing normalizes all whitespace in Bash commands:
```rego
# Policy receives:
input.tool_input.command == "rm -rf .cupcake"  # ✅ Normalized
```

**Details**: See `cupcake-core/src/preprocessing/normalizers.rs`

### TOB-4: Symlink Resolution

**Attack Vector**: Create symlinks to bypass path-based protections

```bash
# Attacker creates symlink:
ln -s .cupcake/policies/ /tmp/innocent_dir

# Then writes through the symlink:
Write("/tmp/innocent_dir/evil.rego", malicious_content)

# Old policy checks: "/tmp/innocent_dir/evil.rego" - doesn't contain ".cupcake/" - BYPASS! ❌
```

**Defense**: Preprocessing canonicalizes ALL file paths:
```rego
# Policy receives:
input.resolved_file_path == "/home/user/project/.cupcake/policies/evil.rego"  # ✅ Detected!
input.is_symlink == true  # ⚠️ Warning
```

**Coverage**:
- ✅ Direct symlinks
- ✅ Symlink chains (A → B → C)
- ✅ Dangling symlinks (target doesn't exist yet)
- ✅ Relative paths (`../../.cupcake/`)
- ✅ Multi-platform (Unix, Linux, macOS, Windows)

**Details**: See `cupcake-core/src/preprocessing/symlink_resolver.rs`

### TOB-2: Script Inspection (Opt-In)

**Attack Vector**: Hide malicious commands in external scripts

```bash
# Innocent looking:
bash ./deploy.sh

# But deploy.sh contains:
rm -rf .cupcake
```

**Defense**: When enabled, preprocessing reads script content:
```json
{
  "tool_input": {
    "command": "bash ./deploy.sh"
  },
  // Added by preprocessing:
  "executed_script_path": "/home/user/project/deploy.sh",
  "executed_script_content": "#!/bin/bash\nrm -rf .cupcake\n..."
}
```

**Status**: Opt-in feature (performance cost ~1ms per script)

**Configuration**:
```rust
let config = PreprocessConfig::with_script_inspection();
```

**Details**: See `cupcake-core/src/preprocessing/script_inspector.rs`

## Policy Patterns

### Basic: Single-File Protection

```rego
# Protect .cupcake/ from write operations
halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name in {"Edit", "Write", "NotebookEdit"}

    # Just use the canonical path - preprocessing guarantees it exists!
    file_path := input.resolved_file_path

    # Check if trying to modify .cupcake/
    startswith(lower(file_path), ".cupcake/")

    decision := {
        "rule_id": "PROTECT-CUPCAKE",
        "reason": concat("", ["Blocked write to ", file_path]),
        "severity": "HIGH"
    }
}
```

### Advanced: MultiEdit Support

```rego
# Protect .cupcake/ from MultiEdit operations
halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "MultiEdit"

    # Check each edit in the array
    some edit in input.tool_input.edits
    file_path := edit.resolved_file_path  # On each edit object!

    # Check if THIS edit targets .cupcake/
    startswith(lower(file_path), ".cupcake/")

    decision := {
        "rule_id": "PROTECT-CUPCAKE",
        "reason": concat("", ["Blocked MultiEdit write to ", file_path]),
        "severity": "HIGH"
    }
}
```

### Handling Pattern-Based Tools

```rego
# Protect .cupcake/ from ALL tools (including Glob/Grep)
halt contains decision if {
    input.hook_event_name == "PreToolUse"

    # Get file path with smart fallback
    file_path := get_file_path_with_fallback
    file_path != ""

    # Check for .cupcake/ reference
    contains(lower(file_path), ".cupcake/")

    decision := {
        "rule_id": "PROTECT-CUPCAKE",
        "reason": concat("", ["Blocked access to ", file_path]),
        "severity": "HIGH"
    }
}

# Helper: Prefer canonical path, fall back to raw input
get_file_path_with_fallback := path if {
    # First choice: Canonical path from preprocessing
    path := input.resolved_file_path
    path != null
} else := path if {
    # Fallback: For Glob/Grep, check raw pattern/path fields
    path := input.tool_input.pattern
} else := path if {
    path := input.tool_input.path
} else := ""
```

### Symlink Awareness

```rego
# Extra validation for symlinks
halt contains decision if {
    input.tool_name == "Write"
    file_path := input.resolved_file_path

    # Check if it's a symlink
    input.is_symlink == true

    # Extra strict: Block ALL symlink writes to sensitive areas
    sensitive_dirs := {".cupcake/", ".git/", ".env"}
    some dir in sensitive_dirs
    contains(lower(file_path), dir)

    decision := {
        "rule_id": "BLOCK-SYMLINK-WRITE",
        "reason": concat("", [
            "Blocked symlink write: ",
            input.original_file_path,  // Show the link name
            " → ",
            file_path  // Show the real target
        ]),
        "severity": "CRITICAL"
    }
}
```

## Performance Characteristics

| Operation | Time | Impact |
|-----------|------|--------|
| Whitespace normalization | <1μs | Negligible |
| Path canonicalization | ~15μs | Negligible |
| Symlink detection | ~15μs | Negligible |
| **Total preprocessing** | **~30-100μs** | **<0.1% of eval time** |
| Policy evaluation | 10-100ms | Dominant cost |

**Verdict**: Preprocessing overhead is unmeasurable in practice.

## Configuration

### Default (Recommended)

```rust
use cupcake_core::preprocessing::PreprocessConfig;

// Uses recommended defaults:
// - Whitespace normalization: ON
// - Symlink resolution: ON
// - Script inspection: OFF (performance)
// - Audit logging: ON
let config = PreprocessConfig::default();
```

### Custom Configuration

```rust
use cupcake_core::preprocessing::PreprocessConfig;

let config = PreprocessConfig {
    normalize_whitespace: true,         // TOB-3 defense
    enable_symlink_resolution: true,    // TOB-4 defense
    enable_script_inspection: false,    // Opt-in (TOB-2)
    audit_transformations: true,        // Log all changes
};
```

### Available Presets

```rust
// Full security (all defenses enabled)
PreprocessConfig::debug()

// Security-focused (default defenses only)
PreprocessConfig::default()

// Minimal (just normalization)
PreprocessConfig::minimal()

// With script inspection (TOB-2 defense)
PreprocessConfig::with_script_inspection()

// Disabled (testing only)
PreprocessConfig::disabled()
```

## Testing with Preprocessing

### Integration Tests

Preprocessing happens automatically in the Engine, so tests don't need to call it explicitly:

```rust
use cupcake_core::engine::Engine;
use cupcake_core::harness::types::HarnessType;

#[tokio::test]
async fn test_symlink_protection() -> Result<()> {
    let engine = Engine::new(policy_dir, HarnessType::ClaudeCode).await?;

    let event = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "link_to_cupcake.txt",  // Symlink
            "content": "malicious"
        },
        "cwd": "/tmp"
    });

    // Preprocessing happens automatically in evaluate()
    let decision = engine.evaluate(&event, None).await?;

    // Policy should block based on resolved_file_path
    assert_matches!(decision, FinalDecision::Halt { .. });
    Ok(())
}
```

### Unit Testing Preprocessing

If you need to test preprocessing behavior in isolation:

```rust
use cupcake_core::preprocessing::{preprocess_input, PreprocessConfig};
use cupcake_core::harness::types::HarnessType;

#[test]
fn test_symlink_resolution() {
    let mut event = json!({
        "tool_name": "Write",
        "tool_input": {
            "file_path": "symlink.txt"
        },
        "cwd": "/tmp"
    });

    let config = PreprocessConfig::default();
    preprocess_input(&mut event, &config, HarnessType::ClaudeCode);

    // Check that enriched fields were added
    assert!(event.get("resolved_file_path").is_some());
    assert!(event.get("is_symlink").is_some());
}
```

## Comparison with Manual Validation

### Without Preprocessing (Old Way)

```rego
# Policy must handle EVERYTHING:
deny contains decision if {
    file_path := get_file_path  # Extract from various fields

    # Must canonicalize in Rego (IMPOSSIBLE - no filesystem access!)
    # Must handle symlinks (IMPOSSIBLE - no symlink detection in Rego!)
    # Must handle relative paths (COMPLEX - must track cwd)
    # Must normalize whitespace (TEDIOUS - many edge cases)

    # Finally check the path
    startswith(file_path, ".cupcake/")

    decision := {...}
}
```

**Problems:**
- ❌ Rego running in WASM has NO filesystem access
- ❌ Can't detect symlinks from Rego
- ❌ Can't canonicalize paths from Rego
- ❌ Every policy must duplicate this logic
- ❌ Easy to get wrong → security vulnerabilities

### With Preprocessing (New Way)

```rego
# Policy is simple and secure:
halt contains decision if {
    # Just grab the canonical path - it's guaranteed to be there!
    file_path := input.resolved_file_path

    # Check the path (symlinks already resolved, paths already canonical)
    startswith(lower(file_path), ".cupcake/")

    decision := {
        "rule_id": "PROTECT-CUPCAKE",
        "reason": concat("", ["Blocked write to ", file_path]),
        "severity": "HIGH"
    }
}
```

**Benefits:**
- ✅ Rust has full filesystem access
- ✅ Symlinks detected and resolved automatically
- ✅ Paths canonicalized automatically
- ✅ Every policy gets this protection for free
- ✅ Hard to get wrong → secure by default

## Implementation Details

### Source Files

- `cupcake-core/src/preprocessing/mod.rs` - Main preprocessing pipeline
- `cupcake-core/src/preprocessing/config.rs` - Configuration
- `cupcake-core/src/preprocessing/normalizers.rs` - Whitespace normalization (TOB-3)
- `cupcake-core/src/preprocessing/symlink_resolver.rs` - Path canonicalization (TOB-4)
- `cupcake-core/src/preprocessing/script_inspector.rs` - Script inspection (TOB-2)

### Integration Point

Preprocessing is called at the **very beginning** of `Engine.evaluate()`:

```rust
// cupcake-core/src/engine/mod.rs
pub async fn evaluate(
    &self,
    input: &Value,
    debug_capture: Option<&mut DebugCapture>,
) -> Result<decision::FinalDecision> {
    // STEP 0: ALWAYS PREPROCESS (Self-Defending Engine)
    let mut safe_input = input.clone();
    let preprocess_config = PreprocessConfig::default();
    preprocess_input(&mut safe_input, &preprocess_config, self.config.harness);

    // Continue with SAFE input...
    let event_name = safe_input.get("hook_event_name")...
}
```

### Why Inside the Engine?

**Decision Rationale**: Moving preprocessing inside the Engine (rather than requiring callers to preprocess) provides **defense-in-depth**:

1. **Impossible to Bypass**: All paths go through preprocessing
2. **Universal Protection**: CLI, FFI bindings, tests—all protected
3. **Future-Proof**: New integrations automatically secure
4. **Matches Documentation**: "Automatic protection at engine level"

**Before (Vulnerable)**:
```
CLI → preprocess() → engine.evaluate() ✅
FFI → engine.evaluate() ❌ VULNERABLE
```

**After (Secure)**:
```
CLI → engine.evaluate() → [auto-preprocess] ✅
FFI → engine.evaluate() → [auto-preprocess] ✅
```

## References

- [TOB-3 Fix Documentation](../../SECURITY_PREPROCESSING.md#tob-3-whitespace-bypass)
- [TOB-4 Fix Documentation](../../TOB4_IMPLEMENTATION_LOG.md)
- [Security Architecture](./security.md)
- [Policy Development Guide](../user-guide/writing-policies.md)

## See Also

- **For Policy Authors**: [Writing Secure Policies](../user-guide/writing-policies.md)
- **For Developers**: [Preprocessing Architecture](./architecture.md#preprocessing)
- **For Security**: [TOB Audit Fixes](./security.md#tob-audit-remediation)
