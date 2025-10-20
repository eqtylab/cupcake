# METADATA
# scope: package
# title: Protected Paths - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-PROTECTED-PATHS
#   routing:
#     required_events: ["afterFileEdit"]
package cupcake.policies.builtins.protected_paths

import data.cupcake.helpers.paths
import rego.v1

# Block file edits to protected paths
deny contains decision if {
	input.hook_event_name == "afterFileEdit"

	# Get the file path from Cursor's raw schema
	file_path := input.file_path

	# Get the list of protected paths from builtin config
	protected_list := input.builtin_config.protected_paths.paths

	# Check if the edited file is in a protected path
	is_protected(file_path, protected_list)

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS",
		"reason": concat("", [
			"File modification blocked: ",
			file_path,
			" is in a protected path. Protected paths are: ",
			concat(", ", protected_list),
		]),
		"severity": "HIGH",
	}
}

# Check if a file path starts with any protected path
is_protected(file_path, protected_list) if {
	some protected_path in protected_list
	paths.targets_protected(file_path, protected_path)
}
