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

<div class="cast-player" data-cast="/assets/cupcake-init.cast" data-autoplay="true" data-loop="true"></div>

This creates the `.cupcake/` directory with:

- `rulebook.yml` - Configuration file
- `policies/` - Rego policy files
- `signals/` - External data providers
- `actions/` - Automated response scripts

## Core Commands

### `cupcake --help`

View all available commands and options:

```bash
cupcake --help
```

<div class="cast-player" data-cast="/assets/cupcake-help.cast" data-autoplay="true" data-loop="true"></div>

### `cupcake inspect`

Inspect loaded policies and their routing metadata:

```bash
cupcake inspect
cupcake inspect --table  # Compact table view
```

<div class="cast-player" data-cast="/assets/cupcake-inspect.cast" data-autoplay="true" data-loop="true"></div>

This shows:

- Policy packages and their event/tool routing
- Enabled builtins
- Signal configurations

### `cupcake verify`

Verify your configuration and policies are valid:

```bash
cupcake verify --harness claude
```

<div class="cast-player" data-cast="/assets/cupcake-verify.cast" data-autoplay="true" data-loop="true"></div>

Use this to:

- Validate policy syntax
- Check rulebook configuration
- Ensure OPA compilation succeeds

### `cupcake trust`

Manage script trust and integrity verification:

```bash
cupcake trust init      # Initialize trust manifest
cupcake trust list      # List trusted scripts
cupcake trust verify    # Verify against manifest
```

<div class="cast-player" data-cast="/assets/cupcake-trust.cast" data-autoplay="true" data-loop="true"></div>

The trust system ensures:

- Signal scripts haven't been tampered with
- Action scripts are verified before execution
- Policy files maintain integrity

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
