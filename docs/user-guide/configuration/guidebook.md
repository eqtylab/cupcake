# Guidebook Design Specification

**Status:** APPROVED DESIGN  
**Date:** August 14, 2025  
**Context:** Final design for project-level guidebook integration

## Overview

The guidebook provides **project-level configuration** using **convention over configuration** principles. It's a single optional file that enhances the existing metadata-driven architecture without replacing it.

## Directory Structure

```
my-project/
├── .cupcake/
│   ├── guidebook.yml      # ← Optional project config (only for customization)
│   ├── policies/          # ← Rego policy files (convention-based)
│   ├── signals/           # ← Signal scripts (convention-based)
│   └── actions/           # ← Action scripts (convention-based)
├── src/                   # ← User's actual project
├── README.md
└── .gitignore
```

## Core Principles

1. **Convention Over Configuration**: Cupcake works with zero configuration out of the box
2. **Single Project-Level Guidebook**: One optional `guidebook.yml` per project, not per policy
3. **Clean Project Root**: All Cupcake files contained in `.cupcake/` directory
4. **Minimal Configuration**: Guidebook only declares non-default behavior

## Zero-Config Experience

### Project Initialization
```bash
cupcake init
```

Creates:
```
.cupcake/
├── policies/
│   └── example.rego
├── signals/          # ← Empty, ready for scripts
└── actions/          # ← Empty, ready for scripts
# No guidebook.yml created unless customization needed
```

### Engine Behavior with Defaults
1. Look for `.cupcake/` directory
2. Check for `.cupcake/guidebook.yml` → load if present
3. Use conventions within `.cupcake/`:
   - Policies: `.cupcake/policies/`
   - Signals: `.cupcake/signals/`
   - Actions: `.cupcake/actions/`

## Guidebook Configuration (When Needed)

**Minimal guidebook example:**
```yaml
# .cupcake/guidebook.yml - Only declare what's non-default
signals:
  git_branch: "git branch --show-current"
  test_status: "./scripts/check_tests.sh"
  deployment_status: "kubectl get deployments"

timeouts:
  signal_default: 10  # Override 5s default
```

**NOT this verbose approach:**
```yaml
# BAD - Don't make users declare obvious defaults
directories:
  signals: ".cupcake/signals"     # ← Convention, don't declare
  actions: ".cupcake/actions"     # ← Convention, don't declare
  policies: ".cupcake/policies"   # ← Convention, don't declare
```

## Integration with Current Architecture

### Policies Remain Unchanged
```rego
package cupcake.policies.bash_guard

import rego.v1

# METADATA
# custom:
#   routing:
#     required_signals: ["git_branch", "test_status"]

deny contains decision if {
    input.signals.git_branch == "main"
    contains(input.tool_input.command, "git push")
    
    decision := {
        "reason": "Cannot push directly to main branch",
        "rule_id": "GIT-001"
    }
}
```

### Enhanced Evaluation Flow
1. **Load guidebook** → get signal definitions and project config
2. **Route to policies** → metadata-driven O(1) lookup (unchanged)
3. **Collect required signals** → use guidebook registry to find/execute scripts
4. **Evaluate policies** → pass enriched input to WASM (unchanged)
5. **Execute actions** → use guidebook registry for operational side effects
6. **Synthesize response** → existing synthesis layer (unchanged)

## Performance Characteristics

- **Guidebook loading**: ~1ms (negligible)
- **Signal execution**: Concurrent, timeout-protected
- **Policy evaluation**: Unchanged (~120µs)
- **Total impact**: Minimal, dominated by signal execution time

## Benefits

1. **Clean Integration**: Enhances existing architecture without breaking changes
2. **Zero Config**: Works out of the box with smart defaults
3. **AI Friendly**: Structured YAML format, discoverable via filesystem
4. **Operational Complete**: Provides missing signal gathering and action execution
5. **Familiar Pattern**: `.cupcake/` follows established tooling conventions

## Implementation Priority

This design completes the missing operational pieces (signal gathering, actions) while maintaining the elegant metadata-driven routing architecture we've built.