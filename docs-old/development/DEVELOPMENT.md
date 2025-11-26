# Cupcake Development Guide

## Architecture

For a comprehensive overview of Cupcake's architecture, design principles, and core components, see the **[Architecture Reference](../reference/architecture.md)**.

**Key concepts:**

- **Hybrid Model**: Rego (WASM) for policy logic, Rust (Engine) for orchestration
- **Metadata-Driven Signal Gating**: O(1) lookup determines which signals to execute and whether to run WASM
- **Single Aggregation**: All compiled policies evaluated through `cupcake.system.evaluate` via `walk()`
- **Proactive Signals**: Gathered before evaluation, not reactively
- **Policy Self-Filtering**: Policies must include their own event/tool checks in Rego (routing doesn't filter at WASM level)

## Quick Start

```bash
# Install OPA (required for policy compilation)
brew install opa  # v1.71.0+ REQUIRED for v1.0 Rego syntax

# Build Cupcake
cargo build --release

# Run tests (REQUIRED: Use deterministic-tests feature)
cargo test --features deterministic-tests
# Or use the alias
cargo t

# Enable evaluation tracing for debugging
cargo run -- eval --trace eval --policy-dir .cupcake/policies

# Create a test event and evaluate
echo '{"hook_event_name":"PreToolUse","session_id":"test","transcript_path":"/tmp/test","cwd":"/tmp","tool_name":"Bash","tool_input":{"command":"echo hello"}}' | \
  target/release/cupcake eval --policy-dir .cupcake/policies

# Output: {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}
```

## Metadata-Driven Routing

Policies declare their requirements via OPA metadata blocks:

```rego
# METADATA
# scope: package
# title: Bash Security Guard
# authors: ["Security Team"]
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
#     required_signals: ["git_branch"]
package cupcake.policies.bash_guard

import rego.v1

# Policy rules follow...
```

The engine reads metadata at startup and builds a routing map. When events arrive, the map determines which signals to execute and whether to run WASM at all (early exit if no policies match). Note: Once WASM runs, ALL compiled policies execute via `walk()` - policies must self-filter with their own event/tool checks.

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

1. Engine routes event via metadata lookup
2. If no policies match, early exit with Allow (no WASM, no signals)
3. Collects all `required_signals` from matched policies
4. Executes signals in parallel (shell commands)
5. Enriches input with signal data
6. Evaluates ALL compiled policies via WASM (policies self-filter)

Example signal in `rulebook.yml`:

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

## Debugging

For comprehensive debugging and troubleshooting, including policy evaluation tracing, debug file output, routing visualization, and platform-specific issues, see the **[Debugging Guide](./DEBUGGING.md)**.

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

## Release Process

Cupcake uses automated GitHub Actions for releases. To create a new release:

1. **Merge to main** - Ensure all changes are merged to the main branch
2. **Create and push a version tag**:
   ```bash
   git checkout main
   git pull origin main
   git tag v0.1.8  # Use semantic versioning
   git push origin v0.1.8
   ```
3. **Automated build** - GitHub Actions will:
   - Build binaries for all platforms (macOS, Linux, Windows)
   - Bundle OPA v1.7.1 with each platform
   - Create a draft release with all artifacts
   - Generate SHA256 checksums
   - **Automatically publish** the release when builds complete

The release workflow is defined in `.github/workflows/release.yml`. Releases include pre-built binaries with bundled OPA, eliminating external dependencies.

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

1. **Metadata-Driven Signal Gating** - Policies declare signal requirements, engine handles collection
2. **Single Aggregation** - All evaluation through `cupcake.system.evaluate` via `walk()`
3. **Policy Self-Filtering** - Policies include event/tool checks (routing doesn't filter WASM execution)
4. **Proactive Signals** - Gathered before evaluation, not reactively
5. **Strict Priority** - Synthesis layer enforces decision hierarchy
6. **Trust by Default** - Scripts verified via HMAC before execution

## Performance Targets

- Policy discovery and compilation: < 100ms
- Signal gating lookup: O(1)
- Policy evaluation: < 50ms
- Full request cycle: < 200ms

## References

- `CLAUDE.md` - Project configuration and guidelines
- `src/trust/CLAUDE.md` - Trust system implementation details
- `tests/CLAUDE.md` - Testing requirements

### General

## Development

### Running Tests

**IMPORTANT**: Tests MUST be run with the `deterministic-tests` feature flag. This ensures:

1. Deterministic HMAC key generation for reliable test execution
2. No interference from developer's personal Cupcake configuration (tests create isolated temp configs)

```bash
# Run all tests (REQUIRED for correct behavior)
cargo test --features deterministic-tests

# Or use the Just commands (recommended)
just test              # Run all tests
just test-unit        # Run unit tests only
just test-integration # Run integration tests only
just test-one <name>  # Run specific test

# Alias for quick testing
cargo t  # Configured alias that includes required flags
```

### Releasing

To create a new release, push a version tag: `git tag v0.1.8 && git push origin v0.1.8`. See [Development Guide](./docs/development/DEVELOPMENT.md#release-process) for details.

#### Why Deterministic Tests Feature Is Required

If you use Cupcake as a developer, you likely have a global configuration at `~/Library/Application Support/cupcake` (macOS) or `~/.config/cupcake` (Linux). This global config is designed to override project configs for organizational policy enforcement.

However, during testing, this causes issues:

- Tests expect specific builtin configurations
- Global configs override the test's project configs
- Tests fail with unexpected policy decisions

Integration tests handle this by creating isolated temporary configurations and explicitly disabling global config discovery. The `deterministic-tests` feature flag ensures deterministic HMAC key generation for reliable test execution. Without it, integration tests will experience race conditions and cryptographic verification failures due to non-deterministic key derivation in production mode.

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release
```
