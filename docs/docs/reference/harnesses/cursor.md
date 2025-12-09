---
title: "Cursor"
description: "Technical reference for Cursor harness integration"
---

# Cursor Reference

Cursor integrates with Cupcake through hooks configured in `.cursor/hooks.json` (project-level) or `~/.cursor/hooks.json` (global).

## Supported Events

| Event                  | Type             | Can Block | Response |
| ---------------------- | ---------------- | --------- | -------- |
| `beforeShellExecution` | Before action    | Yes       | `permission`, `user_message`, `agent_message` |
| `beforeMCPExecution`   | Before action    | Yes       | `permission`, `user_message`, `agent_message` |
| `beforeReadFile`       | Before action    | Yes       | `permission` only |
| `beforeSubmitPrompt`   | Before action    | Yes       | `continue`, `user_message` |
| `afterShellExecution`  | After action     | No        | `{}` (fire-and-forget) |
| `afterMCPExecution`    | After action     | No        | `{}` (fire-and-forget) |
| `afterFileEdit`        | After action     | No        | `{}` (fire-and-forget) |
| `afterAgentResponse`   | After action     | No        | `{}` (fire-and-forget) |
| `afterAgentThought`    | After action     | No        | `{}` (fire-and-forget) |
| `stop`                 | Lifecycle        | Yes       | `followup_message` (optional) |

## Common Input Fields

All events include:

```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "conv-123",
  "generation_id": "gen-456",
  "workspace_roots": ["/path/to/project"],
  "model": "gpt-4",
  "cursor_version": "2.0.77",
  "user_email": "user@example.com"
}
```

The `model`, `cursor_version`, and `user_email` fields are optional.

## Event-Specific Fields

### beforeShellExecution / afterShellExecution

```json
{
  "command": "npm install express",
  "cwd": "/path/to/project",
  "output": "...",      // afterShellExecution only
  "duration": 150       // afterShellExecution only (ms)
}
```

### beforeMCPExecution / afterMCPExecution

```json
{
  "tool_name": "database_query",
  "tool_input": "{\"query\": \"SELECT * FROM users\"}",
  "result_json": "...",  // afterMCPExecution only
  "duration": 250        // afterMCPExecution only (ms)
}
```

### beforeReadFile

```json
{
  "file_path": "/path/to/secrets.env",
  "content": "API_KEY=...",
  "attachments": [{"type": "file", "filePath": "/path/to/.cursorrules"}]
}
```

### afterFileEdit

```json
{
  "file_path": "/path/to/main.ts",
  "edits": [{"old_string": "const foo = 1", "new_string": "const foo = 2"}]
}
```

### beforeSubmitPrompt

```json
{
  "prompt": "Fix the bug in main.ts",
  "attachments": [{"type": "rule", "filePath": "/path/to/.cursorrules"}]
}
```

### afterAgentResponse

```json
{
  "text": "Here's the fix for the bug..."
}
```

### afterAgentThought

```json
{
  "text": "I need to analyze the code structure...",
  "duration_ms": 1500
}
```

### stop

```json
{
  "status": "completed",
  "loop_count": 2
}
```

- `status`: `completed`, `aborted`, or `error`
- `loop_count`: Number of auto-followups already triggered (max 5 enforced by Cursor)

## Response Formats

**Response fields use snake_case:** `user_message`, `agent_message` (not camelCase).

### Permission Events (beforeShellExecution, beforeMCPExecution)

```json
// Allow
{"permission": "allow"}

// Deny
{
  "permission": "deny",
  "user_message": "Command blocked by policy",
  "agent_message": "Policy BLOCK-001 triggered"
}

// Ask user
{
  "permission": "ask",
  "question": "Allow system modification?",
  "user_message": "Requires approval"
}
```

### beforeReadFile

```json
{"permission": "allow"}
// or
{"permission": "deny"}
```

No message fields supported.

### beforeSubmitPrompt

```json
// Allow
{"continue": true}

// Block
{
  "continue": false,
  "user_message": "Prompt blocked by policy"
}
```

Context injection is NOT supported.

### stop (Agent Loop Control)

```json
// Allow agent to stop
{}

// Continue agent loop with followup message
{"followup_message": "Tests are still failing. Please fix them."}
```

When `followup_message` is returned, Cursor submits it as the next user message, continuing the agent loop. Cursor enforces a maximum of 5 auto-followups.

### Fire-and-Forget Events

All `after*` events return empty: `{}`

## Setup

### Project-Level (Recommended)

```bash
cd /path/to/project
cupcake init --harness cursor
```

Creates `.cursor/hooks.json` in the project directory.

### Global

```bash
cupcake init --global --harness cursor
```

Creates `~/.cursor/hooks.json` for all projects.

## Policy Examples

### Block Dangerous Commands

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.block_dangerous

import rego.v1

deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "rm -rf")
    decision := {
        "rule_id": "BLOCK-DANGEROUS",
        "reason": "Destructive command blocked",
        "severity": "CRITICAL"
    }
}
```

### Agent Loop Control

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["stop"]
package cupcake.policies.ensure_tests_pass

import rego.v1

deny contains decision if {
    input.hook_event_name == "stop"
    input.loop_count < 5  # Respect Cursor's limit
    input.status == "completed"
    # Add your condition here (e.g., check test results via signal)
    decision := {
        "rule_id": "ENSURE-TESTS",
        "reason": "Please verify tests pass before finishing.",
        "severity": "MEDIUM"
    }
}
```

## Differences from Claude Code

| Feature           | Claude Code                     | Cursor                              |
| ----------------- | ------------------------------- | ----------------------------------- |
| Hook location     | `.claude/` or `~/.claude/`      | `.cursor/` or `~/.cursor/`          |
| Context injection | Supported on prompts            | Not supported                       |
| Input modification| Supported via `updatedInput`    | Not supported                       |
| Stop continuation | `block` + `reason` (feedback)   | `followup_message` (new user msg)   |
| Loop prevention   | `stop_hook_active` (manual)     | `loop_count` (automatic, max 5)     |
| Response casing   | camelCase                       | snake_case                          |

## Resources

- [Setup Guide](../../getting-started/usage/cursor.md)
- [Cursor Tutorial](../../tutorials/cursor.md)
