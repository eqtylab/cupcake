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
├── helpers/                # Optional: shared Rego helpers
│   └── utils.rego
└── policies/               # Required: policies by harness
    ├── claude/
    │   ├── system/
    │   │   └── evaluate.rego    # Required: aggregation policy
    │   └── builtins/
    │       └── my_policy.rego
    ├── cursor/
    │   ├── system/
    │   │   └── evaluate.rego
    │   └── builtins/
    │       └── my_policy.rego
    └── opencode/
        ├── system/
        │   └── evaluate.rego
        └── builtins/
            └── my_policy.rego
```

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
# policies/claude/builtins/my_policy.rego
package cupcake.catalog.my_rulebook.policies.claude.builtins.my_policy

# METADATA
# title: My Policy
# description: Blocks something dangerous
# scope: rule
# custom:
#   decision: deny
#   severity: high
#   routing:
#     events: [pre_tool_use]
#     tools: [bash]

default deny := false

deny {
    input.tool_input.command == "something_dangerous"
}
```

### 3. Create the Aggregation Policy

Each harness needs a `system/evaluate.rego`:

```rego
# policies/claude/system/evaluate.rego
package cupcake.catalog.my_rulebook.policies.claude.system

import data.cupcake.catalog.my_rulebook.policies.claude.builtins

decisions[decision] {
    decision := builtins.my_policy.deny
}
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
