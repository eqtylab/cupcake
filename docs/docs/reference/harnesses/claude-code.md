---
title: "Claude Code"
description: "Technical reference for Claude Code harness integration"
---

# Claude Code Reference

Claude Code integrates with Cupcake through external hooks configured in `.claude/settings.json`. Events are passed via stdin, and responses are returned via stdout.

## Supported Events

Claude Code supports 9 hook events:

| Event              | Description              | Context Injection | Can Block            |
| ------------------ | ------------------------ | ----------------- | -------------------- |
| `PreToolUse`       | Before tool execution    | No                | Yes (allow/deny/ask) |
| `PostToolUse`      | After tool execution     | Yes               | Yes (feedback)       |
| `UserPromptSubmit` | User submits prompt      | Yes               | Yes                  |
| `SessionStart`     | Session starts/resumes   | Yes               | No                   |
| `SessionEnd`       | Session ends             | No                | No                   |
| `PreCompact`       | Before memory compaction | Yes               | No                   |
| `Notification`     | Agent notifications      | No                | No                   |
| `Stop`             | Main agent stopping      | No                | Yes (force continue) |
| `SubagentStop`     | Subagent stopping        | No                | Yes (force continue) |

## Event Fields

### Common Fields

All Claude Code events include:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory"
}
```

### PreToolUse

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "tool_name": "Bash",
  "tool_input": {
    "command": "echo hello"
  }
}
```

**Available Tools:**

- `Bash` - Shell command execution
- `Read` - File reading
- `Write` - File creation
- `Edit` - File editing (also `MultiEdit`)
- `Grep` - Content search
- `Glob` - File pattern matching
- `WebFetch` - Web fetching
- `WebSearch` - Web searching
- `Task` - Subagent tasks
- `mcp__<server>__<tool>` - MCP tools

### PostToolUse

```json
{
  "hook_event_name": "PostToolUse",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
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
  "hook_event_name": "UserPromptSubmit",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "prompt": "Please fix the bug in main.ts"
}
```

### SessionStart

```json
{
  "hook_event_name": "SessionStart",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "source": "startup"
}
```

**Source values:** `startup`, `resume`, `clear`, `compact`

### SessionEnd

```json
{
  "hook_event_name": "SessionEnd",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "reason": "logout"
}
```

**Reason values:** `clear`, `logout`, `prompt_input_exit`, `other`

### PreCompact

```json
{
  "hook_event_name": "PreCompact",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "trigger": "manual",
  "custom_instructions": "Preserve the API documentation"
}
```

**Trigger values:** `manual`, `auto`

### Notification

```json
{
  "hook_event_name": "Notification",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "message": "Permission granted"
}
```

### Stop / SubagentStop

```json
{
  "hook_event_name": "Stop",
  "session_id": "abc123",
  "transcript_path": "/path/to/transcript.md",
  "cwd": "/working/directory",
  "stop_hook_active": false
}
```

## Response Formats

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

**Ask (prompt user):**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "ask",
    "permissionDecisionReason": "This operation requires confirmation"
  }
}
```

### UserPromptSubmit Responses

**Block:**

```json
{
  "decision": "block",
  "reason": "Prompt contains prohibited content"
}
```

**Allow with context injection:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "Remember: Always run tests before committing"
  }
}
```

### SessionStart Responses

**Block:**

```json
{
  "continue": false,
  "stopReason": "Session blocked by policy"
}
```

**Allow with context:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "Project uses TypeScript strict mode"
  }
}
```

### PostToolUse Responses

**Block (provide feedback):**

```json
{
  "decision": "block",
  "reason": "Linting failed: missing semicolon on line 42"
}
```

**Allow with context:**

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "File validated successfully"
  }
}
```

### PreCompact Responses

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreCompact",
    "customInstructions": "Preserve API documentation and test patterns"
  }
}
```

### Stop/SubagentStop Responses

**Force continue:**

```json
{
  "continue": false,
  "stopReason": "Please complete the remaining tasks"
}
```

**Allow stop:**

```json
{}
```

## Hook Configuration

The `cupcake init --harness claude` command configures hooks in `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|MultiEdit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval --harness claude --policy-dir $CLAUDE_PROJECT_DIR/.cupcake"
          }
        ]
      }
    ]
  }
}
```

### Matcher Patterns

- `*` - Match all tools
- `Bash` - Match specific tool
- `Write|Edit` - Match multiple tools (OR)
- `mcp__*` - Match all MCP tools

### Hook Configuration Options

```json
{
  "type": "command",
  "command": "cupcake eval --harness claude",
  "timeout": 30000,
  "cwd": "/path/to/dir"
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
package cupcake.policies.my_policy

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "rule_id": "SAFETY-001",
        "reason": "Destructive command blocked",
        "severity": "CRITICAL"
    }
}
```

### Context Injection Policy

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.context_injection

import rego.v1

add_context contains message if {
    input.hook_event_name == "UserPromptSubmit"
    message := "Remember to follow the coding standards in CONTRIBUTING.md"
}
```

## Resources

- [Claude Code Hooks Documentation](https://docs.anthropic.com/en/docs/claude-code/hooks) - Official reference
- [Setup Guide](../../getting-started/usage/claude-code.md) - Installation and configuration
- [Claude Code Tutorial](../../tutorials/claude-code.md) - Hands-on walkthrough
