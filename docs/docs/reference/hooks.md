---
layout: "@/layouts/mdx-layout.astro"
heading: "Hooks Compatibility"
description: "Reference for which Claude Code hooks work with which tools"
---

## Overview

Claude Code provides several hook events that trigger at different points in the interaction lifecycle. This reference shows which hooks work with which tools.

For complete details on hooks configuration and behavior, see the [Claude Code Hooks Documentation](https://code.claude.com/docs/en/hooks).

## Hook Events and Tool Compatibility

### PreToolUse

Runs **before** Claude processes a tool call. Use this to validate, modify, or block actions before they execute.

**Compatible Tools:**

- `Task` - Subagent tasks
- `Bash` - Shell commands
- `Glob` - File pattern matching
- `Grep` - Content search
- `Read` - File reading
- `Edit` - File editing
- `Write` - File writing
- `WebFetch` - Web fetching
- `WebSearch` - Web searching
- MCP tools (pattern: `mcp__<server>__<tool>`)

**Decision Control:**

- `"allow"` - Bypass permission system
- `"deny"` - Block tool execution
- `"ask"` - Prompt user for confirmation

**Example:**

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

### PostToolUse

Runs **after** a tool completes successfully. Use this for validation, logging, or triggering follow-up actions.

**Compatible Tools:**

- Same as PreToolUse (all tools)

**Decision Control:**

- `"block"` - Provide feedback to Claude about the result
- `undefined` - No feedback

**Example:**

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

### PermissionRequest

Runs when the user is shown a permission dialog. Use this to automatically allow or deny permissions.

**Compatible Tools:**

- Same as PreToolUse (all tools)

**Decision Control:**

- `"allow"` - Grant permission automatically
- `"deny"` - Deny permission automatically

### UserPromptSubmit

Runs when the user submits a prompt, **before** Claude processes it. Use this to add context or validate prompts.

**No tool matcher** - applies to all user prompts.

**Decision Control:**

- `"block"` - Prevent prompt processing
- `undefined` - Allow prompt with optional context

**Context Injection:**

- Plain text to stdout (simple)
- `additionalContext` field (structured)

**Example:**

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

### Stop

Runs when Claude finishes responding. Use this to force continuation or provide follow-up tasks.

**No tool matcher** - applies to main agent stop events.

**Decision Control:**

- `"block"` - Prevent Claude from stopping
- `undefined` - Allow stop

### SubagentStop

Runs when a subagent (Task tool) finishes. Use this to validate subagent completion.

**No tool matcher** - applies to subagent stop events.

**Decision Control:**

- Same as Stop

### SessionStart

Runs when Claude Code starts or resumes a session. Use this to load context or set up environment.

**No tool matcher** - applies to session start events.

**Matchers:**

- `startup` - Fresh session start
- `resume` - Resume from `/resume`
- `clear` - After `/clear`
- `compact` - After compaction

**Context Injection:**

- `additionalContext` field

### SessionEnd

Runs when a Claude Code session ends. Use this for cleanup or logging.

**No tool matcher** - applies to session end events.

**No decision control** - cannot prevent session end.

### PreCompact

Runs before Claude Code compacts the conversation. Use this to add context before compaction.

**Matchers:**

- `manual` - User-triggered `/compact`
- `auto` - Auto-triggered compaction

**Context Injection:**

- stdout joined with `\n\n` (double newline)

### Notification

Runs when Claude Code sends notifications. Use this for custom alerts or logging.

**Matchers:**

- `permission_prompt` - Permission requests
- `idle_prompt` - Idle state (60+ seconds)
- `auth_success` - Authentication success
- `elicitation_dialog` - MCP tool elicitation

## Hook + Tool Combinations for Cupcake

Common Cupcake integration patterns:

### Pattern 1: Validate Before File Changes

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

### Pattern 2: Validate After File Changes

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

### Pattern 3: Add Context to Every Prompt

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

### Pattern 4: Validate Shell Commands

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

## Quick Reference Table

| Hook Event        | Tool Matcher | When It Runs             | Can Block             | Can Add Context |
| ----------------- | ------------ | ------------------------ | --------------------- | --------------- |
| PreToolUse        | Yes          | Before tool executes     | Yes                   | No              |
| PostToolUse       | Yes          | After tool succeeds      | Feedback only         | Yes             |
| PermissionRequest | Yes          | On permission dialog     | Yes                   | No              |
| UserPromptSubmit  | No           | Before processing prompt | Yes                   | Yes             |
| Stop              | No           | After Claude responds    | Yes (forces continue) | No              |
| SubagentStop      | No           | After subagent responds  | Yes (forces continue) | No              |
| SessionStart      | Trigger type | Session start            | No                    | Yes             |
| SessionEnd        | No           | Session end              | No                    | No              |
| PreCompact        | Trigger type | Before compaction        | No                    | Yes             |
| Notification      | Type         | On notification          | No                    | No              |

## Resources

- [Claude Code Hooks Documentation](https://code.claude.com/docs/en/hooks) - Official reference
- [Claude Code Hooks Guide](https://code.claude.com/docs/en/hooks-guide) - Examples and quickstart
