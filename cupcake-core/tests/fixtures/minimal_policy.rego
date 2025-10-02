# METADATA
# scope: package
# custom:
#   routing:
#     required_events: ["UserPromptSubmit"]
package cupcake.policies.minimal

import rego.v1

# Minimal policy for test compilation - never triggers
deny contains decision if {
    input.test_condition == "never_match_12345"
    decision := {
        "reason": "Test policy that never triggers",
        "severity": "LOW",
        "rule_id": "TEST-001"
    }
}