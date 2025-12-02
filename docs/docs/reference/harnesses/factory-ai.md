---
title: "Factory AI"
description: "Technical reference for Factory AI harness integration"
---

# Factory AI Reference

Factory AI (Droid) integrates with Cupcake through external hooks configured in `.factory/settings.json`. It uses a similar architecture to Claude Code but with camelCase field names and additional features like `permissionMode` and `updatedInput`.

## Supported Events

Factory AI supports 9 hook events (same as Claude Code):

| Event              | Description              | Context Injection      | Can Block |
| ------------------ | ------------------------ | ---------------------- | --------- |
| `PreToolUse`       | Before tool execution    | Yes (+ `updatedInput`) | Yes       |
| `PostToolUse`      | After tool execution     | Yes                    | Yes       |
| `UserPromptSubmit` | User submits prompt      | Yes                    | Yes       |
| `SessionStart`     | Session starts/resumes   | Yes                    | No        |
| `SessionEnd`       | Session ends             | No                     | No        |
| `PreCompact`       | Before memory compaction | Yes                    | No        |
| `Notification`     | Agent notifications      | No                     | No        |
| `Stop`             | Main agent stopping      | No                     | Yes       |
| `SubagentStop`     | Subagent stopping        | No                     | Yes       |

## Event Fields

### Common Fields

All Factory AI events include (note **camelCase** naming):

```json
{
  "hookEventName": "PreToolUse",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default"
}
```

**Permission Mode values:** `default`, `plan`, `auto-medium`, `auto-full`

### PreToolUse

```json
{
  "hookEventName": "PreToolUse",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default",
  "tool_name": "Bash",
  "tool_input": {
    "command": "echo hello"
  }
}
```

### PostToolUse

```json
{
  "hookEventName": "PostToolUse",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default",
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "/path/to/file.ts",
    "old_string": "foo",
    "new_string": "bar"
  },
  "tool_response": {
    "success": true
  }
}
```

### UserPromptSubmit

```json
{
  "hookEventName": "UserPromptSubmit",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default",
  "prompt": "Please fix the bug in main.ts"
}
```

### SessionStart

```json
{
  "hookEventName": "SessionStart",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default",
  "source": "startup"
}
```

**Source values:** `startup`, `resume`, `clear`, `compact`

### SessionEnd

```json
{
  "hookEventName": "SessionEnd",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default",
  "reason": "logout"
}
```

**Reason values:** `clear`, `logout`, `PromptInputExit`, `other`

### PreCompact

```json
{
  "hookEventName": "PreCompact",
  "sessionId": "abc123",
  "transcriptPath": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "permissionMode": "default",
  "trigger": "manual",
  "custom_instructions": ""
}
```

## Response Formats

Factory AI uses the same response format as Claude Code, with one key addition: **`updatedInput`** for PreToolUse.

### PreToolUse Responses

**Allow:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow"
  }
}
```

**Deny:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Dangerous command blocked by policy"
  }
}
```

**Ask:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "ask",
    "permissionDecisionReason": "This operation requires confirmation"
  }
}
```

**Allow with Modified Input (Factory-specific):**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "updatedInput": {
      "command": "echo 'sanitized command'"
    }
  }
}
```

The `updatedInput` field allows policies to modify tool parameters before execution. This is unique to Factory AI.

### UserPromptSubmit Responses

**Block:**

```json
{
  "decision": "block",
  "reason": "Prompt contains prohibited content"
}
```

**Allow with context:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "Remember: Always run tests before committing"
  }
}
```

### Other Responses

All other responses follow the same format as Claude Code. See the [Claude Code Reference](claude-code.md#response-formats) for details.

## Hook Configuration

The `cupcake init --harness factory` command configures hooks in `.factory/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness factory --policy-dir \"$FACTORY_PROJECT_DIR\"/.cupcake"
          }
        ]
      }
    ]
  }
}
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
package cupcake.policies.factory.shell_policy

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "rule_id": "FACTORY-SAFETY-001",
        "reason": "Destructive command blocked",
        "severity": "CRITICAL"
    }
}
```

### Using Permission Mode

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.policies.factory.auto_mode_restrictions

import rego.v1

# Block destructive operations in auto modes
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"

    # Check if running in auto mode
    startswith(input.permissionMode, "auto-")

    # Block dangerous commands in auto mode
    contains(input.tool_input.command, "rm")

    decision := {
        "rule_id": "FACTORY-AUTO-001",
        "reason": "Delete operations are not allowed in auto mode",
        "severity": "HIGH"
    }
}
```

## Policy Portability with Claude Code

Factory AI uses the same event structure as Claude Code, making most policies portable:

| Field            | Factory AI       | Claude Code       |
| ---------------- | ---------------- | ----------------- |
| Event name field | `hookEventName`  | `hook_event_name` |
| Session ID       | `sessionId`      | `session_id`      |
| Transcript path  | `transcriptPath` | `transcript_path` |
| Tool name        | `tool_name`      | `tool_name`       |
| Tool input       | `tool_input`     | `tool_input`      |
| Prompt           | `prompt`         | `prompt`          |

**Cupcake normalizes these differences** - policies using `input.hook_event_name` and `input.tool_name` work across both harnesses.

## Key Differences from Claude Code

| Feature            | Claude Code             | Factory AI                 |
| ------------------ | ----------------------- | -------------------------- |
| Field naming       | snake_case              | camelCase                  |
| Event tag field    | `hook_event_name`       | `hookEventName`            |
| Permission mode    | Not available           | `permissionMode` field     |
| Input modification | Not supported           | `updatedInput` in response |
| Config file        | `.claude/settings.json` | `.factory/settings.json`   |
| Project dir var    | `$CLAUDE_PROJECT_DIR`   | `$FACTORY_PROJECT_DIR`     |

## Resources

- [Setup Guide](../../getting-started/usage/factory-ai.md) - Installation and configuration
