---
title: "Factory AI"
description: "Setting up Cupcake with Factory AI"
---

# Factory AI Setup

Factory AI (Droid) supports both **project-level** and **global** configurations, similar to Claude Code. It uses the same hooks-based architecture.

## Project Setup (Recommended)

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness factory
```

This creates a `.cupcake/` directory in your project with:

- `policies/` - Your policy files
- `rulebook.yml` - Configuration file
- `signals/` - External data providers
- `actions/` - Automated responses

And configures Factory AI hooks at `.factory/settings.json`.

## Global Setup

For organization-wide policies that apply to all projects:

```bash
cupcake init --global --harness factory
```

This creates configuration at `~/.config/cupcake/` and sets up global hooks.

## Enable Built-in Policies (Optional)

```bash
# Enable specific builtins
cupcake init --harness factory --builtins git_pre_check,git_block_no_verify

# Global with builtins
cupcake init --global --harness factory --builtins system_protection,sensitive_data_protection
```

Available builtins include:

- `git_pre_check` — Run checks before git operations
- `git_block_no_verify` — Prevent `--no-verify` flag usage
- `system_protection` — Protect system directories
- `sensitive_data_protection` — Block access to sensitive files
- `protected_paths` — Make specific paths read-only
- `factory_enforce_full_file_read` — Enforce reading entire files

See the [Built-in Configuration Reference](../../reference/builtin-config/) for complete details.

## Hook Configuration

The `init` command automatically configures Factory AI hooks in `.factory/settings.json`:

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
    ]
  }
}
```

## Supported Events

Factory AI supports more events than Claude Code:

| Event             | When It Fires               | Use Case                       |
|-------------------|----------------------------|--------------------------------|
| `PreToolUse`      | Before executing any tool   | Block dangerous operations     |
| `PostToolUse`     | After tool execution        | Validate results, run checks   |
| `UserPromptSubmit`| Before sending prompt to LLM| Filter prompts, inject context |
| `SessionStart`    | When session starts/resumes | Load context, set environment  |
| `Stop`            | When agent stops            | Cleanup, logging               |
| `SubagentStop`    | When subagent completes     | Subagent coordination          |

## Similarities with Claude Code

Factory AI uses the same event structure as Claude Code, making policies portable between harnesses:

| Field         | Factory AI                   | Claude Code                  |
|---------------|------------------------------|------------------------------|
| Event type    | `input.hook_event_name`      | `input.hook_event_name`      |
| Tool name     | `input.tool_name`            | `input.tool_name`            |
| Shell command | `input.tool_input.command`   | `input.tool_input.command`   |
| File path     | `input.tool_input.file_path` | `input.tool_input.file_path` |
| Prompt        | `input.prompt`               | `input.prompt`               |

Many policies can be shared across both harnesses without modification.

## Verify Installation

Test that Cupcake is working correctly:

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "hook_event_name": "PreToolUse",
  "session_id": "test-session",
  "cwd": "/tmp",
  "tool_name": "Bash",
  "tool_input": {
    "command": "echo 'Hello from Cupcake!'"
  }
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness factory --policy-dir .cupcake/policies < test-event.json
```

### 3. Expected output

You should see a JSON response indicating the command is allowed:

```json
{
  "continue": true
}
```

## Response Formats

### Allow (Continue)

```json
{
  "continue": true
}
```

### Deny (Block)

```json
{
  "continue": false,
  "stopReason": "Dangerous command blocked"
}
```

### Allow with Context Injection

```json
{
  "continue": true,
  "hookSpecificOutput": {
    "additionalContext": "Remember to run tests before committing"
  }
}
```

Context injection is supported on `UserPromptSubmit` and `SessionStart` events.

