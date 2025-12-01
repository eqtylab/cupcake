---
title: "Built-in Policies"
description: "Using Cupcake's pre-built security policies"
---

# Built-in Policies

Cupcake includes battle-tested security policies ready to use. Enable and configure them in your `.cupcake/rulebook.yml`.

## Enabling Builtins

Edit your `.cupcake/rulebook.yml` to enable built-in policies:

```yaml
builtins:
  git_pre_check:
    enabled: true
    checks:
      - command: "npm test"
        message: "Tests must pass before commit"

  protected_paths:
    enabled: true
    paths:
      - "/etc/"
      - "~/.ssh/"
```

## Available Builtins

See the **[Built-in Configuration Reference](../builtin-config/)** for the complete list of available builtins and all their configuration options.

### Project-Level Builtins

| Builtin                       | Description                                             |
| ----------------------------- | ------------------------------------------------------- |
| `always_inject_on_prompt`     | Add context to every user prompt                        |
| `git_pre_check`               | Run validation commands before git operations           |
| `git_block_no_verify`         | Prevent `--no-verify` flag in git commits               |
| `post_edit_check`             | Run validation after file edits                         |
| `protected_paths`             | Block modifications to specified paths (read allowed)   |
| `rulebook_security_guardrails`| Protect `.cupcake/` files from any access               |
| `enforce_full_file_read`      | Enforce reading entire files under a line limit         |

### Global-Level Builtins

| Builtin                       | Description                                             |
| ----------------------------- | ------------------------------------------------------- |
| `system_protection`           | Protect system directories (`/etc`, `/bin`, etc.)       |
| `sensitive_data_protection`   | Block access to sensitive files (SSH keys, credentials) |
| `cupcake_exec_protection`     | Prevent direct execution of cupcake binary              |

## Enabling at Init Time

You can enable builtins when initializing Cupcake:

```bash
# Enable specific builtins
cupcake init --harness claude --builtins git_pre_check,git_block_no_verify

# Enable multiple security builtins
cupcake init --harness claude --builtins system_protection,sensitive_data_protection,protected_paths
```

## Global vs Project Builtins

- **Project builtins** — Configured in `.cupcake/rulebook.yml`, apply to one project
- **Global builtins** — Configured in `~/.config/cupcake/rulebook.yml`, apply to all projects

Global builtins take precedence and cannot be overridden by project configuration.
