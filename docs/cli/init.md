# Cupcake Init Command

## Overview

The `cupcake init` command creates a new Cupcake project structure in the current directory. It sets up the minimal required files and directories following the `.cupcake/` convention.

## Usage

```bash
cupcake init
```

## What It Creates

Running `cupcake init` creates the following structure:

```
.cupcake/
├── policies/
│   ├── system/
│   │   └── evaluate.rego    # Required aggregation policy
│   └── example.rego         # Placeholder policy (never fires)
├── signals/                 # Empty directory for signal scripts
└── actions/                 # Empty directory for action scripts
```

### Files Created

#### `system/evaluate.rego`
The mandatory system aggregation policy that collects decision verbs from all policies. This file is required for the Cupcake engine to function and should not be modified unless you understand the aggregation system.

#### `example.rego`
A minimal placeholder policy that:
- Prevents OPA compilation issues
- Provides a template structure for writing real policies
- Contains a rule that never fires (checks for an impossible command)
- Can be safely deleted once you add your own policies

## Behavior

### First Run
When run in a directory without `.cupcake/`:
```bash
$ cupcake init
✅ Initialized Cupcake project in .cupcake/
   Add your policies to .cupcake/policies/
   Add signal scripts to .cupcake/signals/
   Add action scripts to .cupcake/actions/
```

### Subsequent Runs
If `.cupcake/` already exists, the command exits cleanly without modifying anything:
```bash
$ cupcake init
Cupcake project already initialized (.cupcake/ exists)
```

## Next Steps

After initialization:

1. **Add Your Policies**: Create new `.rego` files in `.cupcake/policies/` with your security rules
2. **Add Signals** (optional): Place executable scripts in `.cupcake/signals/` to gather runtime information
3. **Add Actions** (optional): Place executable scripts in `.cupcake/actions/` to respond to policy decisions
4. **Test Your Setup**: Use `cupcake verify --policy-dir .` to verify your configuration

## Example Policy

Replace the example policy with something like:

```rego
package cupcake.policies.file_safety

import rego.v1

# METADATA
# scope: rule
# title: File Safety Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]

deny contains decision if {
    contains(input.tool_input.file_path, "/etc/")
    decision := {
        "reason": "System files cannot be modified",
        "severity": "HIGH",
        "rule_id": "FILE-001"
    }
}
```

## Integration with Claude Code

To use your initialized Cupcake project with Claude Code:

1. Initialize a Cupcake project in your workspace
2. Create a `.claude/settings.json` file:
```json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "*",
      "hooks": [{
        "type": "command",
        "command": "cupcake eval --policy-dir /path/to/your/project"
      }]
    }]
  }
}
```
3. Claude Code will now evaluate all tool uses against your policies

## Design Principles

- **Zero Configuration**: Works immediately after init with sensible defaults
- **No Real Impact**: The example policy never fires, ensuring no unexpected blocks
- **Convention Over Configuration**: Uses `.cupcake/` directory structure
- **Simple and Focused**: Does one thing - creates the minimal required structure