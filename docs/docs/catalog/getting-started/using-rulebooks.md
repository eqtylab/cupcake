---
title: "Using Rulebooks"
description: "How installed rulebooks integrate with Cupcake"
---

# Using Rulebooks

This guide explains how installed catalog rulebooks integrate with your Cupcake setup and how to manage them effectively.

## How Rulebooks Work

When you install a rulebook, Cupcake:

1. Downloads the rulebook tarball from the registry
2. Verifies the SHA256 digest for integrity
3. Extracts policies to `.cupcake/catalog/<rulebook>/`
4. Records the installation in `.cupcake/catalog.lock`

## Evaluation Order

Cupcake evaluates policies in a specific order. Catalog rulebooks are evaluated between global and project policies:

```
1. Global policies    (~/.config/cupcake/policies/)
2. Catalog rulebooks  (.cupcake/catalog/*/policies/)  <-- NEW
3. Project policies   (.cupcake/policies/)
```

This means:

- Global policies apply first (organization-wide rules)
- Catalog rulebooks add pre-built policies
- Project policies can override or extend both

## Policy Namespaces

Catalog policies use isolated namespaces to prevent conflicts:

```
cupcake.catalog.<rulebook>.<harness>.*
```

For example, the `security-hardened` rulebook uses:

```
cupcake.catalog.security_hardened.policies.claude.builtins.dangerous_commands
cupcake.catalog.security_hardened.policies.cursor.system.evaluate
```

## Listing Installed Rulebooks

View all installed rulebooks:

```bash
cupcake catalog list
```

Output:

```
Installed catalog rulebooks:

╭──────────────────┬─────────┬───────────────────────────┬────────────╮
│       Name       │ Version │         Harnesses         │  Installed │
├──────────────────┼─────────┼───────────────────────────┼────────────┤
│ security-hardened│ v1.0.0  │ claude, cursor, opencode  │ 2025-12-04 │
╰──────────────────┴─────────┴───────────────────────────┴────────────╯
```

## Upgrading Rulebooks

Check for available upgrades:

```bash
cupcake catalog upgrade --dry-run
```

Upgrade all installed rulebooks:

```bash
cupcake catalog upgrade
```

Upgrade a specific rulebook:

```bash
cupcake catalog upgrade security-hardened
```

## Uninstalling Rulebooks

Remove an installed rulebook:

```bash
cupcake catalog uninstall security-hardened
```

This removes the policies and updates the lock file.

## Lock File

The `.cupcake/catalog.lock` file tracks installed rulebooks:

```yaml
apiVersion: cupcake.dev/v1
kind: CatalogLock
installed:
  - name: security-hardened
    version: "1.0.0"
    digest: "sha256:abc123..."
    repository: official
    harnesses:
      - claude
      - cursor
      - opencode
      - factory
    installed_at: "2025-12-04T12:00:00Z"
```

!!! tip "Version Control"
Commit `.cupcake/catalog.lock` to version control so team members can install the same rulebook versions with `cupcake catalog install`.

## Harness Filtering

Catalog rulebooks include policies for multiple harnesses. Only policies matching your current harness are evaluated. For example, if you're using Claude Code, only the `policies/claude/` policies from installed rulebooks are loaded.

## Troubleshooting

### Policies Not Loading

1. Verify the rulebook is installed: `cupcake catalog list`
2. Check that policies exist for your harness: `ls .cupcake/catalog/<rulebook>/policies/`
3. Ensure the rulebook has a `system/evaluate.rego` at the rulebook root level

### Namespace Conflicts

If you see unexpected policy behavior, check for namespace conflicts:

```bash
cupcake inspect --debug-routing
```

Catalog policies should all be under `cupcake.catalog.*` namespaces.

### Cache Issues

If you're not seeing the latest versions, refresh the cache:

```bash
cupcake catalog search --refresh
```
