---
title: "Signals"
description: "Extend policy evaluation with external data and capabilities"
---

# Signals

Signals are arbitrary programs that collect additional data to pass to policy evaluation. They enable Cupcake to make context-aware decisions by fetching dynamic information at runtime.

## What Signals Enable

Signals turn Cupcake into a composable orchestration layer. Any program that can output data can be a signal:

| Use Case | Example |
|----------|---------|
| **LLM-as-judge** | Cupcake Watchdog passes agent actions to an LLM for evaluation |
| **External guardrails** | NVIDIA NeMo Guardrails, Guardrails AI, or other policy systems |
| **Version control** | Git status, branch info, uncommitted changes |
| **Dev tools** | Linters, type checkers, test runners |
| **External APIs** | Web services, databases, feature flags |
| **System state** | Time of day, environment variables, file contents |

The pattern is: **complex evaluation externally, simple decision in Rego**.

## How Signals Work

```
Agent action triggers event
         ↓
Engine routes to matching policies
         ↓
Engine executes required signals (concurrently)
         ↓
Signal results injected as input.signals.*
         ↓
Policy evaluates with enriched context
```

## Defining Signals

### Option 1: In rulebook.yml

```yaml
signals:
  git_branch:
    command: "git branch --show-current"
    timeout_seconds: 2

  appointment_check:
    command: ".cupcake/signals/appointment_check.sh"
    timeout_seconds: 5
```

### Option 2: Auto-discovery

Place executable scripts in `.cupcake/signals/`. The filename (minus extension) becomes the signal name:

```
.cupcake/
└── signals/
    ├── git_branch.sh      → signal name: git_branch
    ├── lint_check.py      → signal name: lint_check
    └── api_status         → signal name: api_status
```

## Writing Signal Scripts

Signals receive the event data via stdin (JSON) and output results to stdout.

### Simple Example

```bash
#!/bin/bash
# .cupcake/signals/git_branch.sh
git branch --show-current
```

### With Event Context

```bash
#!/bin/bash
# .cupcake/signals/appointment_check.sh

# Read event data from stdin
event_data=$(cat)

# Extract relevant fields
sql_query=$(echo "$event_data" | jq -r '.tool_input.sql // empty')

# Check appointment timing (your logic here)
if [[ "$sql_query" == *"cancelled"* ]]; then
    echo '{"relevant": true, "within_24_hours": true}'
else
    echo '{"relevant": false}'
fi
```

### Output Format

Signals can return:

- **JSON** - Parsed and accessible as structured data
- **Plain text** - Available as a string value

```bash
# JSON output
echo '{"approved": true, "score": 0.95}'

# Plain text output
echo "main"
```

## Using Signals in Policies

### Declaring Dependencies

Policies declare required signals in metadata for routing optimization:

```rego
# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_signals: ["appointment_check", "git_branch"]
package cupcake.policies.example
```

### Accessing Signal Data

Signal results are available at `input.signals.<signal_name>`:

```rego
deny contains decision if {
    # Access structured signal data
    signal_data := input.signals.appointment_check
    signal_data.relevant == true
    signal_data.within_24_hours == true

    decision := {
        "rule_id": "APPT-001",
        "reason": "Cannot modify appointment within 24 hours",
        "severity": "HIGH"
    }
}

deny contains decision if {
    # Access simple string signal
    branch := input.signals.git_branch
    branch == "main"
    contains(input.tool_input.command, "git push --force")

    decision := {
        "rule_id": "GIT-001",
        "reason": "Force push to main branch blocked",
        "severity": "HIGH"
    }
}
```

### Handling Signal Failures

Failed signals return structured error information:

```rego
deny contains decision if {
    signal_result := input.signals.validation_check
    signal_result.success == false

    decision := {
        "rule_id": "SIG-ERR",
        "reason": concat("", ["Validation signal failed: ", signal_result.error]),
        "severity": "MEDIUM"
    }
}
```

## Watchdog: LLM-as-Judge via Signals

Cupcake's Watchdog feature demonstrates the power of signals. It wraps an LLM evaluation in a signal:

1. Agent action is passed to the Watchdog signal
2. Signal sends action + rules to an LLM for judgment
3. LLM returns approval/denial with reasoning
4. Rego policy gates on the simple boolean result

```rego
# Watchdog result comes through as a signal
deny contains decision if {
    watchdog := input.signals.watchdog_evaluation
    watchdog.approved == false

    decision := {
        "rule_id": "WATCHDOG-001",
        "reason": watchdog.reasoning,
        "severity": "HIGH"
    }
}
```

This pattern—**complex evaluation externally, simple gate in Rego**—enables integration with any evaluation system while keeping policies clean and declarative.

## Performance Considerations

- **Concurrent execution** - All signals for an event run in parallel
- **Routing optimization** - Signals only execute if a matching policy is routed
- **Configurable timeouts** - Set per-signal timeouts to prevent blocking
- **Early exit** - If no policies match, signals are skipped entirely

## Best Practices

1. **Keep signals fast** - They run on every matching event
2. **Return JSON** - Structured data is easier to work with in Rego
3. **Handle errors gracefully** - Return meaningful error info on failure
4. **Use timeouts** - Prevent slow signals from blocking evaluation
5. **Declare dependencies** - Use `required_signals` metadata for optimization
