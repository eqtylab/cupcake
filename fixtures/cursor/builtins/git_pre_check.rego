# METADATA
# scope: package
# title: Git Pre-Check - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: MEDIUM
#   id: BUILTIN-GIT-PRE-CHECK
#   routing:
#     required_events: ["beforeShellExecution"]
package cupcake.policies.builtins.git_pre_check

import rego.v1

# Ask user before running git operations
ask contains decision if {
    input.hook_event_name == "beforeShellExecution"

    # Get the command from Cursor's raw schema
    command := lower(input.command)

    # Check if it's a git command
    contains(command, "git")

    # Get the pre-check command from builtin config
    precheck_cmd := input.builtin_config.git_pre_check.precheck_command

    decision := {
        "rule_id": "BUILTIN-GIT-PRE-CHECK",
        "reason": concat("", [
            "Git operation detected: ",
            input.command
        ]),
        "question": concat("", [
            "Run pre-check command: ",
            precheck_cmd,
            " before executing?"
        ]),
        "severity": "MEDIUM"
    }
}
