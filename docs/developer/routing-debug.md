# Routing Debug System

## Problem

Cupcake uses a HashMap to route events to policies at runtime. When policies don't fire as expected, you need to see what's actually in that routing map.

## Solution

Dump the routing map to disk when `CUPCAKE_DEBUG_ROUTING=1` is set. Three output formats: JSON for programs, text for humans, DOT for graphs.

## Implementation

The routing map gets built during engine initialization. After it's complete, we serialize it to disk if debug mode is enabled. Zero overhead when disabled.

### Code Location
- Module: `cupcake-core/src/engine/routing_debug.rs`
- Integration point: `Engine::initialize()` in `mod.rs`

### Data Captured
- All routing keys (e.g., `PreToolUse:Bash`, `UserPromptSubmit`)
- Policies mapped to each key
- Policy metadata (events, tools, signals)
- File paths for each policy
- Statistics (total routes, wildcards, coverage)

## Usage

```bash
# Enable and run
CUPCAKE_DEBUG_ROUTING=1 cupcake eval < event.json

# Or with Claude Code CLI
CUPCAKE_DEBUG_ROUTING=1 claude -p "hello world"

# Output location
ls .cupcake/debug/routing/
```

## Output Formats

### Text Format
Shows the routing map organized by routes with policies listed under each:

```
Route: PreToolUse:Bash [SPECIFIC]
  Policies (5):
    1. cupcake.policies.security
       File: ./.cupcake/policies/security_policy.rego
       Events: PreToolUse
       Tools: Bash, Edit
```

### JSON Format
Complete routing data for programmatic analysis:

```json
{
  "timestamp": "2025-09-18_13-54-10",
  "project": {
    "routing_entries": {
      "PreToolUse:Bash": [...]
    }
  },
  "statistics": {
    "total_routes": 7,
    "wildcard_routes": 4
  }
}
```

### DOT Format
Graphviz format that generates visual routing diagrams:

```bash
dot -Tpng routing_map_*.dot -o routing.png
```

The graph shows three layers:
1. Events (yellow ovals)
2. Tools (green diamonds)
3. Policies (blue boxes)

Edges show the routing relationships from events through tools to policies.

## Key Concepts

**Routing Keys**
- `PreToolUse:Bash` routes PreToolUse events with Bash tool
- `PreToolUse` routes all PreToolUse events regardless of tool
- MCP tools use full names like `PreToolUse:mcp__postgres__execute_sql`

**Wildcard Policies**
Policies with events but no tools match all tools for that event. They appear in both the wildcard route and all specific tool routes.

**Global vs Project**
Global policies (from user config directory) and project policies (from `.cupcake/`) are tracked separately with different namespaces.

## Why This Matters

1. **Debugging**: See exactly which policies will fire for specific events
2. **Verification**: Confirm routing configuration matches expectations
3. **Performance**: Identify routes with many policies that might slow evaluation
4. **Understanding**: Visual graphs make complex routing relationships clear

## Performance

No impact on production. Single environment variable check returns early if not set. Debug writes happen once at startup, not during event evaluation.