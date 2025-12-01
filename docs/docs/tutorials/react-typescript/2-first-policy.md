---
title: "2. First Policy"
description: "Writing your first Cupcake policy to enforce component usage"
---

# Writing Your First Policy

## Step 1: Create the Policy File

Create a new file in your Cupcake policies directory:

```bash
touch .cupcake/policies/claude/components.rego
```

## Step 2: Write the Policy

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

## Step 3: Understanding the Policy

Let's break down what this policy does:

### Routing Metadata

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

### Single Unified Rule

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

### Content Field Normalization

Cupcake automatically normalizes the content fields:

- **Write tool**: `content` is copied to `new_string`
- **Edit tool**: Already has `new_string`

This allows you to use `input.tool_input.new_string` for both tools, keeping your policy DRY (Don't Repeat Yourself).

### The Decision Object

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
