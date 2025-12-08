---
title: "Authoring Rulebooks"
description: "Create and publish your own Cupcake rulebooks"
---

# Authoring Rulebooks

This guide covers how to create, validate, and publish your own Cupcake rulebooks to share with the community.

## Overview

A rulebook is a packaged collection of Cupcake policies with:

- **Manifest** (`manifest.yaml`) - Metadata about the rulebook
- **Policies** - Rego policies organized by harness
- **Documentation** - README and CHANGELOG

## Rulebook Structure

```
my-rulebook/
├── manifest.yaml           # Required: rulebook metadata
├── README.md               # Recommended: documentation
├── CHANGELOG.md            # Recommended: version history
├── system/
│   └── evaluate.rego       # Required: shared aggregation entrypoint
├── helpers/                # Optional: shared Rego helpers
│   └── utils.rego
└── policies/               # Required: policies by harness
    ├── claude/
    │   └── my_policy.rego  # Policy files directly in harness dir
    ├── cursor/
    │   └── my_policy.rego
    └── opencode/
        └── my_policy.rego
```

Key points:

- `system/evaluate.rego` is at the **rulebook root**, shared across all harnesses
- `helpers/` contains shared functions that can be imported by any policy
- Policy files go **directly** in `policies/<harness>/` (no subdirectories)

## Quick Start

### 1. Create the Manifest

```yaml
# manifest.yaml
apiVersion: cupcake.dev/v1
kind: RulebookManifest
metadata:
  name: my-rulebook
  version: 0.1.0
  description: Short description of what this rulebook does
  harnesses:
    - claude
    - cursor
    - opencode
  keywords:
    - security
    - best-practices
  author: Your Name
  license: MIT
```

### 2. Create Policies

Create a policy for each harness you support:

```rego
# policies/claude/my_policy.rego
package cupcake.catalog.my_rulebook.policies.my_policy

import rego.v1

# METADATA
# scope: package
# title: My Policy
# description: Blocks something dangerous
# custom:
#   severity: high
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    input.tool_input.command == "something_dangerous"
    
    decision := {
        "rule_id": "MY-001",
        "reason": "This command is blocked",
        "severity": "HIGH",
    }
}
```

### 3. Create the Aggregation Entrypoint

Create a single `system/evaluate.rego` at the rulebook root:

```rego
# system/evaluate.rego
package cupcake.catalog.my_rulebook.system

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
        walk(data.cupcake.catalog.my_rulebook.policies, [path, value])
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

### 4. Validate

```bash
cupcake catalog lint ./my-rulebook
```

### 5. Package

```bash
cupcake catalog package ./my-rulebook --output ./dist
```

## Next Steps

- [Manifest Reference](manifest.md) - All manifest fields explained
- [Writing Policies](policies.md) - Policy patterns and best practices
- [Publishing](publishing.md) - How to submit to the catalog
