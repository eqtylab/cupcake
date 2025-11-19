---
layout: "@/layouts/mdx-layout.astro"
heading: "2. First Signal"
description: "Using signals to run validation scripts after file edits"
---

## Overview

Signals let policies run scripts and use their output in decisions. We'll create a simple linting check that runs after Claude edits a file.

## Step 1: Create the Lint Script

Create the signals directory and script file:

```bash
mkdir -p .cupcake/signals
touch .cupcake/signals/simple-lint.sh
```

Edit `.cupcake/signals/simple-lint.sh`:

```bash
#!/bin/bash
# Simple lint check: only allow single quotes in src/ files

# Check all .tsx files in src/ for double quotes (excluding imports)
FAILED=0

# Use find to get all .tsx files in src/
while IFS= read -r file; do
    if grep -v "^import" "$file" | grep -q '"'; then
        echo "FAIL: $file uses double quotes"
        FAILED=1
    fi
done < <(find src -name "*.tsx" -type f 2>/dev/null)

if [ $FAILED -eq 1 ]; then
    exit 1
fi

echo "PASS: All files use single quotes"
exit 0
```

Make it executable:

```bash
chmod +x .cupcake/signals/simple-lint.sh
```

**Note**: Scripts in `.cupcake/signals/` are auto-discovered by Cupcake. No configuration needed.

## Step 2: Write the Policy

Create the policy file:

```bash
touch .cupcake/policies/claude/post_edit_lint.rego
```

Edit `.cupcake/policies/claude/post_edit_lint.rego`:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PostToolUse"]
#     required_tools: ["Edit"]
#   signals:
#     - simple-lint
package cupcake.policies.post_edit_lint

import rego.v1

# Run lint check after file edits in src/
deny contains decision if {
    input.tool_name == "Edit"
    file_path := input.tool_input.file_path

    # Only run when src/ files are edited
    contains(file_path, "src/")
    endswith(file_path, ".tsx")

    # Get lint result from signal
    lint_result := input.signals.simple_lint

    # Check if lint failed (exit code != 0)
    is_object(lint_result)
    lint_result.exit_code != 0

    decision := {
        "rule_id": "LINT-001",
        "reason": lint_result.output,
        "severity": "MEDIUM"
    }
}
```

**Key points**:
- `PostToolUse` runs after the edit completes
- Signal checks all files in `src/` directory
- **Signal return format**:
  - Success (exit 0): Returns stdout as a string
  - Failure (exit != 0): Returns object `{exit_code: 1, output: "...", error: "..."}`
- Always check `is_object(lint_result)` and `lint_result.exit_code != 0` to detect failures
- Use `lint_result.output` to access the signal's output in your deny reason

## Testing

Ask Claude to edit a file with double quotes:

```
Update src/components/Button.tsx and add a button with text "Click Me"
```

The lint check will fail and show: "FAIL: File uses double quotes. Please use single quotes instead."

Claude will then fix it to use single quotes.
