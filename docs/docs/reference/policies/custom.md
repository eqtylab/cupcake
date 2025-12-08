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

| Event              | Description                         |
| ------------------ | ----------------------------------- |
| `PreToolUse`       | Before a tool executes              |
| `PostToolUse`      | After a tool executes               |
| `UserPromptSubmit` | Before sending prompt to LLM        |
| `SessionStart`     | When session starts or resumes      |
| `SessionEnd`       | When session ends                   |
| `Stop`             | When agent stops                    |
| `SubagentStop`     | When subagent (Task tool) completes |
| `PreCompact`       | Before memory compaction            |
| `Notification`     | On agent notifications              |

## Decision Verbs

Policies emit decisions using these verbs (in priority order):

| Verb          | Priority | Effect                                   | Supported Events |
| ------------- | -------- | ---------------------------------------- | ---------------- |
| `halt`        | Highest  | Block and stop the session immediately   | All              |
| `deny`        | High     | Block the action (policy violation)      | All              |
| `block`       | High     | Block the action (same priority as deny) | All              |
| `ask`         | Medium   | Prompt user for confirmation             | Tool events      |
| `modify`      | Medium   | Allow with modified input                | PreToolUse only  |
| `add_context` | N/A      | Inject context into the prompt           | Prompt events    |

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

### Modify Example

The `modify` verb allows a tool to proceed with transformed input. Use it to sanitize commands, add safety flags, or enforce conventions:

```rego
modify contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "rule_id": "SANITIZE-001",
        "reason": "Dangerous command sanitized",
        "severity": "HIGH",
        "priority": 80,
        "updated_input": {
            "command": "echo 'Blocked: rm -rf commands are not allowed'"
        }
    }
}
```

**Key fields:**

- **priority** (1-100) — Higher values win when multiple policies modify the same field
- **updated_input** — Partial object merged with original tool input

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
├── rulebook.yml
├── system/               # System aggregation entrypoint
│   └── evaluate.rego
├── policies/
│   ├── claude/           # Claude Code policies
│   │   ├── builtins/     # Built-in policies
│   │   └── my_policy.rego
│   ├── cursor/           # Cursor policies
│   │   ├── builtins/
│   │   └── cursor_rules.rego
│   ├── factory/          # Factory AI policies
│   │   ├── builtins/
│   │   └── factory_rules.rego
│   └── opencode/         # OpenCode policies
│       ├── builtins/
│       └── opencode_rules.rego
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
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}' > test.json

# Evaluate
cupcake eval --harness claude < test.json
```
