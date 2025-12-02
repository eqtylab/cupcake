---
layout: "@/layouts/mdx-layout.astro"
title: "React + TypeScript"
heading: "React + TypeScript Tutorial"
description: "Writing policies for React applications with Cupcake"
---

This tutorial walks you through writing Cupcake policies for a React + TypeScript application. By the end, you'll have working policies that enforce your team's coding standards.

## Tutorial Scenario

In this tutorial, we'll solve a real-world problem: **enforcing the use of custom components**.

Your team has built a custom `DatePicker` component with consistent styling, validation, and behavior. However, Claude sometimes uses the basic HTML `<input type="date">` element instead, which causes issues:

- **Inconsistent styling** across different browsers
- **Design system violations** - doesn't match your UI library
- **Missing validation logic** - your custom component has built-in date range validation

We'll write a policy that blocks HTML date inputs and guides Claude to use your `DatePicker` component instead.

## What You'll Learn

1. **Setup** - Prerequisites and understanding hooks
2. **First Policy** - Writing a policy to enforce component usage
3. **First Signal** - Using signals to run validation scripts
4. **Obscure Rules** - Project-wide restrictions based on README content

---

## Setup

### Prerequisites

- Cupcake installed ([Installation Guide](../getting-started/installation.md))
- Cupcake initialized in your project ([Usage Guide](../getting-started/usage/index.md))
- A React + TypeScript application
- Claude Code as your AI coding agent

### Understanding Hooks and Tools

Cupcake integrates with Claude Code through **hooks** - events that trigger at different points in the interaction lifecycle.

#### Hook Events vs Tools

There are two concepts to understand:

**1. Hook Events** - _When_ something runs:

- `PreToolUse` - Before Claude executes a tool
- `PostToolUse` - After a tool completes successfully
- `UserPromptSubmit` - Before processing user input
- `SessionStart` - When a session starts
- And more...

**2. Tools** - _What_ Claude is trying to do:

- `Write` - Creating a new file
- `Edit` - Modifying an existing file
- `Bash` - Running shell commands
- `Read` - Reading file contents
- `Grep` - Searching for text
- And more...

#### How They Work Together

Hook events and tools combine to give you precise control:

```
Hook Event (WHEN) + Tool Matcher (WHAT) = Precise Trigger
```

**Examples:**

| Hook Event         | Tool Matcher   | Meaning                                |
| ------------------ | -------------- | -------------------------------------- |
| `PreToolUse`       | `Write\|Edit`  | Before Claude writes OR edits any file |
| `PostToolUse`      | `Bash`         | After Claude runs a shell command      |
| `PreToolUse`       | `*`            | Before Claude uses ANY tool            |
| `UserPromptSubmit` | _(no matcher)_ | Before processing any user prompt      |

**For this tutorial**, we'll use:

- **Hook Event**: `PreToolUse` (before execution)
- **Tool Matchers**: `Write` and `Edit` (file operations)
- **Result**: Our policy runs before Claude creates or modifies files

#### Configuration

Hook events are configured in `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cupcake eval"
          }
        ]
      }
    ]
  }
}
```

This configuration tells Claude Code:

1. On `PreToolUse` events (before tool execution)
2. When the tool matches `Write|Edit` (file operations)
3. Run `cupcake eval` to evaluate policies

**Learn More:**

- [Claude Code Hooks Documentation](https://docs.anthropic.com/en/docs/claude-code/hooks) - Official reference
- [Hooks Compatibility Reference](../reference/hooks.md) - Which hooks work with which tools

---

## Writing Your First Policy

### Step 1: Create the Policy File

Create a new file in your Cupcake policies directory:

```bash
touch .cupcake/policies/claude/components.rego
```

### Step 2: Write the Policy

Open `.cupcake/policies/claude/components.rego` and add:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]
package cupcake.policies.components

import rego.v1

# Block HTML date inputs in React files
deny contains decision if {
    # Match both Write and Edit tools
    input.tool_name in {"Write", "Edit"}

    # Only check .tsx files
    file_path := input.tool_input.file_path
    endswith(file_path, ".tsx")

    # Get content - Cupcake normalizes Write's "content" to "new_string"
    # so we can use the same field for both Write and Edit
    content := input.tool_input.new_string
    contains(lower(content), "<input")
    contains(lower(content), "type=\"date\"")

    decision := {
        "rule_id": "COMPONENT-001",
        "reason": "Use the custom DatePicker component instead of HTML <input type=\"date\">",
        "severity": "MEDIUM",
        "suggestion": "Replace with: <DatePicker value={value} onChange={setValue} />"
    }
}
```

### Step 3: Understanding the Policy

Let's break down what this policy does:

#### Routing Metadata

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]
package cupcake.policies.components
```

**IMPORTANT**: The METADATA block must be the FIRST thing in the file, before the `package` declaration. This tells Cupcake's routing engine when to evaluate this policy:

- `required_events: ["PreToolUse"]` - Run before a tool executes
- `required_tools: ["Write", "Edit"]` - Only for file operations

#### Single Unified Rule

The policy uses a single rule that handles both `Write` and `Edit` operations:

```rego
deny contains decision if {
    input.tool_name in {"Write", "Edit"}
    # ...
}
```

**Key points**:

- `input.tool_name in {"Write", "Edit"}` - Matches either tool using set membership
- `input.tool_input.new_string` - Unified field for content

#### Content Field Normalization

Cupcake automatically normalizes the content fields:

- **Write tool**: `content` is copied to `new_string`
- **Edit tool**: Already has `new_string`

This allows you to use `input.tool_input.new_string` for both tools, keeping your policy DRY (Don't Repeat Yourself).

#### The Decision Object

```rego
decision := {
    "rule_id": "COMPONENT-001",
    "reason": "Use the custom DatePicker component...",
    "severity": "MEDIUM",
    "suggestion": "Replace with: <DatePicker ... />"
}
```

- `rule_id` - Unique identifier for this rule
- `reason` - Why the action is being blocked
- `severity` - HIGH, MEDIUM, or LOW
- `suggestion` - (Optional) How to fix the issue

### Testing Your Policy

Ask Claude to create a form with a date input:

```
Create a simple form with a date input field in src/components/Form.tsx
```

Claude will attempt to write `<input type="date" ...>` but Cupcake will block it and show the policy violation. Claude will then correct itself and use the `DatePicker` component instead.

---

## Your First Signal

Signals let policies run scripts and use their output in decisions. We'll create a simple linting check that runs after Claude edits a file.

### Step 1: Create the Lint Script

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

### Step 2: Write the Policy

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

### Testing

Ask Claude to edit a file with double quotes:

```
Update src/components/Button.tsx and add a button with text "Click Me"
```

The lint check will fail and show: "FAIL: File uses double quotes. Please use single quotes instead."

Claude will then fix it to use single quotes.

---

## Obscure Rules

Sometimes you need policies based on project state. We'll create a signal that checks if README.md contains "CODE FREEZE" and blocks all file modifications until it's removed.

### Step 1: Create the Signal Script

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

### Step 2: Write the Policy

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

### Testing

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

---

## Key Takeaways

1. **Routing metadata** controls when policies run (which events, which tools)
2. **Content normalization** lets you write DRY policies for Write and Edit
3. **Signals** provide dynamic data from external scripts
4. **Decision objects** give Claude clear feedback on what went wrong and how to fix it

## Next Steps

- [Built-in Policies](../reference/policies/builtins.md) - Enable pre-built security policies
- [Custom Policies](../reference/policies/custom.md) - Deep dive into Rego policy syntax
- [Hooks Reference](../reference/hooks.md) - Complete hook and tool compatibility matrix
