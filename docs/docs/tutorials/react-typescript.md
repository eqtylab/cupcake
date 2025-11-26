---
layout: "@/layouts/mdx-layout.astro"
heading: "React + TypeScript"
description: "Writing your first policy for React applications"
---

## Overview

This tutorial walks you through writing your first Cupcake policy for a React + TypeScript application. By the end, you'll have a working policy that enforces your team's coding standards.

## Prerequisites

- Cupcake installed ([Installation Guide](/getting-started/installation))
- Cupcake initialized in your project ([Usage Guide](/getting-started/usage))
- A React + TypeScript application
- Claude Code as your AI coding agent

## Understanding Hooks and Tools

Cupcake integrates with Claude Code through **hooks** - events that trigger at different points in the interaction lifecycle.

### Hook Events vs Tools

There are two concepts to understand:

**1. Hook Events** - *When* something runs:
- `PreToolUse` - Before Claude executes a tool
- `PostToolUse` - After a tool completes successfully
- `UserPromptSubmit` - Before processing user input
- `SessionStart` - When a session starts
- And more...

**2. Tools** - *What* Claude is trying to do:
- `Write` - Creating a new file
- `Edit` - Modifying an existing file
- `Bash` - Running shell commands
- `Read` - Reading file contents
- `Grep` - Searching for text
- And more...

### How They Work Together

Hook events and tools combine to give you precise control:

```
Hook Event (WHEN) + Tool Matcher (WHAT) = Precise Trigger
```

**Examples:**

| Hook Event | Tool Matcher | Meaning |
|------------|--------------|---------|
| `PreToolUse` | `Write\|Edit` | Before Claude writes OR edits any file |
| `PostToolUse` | `Bash` | After Claude runs a shell command |
| `PreToolUse` | `*` | Before Claude uses ANY tool |
| `UserPromptSubmit` | *(no matcher)* | Before processing any user prompt |

**For this tutorial**, we'll use:
- **Hook Event**: `PreToolUse` (before execution)
- **Tool Matchers**: `Write` and `Edit` (file operations)
- **Result**: Our policy runs before Claude creates or modifies files

### Configuration

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
- [Claude Code Hooks Documentation](https://code.claude.com/docs/en/hooks) - Official reference
- [Hooks Compatibility Reference](/reference/hooks) - Which hooks work with which tools

## Tutorial Scenario

In this tutorial, we'll solve a real-world problem: **enforcing the use of custom components**.

Your team has built a custom `DatePicker` component with consistent styling, validation, and behavior. However, Claude sometimes uses the basic HTML `<input type="date">` element instead, which causes issues:

- **Inconsistent styling** across different browsers
- **Design system violations** - doesn't match your UI library
- **Missing validation logic** - your custom component has built-in date range validation

We'll write a policy that blocks HTML date inputs and guides Claude to use your `DatePicker` component instead.

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

**Routing Metadata (lines 1-6)**:
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

**Single Unified Rule**:

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

**Content Field Normalization**:

Cupcake automatically normalizes the content fields:
- **Write tool**: `content` is copied to `new_string`
- **Edit tool**: Already has `new_string`

This allows you to use `input.tool_input.new_string` for both tools, keeping your policy DRY (Don't Repeat Yourself).

**The Decision Object**:

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

## Testing Your Policy

Ask Claude to create a form with a date input:

```
Create a simple form with a date input field in src/components/Form.tsx
```

Claude will attempt to write `<input type="date" ...>` but Cupcake will block it and show the policy violation. Claude will then correct itself and use the `DatePicker` component instead.
