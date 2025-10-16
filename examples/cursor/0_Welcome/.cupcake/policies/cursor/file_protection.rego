# METADATA
# scope: package
# title: Cursor File Protection
# custom:
#   routing:
#     required_events: ["beforeReadFile", "afterFileEdit"]
package cupcake.policies.cursor.file_protection

import rego.v1

# Protect sensitive files from reading
deny contains decision if {
    input.hook_event_name == "beforeReadFile"
    sensitive_patterns := [".ssh/id_", ".aws/credentials", ".env", "secrets"]
    some pattern in sensitive_patterns
    contains(input.file_path, pattern)

    decision := {
        "rule_id": "CURSOR-FILE-READ-001",
        "reason": "Access to sensitive file blocked",
        "agent_context": concat("", [
            "Attempted to read sensitive file containing '", pattern, "'. ",
            "These files contain secrets that should not be exposed. ",
            "Instead: 1) Ask user to provide redacted version, ",
            "2) Use environment variables, ",
            "3) Create example/template files without real secrets."
        ]),
        "severity": "CRITICAL"
    }
}

# Validate file edits
deny contains decision if {
    input.hook_event_name == "afterFileEdit"
    contains(input.file_path, "/etc/")

    decision := {
        "rule_id": "CURSOR-FILE-EDIT-001",
        "reason": "System file modification blocked",
        "agent_context": "Attempted to modify system file in /etc/. System files require manual intervention. Create configuration in user space instead.",
        "severity": "HIGH"
    }
}
