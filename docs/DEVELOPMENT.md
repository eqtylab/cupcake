# Cupcake Development Guide

## The Philosophy: Simplicity for the User, Intelligence in the Engine

Instead of requiring users to wire together complex pipelines, Cupcake discovers your policies, understands their requirements, and intelligently routes events to them.

## Quick Start

```bash
# Install OPA (required for policy compilation)
brew install opa  # or your package manager

# Build Cupcake
cargo build --release

# Test with an example event
cat examples/events/pre_tool_use_bash_safe.json | \
  target/release/cupcake eval --policy-dir ./examples/policies

# Output: {"hook_specific_output":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}
```

## Architecture Overview

```
Claude Code Event (JSON) → Cupcake → Claude Code Response (JSON)
                            ↓
                    1. Parse Event (ClaudeHarness)
                    2. Route to Policies (Engine)
                    3. Evaluate with WASM (OPA)
                    4. Aggregate Decisions
                    5. Format Response (ClaudeHarness)
```

### Core Components

1. **Engine** (`src/engine/`) - The intelligent core

   - Scanner: Discovers `.rego` files
   - Parser: Extracts `selector` blocks
   - Router: Matches events to policies
   - Compiler: Creates unified WASM module
   - Runtime: Executes policies and aggregates decisions

2. **ClaudeHarness** (`src/harness/`) - Pure data translation

   - Events: Strongly-typed Claude Code event structures
   - Response: Spec-compliant JSON builders
   - Zero business logic - just data transformation

3. **Policies** (`examples/policies/`) - Your governance rules
   - Standard OPA/Rego syntax
   - Selector block for routing
   - Decision object output

## The Selector Pattern

Every policy declares what events it handles via a `selector` block:

```rego
# Simple selector - just the event
selector := {
    "event": "PreToolUse"
}

# Specific tool selector
selector := {
    "event": "PreToolUse",
    "tools": ["Bash", "Write"]
}

# Pattern matching (future)
selector := {
    "event": "PreToolUse",
    "tool_pattern": ".*Test$"  # Matches any tool ending in "Test"
}
```

The engine reads these selectors at startup and builds an intelligent routing map. When an event arrives, only relevant policies are evaluated.

## The Decision Object

Policies output a standardized decision object that the engine understands:

```rego
decision := {
    "deny": deny_list,           # List of violations (or empty)
    "additional_context": [...],  # Context to inject
    "missing_signals": [...]      # Signals needed for re-evaluation
}
```

### Violation Objects

When denying, provide structured violations:

```rego
deny_list := [violation] if {
    contains(input.command, "rm -rf /")
    violation := {
        "id": "dangerous_rm",
        "msg": "Dangerous rm command",
        "meta": {"command": input.command},
        "feedback": {
            "permissionDecision": "deny",
            "permissionDecisionReason": "This command is too dangerous"
        }
    }
}
```

## Two-Pass Reactive Evaluation

The engine implements intelligent signal fetching:

1. **Pass 1**: Evaluate with initial input
2. If policies request signals via `missing_signals`, fetch them
3. **Pass 2**: Re-evaluate with enriched input

This means signals are only executed when actually needed, not proactively.

## Running Tests

```bash
# Run all tests
cargo test

# Run with single thread (required for WASM tests)
cargo test -- --test-threads=1

# Run specific test
cargo test test_selector_parsing
```

## Benchmarking

```bash
# Run performance benchmarks
cargo bench

# Target: <50ms for complete evaluation
```

## Adding a New Policy

1. Create a `.rego` file in your policies directory
2. Add the required imports and selector:

   ```rego
   package cupcake.policies.my_policy
   import rego.v1

   selector := {
       "event": "PreToolUse",
       "tools": ["MyTool"]
   }
   ```

3. Write your rules using OPA v1.7.1 syntax (remember `if` keywords!)
4. Output the decision object
5. Test with: `cat event.json | cupcake eval --policy-dir ./policies`

## Integration with Claude Code

Cupcake is designed to be used as a hook processor for Claude Code:

```bash
# In your Claude Code hooks configuration
pre-tool-use = "cat $HOOK_INPUT | cupcake eval --policy-dir /path/to/policies"
```

The output is guaranteed to match Claude Code's expected JSON format for each hook type.

## Debugging

```bash
# Enable debug logging
RUST_LOG=debug cupcake eval --policy-dir ./policies

# Use --debug flag for verbose output
cupcake eval --policy-dir ./policies --debug

# Verify engine initialization
cupcake verify --policy-dir ./policies
```

## Common Issues

### "Missing hookEventName in input"

The engine expects Claude Code's standard event format. Ensure your JSON includes:

- `hook_event_name` (snake_case)
- `session_id`, `transcript_path`, `cwd`
- Hook-specific fields (e.g., `tool_name`, `tool_input` for PreToolUse)

### "Failed to parse selector block"

Selectors must be valid JSON-like Rego objects. Check for:

- Proper quoting of keys and string values
- Correct field names (`event`, not `hook`)
- Array syntax for `tools` field

### "OPA compilation failed"

Ensure you're using OPA v1.7.1+ syntax:

- Use `import rego.v1` at the top
- Include `if` keyword in rule bodies
- Use `contains` for set operations

## Architecture Principles

1. **The Engine is Intelligent** - It discovers, compiles, routes, and aggregates automatically
2. **Policies are Declarative** - They declare what they handle (selector) and what they decide
3. **The Harness is a Translator** - Pure data transformation, no business logic
4. **Signals are Reactive** - Only fetched when needed, not proactively
5. **Actions are Asynchronous** - Fire-and-forget, never block the response

Remember: The CRITICAL_GUIDING_STAR is always the source of truth. When in doubt, refer to it.
