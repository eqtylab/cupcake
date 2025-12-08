---
title: "Cupcake Catalog"
description: "Discover and share reusable Cupcake rulebooks"
---

# Cupcake Catalog

Cupcake Catalog is the official registry for discovering and sharing reusable [Cupcake](https://cupcake.eqtylab.io/) rulebooks. Install pre-built policy bundles with a single command, or publish your own rulebooks for the community.

## Quick Start

```bash
# Search for available rulebooks
cupcake catalog search

# Install a rulebook
cupcake catalog install security-hardened

# List installed rulebooks
cupcake catalog list
```

## What is a Rulebook?

A rulebook is a packaged collection of Cupcake policies that can be installed and used across projects. Rulebooks provide:

- **Pre-built policies** for common use cases (security, compliance, best practices)
- **Multi-harness support** with policies for Claude Code, Cursor, OpenCode, and Factory
- **Version management** with semver support and automatic upgrades
- **Namespace isolation** so catalog policies don't conflict with project policies

## Featured Rulebooks

| Rulebook            | Description                                                     | Harnesses |
| ------------------- | --------------------------------------------------------------- | --------- |
| `security-hardened` | Block dangerous commands, flags, and destructive Git operations | All       |

## How It Works

1. **Browse** the catalog to find rulebooks that match your needs
2. **Install** rulebooks with `cupcake catalog install <name>`
3. **Policies activate** automatically when you run Cupcake
4. **Upgrade** to newer versions with `cupcake catalog upgrade`

Installed rulebooks are stored in `.cupcake/catalog/` and evaluated as an overlay between your global and project policies.

## For Rulebook Authors

Want to create and share your own rulebook? Check out the [Authoring Guide](authoring/index.md) to learn how to:

- Structure your rulebook with the proper manifest and policies
- Follow namespace conventions for catalog integration
- Validate and package your rulebook
- Publish to the catalog

## Links

- [Cupcake Documentation](https://cupcake.eqtylab.io/)
- [GitHub Repository](https://github.com/eqtylab/cupcake-catalog)
- [Contributing Guide](https://github.com/eqtylab/cupcake-catalog/blob/main/CONTRIBUTING.md)
