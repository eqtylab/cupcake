---
title: "Manifest Reference"
description: "Complete reference for rulebook manifest.yaml"
---

# Manifest Reference

Every rulebook requires a `manifest.yaml` file that describes its metadata and configuration.

## Example Manifest

```yaml
apiVersion: cupcake.dev/v1
kind: RulebookManifest
metadata:
  name: security-hardened
  version: 1.0.0
  description: |
    Comprehensive security policies that block dangerous commands,
    risky flags, and destructive Git operations.
  harnesses:
    - claude
    - cursor
    - opencode
    - factory
  keywords:
    - security
    - hardening
    - dangerous-commands
    - git-safety
  author: EQTY Lab
  license: Apache-2.0
  homepage: https://github.com/eqtylab/cupcake-catalog
```

## Required Fields

### `apiVersion`

API version for the manifest format. Currently only `cupcake.dev/v1` is supported.

```yaml
apiVersion: cupcake.dev/v1
```

### `kind`

Must be `RulebookManifest`.

```yaml
kind: RulebookManifest
```

### `metadata.name`

Unique identifier for the rulebook. Must:

- Be lowercase
- Use only letters, numbers, and hyphens
- Start with a letter
- Be 3-50 characters long

```yaml
metadata:
  name: my-rulebook
```

!!! note "Rego Namespace"
The name is converted to a Rego-safe format by replacing hyphens with underscores. `security-hardened` becomes `security_hardened` in namespaces.

### `metadata.version`

Semantic version (semver) of the rulebook.

```yaml
metadata:
  version: 1.2.3
```

### `metadata.description`

Brief description of what the rulebook does. Can be multi-line using YAML block syntax.

```yaml
metadata:
  description: |
    Block dangerous shell commands like rm -rf /,
    prevent use of --force and --no-verify flags,
    and require confirmation for destructive Git operations.
```

### `metadata.harnesses`

List of supported harnesses. Valid values:

- `claude` - Claude Code
- `cursor` - Cursor
- `opencode` - OpenCode
- `factory` - Factory AI

```yaml
metadata:
  harnesses:
    - claude
    - cursor
```

You must provide policies for each harness you list.

## Optional Fields

### `metadata.keywords`

Searchable keywords to help users find your rulebook.

```yaml
metadata:
  keywords:
    - security
    - git
    - dangerous-commands
```

### `metadata.author`

Author name or organization.

```yaml
metadata:
  author: EQTY Lab
```

### `metadata.license`

SPDX license identifier.

```yaml
metadata:
  license: MIT
```

### `metadata.homepage`

URL for the rulebook's homepage or repository.

```yaml
metadata:
  homepage: https://github.com/myorg/my-rulebook
```

## Validation

Validate your manifest with:

```bash
cupcake catalog lint ./my-rulebook
```

This checks:

- Required fields are present
- Name format is valid
- Version is valid semver
- `system/evaluate.rego` exists at rulebook root
- Listed harnesses have corresponding policy directories with `.rego` files
- Namespaces follow the required patterns:
    - Policies: `cupcake.catalog.<name>.policies.*`
    - Helpers: `cupcake.catalog.<name>.helpers.*`
    - System: `cupcake.catalog.<name>.system`
