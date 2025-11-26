# Cupcake OpenCode Plugin

Policy engine plugin for [OpenCode](https://opencode.ai), enabling enforcement of custom policies on AI coding agent actions.

## Features

- ✅ Intercepts tool execution before it happens
- ✅ Evaluates actions against user-defined policies
- ✅ Blocks dangerous operations (deny/block)
- ✅ Supports approval workflows (ask)
- ✅ Configurable fail modes (open/closed)
- ✅ Performance optimized with timeouts

## Installation

### Prerequisites

- OpenCode installed and configured
- Cupcake CLI installed (`cargo install cupcake` or use install script)
- Node.js or Bun for running the plugin

### Install Plugin

**Option 1: From Source (Recommended)**

```bash
# Build the plugin
cd cupcake-plugins/opencode
bun install && bun run build  # or: npm install && npm run build

# Copy to your project - just a single file!
mkdir -p /path/to/your/project/.opencode/plugin
cp dist/cupcake.js /path/to/your/project/.opencode/plugin/
```

**Option 2: Global Installation**

```bash
# Install globally for all projects
mkdir -p ~/.config/opencode/plugin
cp dist/cupcake.js ~/.config/opencode/plugin/
```

## Quick Start

1. **Initialize Cupcake** in your project:

```bash
cupcake init --harness opencode
```

2. **Add example policies**:

```bash
cp -r examples/opencode/0_Welcome/* .cupcake/policies/opencode/
```

3. **Test the integration**:

```bash
opencode
# Try: "run git commit --no-verify"
# Should be blocked by policy
```

## Configuration

Create `.cupcake/opencode.json` to customize plugin behavior:

```json
{
  "enabled": true,
  "cupcakePath": "cupcake",
  "harness": "opencode",
  "logLevel": "info",
  "timeoutMs": 5000,
  "failMode": "closed",
  "cacheDecisions": false
}
```

### Options

| Option           | Type    | Default      | Description                                         |
| ---------------- | ------- | ------------ | --------------------------------------------------- |
| `enabled`        | boolean | `true`       | Enable/disable plugin                               |
| `cupcakePath`    | string  | `"cupcake"`  | Path to cupcake binary                              |
| `harness`        | string  | `"opencode"` | Harness type (always "opencode")                    |
| `logLevel`       | string  | `"info"`     | Log level: "error", "warn", "info", "debug"         |
| `timeoutMs`      | number  | `5000`       | Max policy evaluation time (ms)                     |
| `failMode`       | string  | `"closed"`   | "open" (allow on error) or "closed" (deny on error) |
| `cacheDecisions` | boolean | `false`      | Enable decision caching (experimental)              |

### Fail Modes

**Fail-Closed (Recommended for Production)**

- Blocks operations if policy evaluation fails
- Maximum security, but may block on transient errors

**Fail-Open (Development)**

- Allows operations if policy evaluation fails
- More resilient, but policies may not be enforced

## Writing Policies

Policies are written in [Rego](https://www.openpolicyagent.org/docs/latest/policy-language/) and stored in `.cupcake/policies/opencode/`.

### Example: Block Dangerous Commands

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.safety

import rego.v1

# Block git commit with --no-verify
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command

    contains(command, "git commit")
    contains(command, "--no-verify")

    decision := {
        "rule_id": "GIT_NO_VERIFY",
        "reason": "The --no-verify flag bypasses pre-commit hooks.",
        "severity": "HIGH"
    }
}
```

### Testing Policies

Test policies directly with cupcake CLI:

```bash
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git commit --no-verify"}
}' | cupcake eval --harness opencode
```

Expected output:

```json
{
  "decision": "deny",
  "reason": "The --no-verify flag bypasses pre-commit hooks."
}
```

## Architecture

```
OpenCode → Plugin (tool.execute.before) → Cupcake CLI → Policies (WASM)
                                             ↓
                    Decision (allow/deny/ask) → Enforce → Allow or Throw Error
```

### Event Flow

1. **OpenCode** fires `tool.execute.before` event
2. **Plugin** intercepts and builds Cupcake event JSON
3. **Cupcake CLI** evaluates policies against the event
4. **Policies** return decision (allow/deny/block/ask)
5. **Plugin** enforces decision:
   - `allow` → Return normally (tool executes)
   - `deny`/`block`/`ask` → Throw error (tool blocked)

## Troubleshooting

### Plugin Not Working

**Check plugin is loaded:**

```bash
ls -la .opencode/plugin/
# or
ls -la ~/.config/opencode/plugin/
```

**Check cupcake is in PATH:**

```bash
which cupcake
cupcake --version
```

**Enable debug logging:**

```json
{
  "logLevel": "debug"
}
```

### Policies Not Evaluating

**Check policy directory:**

```bash
ls -la .cupcake/policies/opencode/
```

**Test policy directly:**

```bash
cupcake eval --harness opencode --debug < test_event.json
```

**Check routing metadata:**

- Ensure `required_events` matches event type
- Ensure `required_tools` matches tool name

### Performance Issues

**Increase timeout:**

```json
{
  "timeoutMs": 10000
}
```

**Check policy performance:**

```bash
time cupcake eval --harness opencode < test_event.json
```

## Advanced Usage

### Custom Tool Integration

Custom OpenCode tools work automatically:

```typescript
// .opencode/tool/deploy.ts
export default tool({
  description: "Deploy to environment",
  args: {
    env: tool.schema.string(),
  },
  async execute(args) {
    // Deployment logic
  },
});
```

Cupcake plugin will intercept `deploy` tool and evaluate policies.

### Multi-Project Setup

Use global configuration for organization-wide policies:

```bash
# Global config
~/.config/opencode/plugin/cupcake.js

# Global policies
~/.cupcake/policies/opencode/
```

Project-specific policies override global policies.

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development guidelines.

## License

MIT - See [LICENSE](../../LICENSE)

## Resources

- [Cupcake Documentation](../../docs/)
- [OpenCode Documentation](https://opencode.ai/docs)
- [Example Policies](../../examples/opencode/)
- [Integration Design](../../docs/agents/opencode/integration-design.md)
