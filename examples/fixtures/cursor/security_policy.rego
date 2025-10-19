# METADATA
# scope: package
# title: Cursor Security Policy
# custom:
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.cursor.security

import rego.v1

# Block dangerous shell commands with differentiated feedback
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    dangerous_commands := ["rm -rf", "sudo rm", "format", "fdisk", "> /dev/"]
    some cmd in dangerous_commands
    contains(input.command, cmd)

    decision := {
        "rule_id": "CURSOR-SECURITY-001",
        "reason": concat(" ", ["Dangerous command blocked:", cmd]),
        "agent_context": "This action violates system policies. Recursive deletion of directories is prohibited for security reasons.",
        "severity": "CRITICAL"
    }
}

# Block sudo commands
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "sudo")

    decision := {
        "rule_id": "CURSOR-SUDO-001",
        "reason": "Elevated privileges required",
        "agent_context": "This action violates system policies. Commands requiring elevated privileges are prohibited for security reasons.",
        "severity": "HIGH"
    }
}
