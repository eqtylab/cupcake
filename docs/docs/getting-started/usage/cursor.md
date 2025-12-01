---
title: "Cursor"
description: "Setting up Cupcake with Cursor"
---

# Cursor Setup

**Important**: Cursor only supports **global** hooks (not project-level). All hooks are configured at `~/.cursor/hooks.json`.

## Project Setup with Global Hooks

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness cursor
```

This creates a `.cupcake/` directory in your project with policies and configuration, and sets up **global hooks** at `~/.cursor/hooks.json` that use relative paths (`.cupcake`).

When you open this project in Cursor, the hooks will automatically find the project's `.cupcake/` directory.

## Global Setup

For organization-wide policies:

```bash
cupcake init --global --harness cursor
```

This creates configuration at `~/.config/cupcake/` and sets up global hooks at `~/.cursor/hooks.json` with absolute paths.

## Enable Built-in Policies (Optional)

Cupcake includes pre-built security policies you can enable during initialization:

```bash
# Enable specific project builtins
cupcake init --harness cursor --builtins git_pre_check,git_block_no_verify

# Global with security builtins
cupcake init --global --harness cursor --builtins system_protection,sensitive_data_protection
```

**Project-level builtins** (use with `cupcake init --harness cursor`):

- `always_inject_on_prompt` — Add context to every user prompt
- `git_pre_check` — Run checks before git operations
- `git_block_no_verify` — Prevent `--no-verify` flag usage
- `post_edit_check` — Run validation after file edits
- `protected_paths` — Make specific paths read-only
- `rulebook_security_guardrails` — Protect `.cupcake/` files from modification
- `enforce_full_file_read` — Enforce reading entire files under a line limit

**Global-level builtins** (use with `cupcake init --global --harness cursor`):

- `system_protection` — Protect system directories
- `sensitive_data_protection` — Block access to sensitive files (SSH keys, credentials)
- `cupcake_exec_protection` — Prevent direct execution of cupcake binary

See the [Built-in Configuration Reference](../../reference/builtin-config/) for complete details.

## Hook Configuration

The `init` command automatically configures Cursor hooks in `~/.cursor/hooks.json`:

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

## Supported Events

Cursor supports these hook events:

| Event                  | When It Fires             | Use Case                        |
| ---------------------- | ------------------------- | ------------------------------- |
| `beforeShellExecution` | Before shell command      | Block dangerous commands        |
| `beforeMCPExecution`   | Before MCP tool execution | Control MCP tool access         |
| `afterFileEdit`        | After file is edited      | Validate changes, run checks    |
| `beforeReadFile`       | Before reading a file     | Block access to sensitive files |
| `beforeSubmitPrompt`   | Before prompt submission  | Filter prompts                  |
| `stop`                 | When agent loop ends      | Cleanup, logging                |

## Verify Installation

Test that Cupcake is working correctly:

### 1. Create a test event

Save this to `test-event.json`:

```json
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "test-conversation",
  "generation_id": "test-generation",
  "workspace_roots": ["/tmp"],
  "command": "echo 'Hello from Cupcake!'",
  "cwd": "/tmp"
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness cursor --policy-dir .cupcake < test-event.json
```

### 3. Expected output

You should see a JSON response indicating the command is allowed:

```json
{
  "permission": "allow"
}
```

## Response Formats

Cursor events return different response formats depending on the event type.

### beforeShellExecution / beforeMCPExecution - Allow

```json
{
  "permission": "allow"
}
```

### beforeShellExecution / beforeMCPExecution - Deny

```json
{
  "permission": "deny",
  "userMessage": "Command blocked by policy",
  "agentMessage": "Command blocked by policy"
}
```

### beforeShellExecution / beforeMCPExecution - Ask

```json
{
  "permission": "ask",
  "question": "This operation requires confirmation",
  "userMessage": "This operation requires confirmation",
  "agentMessage": "This operation requires confirmation"
}
```

### beforeReadFile - Allow/Deny

```json
{
  "permission": "allow"
}
```

```json
{
  "permission": "deny"
}
```

Note: `beforeReadFile` does not support `userMessage` or `agentMessage` fields.

### beforeSubmitPrompt - Allow/Deny

```json
{
  "continue": true
}
```

```json
{
  "continue": false
}
```

Note: `beforeSubmitPrompt` only supports a boolean `continue` field. Context injection is **not** supported for Cursor prompts (unlike Claude Code).

### afterFileEdit / stop

These events return an empty response:

```json
{}
```

## Key Differences from Claude Code

| Aspect              | Claude Code                    | Cursor                     |
| ------------------- | ------------------------------ | -------------------------- |
| Hook location       | Project or global              | Global only (`~/.cursor/`) |
| Config file         | `.claude/settings.json`        | `~/.cursor/hooks.json`     |
| Hook format         | Complex with `matcher`, `type` | Simple with just `command` |
| Context injection   | Supported on prompts           | Not supported              |
| PreToolUse response | `permissionDecision` in output | `permission` at top level  |
