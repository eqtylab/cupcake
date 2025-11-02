# METADATA
# scope: package
# title: Global File Lock - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: CRITICAL
#   id: BUILTIN-GLOBAL-FILE-LOCK
#   routing:
#     required_events: ["afterFileEdit", "beforeShellExecution"]
package cupcake.policies.builtins.global_file_lock

import rego.v1

import data.cupcake.helpers.commands

# Block ALL file modifications when global lock is enabled
deny contains decision if {
    input.hook_event_name == "afterFileEdit"

    # Get the lock message from builtin config
    lock_message := input.builtin_config.global_file_lock.message

    decision := {
        "rule_id": "BUILTIN-GLOBAL-FILE-LOCK",
        "reason": lock_message,
        "severity": "CRITICAL"
    }
}

# Block shell commands that could write files
deny contains decision if {
    input.hook_event_name == "beforeShellExecution"

    command := lower(input.command)
    contains_write_pattern(command)

    lock_message := input.builtin_config.global_file_lock.message

    decision := {
        "rule_id": "BUILTIN-GLOBAL-FILE-LOCK",
        "reason": concat(" ", [lock_message, "(detected file write in shell command)"]),
        "severity": "CRITICAL"
    }
}

# Detect file write patterns in shell commands using helper functions
# This provides proper word-boundary matching and handles edge cases
contains_write_pattern(cmd) if {
	# Check for output redirection (>, >>, |, tee)
	commands.has_output_redirect(cmd)
}

contains_write_pattern(cmd) if {
	# Check for file copy/move commands with proper word boundaries
	file_commands := {"cp", "mv"}
	some command in file_commands
	commands.has_verb(cmd, command)
}
