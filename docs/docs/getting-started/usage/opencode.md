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

## Project Setup

### Step 1: Initialize Cupcake

```bash
cd /path/to/your/project
cupcake init --harness opencode
```

This creates a `.cupcake/` directory with policies and configuration.

### Step 2: Create the System Evaluator

```bash
mkdir -p .cupcake/policies/opencode/system

cat > .cupcake/policies/opencode/system/evaluate.rego << 'EOF'
package cupcake.system

import rego.v1

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

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]

    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]

    result := all_decisions
}

default collect_verbs(_) := []
EOF
```

### Step 3: Install the Plugin

Build and install the Cupcake plugin for OpenCode:

```bash
# Build the plugin
cd /path/to/cupcake/cupcake-plugins/opencode
npm install && npm run build

# Install to your project
cd /path/to/your/project
mkdir -p .opencode/plugins/cupcake
cp -r /path/to/cupcake/cupcake-plugins/opencode/dist/* .opencode/plugins/cupcake/
cp /path/to/cupcake/cupcake-plugins/opencode/package.json .opencode/plugins/cupcake/
```

## Global Setup

For organization-wide policies:

```bash
# Initialize global config
cupcake init --global --harness opencode

# Install plugin globally
mkdir -p ~/.config/opencode/plugins/cupcake
cp -r /path/to/cupcake/cupcake-plugins/opencode/dist/* ~/.config/opencode/plugins/cupcake/
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

## Plugin Configuration

Create `.cupcake/opencode.json` to customize behavior:

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
|------------------|-------------|---------------------------------------------------------|
| `enabled`        | `true`      | Enable/disable the plugin                               |
| `cupcakePath`    | `"cupcake"` | Path to cupcake binary                                  |
| `logLevel`       | `"info"`    | Log level: debug, info, warn, error                     |
| `timeoutMs`      | `5000`      | Max policy evaluation time (ms)                         |
| `failMode`       | `"closed"`  | `"open"` (allow on error) or `"closed"` (deny on error) |
| `cacheDecisions` | `false`     | Cache decisions (experimental)                          |

## Tool Name Mapping

OpenCode uses lowercase tool names. Cupcake normalizes them automatically:

| OpenCode  | Cupcake Policy |
|-----------|----------------|
| `bash`    | `Bash`         |
| `edit`    | `Edit`         |
| `write`   | `Write`        |
| `read`    | `Read`         |
| `grep`    | `Grep`         |
| `glob`    | `Glob`         |

## Verify Installation

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "test",
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
|-------------------|--------------------------------|--------------------------------|
| Integration       | External hooks (stdin/stdout)   | In-process TypeScript plugin   |
| Blocking          | Return JSON `{continue: false}` | Throw Error                    |
| Ask Support       | Native                          | Converted to deny with message |
| Context Injection | `additionalContext` field       | Limited (future enhancement)   |

## Event Support

| Event       | Status    | Description                  |
|-------------|-----------|------------------------------|
| PreToolUse  | Supported | Block tools before execution |
| PostToolUse | Supported | Validate after execution     |

