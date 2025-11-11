# Policy Routing System

## Why Routing Matters

Cupcake doesn't evaluate every policy on every agent action. That would be slow and wasteful, especially when policies require external signals like git status checks or API calls.

Instead, Cupcake uses metadata-driven routing to know exactly which policies apply to each event. When Claude wants to run `rm -rf /`, Cupcake only evaluates policies that care about Bash commands, not policies for database access or file editing.

This selective evaluation keeps Cupcake fast. Sub-millisecond decisions with minimal overhead.

## How It Works

### Metadata Declaration

Every policy declares what it cares about using OPA metadata:

```rego
# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch"]
package cupcake.policies.deploy_protection

# This policy only runs for PreToolUse events with Bash tool
```

### Routing Keys

At startup, Cupcake builds a HashMap from metadata:

- `PreToolUse:Bash` → [policies that care about Bash commands]
- `PreToolUse:Edit` → [policies that care about file edits]
- `UserPromptSubmit` → [policies that inject context into prompts]

When an event arrives, Cupcake looks up the relevant key. O(1) performance.

### Wildcard Policies

Some policies care about all tools:

```rego
required_events: ["PreToolUse"]
required_tools: []  # Empty = all tools
```

These appear in the routing map under both the event-only key (`PreToolUse`) and get merged into all specific tool routes during initialization.

## Lifecycle

### 1. Startup: Build the Map

```
Scan .cupcake/policies/ → Parse metadata → Generate routing keys → Build HashMap
```

The engine reads every `.rego` file once, extracts metadata, and builds the routing map. Policies without proper metadata are rejected.

### 2. Runtime: Route Events

```
Event arrives → Extract event:tool → Lookup in HashMap → Evaluate matched policies
```

Example flow:
1. Claude sends `PreToolUse` event with `tool_name: "Bash"`
2. Engine looks up `PreToolUse:Bash` in routing map
3. Finds 5 policies that care about Bash commands
4. Only evaluates those 5 policies

### 3. Signal Optimization

Matched policies declare required signals. The engine:
1. Collects unique signals from matched policies
2. Executes all signals in parallel
3. Injects results into policy evaluation

No matched policies = no signals executed.

## Key Design Decisions

### Two-Phase Evaluation

Global policies evaluate first with early termination:
- Global denial → stop immediately
- No global issues → evaluate project policies

This lets organizations enforce security policies that can't be overridden locally.

### Single WASM Module

All policies compile into one WASM module with a single entrypoint. The routing happens outside WASM in the Rust engine. Policies don't know or care about routing.

### Builtin Auto-Discovery

Builtin policies can't know all their signals at compile time (they discover them based on configuration). The engine auto-discovers signal patterns like `__builtin_protected_paths_*`.

## Examples

### MCP Tool Routing

MCP tools work identically to native tools:
```
PreToolUse:mcp__postgres__execute_sql → [database policies]
```

### Multiple Events

A policy can handle multiple events:
```rego
required_events: ["PreToolUse", "PostToolUse"]
required_tools: ["Bash"]
```

Creates two routing entries:
- `PreToolUse:Bash`
- `PostToolUse:Bash`

## Performance Impact

**Without routing**: 50+ policies evaluated on every event
**With routing**: 2-5 policies evaluated (typical)

Signal execution is the expensive part. Routing ensures we only run signals when necessary, keeping Cupcake fast enough to not annoy developers.

## Implementation

- **Scanner** (`engine/scanner.rs`): Finds policy files
- **Metadata Parser** (`engine/metadata.rs`): Extracts routing directives
- **Router** (`engine/routing.rs`): Builds and queries the HashMap
- **Engine** (`engine/mod.rs`): Orchestrates the flow

The routing map is built once when Cupcake starts. After that, every incoming event uses simple HashMap lookups. No regex matching, no parsing policy files again, no scanning directories. Just a direct key lookup: `event:tool` → list of policies.