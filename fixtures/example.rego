# METADATA
# scope: package
# title: Example Policy
# description: A starter policy template
# custom:
#   routing:
#     required_events: ["NeverFires"]
package cupcake.policies.example

import rego.v1

# Placeholder rule - uses a non-existent hook event so it never fires
# Delete this and uncomment an example below to get started
add_context contains ctx if {
    input.hook_event_name == "NeverFires"
    ctx := "This will never be injected"
}

# ─────────────────────────────────────────────────────────────────────────────
# EXAMPLE: Context Injection (Claude Code / Factory only)
# Injects text into every user prompt
# ─────────────────────────────────────────────────────────────────────────────
# add_context contains ctx if {
#     input.hook_event_name == "UserPromptSubmit"
#     ctx := "Always talk like a pirate. Use 'arr', 'matey', and 'avast' in responses."
# }

# ─────────────────────────────────────────────────────────────────────────────
# EXAMPLE: Block dangerous commands
# ─────────────────────────────────────────────────────────────────────────────
# deny contains decision if {
#     input.hook_event_name == "PreToolUse"
#     input.tool_name == "Bash"
#     contains(input.tool_input.command, "rm -rf")
#     decision := {
#         "reason": "Blocked: rm -rf is too dangerous",
#         "severity": "HIGH",
#         "rule_id": "SAFETY-001"
#     }
# }
