---
layout: "@/layouts/mdx-layout.astro"
title: "Cupcake CLI"
heading: "Cupcake CLI"
description: "Cupcake CLI overview"
---

Cupcake provides a powerful command-line interface for managing AI agent governance policies. This guide walks through the core commands with visual demonstrations.

> **New to Cupcake?** See the [Installation Guide](installation.md) to get started.

## Quick Start

### Initialize a Project

Set up Cupcake in your project with a single command:

```bash
cupcake init --harness claude
```

<div aria-label="Command line screencast: Initializing Cupcake which creates the project structure specified below.">
    <div class="cast-player" data-cast="/assets/cupcake-init.cast" data-autoplay="true" data-loop="true" aria-hidden="true"></div>
</div>

This creates the `.cupcake/` directory with:

- `rulebook.yml` - Configuration file
- `policies/` - Rego policy files
- `signals/` - External data providers

## Core Commands

### `cupcake --help`

View all available commands and options:

```bash
cupcake --help
```

<div aria-label="Command line screencast: viewing the help menu">
    <div class="cast-player" data-cast="/assets/cupcake-help.cast" data-autoplay="true" data-loop="true" aria-hidden="true"></div>
</div>

### `cupcake inspect`

Inspect loaded policies and their routing metadata:

```bash
cupcake inspect
cupcake inspect --table  # Compact table view
```

<div aria-label="Command line screencast: Cupcake printing the loaded policies to the terminal">
    <div class="cast-player" data-cast="/assets/cupcake-inspect.cast" data-autoplay="true" data-loop="true" aria-hidden="true"></div>
</div>

This shows:

- Policy packages and their event/tool routing
- Enabled builtins
- Signal configurations

### `cupcake verify`

Verify your configuration and policies are valid:

```bash
cupcake verify --harness claude
```

<div aria-label="Command line screencast: Cupcake verifying all loaded policies">
    <div class="cast-player" data-cast="/assets/cupcake-verify.cast" data-autoplay="true" data-loop="true" aria-hidden="true"></div>
</div>

Use this to:

- Validate policy syntax
- Check rulebook configuration
- Ensure OPA compilation succeeds

## Supported Harnesses

Cupcake integrates with multiple AI coding agents via the `--harness` flag:

| Harness    | Description                   |
| ---------- | ----------------------------- |
| `claude`   | Claude Code (claude.ai/code)  |
| `cursor`   | Cursor (cursor.com)           |
| `factory`  | Factory AI Droid (factory.ai) |
| `opencode` | OpenCode (opencode.ai)        |

## Next Steps

- [Writing Policies](../reference/policies/custom.md) - Create custom Rego policies
- [Builtin Policies](../reference/policies/builtins.md) - Configure built-in protections
