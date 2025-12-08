---
title: "Namespace Conventions"
description: "Rego namespace conventions for catalog rulebooks"
---

# Namespace Conventions

Catalog rulebooks must follow specific namespace conventions to ensure isolation and prevent conflicts with project policies.

## Required Namespace Pattern

All policies in a catalog rulebook must use this namespace pattern:

```
cupcake.catalog.<rulebook_name>.policies.<harness>.<type>.<policy_name>
```

### Components

| Component         | Description                                | Example              |
| ----------------- | ------------------------------------------ | -------------------- |
| `cupcake.catalog` | Fixed prefix for all catalog policies      | -                    |
| `<rulebook_name>` | Name from manifest (hyphens → underscores) | `security_hardened`  |
| `policies`        | Fixed separator                            | -                    |
| `<harness>`       | Harness name                               | `claude`, `cursor`   |
| `<type>`          | Policy type                                | `builtins`, `system` |
| `<policy_name>`   | Policy identifier                          | `dangerous_commands` |

### Examples

For a rulebook named `security-hardened`:

```rego
# Good - follows convention
package cupcake.catalog.security_hardened.policies.claude.builtins.dangerous_commands
package cupcake.catalog.security_hardened.policies.cursor.system.evaluate
package cupcake.catalog.security_hardened.policies.opencode.builtins.risky_flags

# Bad - missing prefix
package policies.claude.builtins.dangerous_commands

# Bad - wrong prefix
package cupcake.security_hardened.policies.claude.builtins.dangerous_commands

# Bad - doesn't match rulebook name
package cupcake.catalog.other_name.policies.claude.builtins.dangerous_commands
```

## Name Conversion

Rulebook names are converted to Rego-safe identifiers:

| manifest.yaml name      | Rego namespace          |
| ----------------------- | ----------------------- |
| `security-hardened`     | `security_hardened`     |
| `python-best-practices` | `python_best_practices` |
| `my-company-rules`      | `my_company_rules`      |

The conversion:

- Replaces `-` with `_`
- Keeps `_` as-is
- Lowercase only

## Helper Namespace

Shared helpers should use:

```
cupcake.catalog.<rulebook_name>.helpers.<helper_name>
```

Example:

```rego
package cupcake.catalog.security_hardened.helpers.commands

# Helper functions here
get_command_name(input) = name {
    # ...
}
```

## System Policies

The aggregation policy for each harness must be at:

```
cupcake.catalog.<rulebook_name>.policies.<harness>.system
```

Example:

```rego
# policies/claude/system/evaluate.rego
package cupcake.catalog.security_hardened.policies.claude.system

import data.cupcake.catalog.security_hardened.policies.claude.builtins

# Aggregate decisions from all builtin policies
decisions[decision] {
    decision := builtins.dangerous_commands.deny
}
```

## Why Namespaces Matter

### Isolation

Catalog policies are isolated from:

- **Project policies** (`cupcake.policies.*`)
- **Global policies** (`cupcake.global.*`)
- **Other catalog rulebooks** (`cupcake.catalog.other_rulebook.*`)

### Conflict Prevention

Without namespaces, two rulebooks with a `dangerous_commands` policy would conflict:

```rego
# Without namespaces - CONFLICT!
package policies.claude.builtins.dangerous_commands

# With namespaces - No conflict
package cupcake.catalog.security_hardened.policies.claude.builtins.dangerous_commands
package cupcake.catalog.compliance_rules.policies.claude.builtins.dangerous_commands
```

### Discovery

The namespace pattern allows Cupcake to:

1. Identify catalog policies automatically
2. Route evaluations to the correct policies
3. Generate proper imports in aggregation policies

## Validation

The `cupcake catalog lint` command validates namespaces:

```bash
cupcake catalog lint ./my-rulebook
```

Error examples:

```
ERROR: Policy at policies/claude/builtins/example.rego has invalid namespace
'policies.claude.builtins.example'. Expected prefix 'cupcake.catalog.my_rulebook'
```

## Importing Between Files

Within a rulebook, use fully-qualified imports:

```rego
package cupcake.catalog.security_hardened.policies.claude.system

# Import builtins
import data.cupcake.catalog.security_hardened.policies.claude.builtins

# Import helpers
import data.cupcake.catalog.security_hardened.helpers.commands

# Use imported packages
decisions[decision] {
    cmd := commands.get_command_name(input)
    decision := builtins.dangerous_commands.deny
}
```
