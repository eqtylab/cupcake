---
title: "Usage"
description: "Get up and running with Cupcake"
---

# Usage

After [installation](../installation/), you're ready to set up Cupcake for your project. The first step is choosing which AI coding agent (harness) you're using.

## Select Your Harness

Cupcake supports multiple AI coding agents. Select your harness to get started:

| Harness | Status | Guide |
|---------|--------|-------|
| **Claude Code** | Fully Supported | [Setup Guide](claude-code/) |
| **Cursor** | Fully Supported | [Setup Guide](cursor/) |
| **OpenCode** | Fully Supported | [Setup Guide](opencode/) |
| **Factory AI** | Coming Soon | [Setup Guide](factory-ai/) |

## Next Steps

After setting up your harness, you can:

### Option 1: Use Built-in Policies

Cupcake includes battle-tested security policies ready to use. Edit your `.cupcake/rulebook.yml` to enable and configure them:

```yaml
builtins:
  git_pre_check:
    enabled: true
    checks:
      - command: "npm test"
        message: "Tests must pass before commit"

  protected_paths:
    enabled: true
    paths:
      - "/etc/"
      - "~/.ssh/"
```

See the **[Built-in Configuration Reference](../../reference/builtin-config/)** for all available builtins and their options.

### Option 2: Write Custom Policies

Create your own policies in `.cupcake/policies/<harness>/` using OPA Rego:

```rego
package cupcake.policies.example

import rego.v1

# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    input.tool_input.command contains "rm -rf"

    decision := {
        "rule_id": "SAFETY-001",
        "reason": "Dangerous command blocked",
        "severity": "HIGH"
    }
}
```
