---
title: "OpenCode"
description: "Setting up Cupcake with OpenCode"
---

# OpenCode Setup

OpenCode uses an **in-process plugin architecture** rather than external hooks. Cupcake provides a TypeScript plugin that intercepts tool execution directly within OpenCode.

## How It Works

```
OpenCode → Plugin → cupcake eval → Policy Decision → Allow/Block
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

- Creates a `.cupcake/` directory with policies and configuration
- Sets up the system evaluator for OpenCode
- Downloads and installs the Cupcake plugin to `.opencode/plugin/cupcake.js`

OpenCode will automatically load the plugin and enforce your policies.

## Global Setup

For organization-wide policies that apply to all projects:

```bash
cupcake init --global --harness opencode
```

This creates configuration at `~/.config/cupcake/` and installs the plugin globally to `~/.config/opencode/plugin/cupcake.js`.

## Manual Plugin Installation

If automatic download fails (e.g., network issues), you can install the plugin manually:

```bash
# Option 1: Download from GitHub releases
mkdir -p .opencode/plugin
curl -fsSL https://github.com/eqtylab/cupcake/releases/latest/download/opencode-plugin.js \
  -o .opencode/plugin/cupcake.js

# Option 2: Build from source
cd /path/to/cupcake/cupcake-plugins/opencode
npm install && npm run build
mkdir -p /path/to/your/project/.opencode/plugin
cp dist/cupcake.js /path/to/your/project/.opencode/plugin/
```

## Enable Built-in Policies (Optional)

```bash
# Enable specific builtins
cupcake init --harness opencode --builtins git_pre_check,git_block_no_verify

# Global with builtins
cupcake init --global --harness opencode --builtins system_protection,sensitive_data_protection
```

Available builtins include:

- `git_pre_check` — Run checks before git operations
- `git_block_no_verify` — Prevent `--no-verify` flag usage
- `system_protection` — Protect system directories
- `sensitive_data_protection` — Block access to sensitive files

See the [Built-in Configuration Reference](../../reference/builtin-config/) for complete details.

## Plugin Configuration (Optional)

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

| Option           | Default     | Description                                             |
| ---------------- | ----------- | ------------------------------------------------------- |
| `enabled`        | `true`      | Enable/disable the plugin                               |
| `cupcakePath`    | `"cupcake"` | Path to cupcake binary                                  |
| `logLevel`       | `"info"`    | Log level: debug, info, warn, error                     |
| `timeoutMs`      | `5000`      | Max policy evaluation time (ms)                         |
| `failMode`       | `"closed"`  | `"open"` (allow on error) or `"closed"` (deny on error) |
| `cacheDecisions` | `false`     | Cache decisions (experimental)                          |

## Tool Name Mapping

OpenCode uses lowercase tool names. Cupcake normalizes them automatically:

| OpenCode | Cupcake Policy |
| -------- | -------------- |
| `bash`   | `Bash`         |
| `edit`   | `Edit`         |
| `write`  | `Write`        |
| `read`   | `Read`         |
| `grep`   | `Grep`         |
| `glob`   | `Glob`         |

## Verify Installation

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "session_id": "test-session",
  "cwd": "/tmp",
  "tool": "bash",
  "args": {
    "command": "echo 'Hello from Cupcake!'"
  }
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness opencode --policy-dir .cupcake/policies < test-event.json
```

### 3. Expected output

```json
{
  "decision": "allow"
}
```

## Key Differences from Other Harnesses

| Aspect            | Claude Code / Cursor            | OpenCode                       |
| ----------------- | ------------------------------- | ------------------------------ |
| Integration       | External hooks (stdin/stdout)   | In-process TypeScript plugin   |
| Blocking          | Return JSON `{continue: false}` | Throw Error                    |
| Ask Support       | Native                          | Converted to deny with message |
| Context Injection | `additionalContext` field       | Limited (future enhancement)   |

## Event Support

| Event       | Status    | Description                  |
| ----------- | --------- | ---------------------------- |
| PreToolUse  | Supported | Block tools before execution |
| PostToolUse | Supported | Validate after execution     |
