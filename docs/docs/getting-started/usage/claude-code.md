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
# Enable specific builtins
cupcake init --harness claude --builtins git_pre_check,git_block_no_verify

# Global with builtins
cupcake init --global --harness claude --builtins system_protection,sensitive_data_protection
```

Available builtins include:

- `git_pre_check` — Run checks before git operations
- `git_block_no_verify` — Prevent `--no-verify` flag usage
- `system_protection` — Protect system directories
- `sensitive_data_protection` — Block access to sensitive files
- `protected_paths` — Make specific paths read-only

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

You should see a JSON response with an `Allow` decision:

```json
{
  "Allow": {
    "context": []
  }
}
```
