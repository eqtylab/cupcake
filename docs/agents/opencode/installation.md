# OpenCode Installation Guide

Detailed installation instructions for integrating Cupcake with OpenCode.

> For a faster setup, see the [Quickstart Guide](./quickstart.md).

## Prerequisites

1. **OpenCode** installed and working
2. **Cupcake CLI** built or installed
3. **Node.js** v18+ (for building the plugin)

## Building Cupcake

```bash
# Clone the repository
git clone https://github.com/eqtylab/cupcake.git
cd cupcake

# Build release binary
cargo build --release

# Verify it works
./target/release/cupcake --version
```

## Installing Cupcake CLI

Choose one method:

### Option A: Add to PATH

```bash
export PATH="$PATH:/path/to/cupcake/target/release"
```

### Option B: Copy to system bin

```bash
sudo cp target/release/cupcake /usr/local/bin/
```

### Option C: Symlink

```bash
sudo ln -s /path/to/cupcake/target/release/cupcake /usr/local/bin/cupcake
```

## Building the Plugin

```bash
cd cupcake-plugins/opencode

# Install dependencies
npm install

# Build TypeScript to JavaScript
npm run build

# Verify build
ls dist/
# Should see: index.js, types.js, executor.js, etc.
```

## Installing the Plugin

### Project-Level (Recommended)

Install for a specific project:

```bash
cd /path/to/your/project

# Create plugin directory
mkdir -p .opencode/plugins/cupcake

# Copy built plugin
cp -r /path/to/cupcake/cupcake-plugins/opencode/dist/* .opencode/plugins/cupcake/
cp /path/to/cupcake/cupcake-plugins/opencode/package.json .opencode/plugins/cupcake/
```

### Global Installation

Install for all OpenCode projects:

```bash
mkdir -p ~/.config/opencode/plugins/cupcake
cp -r /path/to/cupcake/cupcake-plugins/opencode/dist/* ~/.config/opencode/plugins/cupcake/
cp /path/to/cupcake/cupcake-plugins/opencode/package.json ~/.config/opencode/plugins/cupcake/
```

## Initializing Cupcake

```bash
cd /path/to/your/project

# Initialize with OpenCode harness
cupcake init --harness opencode
```

This creates:

```
.cupcake/
├── rulebook.yml              # Configuration file
├── system/
│   └── evaluate.rego         # System entrypoint (auto-generated)
├── helpers/
│   └── commands.rego         # Shared helper functions
├── policies/
│   └── opencode/
│       └── builtins/         # Built-in policies
│           └── *.rego
└── signals/                  # Signal definitions
```

The system evaluator at `.cupcake/system/evaluate.rego` is automatically created by `cupcake init`.

## Adding Policies

Copy example policies:

```bash
cp -r /path/to/cupcake/examples/opencode/0_Welcome/*.rego .cupcake/policies/opencode/
```

Or create your own:

```bash
cat > .cupcake/policies/opencode/my_policy.rego << 'EOF'
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.my_policy

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "dangerous")

    decision := {
        "rule_id": "MY_RULE",
        "reason": "This command is not allowed",
        "severity": "HIGH"
    }
}
EOF
```

## Plugin Configuration

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

### Configuration Options

| Option           | Default     | Description                                             |
| ---------------- | ----------- | ------------------------------------------------------- |
| `enabled`        | `true`      | Enable/disable the plugin                               |
| `cupcakePath`    | `"cupcake"` | Path to cupcake binary                                  |
| `logLevel`       | `"info"`    | Log level: debug, info, warn, error                     |
| `timeoutMs`      | `5000`      | Max policy evaluation time (ms)                         |
| `failMode`       | `"closed"`  | `"open"` (allow on error) or `"closed"` (deny on error) |
| `cacheDecisions` | `false`     | Cache decisions (experimental)                          |

## Verification

### Test CLI directly

```bash
# Should return deny
echo '{"hook_event_name":"PreToolUse","session_id":"test","cwd":"'$(pwd)'","tool":"bash","args":{"command":"git commit --no-verify"}}' | cupcake eval --harness opencode
```

### Test with OpenCode

```bash
opencode

# In OpenCode, try:
# > run git commit --no-verify -m test
# Should see: Policy violation - blocked
```

## Verification Checklist

- [ ] `cupcake --version` works
- [ ] Plugin built: `ls cupcake-plugins/opencode/dist/`
- [ ] Plugin installed: `ls .opencode/plugins/cupcake/` or `~/.config/opencode/plugins/cupcake/`
- [ ] Cupcake initialized: `ls .cupcake/`
- [ ] System evaluator exists: `ls .cupcake/system/evaluate.rego`
- [ ] Policies exist: `ls .cupcake/policies/opencode/builtins/*.rego`
- [ ] CLI test passes (deny for --no-verify)
- [ ] OpenCode integration works

## Troubleshooting

### cupcake not found

```bash
# Check if in PATH
which cupcake

# If not, add to PATH or use full path in config:
echo '{"cupcakePath": "/full/path/to/cupcake"}' > .cupcake/opencode.json
```

### Policies not evaluating

```bash
# Enable debug logging
cupcake eval --harness opencode --log-level debug < event.json

# Check routing
cupcake eval --harness opencode --debug-routing < event.json
```

### Plugin not loading

Check plugin location and restart OpenCode:

```bash
ls -la .opencode/plugins/cupcake/
# Must contain: index.js, package.json
```

### Performance issues

Increase timeout:

```json
{
  "timeoutMs": 10000
}
```

## Next Steps

- [Quickstart Guide](./quickstart.md) - Fast setup
- [Plugin Reference](./plugin-reference.md) - Configuration details
- [Policy Examples](../../../examples/opencode/) - Example policies
