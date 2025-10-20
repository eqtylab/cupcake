# METADATA
# scope: package
# title: Rulebook Security Guardrails - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: CRITICAL
#   id: BUILTIN-RULEBOOK-SECURITY-GUARDRAILS
#   routing:
#     required_events: ["beforeReadFile", "afterFileEdit", "beforeShellExecution"]
package cupcake.policies.builtins.rulebook_security_guardrails

import rego.v1

# Block reading .cupcake directory files
halt contains decision if {
    input.hook_event_name == "beforeReadFile"

    # Get the file path from Cursor's raw schema
    file_path := input.file_path

    # Check if file is in .cupcake directory
    contains(file_path, ".cupcake")

    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
        "reason": "Access to .cupcake directory is prohibited. This directory contains security policies and trust data.",
        "severity": "CRITICAL"
    }
}

# Block modifications to .cupcake directory files
deny contains decision if {
    input.hook_event_name == "afterFileEdit"

    # Get the file path from Cursor's raw schema
    file_path := input.file_path

    # Check if file is in .cupcake directory
    contains(file_path, ".cupcake")

    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
        "reason": "Modifications to .cupcake directory files are not permitted. This directory contains security policies and trust data.",
        "severity": "CRITICAL"
    }
}

# Block shell commands that attempt to modify .cupcake directory
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"

    # Get the command from Cursor's raw schema
    command := lower(input.command)

    # Check if command targets .cupcake directory
    contains(command, ".cupcake")

    # Check for modification commands
    some mod_cmd in {"rm", "mv", "cp", "chmod", "chown", "delete", "del"}
    contains(command, mod_cmd)

    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
        "reason": "Shell commands that modify .cupcake directory are not permitted. This directory contains security policies and trust data.",
        "severity": "CRITICAL"
    }
}
