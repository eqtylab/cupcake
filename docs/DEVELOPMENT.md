# Cupcake Development Guide

## Architecture: The Hybrid Model

Cupcake implements a **Hybrid Model** where:
- **Rego (WASM)**: Declares policies, evaluates rules, returns decision verbs
- **Rust (Engine)**: Routes events, gathers signals, synthesizes final decisions

The engine is intelligent - it discovers policies, understands their requirements via metadata, and routes events efficiently.

## Quick Start

```bash
# Install OPA (required for policy compilation)
brew install opa  # v0.70.0+ for v1.0 Rego syntax

# Build Cupcake
cargo build --release

# Run tests (REQUIRED: Use deterministic-tests feature)
cargo test --features deterministic-tests
# Or use the alias
cargo t

# Test with an example event
cat examples/events/pre_tool_use_bash_safe.json | \
  target/release/cupcake eval --policy-dir ./examples/policies

# Output: {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}
```

## Architecture Flow

```
Claude Code Event (JSON) → Cupcake → Claude Code Response (JSON)
                            ↓
                    1. Route (O(1) metadata lookup)
                    2. Gather Signals (proactive)
                    3. Evaluate (WASM via cupcake.system.evaluate)
                    4. Synthesize (apply priority hierarchy)
                    5. Execute Actions (async, non-blocking)
                    6. Format Response
```

### Core Components

1. **Engine** (`src/engine/`)
   - Scanner: Discovers `.rego` files
   - Metadata Parser: Extracts routing from `# METADATA` blocks
   - Router: O(1) event-to-policy matching
   - Compiler: Creates unified WASM module
   - Runtime: Executes single aggregation entrypoint
   - Synthesis: Applies decision priority hierarchy

2. **Harness** (`src/harness/`)
   - Events: Strongly-typed Claude Code structures
   - Response: Spec-compliant JSON builders
   - Pure data transformation layer

3. **Trust System** (`src/trust/`)
   - HMAC-based integrity verification
   - Project-specific key derivation
   - Tamper detection for scripts

## Metadata-Driven Routing

Policies declare their requirements via OPA metadata blocks:

```rego
package cupcake.policies.bash_guard
import rego.v1

# METADATA
# scope: rule
# title: Bash Security Guard
# authors: ["Security Team"]
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch"]

# Policy rules follow...
```

The engine reads metadata at startup and builds an intelligent routing map. When events arrive, only relevant policies are evaluated.

## Decision Verbs

Policies output decisions using standardized verb sets:

```rego
# Deny dangerous commands
deny contains decision if {
    contains(input.tool_input.command, "rm -rf /")
    decision := {
        "reason": "Dangerous rm command detected",
        "severity": "HIGH",
        "rule_id": "BASH-001"
    }
}

# Ask for confirmation on production
ask contains decision if {
    input.signals.git_branch == "main"
    decision := {
        "reason": "Production branch - please confirm",
        "severity": "MEDIUM",
        "rule_id": "BASH-002"
    }
}

# Add context for best practices
add_context contains "Remember to run tests before committing" if {
    contains(input.tool_input.command, "git commit")
}
```

### Verb Priority Hierarchy

The synthesis layer enforces strict priority:

1. **halt** - Immediate cessation (highest priority)
2. **deny/block** - Prevent action
3. **ask** - Require confirmation
4. **allow_override** - Explicit permission
5. **add_context** - Inject guidance (lowest priority)

## Single Aggregation Entrypoint

All policies are evaluated through `cupcake.system.evaluate`:

```rego
package cupcake.system
import rego.v1

# The single aggregation entrypoint
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

# Collect all decisions from a verb across the policy hierarchy
collect_verbs(verb_name) := result if {
    # Use walk() to traverse data.cupcake.policies
    # Aggregate all decisions for the verb
    # ... implementation details
}
```

## Signals (Proactive Enrichment)

Signals are gathered **before** policy evaluation (not reactively):

1. Engine routes event to policies
2. Collects all `required_signals` from matched policies
3. Executes signals in parallel
4. Enriches input with signal data
5. Evaluates policies with enriched input

Example signal in `guidebook.yml`:

```yaml
signals:
  git_branch:
    command: "git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown'"
    timeout_seconds: 5
```

## Actions (Fire-and-Forget)

Actions execute asynchronously after decision synthesis:

```yaml
actions:
  on_any_denial:
    - command: "echo 'Action denied' | tee -a /tmp/cupcake.log"
  
  on_halt:
    - command: "notify-send 'Cupcake' 'Critical action halted'"
  
  violations:
    BASH-001:
      - command: "alert-security-team.sh"
```

## Running Tests

```bash
# REQUIRED: Use deterministic-tests feature flag
cargo test --features deterministic-tests

# Or use the configured alias
cargo t

# Run specific test
cargo test test_metadata_parsing --features deterministic-tests

# Run with single thread if needed for WASM tests
cargo test --features deterministic-tests -- --test-threads=1
```

**Why the feature flag?** The trust system uses HMAC with system entropy in production. Tests need deterministic keys to avoid race conditions. See `src/trust/CLAUDE.md` for details.

## Adding a New Policy

1. Create a `.rego` file in your policies directory
2. Add the metadata block with routing requirements:

```rego
package cupcake.policies.my_policy
import rego.v1

# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["MyTool"]
#     required_signals: ["my_signal"]

# Use decision verbs
deny contains decision if {
    # Your logic here
    decision := {
        "reason": "...",
        "severity": "HIGH",
        "rule_id": "MY-001"
    }
}
```

3. Test with: `cat event.json | cupcake eval --policy-dir ./policies`

## Integration with Claude Code

Configure as a hook processor:

```bash
# In your Claude Code hooks configuration
pre-tool-use = "cat $HOOK_INPUT | cupcake eval --policy-dir /path/to/policies"
```

The output matches Claude Code's expected JSON format for each hook type.

## Debugging

```bash
# Enable debug logging
RUST_LOG=debug cupcake eval --policy-dir ./policies

# Verify policy discovery and routing
RUST_LOG=debug cupcake verify --policy-dir ./policies

# Check trust manifest
cupcake trust verify
```

## Common Issues

### "Missing hookEventName in input"

Ensure your JSON includes Claude Code's standard fields:
- `hook_event_name` or `hookEventName`
- `session_id`, `transcript_path`, `cwd`
- Hook-specific fields (e.g., `tool_name`, `tool_input`)

### "Policy missing routing directive"

Non-system policies must have metadata with routing:

```rego
# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
```

### "OPA compilation failed"

Use OPA v1.0 syntax:
- `import rego.v1` at the top
- `if` keyword in rule bodies
- `contains` for set operations

### "Trust manifest verification failed"

Run tests with the deterministic flag:
```bash
cargo test --features deterministic-tests
```

## Architecture Principles

1. **Metadata-Driven** - Policies declare requirements, engine handles routing
2. **Single Aggregation** - All evaluation through `cupcake.system.evaluate`
3. **Proactive Signals** - Gathered before evaluation, not reactively
4. **Strict Priority** - Synthesis layer enforces decision hierarchy
5. **Trust by Default** - Scripts verified via HMAC before execution

## Performance Targets

- Policy discovery and compilation: < 100ms
- Event routing: O(1) lookup
- Policy evaluation: < 50ms
- Full request cycle: < 200ms

## References

- `CLAUDE.md` - Project configuration and guidelines
- `src/trust/CLAUDE.md` - Trust system implementation details
- `tests/CLAUDE.md` - Testing requirements
- `examples/0_start_here_demo/` - Complete working examples