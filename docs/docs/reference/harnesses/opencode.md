---
title: "OpenCode"
description: "Technical reference for OpenCode harness integration"
---

# OpenCode Reference

OpenCode uses an **in-process TypeScript plugin** rather than external hooks. The plugin intercepts tool execution and spawns `cupcake eval` for policy evaluation.

## Architecture

```
OpenCode Process
  └── Cupcake Plugin (.opencode/plugin/cupcake.js)
        └── spawns: cupcake eval --harness opencode
              └── Returns: { decision: "allow" | "deny" }
                    └── Plugin throws Error to block
```

Unlike Claude Code and Cursor which use stdin/stdout hooks, OpenCode's plugin:

- Runs inside the OpenCode process
- Spawns `cupcake eval` as a subprocess for each tool call
- Throws an Error to block tool execution

## Supported Events

OpenCode has a simpler event model focused on tool execution:

| Event         | Description                                   |
| ------------- | --------------------------------------------- |
| `PreToolUse`  | Before tool execution (`tool.execute.before`) |
| `PostToolUse` | After tool execution (`tool.execute.after`)   |

**Note:** OpenCode does not support prompt events, session events, or compaction events.

## Event Fields

### Common Fields

All OpenCode events include:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "session-123",
  "cwd": "/path/to/project",
  "agent": "main",
  "message_id": "msg-456"
}
```

### PreToolUse

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "session-123",
  "cwd": "/path/to/project",
  "agent": "main",
  "message_id": "msg-456",
  "tool": "bash",
  "args": {
    "command": "npm install express"
  }
}
```

### PostToolUse

```json
{
  "hook_event_name": "PostToolUse",
  "session_id": "session-123",
  "cwd": "/path/to/project",
  "agent": "main",
  "message_id": "msg-456",
  "tool": "bash",
  "args": {
    "command": "npm install express"
  },
  "result": {
    "success": true,
    "output": "added 57 packages",
    "exit_code": 0
  }
}
```

### Tool Name Mapping

OpenCode uses lowercase tool names. Cupcake normalizes them automatically:

| OpenCode | Cupcake Policy |
| -------- | -------------- |
| `bash`   | `Bash`         |
| `edit`   | `Edit`         |
| `write`  | `Write`        |
| `read`   | `Read`         |
| `grep`   | `Grep`         |
| `glob`   | `Glob`         |

## Response Format

OpenCode uses a simple, unified response format:

**Allow:**

```json
{
  "decision": "allow"
}
```

**Deny:**

```json
{
  "decision": "deny",
  "reason": "Policy blocked: dangerous command"
}
```

**Block (hard block):**

```json
{
  "decision": "block",
  "reason": "Critical security violation"
}
```

**Ask (converted to deny):**

```json
{
  "decision": "deny",
  "reason": "This operation requires approval: <original ask reason>"
}
```

**Note:** OpenCode does not have native "ask" support. Ask decisions are converted to deny with the approval message included in the reason.

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

| Option           | Default     | Description                                             |
| ---------------- | ----------- | ------------------------------------------------------- |
| `enabled`        | `true`      | Enable/disable the plugin                               |
| `cupcakePath`    | `"cupcake"` | Path to cupcake binary                                  |
| `logLevel`       | `"info"`    | Log level: debug, info, warn, error                     |
| `timeoutMs`      | `5000`      | Max policy evaluation time (ms)                         |
| `failMode`       | `"closed"`  | `"open"` (allow on error) or `"closed"` (deny on error) |
| `cacheDecisions` | `false`     | Cache decisions (experimental)                          |

### Fail Mode

- **`closed`** (default, recommended): If policy evaluation fails or times out, the action is denied. This is the secure option.
- **`open`**: If policy evaluation fails, the action is allowed. Use only in development.

## Plugin Installation

The plugin is automatically installed by `cupcake init --harness opencode`:

**Project-level:**

```
.opencode/plugin/cupcake.js
```

**Global:**

```
~/.config/opencode/plugin/cupcake.js
```

### Manual Installation

```bash
# Download from releases
mkdir -p .opencode/plugin
curl -fsSL https://github.com/eqtylab/cupcake/releases/download/opencode-plugin-latest/opencode-plugin.js \
  -o .opencode/plugin/cupcake.js

# Or build from source
cd cupcake-plugins/opencode
npm install && npm run build
cp dist/cupcake.js /path/to/project/.opencode/plugin/
```

## Writing Policies

### Basic Policy Structure

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.shell_policy

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "rule_id": "OC-SAFETY-001",
        "reason": "Destructive command blocked",
        "severity": "CRITICAL"
    }
}
```

### Post-Execution Validation

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PostToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.post_exec

import rego.v1

deny contains decision if {
    input.hook_event_name == "PostToolUse"
    input.tool_name == "Bash"

    # Check if command failed
    input.result.success == false

    decision := {
        "rule_id": "OC-EXEC-001",
        "reason": concat("", ["Command failed: ", input.result.error]),
        "severity": "MEDIUM"
    }
}
```

## Key Differences from Other Harnesses

| Feature            | Claude Code / Cursor          | OpenCode                       |
| ------------------ | ----------------------------- | ------------------------------ |
| Integration        | External hooks (stdin/stdout) | In-process TypeScript plugin   |
| Blocking mechanism | Return JSON response          | Throw Error                    |
| Ask support        | Native                        | Converted to deny with message |
| Context injection  | `additionalContext` field     | Limited (future enhancement)   |
| Prompt events      | Yes                           | No                             |
| Session events     | Yes                           | No                             |

## Resources

- [Setup Guide](../../getting-started/usage/opencode.md) - Installation and configuration
- [OpenCode Tutorial](../../tutorials/opencode.md) - Hands-on walkthrough
- [Plugin Source Code](https://github.com/eqtylab/cupcake/tree/main/cupcake-plugins/opencode) - TypeScript plugin implementation
