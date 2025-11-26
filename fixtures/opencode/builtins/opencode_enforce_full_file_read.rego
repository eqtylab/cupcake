# METADATA
# scope: package
# title: Enforce Full File Read - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: MEDIUM
#   id: BUILTIN-ENFORCE-FULL-READ
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Read"]
package cupcake.policies.builtins.opencode_enforce_full_file_read

import rego.v1

# Deny partial reads of files (MVP: enforce for all files)
deny contains decision if {
	# Only apply to Read tool
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Read"

	# Check if offset or limit parameters are present
	has_partial_read_params

	# Get configured message from signal (with fallback)
	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-ENFORCE-FULL-READ",
		"reason": message,
		"severity": "MEDIUM",
	}
}

# Check if the Read tool has offset or limit parameters
has_partial_read_params if {
	# Check for offset parameter
	"offset" in object.keys(input.tool_input)
}

has_partial_read_params if {
	# Check for limit parameter
	"limit" in object.keys(input.tool_input)
}

# Get configured message from builtin config
get_configured_message := msg if {
	# Direct access to builtin config (no signal execution needed)
	msg := input.builtin_config.opencode_enforce_full_file_read.message
} else := msg if {
	# Fallback to default message
	msg := "Please read the entire file first (files under 2000 lines must be read completely)"
}

# Future enhancement: Get max lines threshold
# This would be used in a future version to check file size
# and only enforce full reads for files under the threshold
get_max_lines_threshold := lines if {
	# Direct access to builtin config (no signal execution needed)
	lines := input.builtin_config.opencode_enforce_full_file_read.max_lines
} else := lines if {
	# Default to 2000 lines
	lines := 2000
}
