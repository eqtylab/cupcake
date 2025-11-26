# Routing Analysis Report

## Executive Summary

**Routing does NOT control which Rego policies execute in WASM.** All policies compiled into the WASM module execute every time. Routing serves two actual purposes:

1. **Early exit optimization**: Skip WASM evaluation entirely when no policies are relevant
2. **Signal collection gating**: Only collect signals for "matched" policies

The narrative that routing "routes events to certain Rego policies" is misleading. The Rego policies themselves contain `input.hook_event_name == "..."` checks that determine whether they produce decisions.

---

## Evidence from Code Tracing

### 1. WASM Compilation: All Policies, Single Entrypoint

**File**: `cupcake-core/src/engine/compiler.rs:111-117`

```rust
pub async fn compile_policies(
    policies: &[PolicyUnit],
    opa_path_override: Option<PathBuf>,
) -> Result<Vec<u8>> {
    compile_policies_with_namespace(policies, "cupcake.system", opa_path_override).await
}
```

**Finding**: The compiler takes ALL discovered policies and compiles them into ONE WASM module with ONE entrypoint (`cupcake.system.evaluate`). The routing metadata has zero effect on what gets compiled.

**File**: `cupcake-core/src/engine/compiler.rs:246-249`

```rust
// Add the single aggregation entrypoint for the Hybrid Model
let entrypoint = format!("{}/evaluate", namespace.replace('.', "/"));
opa_cmd.arg("-e").arg(&entrypoint);
```

### 2. System Evaluate: Walks ALL Policies

**File**: `fixtures/claude/system/evaluate.rego:34-48`

```rego
collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    ...
}
```

**Finding**: The `walk(data.cupcake.policies, ...)` traverses the ENTIRE policy tree. There is no filtering by routing metadata. Every policy in the compiled WASM is evaluated.

### 3. Policies Filter Themselves

**File**: `fixtures/claude/builtins/protected_paths.rego:19-24`

```rego
halt contains decision if {
    input.hook_event_name == "PreToolUse"      # <-- Policy checks event itself
    single_file_tools := {"Edit", "Write", "NotebookEdit"}
    input.tool_name in single_file_tools        # <-- Policy checks tool itself
    ...
}
```

**Finding**: Policies contain their own `input.hook_event_name ==` and `input.tool_name in` checks. The routing metadata in comments is decorative - the filtering happens in Rego logic.

### 4. What Routing Actually Does

**File**: `cupcake-core/src/engine/mod.rs:1010-1016`

```rust
if matched_policies.is_empty() {
    info!("No policies matched for this event - allowing");
    return Ok(decision::FinalDecision::Allow { context: vec![] });
}
```

**Finding #1**: If routing matches zero policies, WASM is NEVER called. This is an optimization, not selective execution.

**File**: `cupcake-core/src/engine/mod.rs:1300-1306`

```rust
// Collect all unique required signals from matched policies
let mut required_signals = std::collections::HashSet::new();
for policy in matched_policies {
    for signal_name in &policy.routing.required_signals {
        required_signals.insert(signal_name.clone());
    }
}
```

**Finding #2**: Routing determines which signals to collect. Only signals from "matched" policies are gathered. This prevents running expensive signal commands (arbitrary shell programs) when they're not needed.

---

## What Routing IS

1. **Early exit gate**: If `route_event()` returns empty, skip WASM entirely and return `Allow`
2. **Signal collection optimizer**: Only execute signals declared in `required_signals` for matched policies
3. **Debugging aid**: The routing map can be dumped for visualization (`--debug-routing`)

## What Routing IS NOT

1. **Policy selector**: Cannot selectively execute certain Rego rules
2. **WASM filter**: Cannot compile or load a subset of policies
3. **Event router to specific policies**: Policies always self-filter via their own Rego conditions

---

## The Metadata Paradox

The `# METADATA` blocks in policies declare:
```yaml
routing:
  required_events: ["PreToolUse"]
  required_tools: ["Bash"]
```

This metadata is parsed at Rust level and used for:
- Building the routing map (for early exit + signal gathering)
- NOT for controlling WASM execution

The policies ALSO contain the same logic in Rego:
```rego
input.hook_event_name == "PreToolUse"
input.tool_name == "Bash"
```

This creates redundancy - the routing metadata mirrors what the policy already enforces internally.

---

## Signal Collection: The Real Value

The only meaningful runtime effect of routing is signal collection:

```rust
// From gather_signals():
for policy in matched_policies {
    for signal_name in &policy.routing.required_signals {
        required_signals.insert(signal_name.clone());
    }
}
```

Signals are arbitrary shell commands that enrich the input:
- Git branch status
- File validation results
- External API calls

Without routing, you would either:
1. Run ALL signals always (expensive)
2. Have no way to know which signals to run

Routing provides the mapping: "For PreToolUse:Bash events, collect these specific signals."

---

## Recommendations

### Keep
- Signal collection gating via `required_signals`
- Early exit optimization (skip WASM when no policies match)
- Debug routing visualization

### Clarify in Documentation
- Routing does NOT control Rego execution
- All compiled policies execute; they self-filter
- Routing's purpose is signal optimization + early exit

### Consider Simplifying
The `required_events` and `required_tools` metadata is essentially duplicating what the Rego policy already checks. Consider:
1. Removing the metadata-based routing for events/tools
2. Keeping only `required_signals` in metadata
3. Documenting that policies must self-filter (which they already do)

Or alternatively, trust the metadata and remove the redundant Rego checks - but this is risky as the WASM has no access to routing data.

---

## Conclusion

Routing is an **optimization layer**, not a **routing layer**. It prevents unnecessary signal collection and provides an early exit when no policies could possibly match. The name "routing" is misleading - "signal gating" or "evaluation gating" would be more accurate.

The Rego policies are the source of truth for event/tool filtering. They contain explicit checks that run every WASM evaluation. The routing metadata is metadata used by the Rust host, not by the Rego runtime.
