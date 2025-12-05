---
title: "Cursor"
description: "Setting up Cupcake with Cursor"
---

# Cursor Setup

Cursor only supports **global** hooks (not project-level). All hooks are configured at `~/.cursor/hooks.json`.

## Project Setup with Global Hooks

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness cursor
```

This creates:

- `.cupcake/` directory in your project with policies
- `~/.cursor/hooks.json` with global hook configuration (uses relative paths)

When you open this project in Cursor, the hooks will automatically find the project's `.cupcake/` directory.

## Global Setup

For organization-wide policies:

```bash
cupcake init --global --harness cursor
```

This creates configuration at `~/.config/cupcake/` and sets up global hooks with absolute paths.

## Enable Built-in Policies

```bash
# Project-level builtins
cupcake init --harness cursor --builtins git_pre_check,protected_paths

# Global security builtins
cupcake init --global --harness cursor --builtins system_protection,sensitive_data_protection
```

**Project-level builtins:**

- `always_inject_on_prompt` - Add context to every user prompt
- `git_pre_check` - Run checks before git operations
- `git_block_no_verify` - Prevent `--no-verify` flag usage
- `post_edit_check` - Run validation after file edits
- `protected_paths` - Make specific paths read-only
- `rulebook_security_guardrails` - Protect `.cupcake/` files
- `enforce_full_file_read` - Enforce reading entire files

**Global-level builtins:**

- `system_protection` - Protect system directories
- `sensitive_data_protection` - Block access to sensitive files
- `cupcake_exec_protection` - Prevent cupcake binary execution

See the [Built-in Configuration Reference](../../reference/builtin-config.md) for complete details.

## Verify Installation

Test that Cupcake is working:

```bash
# Create test event
cat > test-event.json << 'EOF'
{
  "hook_event_name": "beforeShellExecution",
  "conversation_id": "test",
  "generation_id": "test",
  "workspace_roots": ["/tmp"],
  "command": "echo 'Hello from Cupcake!'",
  "cwd": "/tmp"
}
EOF

# Evaluate
cupcake eval --harness cursor < test-event.json
```

Expected output:

```json
{
  "permission": "allow"
}
```

## Next Steps

- [Cursor Reference](../../reference/harnesses/cursor.md) - Events, response formats, hook configuration
- [Writing Policies](../../reference/policies/custom.md) - Create custom Rego policies
- [Cursor Tutorial](../../tutorials/cursor.md) - Hands-on walkthrough
