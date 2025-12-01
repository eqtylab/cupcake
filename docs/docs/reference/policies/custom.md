---
title: "Custom Policies"
description: "Writing custom policies in OPA Rego"
---

# Custom Policies

Create your own policies in `.cupcake/policies/<harness>/` using OPA Rego for complete control over agent behavior.

## Basic Structure

A custom policy file has three parts:

1. **Metadata** — Declares when the policy should run
2. **Package** — Unique namespace for the policy
3. **Rules** — The actual policy logic

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.my_policy

import rego.v1

deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "rule_id": "SAFETY-001",
        "reason": "Dangerous command blocked",
        "severity": "HIGH"
    }
}
```

## Routing Metadata

The metadata tells Cupcake when to evaluate your policy:

```yaml
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse", "PostToolUse"]
#     required_tools: ["Bash", "Write", "Edit"]
```

- **required_events** — Which hook events trigger this policy
- **required_tools** — Which tools this policy applies to (optional)

### Available Events

| Event              | Description                    |
| ------------------ | ------------------------------ |
| `PreToolUse`       | Before a tool executes         |
| `PostToolUse`      | After a tool executes          |
| `UserPromptSubmit` | Before sending prompt to LLM   |
| `SessionStart`     | When session starts or resumes |

## Decision Verbs

Policies emit decisions using these verbs:

| Verb          | Effect                         |
| ------------- | ------------------------------ |
| `deny`        | Block the action               |
| `halt`        | Block and stop the session     |
| `ask`         | Prompt user for confirmation   |
| `add_context` | Inject context into the prompt |

### Deny Example

```rego
deny contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "--no-verify")

    decision := {
        "rule_id": "GIT-001",
        "reason": "Cannot bypass pre-commit hooks",
        "severity": "HIGH"
    }
}
```

### Ask Example

```rego
ask contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git push")

    decision := {
        "rule_id": "GIT-002",
        "reason": "Pushing to remote repository",
        "question": "Do you want to allow this push?",
        "severity": "MEDIUM"
    }
}
```

### Context Injection Example

```rego
add_context contains context if {
    input.hook_event_name == "UserPromptSubmit"
    context := "Remember: Always run tests before committing."
}
```

## Accessing Input Data

The `input` object contains event data:

```rego
# Tool name
input.tool_name  # "Bash", "Write", "Edit", etc.

# Tool-specific input
input.tool_input.command      # For Bash
input.tool_input.file_path    # For Write/Edit/Read
input.tool_input.content      # For Write

# Event metadata
input.hook_event_name  # "PreToolUse", "PostToolUse", etc.
input.session_id
input.cwd

# For UserPromptSubmit
input.prompt
```

## File Organization

Place policies in the harness-specific directory:

```
.cupcake/
├── policies/
│   ├── claude/           # Claude Code policies
│   │   ├── my_policy.rego
│   │   └── another.rego
│   ├── cursor/           # Cursor policies
│   │   └── cursor_rules.rego
│   └── factory/          # Factory AI policies
│       └── factory_rules.rego
└── rulebook.yml
```

## Testing Policies

Test your policy with a JSON event:

```bash
# Create test event
echo '{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {"command": "rm -rf /"},
  "session_id": "test",
  "cwd": "/tmp"
}' > test.json

# Evaluate
cupcake eval --harness claude < test.json
```
