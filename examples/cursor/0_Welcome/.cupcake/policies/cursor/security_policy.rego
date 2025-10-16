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
        "agent_context": concat("", [
            cmd, " detected in command. This is a destructive operation. ",
            "Alternatives: 1) Use 'trash' command for safe deletion, ",
            "2) Be more specific with paths, ",
            "3) Use --dry-run flag first to preview changes."
        ]),
        "severity": "CRITICAL"
    }
}

# Block sudo with helpful agent guidance
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"
    contains(input.command, "sudo")

    decision := {
        "rule_id": "CURSOR-SUDO-001",
        "reason": "Elevated privileges required",
        "agent_context": "sudo detected. Elevated privileges are dangerous. Consider: 1) Use specific commands without sudo, 2) Modify file permissions instead, 3) Use Docker containers for isolation. If you must use sudo, ask the user to run it manually.",
        "severity": "HIGH"
    }
}
