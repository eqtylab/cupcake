---
title: "Writing Policies"
description: "Best practices for writing catalog rulebook policies"
---

# Writing Policies

This guide covers how to write effective policies for your rulebook.

## Namespace Convention

All catalog files **must** use these namespace patterns:

| Directory | Pattern | Example |
|-----------|---------|---------|
| `policies/<harness>/` | `cupcake.catalog.<name>.policies.<policy>` | `cupcake.catalog.security_hardened.policies.dangerous_commands` |
| `helpers/` | `cupcake.catalog.<name>.helpers.<helper>` | `cupcake.catalog.security_hardened.helpers.commands` |
| `system/` | `cupcake.catalog.<name>.system` | `cupcake.catalog.security_hardened.system` |

!!! warning "Namespace Validation"
    `cupcake catalog lint` will fail if your policies don't follow these patterns.

## Policy Structure

### Basic Policy

```rego
# policies/claude/example.rego
package cupcake.catalog.my_rulebook.policies.example

import rego.v1

# METADATA
# scope: package
# title: Example Policy
# description: Demonstrates policy structure
# custom:
#   severity: medium
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    input.tool_input.command == "bad_command"
    
    decision := {
        "rule_id": "EXAMPLE-001",
        "reason": "This command is not allowed",
        "severity": "MEDIUM",
    }
}
```

### Key Components

1. **Package declaration** - Must follow namespace convention
2. **`import rego.v1`** - Use modern Rego syntax
3. **METADATA block** - Routing and documentation
4. **Decision rule** - Returns decision objects with rule_id, reason, severity

## Routing Metadata

The `routing` section in METADATA controls when your policy is evaluated:

```yaml
# custom:
#   routing:
#     events: [pre_tool_use, post_tool_use]
#     tools: [bash, write]
```

### Events

| Event           | Description               |
| --------------- | ------------------------- |
| `pre_tool_use`  | Before a tool executes    |
| `post_tool_use` | After a tool executes     |
| `session_start` | When agent session begins |
| `session_end`   | When agent session ends   |
| `notification`  | Agent notifications       |
| `*`             | All events                |

### Tools

Specify which tools trigger evaluation:

- `bash` - Shell commands
- `write` - File writes
- `read` - File reads
- `edit` - File edits
- `*` - All tools

## Decision Types

| Decision | Effect                       |
| -------- | ---------------------------- |
| `allow`  | Explicitly permit the action |
| `deny`   | Block the action             |
| `ask`    | Prompt user for confirmation |
| `halt`   | Stop the agent entirely      |

## Severity Levels

| Severity   | Use Case                  |
| ---------- | ------------------------- |
| `critical` | Immediate security threat |
| `high`     | Significant risk          |
| `medium`   | Moderate concern          |
| `low`      | Minor issue               |
| `info`     | Informational only        |

## Aggregation Entrypoint

Each rulebook needs a single `system/evaluate.rego` at the **root level** (not per-harness):

```rego
# system/evaluate.rego
package cupcake.catalog.my_rulebook.system

import rego.v1

# METADATA
# scope: package
# custom:
#   entrypoint: true

evaluate := {
    "halts": collect_verbs("halt"),
    "denials": collect_verbs("deny"),
    "blocks": collect_verbs("block"),
    "asks": collect_verbs("ask"),
    "allow_overrides": collect_verbs("allow_override"),
    "add_context": collect_verbs("add_context"),
}

collect_verbs(verb_name) := result if {
    verb_sets := [value |
        walk(data.cupcake.catalog.my_rulebook.policies, [path, value])
        path[count(path) - 1] == verb_name
    ]
    all_decisions := [decision |
        some verb_set in verb_sets
        some decision in verb_set
    ]
    result := all_decisions
}

default collect_verbs(_) := []
```

This entrypoint uses `walk()` to automatically discover all decision verbs across all policies, so you don't need to manually import each policy.

## Helper Functions

Place shared helpers in `helpers/` at the rulebook root:

```rego
# helpers/commands.rego
package cupcake.catalog.my_rulebook.helpers.commands

import rego.v1

# Check if command contains a verb with word boundaries
has_verb(command, verb) if {
    pattern := concat("", ["(^|\\s)", verb, "(\\s|$)"])
    regex.match(pattern, command)
}

# Check if command has any of the specified flags
has_any_flag(command, flag_set) if {
    some flag in flag_set
    pattern := concat("", ["(^|\\s)", flag, "(\\s|$|=)"])
    regex.match(pattern, command)
}
```

Import and use helpers in your policies:

```rego
# policies/claude/dangerous_flags.rego
package cupcake.catalog.my_rulebook.policies.dangerous_flags

import data.cupcake.catalog.my_rulebook.helpers.commands
import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    
    cmd := lower(input.tool_input.command)
    commands.has_verb(cmd, "git")
    commands.has_any_flag(cmd, {"--no-verify", "-n"})
    
    decision := {
        "rule_id": "GIT-001",
        "reason": "Blocked --no-verify flag on git command",
        "severity": "HIGH",
    }
}
```

## Testing Policies

Test your policies locally:

```bash
# Create a test event
cat > test-event.json << 'EOF'
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf /" },
  "session_id": "test",
  "cwd": "/tmp",
  "transcript_path": "/tmp/transcript.md"
}
EOF

# Evaluate against the test input
cupcake eval --harness claude < test-event.json
```

## Best Practices

1. **Be specific** - Target exactly what you want to block
2. **Provide context** - Use clear reason messages
3. **Test edge cases** - Try variations of blocked inputs
4. **Document decisions** - Explain why something is blocked
5. **Use appropriate severity** - Don't cry wolf with everything as critical
