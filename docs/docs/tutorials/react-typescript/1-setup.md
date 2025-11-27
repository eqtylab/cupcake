---
title: "1. Setup"
description: "Prerequisites and understanding hooks for Cupcake policies"
---

# Setup

## Prerequisites

- Cupcake installed ([Installation Guide](../../getting-started/installation/))
- Cupcake initialized in your project ([Usage Guide](../../getting-started/usage/))
- A React + TypeScript application
- Claude Code as your AI coding agent

## Understanding Hooks and Tools

Cupcake integrates with Claude Code through **hooks** - events that trigger at different points in the interaction lifecycle.

### Hook Events vs Tools

There are two concepts to understand:

**1. Hook Events** - *When* something runs:
- `PreToolUse` - Before Claude executes a tool
- `PostToolUse` - After a tool completes successfully
- `UserPromptSubmit` - Before processing user input
- `SessionStart` - When a session starts
- And more...

**2. Tools** - *What* Claude is trying to do:
- `Write` - Creating a new file
- `Edit` - Modifying an existing file
- `Bash` - Running shell commands
- `Read` - Reading file contents
- `Grep` - Searching for text
- And more...

### How They Work Together

Hook events and tools combine to give you precise control:

```
Hook Event (WHEN) + Tool Matcher (WHAT) = Precise Trigger
```

**Examples:**

| Hook Event | Tool Matcher | Meaning |
|------------|--------------|---------|
| `PreToolUse` | `Write\|Edit` | Before Claude writes OR edits any file |
| `PostToolUse` | `Bash` | After Claude runs a shell command |
| `PreToolUse` | `*` | Before Claude uses ANY tool |
| `UserPromptSubmit` | *(no matcher)* | Before processing any user prompt |

**For this tutorial**, we'll use:
- **Hook Event**: `PreToolUse` (before execution)
- **Tool Matchers**: `Write` and `Edit` (file operations)
- **Result**: Our policy runs before Claude creates or modifies files

### Configuration

Hook events are configured in `.claude/settings.json`:

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

This configuration tells Claude Code:
1. On `PreToolUse` events (before tool execution)
2. When the tool matches `Write|Edit` (file operations)
3. Run `cupcake eval` to evaluate policies

**Learn More:**
- [Claude Code Hooks Documentation](https://docs.anthropic.com/en/docs/claude-code/hooks) - Official reference
- [Hooks Compatibility Reference](../../reference/hooks/) - Which hooks work with which tools
