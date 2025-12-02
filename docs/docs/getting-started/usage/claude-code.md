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

This creates:

- `.cupcake/` directory with policies and configuration
- `.claude/settings.json` with hook configuration

## Global Setup

For organization-wide policies that apply to all projects:

```bash
cupcake init --global --harness claude
```

This creates configuration at `~/.config/cupcake/` and sets up hooks at `~/.claude/settings.json`.

## Enable Built-in Policies

Cupcake includes pre-built security policies you can enable during initialization:

```bash
# Project-level builtins
cupcake init --harness claude --builtins git_pre_check,protected_paths

# Global security builtins
cupcake init --global --harness claude --builtins system_protection,sensitive_data_protection
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
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "echo 'Hello from Cupcake!'" },
  "session_id": "test",
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}
EOF

# Evaluate
cupcake eval --harness claude < test-event.json
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

- [Claude Code Reference](../../reference/harnesses/claude-code.md) - Events, response formats, hook configuration
- [Writing Policies](../../reference/policies/custom.md) - Create custom Rego policies
- [Claude Code Tutorial](../../tutorials/claude-code.md) - Hands-on walkthrough
