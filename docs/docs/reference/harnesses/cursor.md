---
title: "Cursor"
description: "Technical reference for Cursor harness integration"
---

# Cursor Reference

Cursor integrates with Cupcake through global hooks configured in `~/.cursor/hooks.json`. Unlike Claude Code, Cursor only supports global hooks - not project-level configuration.

## Supported Events

Cursor supports 6 hook events:

| Event                  | Description          | Response Schema           |
| ---------------------- | -------------------- | ------------------------- |
| `beforeShellExecution` | Before shell command | Full permission model     |
| `beforeMCPExecution`   | Before MCP tool      | Full permission model     |
| `beforeReadFile`       | Before file read     | Minimal (permission only) |
| `afterFileEdit`        | After file edited    | Fire-and-forget           |
| `beforeSubmitPrompt`   | Before prompt submit | Continue only             |
| `stop`                 | Agent loop ends      | Fire-and-forget           |

**Important:** Cursor's `beforeSubmitPrompt` does NOT support context injection.

## Event Fields

### Common Fields

All Cursor events include:

```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"]
}
```

### beforeShellExecution

```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "command": "npm install express",
  "cwd": "/path/to/project"
}
```

### beforeMCPExecution

```json
{
  "hook_event_name": "beforeMCPExecution",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "tool_name": "database_query",
  "tool_input": {
    "query": "SELECT * FROM users"
  },
  "url": "http://localhost:3000",
  "command": "npx mcp-server"
}
```

### beforeReadFile

```json
{
  "hook_event_name": "beforeReadFile",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "file_path": "/path/to/project/secrets.env",
  "content": "API_KEY=...",
  "attachments": [
    {
      "type": "file",
      "file_path": "/path/to/project/.cursorrules"
    }
  ]
}
```

### afterFileEdit

```json
{
  "hook_event_name": "afterFileEdit",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "file_path": "/path/to/project/src/main.ts",
  "edits": [
    {
      "old_string": "const foo = 1",
      "new_string": "const foo = 2"
    }
  ]
}
```

### beforeSubmitPrompt

```json
{
  "hook_event_name": "beforeSubmitPrompt",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "prompt": "Fix the bug in main.ts",
  "attachments": [
    {
      "type": "rule",
      "file_path": "/path/to/project/.cursorrules"
    }
  ]
}
```

### stop

```json
{
  "hook_event_name": "stop",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "status": "completed"
}
```

**Status values:** `completed`, `aborted`, `error`

## Response Formats

### Full Permission Model

Used by `beforeShellExecution` and `beforeMCPExecution`:

**Allow:**

```json
{
  "permission": "allow"
}
```

**Deny:**

```json
{
  "permission": "deny",
  "userMessage": "This command is not allowed",
  "agentMessage": "Policy blocked: dangerous command pattern detected"
}
```

**Ask (prompt user):**

```json
{
  "permission": "ask",
  "question": "This command modifies system files. Continue?",
  "userMessage": "System modification detected",
  "agentMessage": "Awaiting user confirmation for system modification"
}
```

### Minimal Schema

Used by `beforeReadFile`:

**Allow:**

```json
{
  "permission": "allow"
}
```

**Deny:**

```json
{
  "permission": "deny"
}
```

Note: `beforeReadFile` does not support `userMessage` or `agentMessage`.

### Continue Only

Used by `beforeSubmitPrompt`:

**Allow:**

```json
{
  "continue": true
}
```

**Block:**

```json
{
  "continue": false
}
```

Note: `beforeSubmitPrompt` only supports a boolean `continue` field. **Context injection is NOT supported.**

### Fire-and-Forget

Used by `afterFileEdit` and `stop`:

```json
{}
```

These events don't expect a response that affects agent behavior.

## Hook Configuration

The `cupcake init --harness cursor` command configures hooks in `~/.cursor/hooks.json`:

```json
{
  "version": 1,
  "hooks": {
    "beforeShellExecution": [
      {
        "command": "cupcake eval --harness cursor --policy-dir .cupcake"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "cupcake eval --harness cursor --policy-dir .cupcake"
      }
    ],
    "afterFileEdit": [
      {
        "command": "cupcake eval --harness cursor --policy-dir .cupcake"
      }
    ],
    "beforeReadFile": [
      {
        "command": "cupcake eval --harness cursor --policy-dir .cupcake"
      }
    ],
    "beforeSubmitPrompt": [
      {
        "command": "cupcake eval --harness cursor --policy-dir .cupcake"
      }
    ],
    "stop": [
      {
        "command": "cupcake eval --harness cursor --policy-dir .cupcake"
      }
    ]
  }
}
```

**Note:** Cursor hooks use relative paths (`.cupcake`) which resolve to the current project directory. For global policies, use absolute paths.

## Writing Policies

### Basic Policy Structure

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.cursor.shell_policy

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")

    decision := {
        "rule_id": "CURSOR-SAFETY-001",
        "reason": "Destructive command blocked",
        "severity": "CRITICAL"
    }
}
```

### Protecting Sensitive Files

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeReadFile"]
package cupcake.policies.cursor.protect_secrets

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeReadFile"
    endswith(input.file_path, ".env")

    decision := {
        "rule_id": "CURSOR-SECRET-001",
        "reason": "Access to .env files is restricted",
        "severity": "HIGH"
    }
}
```

### Post-Edit Validation

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["afterFileEdit"]
#   signals:
#     - eslint-check
package cupcake.policies.cursor.post_edit_lint

import rego.v1

deny contains decision if {
    input.hook_event_name == "afterFileEdit"
    endswith(input.file_path, ".ts")

    lint_result := input.signals.eslint_check
    is_object(lint_result)
    lint_result.exit_code != 0

    decision := {
        "rule_id": "CURSOR-LINT-001",
        "reason": concat("", ["Linting failed: ", lint_result.output]),
        "severity": "MEDIUM"
    }
}
```

## Key Differences from Claude Code

| Feature           | Claude Code                                  | Cursor                                        |
| ----------------- | -------------------------------------------- | --------------------------------------------- |
| Hook location     | Project or global                            | Global only (`~/.cursor/`)                    |
| Config file       | `.claude/settings.json`                      | `~/.cursor/hooks.json`                        |
| Config format     | Complex with `matcher`, `type`               | Simple with just `command`                    |
| Context injection | Supported on prompts                         | Not supported                                 |
| Response field    | `permissionDecision` in `hookSpecificOutput` | `permission` at top level                     |
| Event naming      | `PreToolUse`, `PostToolUse`                  | `beforeShellExecution`, `afterFileEdit`, etc. |

## Resources

- [Setup Guide](../../getting-started/usage/cursor.md) - Installation and configuration
- [Cursor Tutorial](../../tutorials/cursor.md) - Hands-on walkthrough
