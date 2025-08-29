# Cupcake Metadata System

Based on thorough analysis of the actual code, here's the comprehensive understanding of how Cupcake's metadata system works:

## Metadata System Architecture

### Data Models

**RoutingDirective** (`src/engine/metadata.rs:17-29`):
```rust
pub struct RoutingDirective {
    pub required_events: Vec<String>,    // ["PreToolUse", "PostToolUse"] 
    pub required_tools: Vec<String>,     // ["Bash", "WebFetch", "*"]
    pub required_signals: Vec<String>,   // ["git_branch", "test_status"]
}
```

**PolicyMetadata** (`src/engine/metadata.rs:42-63`):
```rust
pub struct PolicyMetadata {
    pub scope: Option<String>,           // "rule", "document", "package"
    pub title: Option<String>,           // "Bash Security Guard"
    pub authors: Vec<String>,            // ["Security Team"]
    pub organizations: Vec<String>,      // ["Engineering", "DevOps"]
    pub custom: CustomMetadata,          // Cupcake-specific fields
}
```

**CustomMetadata** (`src/engine/metadata.rs:66-79`):
```rust
pub struct CustomMetadata {
    pub severity: Option<String>,        // "HIGH", "MEDIUM", "LOW"
    pub id: Option<String>,              // "BASH-001"
    pub routing: Option<RoutingDirective>, // The routing directive
}
```

### Metadata Parsing Process

1. **Extraction** (`src/engine/metadata.rs:104-136`):
   - Finds `# METADATA` comment blocks in Rego files
   - Strips `# ` prefixes to convert comments to YAML
   - Stops at first non-comment line

2. **Validation** (`src/engine/metadata.rs:140-189`):
   - **System policies** (`cupcake.system.*`): Must have empty routing (they're aggregation functions)
   - **Regular policies**: Can specify events/tools, but tools require events
   - **Validates against known Claude Code event types**: PreToolUse, PostToolUse, UserPromptSubmit, Stop, SubagentStop, Notification, PreCompact, SessionStart

### Routing System Implementation

**Key Generation** (`src/engine/routing.rs:13-36`):
- `PreToolUse` + `[]` → `["PreToolUse"]`
- `PreToolUse` + `["Bash"]` → `["PreToolUse:Bash"]` 
- `PreToolUse` + `["*"]` → `["PreToolUse:*"]`
- `PreToolUse` + `["Bash", "Shell"]` → `["PreToolUse:Bash", "PreToolUse:Shell"]`

**Routing Map Construction** (`src/engine/mod.rs:275-330`):
1. For each policy, generate all routing keys from metadata
2. Add policy to routing map for each key
3. **Wildcard expansion**: `PreToolUse:*` policies are also added to specific tool keys
4. Result: `HashMap<String, Vec<PolicyUnit>>` for O(1) lookups

**Event Matching** (`src/engine/mod.rs:343-350`):
```rust
pub fn route_event(&self, event_name: &str, tool_name: Option<&str>) -> Vec<&PolicyUnit> {
    let key = routing::create_event_key(event_name, tool_name); // "PreToolUse:Bash"
    self.routing_map.get(&key).map(|policies| policies.iter().collect()).unwrap_or_default()
}
```

## Signal Collection - PURELY Metadata-Driven

**Key Finding**: Signal collection is **entirely metadata-based**, not "intelligent discovery" via policy rules.

**Process** (`src/engine/mod.rs:415-453`):
1. **Collection**: Gather all `required_signals` from matched policies' routing directives
2. **Deduplication**: Use HashSet to collect unique signal names
3. **Execution**: Execute signals in parallel via `join_all`
4. **Enrichment**: Add results to input under `"signals"` key
5. **Trust**: All signal commands verified via HMAC before execution

**Example Flow**:
```
Event: PreToolUse:Bash
Matched Policies: [bash_guard, test_metadata]
Required Signals: ["git_branch", "test_status", "test_signal"]
→ Execute 3 signals in parallel
→ Enrich input: input.signals = { git_branch: "main", test_status: {...}, test_signal: "value" }
```

## Metadata Requirements and Semantics

### Required Fields
- **None are truly required** - all fields have `#[serde(default)]`
- **System policies**: Should have empty routing (validated at runtime)
- **Regular policies**: Should have `required_events` if they specify `required_tools`

### Optional Fields
- `scope`: OPA standard (`rule`, `document`, `package`, `subpackages`)
- `title`: Human-readable policy name
- `authors`: List of policy authors
- `organizations`: Responsible organizations  
- `severity`: `HIGH`, `MEDIUM`, `LOW`
- `id`: Unique policy identifier (e.g., `BASH-001`)
- `routing`: The core directive for Cupcake's routing

### Field Semantics

**required_events**: Which Claude Code hook events this policy handles
- Empty = never routed (except for system policies)
- `["PreToolUse"]` = only PreToolUse events
- `["PreToolUse", "PostToolUse"]` = both pre and post tool events

**required_tools**: Which tools this policy applies to within the specified events  
- Empty = all tools for the event
- `["Bash"]` = only Bash tool
- `["*"]` = explicit wildcard (same as empty, but intentional)
- `["Bash", "Shell"]` = multiple specific tools

**required_signals**: External data needed by this policy
- Empty = policy works with event data alone
- `["git_branch"]` = needs git branch info
- `["test_status", "git_branch"]` = needs multiple signals

## WASM Compilation Relationship

The metadata **does not affect WASM compilation**. The engine:
1. Compiles **all policies** into a single WASM module regardless of routing
2. Uses routing for **execution-time policy selection**  
3. Calls the single `cupcake.system.evaluate` entrypoint
4. The system policy uses `walk(data.cupcake.policies)` to discover all decision verbs

## Single Aggregation Entrypoint

**Key Insight**: The system uses `walk()` for automatic policy discovery within WASM, while metadata enables **pre-filtering** at the host level.

```rego
evaluate := decision_set if {
    decision_set := {
        "halts": collect_verbs("halt"),
        "denials": collect_verbs("deny"),
        "blocks": collect_verbs("block"),
        "asks": collect_verbs("ask"),
        "allow_overrides": collect_verbs("allow_override"),  
        "add_context": collect_verbs("add_context")
    }
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    # ... flatten and return
}
```

## Summary

Cupcake's metadata system is a **declarative routing system** that enables O(1) policy selection without changing WASM compilation. Signal collection is purely metadata-driven, not intelligent discovery. The system maintains simplicity by using standard OPA metadata while adding Cupcake-specific routing semantics.