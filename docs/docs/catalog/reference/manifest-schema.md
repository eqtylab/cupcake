---
title: "Manifest Schema"
description: "Complete schema reference for manifest.yaml"
---

# Manifest Schema

This page documents the complete schema for rulebook `manifest.yaml` files.

## Schema Version

Current schema version: `cupcake.dev/v1`

## Full Schema

```yaml
# Required: API version
apiVersion: cupcake.dev/v1

# Required: Kind identifier
kind: RulebookManifest

# Required: Rulebook metadata
metadata:
  # Required: Unique rulebook name
  # Format: lowercase, letters/numbers/hyphens, 3-50 chars
  name: string

  # Required: Semantic version
  # Format: MAJOR.MINOR.PATCH (e.g., 1.2.3)
  version: string

  # Required: Brief description
  # Can be multi-line using YAML block syntax
  description: string

  # Required: List of supported harnesses
  # Values: claude, cursor, opencode, factory
  harnesses:
    - string

  # Optional: Searchable keywords
  keywords:
    - string

  # Optional: Author name or organization
  author: string

  # Optional: SPDX license identifier
  license: string

  # Optional: Homepage or repository URL
  homepage: string
```

## Field Constraints

### name

| Constraint | Value                    |
| ---------- | ------------------------ |
| Required   | Yes                      |
| Format     | `^[a-z][a-z0-9-]{2,49}$` |
| Min length | 3                        |
| Max length | 50                       |

Valid examples:

- `security-hardened`
- `python-best-practices`
- `my-company-rules`

Invalid examples:

- `Security-Hardened` (uppercase)
- `123-rulebook` (starts with number)
- `my_rulebook` (underscores)
- `ab` (too short)

### version

| Constraint | Value            |
| ---------- | ---------------- |
| Required   | Yes              |
| Format     | Semantic version |

Must be a valid [semver](https://semver.org/) string:

- `1.0.0` - Valid
- `0.1.0` - Valid
- `2.0.0-beta.1` - Valid (pre-release)
- `1.0` - Invalid (missing patch)
- `v1.0.0` - Invalid (no `v` prefix)

### harnesses

| Constraint   | Value                                     |
| ------------ | ----------------------------------------- |
| Required     | Yes                                       |
| Min items    | 1                                         |
| Valid values | `claude`, `cursor`, `opencode`, `factory` |

Each listed harness must have a corresponding `policies/<harness>/` directory.

### keywords

| Constraint             | Value |
| ---------------------- | ----- |
| Required               | No    |
| Max items              | 20    |
| Max length per keyword | 50    |

Keywords are used by `cupcake catalog search`.

## Example: Minimal Manifest

```yaml
apiVersion: cupcake.dev/v1
kind: RulebookManifest
metadata:
  name: my-rulebook
  version: 0.1.0
  description: A simple rulebook
  harnesses:
    - claude
```

## Example: Full Manifest

```yaml
apiVersion: cupcake.dev/v1
kind: RulebookManifest
metadata:
  name: security-hardened
  version: 1.0.0
  description: |
    Comprehensive security policies for AI coding agents.

    Features:
    - Block dangerous shell commands (rm -rf /, mkfs, dd)
    - Prevent risky flags (--force, --no-verify)
    - Require confirmation for destructive Git operations
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
    - best-practices
  author: EQTY Lab
  license: Apache-2.0
  homepage: https://github.com/eqtylab/cupcake-catalog
```

## Validation

Validate your manifest with:

```bash
cupcake catalog lint ./my-rulebook
```

Common validation errors:

| Error                        | Cause                          | Fix                                    |
| ---------------------------- | ------------------------------ | -------------------------------------- |
| `Invalid name format`        | Name doesn't match pattern     | Use lowercase, letters/numbers/hyphens |
| `Invalid harness`            | Unknown harness value          | Use: claude, cursor, opencode, factory |
| `Missing policies directory` | Listed harness has no policies | Create `policies/<harness>/` directory |
| `Invalid version`            | Version isn't valid semver     | Use format like `1.0.0`                |
