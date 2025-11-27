---
title: "Cursor"
description: "Setting up Cupcake with Cursor"
---

# Cursor Setup

**Important**: Cursor only supports **global** hooks (not project-level).

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
# Enable specific builtins
cupcake init --harness cursor --builtins git_pre_check,git_block_no_verify

# Global with builtins
cupcake init --global --harness cursor --builtins system_protection,sensitive_data_protection
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
  "eventType": "beforeShellExecution",
  "command": "echo 'Hello from Cupcake!'",
  "workingDirectory": "/tmp"
}
```

### 2. Evaluate the event

```bash
cupcake eval --harness cursor --policy-dir .cupcake/policies < test-event.json
```

### 3. Expected output

You should see a JSON response indicating the command is allowed:

```json
{
  "continue": true
}
```
