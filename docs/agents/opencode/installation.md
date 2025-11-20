# OpenCode Integration - Installation & Setup Guide

This guide walks you through integrating Cupcake's policy engine with OpenCode.

## Prerequisites

Before you begin, ensure you have:

1. **OpenCode installed**: Follow the [OpenCode installation guide](https://opencode.ai/docs)
2. **Cupcake installed**:
   ```bash
   # Install cupcake (choose one method)
   cargo install cupcake
   # OR
   curl -fsSL https://cupcake.sh/install | bash
   ```
3. **Node.js** (v18+) or **Bun** installed

## Step 1: Build the Cupcake Plugin

```bash
# Navigate to the plugin directory
cd /path/to/cupcake/plugins/opencode

# Install dependencies
npm install

# Build the plugin
npm run build

# Verify build succeeded
ls -la dist/
# Should see: index.js, index.d.ts, etc.
```

## Step 2: Install the Plugin

You have two options for installing the plugin:

### Option A: Project-Level Installation (Recommended)

Install the plugin for a specific OpenCode project:

```bash
# Navigate to your project
cd /path/to/your/project

# Create plugin directory
mkdir -p .opencode/plugin

# Copy the built plugin
cp -r /path/to/cupcake/plugins/opencode/dist/* .opencode/plugin/cupcake/

# OR create a symlink for easier development
ln -s /path/to/cupcake/plugins/opencode/dist .opencode/plugin/cupcake
```

### Option B: Global Installation

Install the plugin globally for all OpenCode projects:

```bash
# Create global plugin directory
mkdir -p ~/.config/opencode/plugin

# Copy the built plugin
cp -r /path/to/cupcake/plugins/opencode/dist/* ~/.config/opencode/plugin/cupcake/

# OR create a symlink
ln -s /path/to/cupcake/plugins/opencode/dist ~/.config/opencode/plugin/cupcake
```

**Note**: Project-level plugins override global plugins.

## Step 3: Initialize Cupcake for OpenCode

```bash
# Navigate to your project
cd /path/to/your/project

# Initialize cupcake with OpenCode harness
cupcake init --harness opencode

# This creates:
# - .cupcake/rulebook.yml (configuration)
# - .cupcake/policies/opencode/ (policy directory)
```

## Step 4: Add Example Policies

Copy example policies to get started:

```bash
# Copy all example policies
cp -r /path/to/cupcake/examples/opencode/0_Welcome/* .cupcake/policies/opencode/

# Verify policies are in place
ls -la .cupcake/policies/opencode/
# Should see: minimal_protection.rego, git_workflow.rego, file_protection.rego
```

## Step 5: Test the Integration

### Test with Direct CLI

First, verify Cupcake works with OpenCode events:

```bash
# Test a deny scenario
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git commit --no-verify"}
}' | cupcake eval --harness opencode

# Expected output:
# {"decision":"deny","reason":"The --no-verify flag bypasses pre-commit hooks..."}

# Test an allow scenario
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "git status"}
}' | cupcake eval --harness opencode

# Expected output:
# {"decision":"allow"}
```

### Test with OpenCode

Now test the full integration with OpenCode:

```bash
# Start OpenCode in your project
cd /path/to/your/project
opencode

# In OpenCode, try a blocked command:
# > "run git commit --no-verify"
# Should see: ❌ Policy Violation - blocked!

# Try an allowed command:
# > "run git status"
# Should execute normally
```

## Step 6: Configure Plugin (Optional)

Create `.cupcake/opencode.json` to customize plugin behavior:

```bash
cat > .cupcake/opencode.json <<'EOF'
{
  "enabled": true,
  "cupcakePath": "cupcake",
  "harness": "opencode",
  "logLevel": "info",
  "timeoutMs": 5000,
  "failMode": "closed",
  "cacheDecisions": false
}
EOF
```

### Configuration Options

| Option           | Default     | Description                                             |
| ---------------- | ----------- | ------------------------------------------------------- |
| `enabled`        | `true`      | Enable/disable plugin                                   |
| `cupcakePath`    | `"cupcake"` | Path to cupcake binary                                  |
| `logLevel`       | `"info"`    | Log level: `"error"`, `"warn"`, `"info"`, `"debug"`     |
| `timeoutMs`      | `5000`      | Max policy evaluation time (ms)                         |
| `failMode`       | `"closed"`  | `"open"` (allow on error) or `"closed"` (deny on error) |
| `cacheDecisions` | `false`     | Enable decision caching (experimental)                  |

**Fail Mode Guidance:**

- **Production**: Use `"closed"` (deny on error) for maximum security
- **Development**: Use `"open"` (allow on error) for faster iteration

## Step 7: Write Custom Policies

Create your own policies in `.cupcake/policies/opencode/`:

```rego
# .cupcake/policies/opencode/my_policy.rego

# METADATA
# scope: package
# title: My Custom Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.my_policy

import rego.v1

# Block dangerous commands
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command

    # Add your conditions here
    contains(command, "rm -rf /")

    decision := {
        "rule_id": "DANGEROUS_RM",
        "reason": "Cannot delete root directory!",
        "severity": "CRITICAL"
    }
}
```

Test your policy:

```bash
echo '{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "'$(pwd)'",
  "tool": "bash",
  "args": {"command": "rm -rf /"}
}' | cupcake eval --harness opencode
```

## Verification Checklist

- [ ] Cupcake CLI is installed and in PATH (`cupcake --version`)
- [ ] Plugin is built (`ls plugins/opencode/dist/`)
- [ ] Plugin is installed (`.opencode/plugin/cupcake/` or `~/.config/opencode/plugin/cupcake/`)
- [ ] Cupcake is initialized (`.cupcake/rulebook.yml` exists)
- [ ] Policies exist (`.cupcake/policies/opencode/*.rego`)
- [ ] CLI test passes (echo test above works)
- [ ] OpenCode integration works (blocked commands are denied)

## Troubleshooting

### Plugin Not Loading

**Check plugin location:**

```bash
# Project-level
ls -la .opencode/plugin/cupcake/
# Should see: index.js, index.d.ts, etc.

# Global
ls -la ~/.config/opencode/plugin/cupcake/
```

**Check OpenCode recognizes the plugin:**

```bash
# Enable debug logging in plugin config
cat > .cupcake/opencode.json <<'EOF'
{
  "logLevel": "debug"
}
EOF

# Run OpenCode and watch logs
opencode
# Should see: [cupcake-plugin] INFO: Cupcake plugin initialized
```

### Policies Not Evaluating

**Test policy compilation:**

```bash
cd .cupcake/policies/opencode
opa build -t wasm -e cupcake/system/evaluate .
# Should succeed without errors
```

**Check routing metadata:**

```bash
# Inspect policies
cupcake inspect --policy-dir .cupcake/policies

# Should show routing info for each policy
```

**Enable debug logging:**

```bash
cupcake eval --harness opencode --log-level debug < test_event.json
```

### Cupcake Not Found

**Check PATH:**

```bash
which cupcake
# Should show: /usr/local/bin/cupcake or similar

# If not found, add to PATH or specify full path in config:
cat > .cupcake/opencode.json <<'EOF'
{
  "cupcakePath": "/full/path/to/cupcake"
}
EOF
```

### Performance Issues

**Increase timeout:**

```bash
cat > .cupcake/opencode.json <<'EOF'
{
  "timeoutMs": 10000
}
EOF
```

**Profile policy evaluation:**

```bash
time cupcake eval --harness opencode < test_event.json
```

## Next Steps

1. **Write Custom Policies**: See [Policy Examples](./policy-examples.md)
2. **Explore Builtins**: Learn about built-in policies in `docs/user-guide/configuration/builtins.md`
3. **Set Up CI/CD**: Integrate Cupcake into your development workflow
4. **Share Policies**: Contribute policies back to the community

## Advanced Configuration

### Global Organization Policies

Set up organization-wide policies that apply to all projects:

```bash
# Create global policy directory
mkdir -p ~/.cupcake/policies/opencode

# Add organization policies
cp org-policies/*.rego ~/.cupcake/policies/opencode/

# Configure global rulebook
cat > ~/.cupcake/rulebook.yml <<'EOF'
global_config:
  enabled: true
  policy_dir: ~/.cupcake/policies/opencode

builtins:
  git_block_no_verify:
    enabled: true
  protected_paths:
    enabled: true
    paths:
      - ".env"
      - "secrets/"
EOF
```

### Multi-Project Setup

For teams managing multiple projects:

```bash
# Use global plugin + project-specific policies
# Global: ~/.config/opencode/plugin/cupcake/
# Per-project: /project/.cupcake/policies/opencode/

# Policies are merged: project policies override global
```

## Support

- **Documentation**: [Full docs](../../../README.md)
- **Examples**: `examples/opencode/`
- **Issues**: GitHub Issues
- **Community**: Discord/Slack (if available)

---

**Congratulations!** 🎉 You've successfully integrated Cupcake with OpenCode. Your AI coding agent is now policy-aware!
