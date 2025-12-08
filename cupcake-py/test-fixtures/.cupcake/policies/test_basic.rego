# METADATA
# scope: package
# title: Basic Test Policy
# authors: ["Test Suite"]
# custom:
#   severity: LOW
#   id: TEST-001
#   routing:
#     required_events: ["test", "PreToolUse"]
#     required_tools: []
package cupcake.policies.test_basic

import rego.v1

# Simple context injection for test events
add_context contains "Test event allowed" if {
    input.hookEventName == "test"
}

# Handle PreToolUse events for testing
deny contains decision if {
    input.hookEventName == "PreToolUse"
    input.tool_name == "DangerousTool"
    decision := {
        "reason": "Dangerous tool blocked in tests",
        "severity": "HIGH",
        "rule_id": "TEST-001-DENY"
    }
}

# Add context for all test events
add_context contains "Running in test mode" if {
    input.hookEventName == "test"
}