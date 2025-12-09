# METADATA
# scope: package
# title: Protected Paths - Builtin Policy (Cursor)
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-PROTECTED-PATHS
#   routing:
#     required_events: ["afterFileEdit", "beforeShellExecution"]
package cupcake.policies.builtins.protected_paths

import rego.v1

# Block file edits to protected paths
deny contains decision if {
	input.hook_event_name == "afterFileEdit"

	# Get the file path from Cursor's raw schema
	# TOB-4 fix: Use canonical path (always provided by Rust preprocessing)
	file_path := input.resolved_file_path

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

# Block destructive shell commands that would affect a parent directory containing protected paths
# This catches cases like `rm -rf /home/user/*` when `/home/user/.cupcake/` is protected
# The `affected_parent_directories` field is populated by Rust preprocessing for destructive commands
deny contains decision if {
	input.hook_event_name == "beforeShellExecution"

	# Get affected parent directories from preprocessing
	# This is populated for commands like rm -rf, chmod -R, etc.
	affected_dirs := input.affected_parent_directories
	count(affected_dirs) > 0

	# Get the list of protected paths from builtin config
	protected_list := input.builtin_config.protected_paths.paths

	# Check if any protected path is a CHILD of an affected directory
	some affected_dir in affected_dirs
	some protected_path in protected_list
	protected_is_child_of_affected(protected_path, affected_dir)

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS-PARENT",
		"reason": concat("", [
			"Destructive command blocked: ",
			protected_path,
			" would be affected by operation on ",
			affected_dir,
		]),
		"severity": "HIGH",
	}
}

# Check if a file path starts with any protected path
# Now uses resolved_file_path directly (canonical, absolute path from preprocessing)
is_protected(file_path, protected_list) if {
	some protected_path in protected_list
	# Case-insensitive check for protected path match
	startswith(lower(file_path), lower(protected_path))
}

# Check if a protected path is a child of an affected directory
# This is the "reverse" check for parent directory protection:
# protected_path: /home/user/.cupcake/config.yml
# affected_dir:   /home/user/
# Returns true because the protected path is inside the affected directory
protected_is_child_of_affected(protected_path, affected_dir) if {
	# Normalize: ensure affected_dir ends with /
	affected_normalized := ensure_trailing_slash(affected_dir)

	# Check if protected path starts with the affected directory
	startswith(lower(protected_path), lower(affected_normalized))
}

protected_is_child_of_affected(protected_path, affected_dir) if {
	# Also check exact match (rm -rf /home/user/.cupcake)
	lower(protected_path) == lower(affected_dir)
}

protected_is_child_of_affected(protected_path, affected_dir) if {
	# Handle case where affected_dir is specified without trailing slash
	# but protected_path has it as a prefix
	not endswith(affected_dir, "/")
	prefix := concat("", [lower(affected_dir), "/"])
	startswith(lower(protected_path), prefix)
}

# Helper to ensure path ends with /
ensure_trailing_slash(path) := result if {
	endswith(path, "/")
	result := path
} else := result if {
	result := concat("", [path, "/"])
}

