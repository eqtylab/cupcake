---
title: "Writing Policies"
description: "Best practices for writing catalog rulebook policies"
---

# Writing Policies

This guide covers how to write effective policies for your rulebook.

## Namespace Convention

All catalog policies **must** use the namespace pattern:

```
cupcake.catalog.<rulebook_name>.policies.<harness>.<type>.<policy_name>
```

For example:

```rego
package cupcake.catalog.security_hardened.policies.claude.builtins.dangerous_commands
```

!!! warning "Namespace Validation"
`cupcake catalog lint` will fail if your policies don't follow this pattern.

## Policy Structure

### Basic Policy

```rego
# policies/claude/builtins/example.rego
package cupcake.catalog.my_rulebook.policies.claude.builtins.example

# METADATA
# title: Example Policy
# description: Demonstrates policy structure
# scope: rule
# custom:
#   decision: deny
#   severity: medium
#   routing:
#     events: [pre_tool_use]
#     tools: [bash]

default deny := false

deny {
    # Your condition here
    input.tool_input.command == "bad_command"
}

reasons[reason] {
    deny
    reason := {
        "message": "This command is not allowed",
        "severity": "medium"
    }
}
```

### Key Components

1. **Package declaration** - Must follow namespace convention
2. **METADATA block** - Routing and documentation
3. **Default rule** - Set safe default (usually `false`)
4. **Decision rule** - The actual policy logic
5. **Reasons** - Human-readable explanation

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

## Aggregation Policy

Each harness needs a `system/evaluate.rego` that aggregates decisions:

```rego
# policies/claude/system/evaluate.rego
package cupcake.catalog.my_rulebook.policies.claude.system

import data.cupcake.catalog.my_rulebook.policies.claude.builtins

# Aggregate all deny decisions
decisions[decision] {
    decision := builtins.dangerous_commands.deny
}

decisions[decision] {
    decision := builtins.dangerous_flags.deny
}

# Aggregate all reasons
all_reasons[reason] {
    reason := builtins.dangerous_commands.reasons[_]
}

all_reasons[reason] {
    reason := builtins.dangerous_flags.reasons[_]
}
```

## Helper Functions

Place shared helpers in `helpers/`:

```rego
# helpers/commands.rego
package cupcake.catalog.my_rulebook.helpers.commands

# Extract command name from shell input
get_command(input) = cmd {
    parts := split(input.tool_input.command, " ")
    cmd := parts[0]
}

# Check if command is in a list
command_in_list(cmd, list) {
    list[_] == cmd
}
```

Use in policies:

```rego
import data.cupcake.catalog.my_rulebook.helpers.commands

deny {
    cmd := commands.get_command(input)
    commands.command_in_list(cmd, dangerous_commands)
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
