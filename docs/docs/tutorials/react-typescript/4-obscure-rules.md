---
title: "4. Obscure Rules"
description: "Using signals to enforce project-wide restrictions based on README content"
---

## Overview

Sometimes you need policies based on project state. We'll create a signal that checks if README.md contains "CODE FREEZE" and blocks all file modifications until it's removed.

## Step 1: Create the Signal Script

Create the signal script:

```bash
mkdir -p .cupcake/signals
touch .cupcake/signals/check-code-freeze.sh
```

Edit `.cupcake/signals/check-code-freeze.sh`:

```bash
#!/bin/bash
# Check if README.md contains CODE FREEZE marker

if [ ! -f "README.md" ]; then
    echo "No README.md found"
    exit 0
fi

if grep -q "CODE FREEZE" README.md; then
    echo "CODE FREEZE is active in README.md"
    exit 1
fi

echo "No code freeze detected"
exit 0
```

Make it executable:

```bash
chmod +x .cupcake/signals/check-code-freeze.sh
```

## Step 2: Write the Policy

Create the policy file:

```bash
touch .cupcake/policies/claude/code_freeze.rego
```

Edit `.cupcake/policies/claude/code_freeze.rego`:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]
#   signals:
#     - check-code-freeze
package cupcake.policies.code_freeze

import rego.v1

# Block all writes when code freeze is active
deny contains decision if {
    input.tool_name == "Write"

    freeze_check := input.signals.check_code_freeze

    is_object(freeze_check)
    freeze_check.exit_code != 0

    decision := {
        "rule_id": "FREEZE-001",
        "reason": concat("", [
            "CODE FREEZE is active. ",
            freeze_check.output,
            ". Remove 'CODE FREEZE' from README.md to resume development."
        ]),
        "severity": "HIGH"
    }
}

# Block all edits when code freeze is active
deny contains decision if {
    input.tool_name == "Edit"

    freeze_check := input.signals.check_code_freeze

    is_object(freeze_check)
    freeze_check.exit_code != 0

    decision := {
        "rule_id": "FREEZE-001",
        "reason": concat("", [
            "CODE FREEZE is active. ",
            freeze_check.output,
            ". Remove 'CODE FREEZE' from README.md to resume development."
        ]),
        "severity": "HIGH"
    }
}
```

**Key points**:

- `PreToolUse` runs before the action executes
- Signal runs on every Write/Edit attempt
- When README contains "CODE FREEZE", signal exits with code 1
- Policy blocks the action and shows the freeze message

## Testing

Add "CODE FREEZE" to your README.md:

```markdown
# My Project

**CODE FREEZE** - No changes allowed until release.
```

Ask Claude to edit any file:

```
Update src/App.tsx and add a new button
```

Cupcake will block the action with: "CODE FREEZE is active. Remove 'CODE FREEZE' from README.md to resume development."

Remove "CODE FREEZE" from README.md and Claude can edit files normally.
