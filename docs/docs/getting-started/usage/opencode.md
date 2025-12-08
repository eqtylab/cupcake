---
title: "OpenCode"
description: "Setting up Cupcake with OpenCode"
---

# OpenCode Setup

OpenCode uses an **in-process plugin architecture** rather than external hooks. Cupcake provides a TypeScript plugin that intercepts tool execution directly within OpenCode.

## How It Works

```
OpenCode -> Plugin -> cupcake eval -> Policy Decision -> Allow/Block
```

Unlike Claude Code/Cursor which use stdin/stdout hooks, OpenCode's plugin:

- Runs inside the OpenCode process
- Spawns `cupcake eval --harness opencode` for each tool call
- Throws an Error to block tool execution

## Project Setup (Recommended)

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness opencode
```

This automatically:

- Creates `.cupcake/` directory with policies and configuration
- Downloads and installs the Cupcake plugin to `.opencode/plugin/cupcake.js`

OpenCode will automatically load the plugin and enforce your policies.

## Global Setup

For organization-wide policies:

```bash
cupcake init --global --harness opencode
```

This creates configuration at `~/.config/cupcake/` and installs the plugin globally to `~/.config/opencode/plugin/cupcake.js`.

## Manual Plugin Installation

If automatic download fails (e.g., network issues):

```bash
# Download from GitHub releases
mkdir -p .opencode/plugin
curl -fsSL https://github.com/eqtylab/cupcake/releases/download/opencode-plugin-latest/opencode-plugin.js \
  -o .opencode/plugin/cupcake.js

# Or build from source
cd /path/to/cupcake/cupcake-plugins/opencode
npm install && npm run build
cp dist/cupcake.js /path/to/your/project/.opencode/plugin/
```

## Enable Built-in Policies

```bash
# Project-level builtins
cupcake init --harness opencode --builtins git_pre_check,protected_paths

# Global security builtins
cupcake init --global --harness opencode --builtins system_protection,sensitive_data_protection
```

See the [Built-in Configuration Reference](../../reference/builtin-config.md) for complete details.

## Verify Installation

Test that Cupcake is working:

```bash
# Create test event
cat > test-event.json << 'EOF'
{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
  "cwd": "/tmp",
  "tool": "bash",
  "args": { "command": "echo 'Hello from Cupcake!'" }
}
EOF

# Evaluate
cupcake eval --harness opencode < test-event.json
```

Expected output:

```json
{
  "decision": "allow"
}
```

## Next Steps

- [OpenCode Reference](../../reference/harnesses/opencode.md) - Events, response formats, plugin configuration
- [Writing Policies](../../reference/policies/custom.md) - Create custom Rego policies
- [OpenCode Tutorial](../../tutorials/opencode.md) - Hands-on walkthrough
