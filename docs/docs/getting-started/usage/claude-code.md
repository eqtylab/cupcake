---
title: "Claude Code"
description: "Setting up Cupcake with Claude Code"
---

# Claude Code Setup

Claude Code supports both **project-level** and **global** configurations.

## Project Setup (Recommended)

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness claude
```

This creates a `.cupcake/` directory in your project with:

- `policies/` - Your policy files
- `rulebook.yml` - Configuration file
- `signals/` - External data providers
- `actions/` - Automated responses

And configures Claude Code hooks at `.claude/settings.json`.

## Global Setup

For organization-wide policies that apply to all projects:

```bash
cupcake init --global --harness claude
```

This creates configuration at `~/.config/cupcake/` (or equivalent on your platform) and sets up hooks at `~/.claude/settings.json`.

## Enable Built-in Policies (Optional)

Cupcake includes pre-built security policies you can enable during initialization:

```bash
# Enable specific project builtins
cupcake init --harness claude --builtins git_pre_check,git_block_no_verify

# Global with security builtins
cupcake init --global --harness claude --builtins system_protection,sensitive_data_protection
```

**Project-level builtins** (use with `cupcake init --harness claude`):

- `always_inject_on_prompt` — Add context to every user prompt
- `git_pre_check` — Run checks before git operations
- `git_block_no_verify` — Prevent `--no-verify` flag usage
- `post_edit_check` — Run validation after file edits
- `protected_paths` — Make specific paths read-only
- `rulebook_security_guardrails` — Protect `.cupcake/` files from modification
- `enforce_full_file_read` — Enforce reading entire files under a line limit

**Global-level builtins** (use with `cupcake init --global --harness claude`):

- `system_protection` — Protect system directories
- `sensitive_data_protection` — Block access to sensitive files (SSH keys, credentials)
- `cupcake_exec_protection` — Prevent direct execution of cupcake binary

See the [Built-in Configuration Reference](../../reference/builtin-config/) for complete details.

## Verify Installation

Test that Cupcake is working correctly:

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "echo 'Hello from Cupcake!'"
  },
  "session_id": "test-session",
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness claude --policy-dir .cupcake/policies < test-event.json
```

### 3. Expected output

You should see a JSON response indicating the command is allowed:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow"
  }
}
```

## Hook Configuration

The `init` command automatically configures Claude Code hooks in `.claude/settings.json`:

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

## Supported Events

Claude Code supports these hook events:

| Event              | When It Fires                | Use Case                       |
| ------------------ | ---------------------------- | ------------------------------ |
| `PreToolUse`       | Before executing any tool    | Block dangerous operations     |
| `PostToolUse`      | After tool execution         | Validate results, run checks   |
| `UserPromptSubmit` | Before sending prompt to LLM | Filter prompts, inject context |
| `SessionStart`     | When session starts/resumes  | Load context, set environment  |
| `SessionEnd`       | When session ends            | Cleanup, logging               |
| `Stop`             | When agent stops             | Cleanup, logging               |
| `SubagentStop`     | When subagent completes      | Subagent coordination          |
| `PreCompact`       | Before memory compaction     | Preserve important context     |
| `Notification`     | On agent notifications       | Monitor agent activity         |

## Response Formats

### PreToolUse - Allow

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow"
  }
}
```

### PreToolUse - Deny

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "Dangerous command blocked by policy"
  }
}
```

### PreToolUse - Ask

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "ask",
    "permissionDecisionReason": "This operation requires confirmation"
  }
}
```

### UserPromptSubmit - Context Injection

```json
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "Remember to run tests before committing"
  }
}
```

Context injection is supported on `UserPromptSubmit` and `SessionStart` events.
