---
title: "Factory AI"
description: "Setting up Cupcake with Factory AI"
---

# Factory AI Setup

Factory AI (Droid) supports both **project-level** and **global** configurations. It uses a hooks-based architecture similar to Claude Code.

## Project Setup (Recommended)

Navigate to your project directory and initialize Cupcake:

```bash
cupcake init --harness factory
```

This creates:

- `.cupcake/` directory with policies and configuration
- `.factory/settings.json` with hook configuration

## Global Setup

For organization-wide policies:

```bash
cupcake init --global --harness factory
```

This creates configuration at `~/.config/cupcake/` and sets up global hooks.

## Enable Built-in Policies

```bash
# Project-level builtins
cupcake init --harness factory --builtins git_pre_check,protected_paths

# Global security builtins
cupcake init --global --harness factory --builtins system_protection,sensitive_data_protection
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
  "hookEventName": "PreToolUse",
  "sessionId": "test",
  "transcriptPath": "/tmp/transcript.md",
  "cwd": "/tmp",
  "permissionMode": "default",
  "tool_name": "Bash",
  "tool_input": { "command": "echo 'Hello from Cupcake!'" }
}
EOF

# Evaluate
cupcake eval --harness factory < test-event.json
```

Expected output:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow"
  }
}
```

## Next Steps

- [Factory AI Reference](../../reference/harnesses/factory-ai.md) - Events, response formats, hook configuration
- [Writing Policies](../../reference/policies/custom.md) - Create custom Rego policies
