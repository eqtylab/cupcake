# METADATA
# scope: package
# title: Example Policy
# description: A minimal example policy that never fires
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.example

import rego.v1

# This rule will never fire - it's just here to prevent OPA compilation issues
# It checks for a command that nobody would ever type
deny contains decision if {
    input.tool_input.command == "CUPCAKE_EXAMPLE_RULE_THAT_NEVER_FIRES_12345"
    decision := {
        "reason": "This will never happen",
        "severity": "LOW",
        "rule_id": "EXAMPLE-001"
    }
}

# Replace the above with your actual policies
# Example of a real policy:
# deny contains decision if {
#     contains(input.tool_input.command, "rm -rf /")
#     decision := {
#         "reason": "Dangerous command blocked",
#         "severity": "HIGH",
#         "rule_id": "SAFETY-001"
#     }
# }
