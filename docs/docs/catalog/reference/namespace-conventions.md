---
title: "Namespace Conventions"
description: "Rego namespace conventions for catalog rulebooks"
---

# Namespace Conventions

Catalog rulebooks must follow specific namespace conventions to ensure isolation and prevent conflicts with project policies.

## Required Namespace Patterns

All files in a catalog rulebook must use one of these namespace patterns:

| Directory | Pattern | Example |
|-----------|---------|---------|
| `policies/<harness>/` | `cupcake.catalog.<name>.policies.<policy>` | `cupcake.catalog.security_hardened.policies.dangerous_commands` |
| `helpers/` | `cupcake.catalog.<name>.helpers.<helper>` | `cupcake.catalog.security_hardened.helpers.commands` |
| `system/` | `cupcake.catalog.<name>.system` | `cupcake.catalog.security_hardened.system` |

### Components

| Component | Description | Example |
|-----------|-------------|---------|
| `cupcake.catalog` | Fixed prefix for all catalog files | - |
| `<name>` | Rulebook name (hyphens → underscores) | `security_hardened` |
| `policies` / `helpers` / `system` | Directory type | - |
| `<policy>` or `<helper>` | File identifier | `dangerous_commands` |

## Name Conversion

Rulebook names are converted to Rego-safe identifiers:

| manifest.yaml name | Rego namespace |
|--------------------|----------------|
| `security-hardened` | `security_hardened` |
| `python-best-practices` | `python_best_practices` |
| `my-company-rules` | `my_company_rules` |

The conversion:

- Replaces `-` with `_`
- Keeps `_` as-is
- Lowercase only

## Policy Namespace

Policies in `policies/<harness>/` use:

```
cupcake.catalog.<rulebook_name>.policies.<policy_name>
```

Example:

```rego
# policies/claude/dangerous_commands.rego
package cupcake.catalog.security_hardened.policies.dangerous_commands

import rego.v1

deny contains decision if {
    # policy logic
}
```

!!! note "Same package across harnesses"
    Each harness has its own file with the **same package name**. The policies are compiled separately per-harness, so there's no conflict.

## Helper Namespace

Shared helpers in `helpers/` use:

```
cupcake.catalog.<rulebook_name>.helpers.<helper_name>
```

Example:

```rego
# helpers/commands.rego
package cupcake.catalog.security_hardened.helpers.commands

import rego.v1

has_verb(command, verb) if {
    pattern := concat("", ["(^|\\s)", verb, "(\\s|$)"])
    regex.match(pattern, command)
}
```

## System Namespace

The aggregation entrypoint in `system/` uses exactly:

```
cupcake.catalog.<rulebook_name>.system
```

Example:

```rego
# system/evaluate.rego
package cupcake.catalog.security_hardened.system

import rego.v1

# METADATA
# scope: package
# custom:
#   entrypoint: true

evaluate := {
    "halts": collect_verbs("halt"),
    "denials": collect_verbs("deny"),
    "blocks": collect_verbs("block"),
    "asks": collect_verbs("ask"),
    "allow_overrides": collect_verbs("allow_override"),
    "add_context": collect_verbs("add_context"),
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.catalog.security_hardened.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}

default collect_verbs(_) := []
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
package policies.dangerous_commands

# With namespaces - No conflict
package cupcake.catalog.security_hardened.policies.dangerous_commands
package cupcake.catalog.compliance_rules.policies.dangerous_commands
```

### Discovery

The namespace pattern allows Cupcake to:

1. Identify catalog policies automatically
2. Route evaluations to the correct policies
3. Collect decisions from all policies via `walk()`

## Validation

The `cupcake catalog lint` command validates namespaces:

```bash
cupcake catalog lint ./my-rulebook
```

Error examples:

```
ERROR: Policy at policies/claude/example.rego has invalid namespace
'policies.example'. Expected prefix 'cupcake.catalog.my_rulebook.policies'

ERROR: System file at system/evaluate.rego has invalid namespace
'cupcake.catalog.wrong_name.system'. Expected exactly 'cupcake.catalog.my_rulebook.system'
```

## Importing Between Files

Within a rulebook, use fully-qualified imports:

```rego
package cupcake.catalog.security_hardened.policies.dangerous_flags

# Import helpers
import data.cupcake.catalog.security_hardened.helpers.commands

import rego.v1

deny contains decision if {
    cmd := lower(input.tool_input.command)
    commands.has_verb(cmd, "git")
    commands.has_any_flag(cmd, {"--no-verify"})
    
    decision := {
        "rule_id": "SEC-002",
        "reason": "Blocked --no-verify flag",
        "severity": "HIGH",
    }
}
```
