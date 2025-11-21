# METADATA
# scope: package
# title: Global File Lock - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-GLOBAL-FILE-LOCK
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash", "Task"]
package cupcake.policies.builtins.global_file_lock

import rego.v1

import data.cupcake.helpers.commands

# Block all file write operations when enabled
halt contains decision if {
	# Global file lock is active - prevents all file write operations
	input.hook_event_name == "PreToolUse"

	# Check for file editing tools
	editing_tools := {"Edit", "Write", "MultiEdit", "NotebookEdit"}
	input.tool_name in editing_tools

	# Get configured message from signals (fallback to default)
	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-GLOBAL-FILE-LOCK",
		"reason": message,
		"severity": "HIGH",
	}
}

# Also block Bash commands that write files
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	# Check if command contains file write patterns
	# Fix: Bash tool uses tool_input.command, not params.command
	command := lower(input.tool_input.command)
	contains_write_pattern(command)

	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-GLOBAL-FILE-LOCK",
		"reason": concat(" ", [message, "(detected file write in bash command)"]),
		"severity": "HIGH",
	}
}

# Helper: Check if bash command contains write patterns using helper functions
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

# Get configured message (would come from signal in real implementation)
get_configured_message := msg if {
	# In production, this would query a signal like __builtin_global_file_lock_message
	# For now, use a default message
	msg := "File editing is disabled globally by policy"
}
