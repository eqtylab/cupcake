---
title: "CLI Reference"
description: "Complete reference for cupcake catalog commands"
---

# CLI Reference

Complete reference for all `cupcake catalog` commands.

## catalog search

Search for rulebooks in the catalog.

```bash
cupcake catalog search [QUERY] [OPTIONS]
```

### Arguments

| Argument | Description                                        |
| -------- | -------------------------------------------------- |
| `QUERY`  | Search term (searches name, description, keywords) |

### Options

| Option                | Description                                                |
| --------------------- | ---------------------------------------------------------- |
| `--harness <HARNESS>` | Filter by harness type (claude, cursor, opencode, factory) |
| `--json`              | Output results as JSON                                     |
| `--refresh`           | Bypass index cache and fetch fresh data                    |

### Examples

```bash
# Search all rulebooks
cupcake catalog search

# Search for security-related rulebooks
cupcake catalog search security

# Filter by harness
cupcake catalog search --harness claude

# JSON output for scripting
cupcake catalog search --json | jq '.[].name'
```

---

## catalog show

Show detailed information about a rulebook.

```bash
cupcake catalog show <NAME> [OPTIONS]
```

### Arguments

| Argument | Description   |
| -------- | ------------- |
| `NAME`   | Rulebook name |

### Options

| Option      | Description        |
| ----------- | ------------------ |
| `--json`    | Output as JSON     |
| `--refresh` | Bypass index cache |

### Examples

```bash
# Show rulebook details
cupcake catalog show security-hardened

# JSON output
cupcake catalog show security-hardened --json
```

---

## catalog install

Install a rulebook from the catalog.

```bash
cupcake catalog install <NAME[@VERSION]> [OPTIONS]
```

### Arguments

| Argument         | Description                                      |
| ---------------- | ------------------------------------------------ |
| `NAME[@VERSION]` | Rulebook name, optionally with version specifier |

### Options

| Option                | Description                 |
| --------------------- | --------------------------- |
| `--version <VERSION>` | Specific version to install |
| `--refresh`           | Bypass index cache          |

### Version Specifiers

| Format  | Description                  | Example                   |
| ------- | ---------------------------- | ------------------------- |
| `1.2.3` | Exact version                | `security-hardened@1.2.3` |
| `^1.2`  | Compatible (>=1.2.0 <2.0.0)  | `security-hardened@^1.2`  |
| `~1.2`  | Patch-level (>=1.2.0 <1.3.0) | `security-hardened@~1.2`  |
| (none)  | Latest version               | `security-hardened`       |

### Examples

```bash
# Install latest version
cupcake catalog install security-hardened

# Install specific version
cupcake catalog install security-hardened@1.0.0

# Install compatible version
cupcake catalog install security-hardened@^1.0

# Install with version flag
cupcake catalog install security-hardened --version 1.0.0
```

---

## catalog list

List installed catalog rulebooks.

```bash
cupcake catalog list
```

### Output

Shows a table with:

- Name
- Version
- Supported harnesses
- Installation date

---

## catalog upgrade

Upgrade installed rulebooks to newer versions.

```bash
cupcake catalog upgrade [NAME] [OPTIONS]
```

### Arguments

| Argument | Description                          |
| -------- | ------------------------------------ |
| `NAME`   | Rulebook to upgrade (all if omitted) |

### Options

| Option      | Description                                        |
| ----------- | -------------------------------------------------- |
| `--dry-run` | Show what would be upgraded without making changes |
| `--refresh` | Bypass index cache                                 |

### Examples

```bash
# Preview available upgrades
cupcake catalog upgrade --dry-run

# Upgrade all installed rulebooks
cupcake catalog upgrade

# Upgrade specific rulebook
cupcake catalog upgrade security-hardened
```

---

## catalog uninstall

Remove an installed rulebook.

```bash
cupcake catalog uninstall <NAME>
```

### Arguments

| Argument | Description           |
| -------- | --------------------- |
| `NAME`   | Rulebook to uninstall |

### Examples

```bash
cupcake catalog uninstall security-hardened
```

---

## catalog lint

Validate a local rulebook before publishing.

```bash
cupcake catalog lint <PATH>
```

### Arguments

| Argument | Description                |
| -------- | -------------------------- |
| `PATH`   | Path to rulebook directory |

### Validation Checks

- `manifest.yaml` exists and is valid
- Required metadata fields present
- Name format is valid
- `system/evaluate.rego` exists at rulebook root
- Harness directories exist in `policies/` with `.rego` files
- Policy namespaces follow `cupcake.catalog.<name>.policies.*` pattern
- Helper namespaces follow `cupcake.catalog.<name>.helpers.*` pattern
- System namespace is exactly `cupcake.catalog.<name>.system`
- README.md exists (warning if missing)
- CHANGELOG.md exists (warning if missing)

### Examples

```bash
# Validate a rulebook
cupcake catalog lint ./my-rulebook
```

### Exit Codes

| Code | Description       |
| ---- | ----------------- |
| `0`  | Validation passed |
| `1`  | Validation failed |

---

## catalog package

Package a rulebook for distribution.

```bash
cupcake catalog package <PATH> [OPTIONS]
```

### Arguments

| Argument | Description                |
| -------- | -------------------------- |
| `PATH`   | Path to rulebook directory |

### Options

| Option               | Description                                   |
| -------------------- | --------------------------------------------- |
| `--output, -o <DIR>` | Output directory (default: current directory) |

### Output

Creates a tarball named `<name>-<version>.tar.gz` and displays:

- File path
- Size in bytes
- SHA256 digest

### Examples

```bash
# Package to current directory
cupcake catalog package ./my-rulebook

# Package to specific directory
cupcake catalog package ./my-rulebook --output ./dist
```

---

## catalog repo

Manage catalog repositories.

### repo list

List configured repositories.

```bash
cupcake catalog repo list
```

### repo add

Add a custom repository.

```bash
cupcake catalog repo add <NAME> <URL>
```

| Argument | Description       |
| -------- | ----------------- |
| `NAME`   | Repository name   |
| `URL`    | URL to index.yaml |

### repo remove

Remove a repository.

```bash
cupcake catalog repo remove <NAME>
```

!!! note
The default "official" registry cannot be removed.

### Examples

```bash
# List repositories
cupcake catalog repo list

# Add custom repository
cupcake catalog repo add mycompany https://registry.mycompany.com/index.yaml

# Remove repository
cupcake catalog repo remove mycompany
```
