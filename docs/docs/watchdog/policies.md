# Writing Policies with Watchdog

When Watchdog is enabled, its judgment is available to your Rego policies at `input.signals.watchdog`.

## Watchdog Output Schema

```json
{
  "allow": true,
  "confidence": 0.95,
  "reasoning": "This git push command appears safe and aligned with typical development workflow.",
  "concerns": [],
  "suggestions": []
}
```

Or when flagging a concern:

```json
{
  "allow": false,
  "confidence": 0.82,
  "reasoning": "This command reads SSH private keys which could indicate data exfiltration.",
  "concerns": ["sensitive_file_access", "potential_exfiltration"],
  "suggestions": ["Consider using a deploy key instead", "Verify this action is intended"]
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `allow` | boolean | Whether Watchdog recommends allowing the action |
| `confidence` | float (0-1) | How confident Watchdog is in this judgment |
| `reasoning` | string | Human-readable explanation |
| `concerns` | array | Specific concerns identified (empty if none) |
| `suggestions` | array | Alternative approaches or next steps |

## Example Policies

### Block High-Confidence Denials

```rego
package cupcake.policies.watchdog_security

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"

    watchdog := input.signals.watchdog
    watchdog.allow == false
    watchdog.confidence > 0.7

    decision := {
        "rule_id": "WATCHDOG-DENY",
        "reason": watchdog.reasoning,
        "severity": "HIGH",
    }
}
```

### Ask for Confirmation on Medium Confidence

```rego
ask contains decision if {
    input.hook_event_name == "PreToolUse"

    watchdog := input.signals.watchdog
    watchdog.allow == false
    watchdog.confidence > 0.4
    watchdog.confidence <= 0.7

    decision := {
        "rule_id": "WATCHDOG-ASK",
        "reason": concat("", ["Watchdog flagged: ", watchdog.reasoning]),
        "question": "Do you want to proceed?",
        "severity": "MEDIUM",
    }
}
```

### Add Context from Suggestions

```rego
add_context contains msg if {
    input.hook_event_name == "PreToolUse"

    watchdog := input.signals.watchdog
    watchdog.allow == true
    count(watchdog.suggestions) > 0

    msg := concat("\n", watchdog.suggestions)
}
```

## Combining with Deterministic Rules

Watchdog works alongside your existing policies. A common pattern:

1. **Deterministic rules handle known patterns**: Block `rm -rf /`, protect `.env` files, etc.
2. **Watchdog catches the unexpected**: Novel attacks, misaligned intent, subtle issues

```rego
# Deterministic rule - always block this pattern
halt contains decision if {
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")

    decision := {
        "rule_id": "BLOCK-DANGEROUS-RM",
        "reason": "Refusing to delete root filesystem",
        "severity": "CRITICAL",
    }
}

# Watchdog rule - catch things we didn't anticipate
deny contains decision if {
    input.signals.watchdog.allow == false
    input.signals.watchdog.confidence > 0.8

    decision := {
        "rule_id": "WATCHDOG-DENY",
        "reason": input.signals.watchdog.reasoning,
        "severity": "HIGH",
    }
}
```

## Handling Missing Watchdog Data

If Watchdog is disabled or fails, `input.signals.watchdog` may not exist. Guard against this:

```rego
deny contains decision if {
    # Only evaluate if watchdog data exists
    watchdog := input.signals.watchdog
    watchdog != null

    watchdog.allow == false
    watchdog.confidence > 0.7

    decision := { ... }
}
```

## Policy Routing

Watchdog runs automatically when enabled—you don't need to declare it in your policy's `required_signals`. The engine injects Watchdog results into every event evaluation.

Your policy's routing metadata should focus on events and tools:

```rego
# METADATA
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash", "Edit"]
package cupcake.policies.my_watchdog_policy
```
