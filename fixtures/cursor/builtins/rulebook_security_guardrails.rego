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

import data.cupcake.helpers.commands

# Block reading protected path files
halt contains decision if {
	input.hook_event_name == "beforeReadFile"

	# Get the file path from Cursor's raw schema
	# TOB-4 fix: Use canonical path (always provided by Rust preprocessing)
	file_path := input.resolved_file_path

	# Check if file matches any protected path
	is_protected_path(file_path)

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
		"reason": "Access to protected directories is prohibited. These directories contain security-critical data.",
		"severity": "CRITICAL",
	}
}

# Block modifications to protected path files
deny contains decision if {
	input.hook_event_name == "afterFileEdit"

	# Get the file path from Cursor's raw schema
	# TOB-4 fix: Use canonical path (always provided by Rust preprocessing)
	file_path := input.resolved_file_path

	# Check if file matches any protected path
	is_protected_path(file_path)

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
		"reason": "Modifications to protected directories are not permitted. These directories contain security-critical data.",
		"severity": "CRITICAL",
	}
}

# Block ANY shell commands that reference protected paths (total lockdown)
deny contains decision if {
	input.hook_event_name == "beforeShellExecution"

	# Get the command from Cursor's raw schema
	cmd := lower(input.command)

	# Check if command references any protected path
	some protected_path in get_protected_paths
	contains_protected_reference(cmd, protected_path)

	# NO dangerous verb check - block ALL commands referencing protected paths

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
		"reason": "Shell commands referencing protected directories are not permitted. These directories contain security-critical data.",
		"severity": "CRITICAL",
	}
}

# Block symlink creation involving any protected path (TOB-EQTY-LAB-CUPCAKE-4)
deny contains decision if {
	input.hook_event_name == "beforeShellExecution"

	command := lower(input.command)

	# Check if command creates symlink involving ANY protected path (source OR target)
	some protected_path in get_protected_paths
	commands.symlink_involves_path(command, protected_path)

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY-GUARDRAILS",
		"reason": "Symlink creation involving protected directories is not permitted. These directories contain security-critical data.",
		"severity": "CRITICAL",
	}
}

# Check if a file path matches any protected path
is_protected_path(path) if {
	protected_paths := get_protected_paths
	some protected_path in protected_paths
	path_matches(path, protected_path)
}

# Path matching logic (supports substring and directory matching)
path_matches(path, pattern) if {
	# Exact match (case-insensitive)
	lower(path) == lower(pattern)
}

path_matches(path, pattern) if {
	# Substring match - handles both file and directory references
	lower_path := lower(path)
	lower_pattern := lower(pattern)
	contains(lower_path, lower_pattern)
}

path_matches(path, pattern) if {
	# Directory match without trailing slash
	not endswith(pattern, "/")
	lower_path := lower(path)
	lower_pattern := lower(pattern)
	pattern_with_slash := concat("", [lower_pattern, "/"])
	contains(lower_path, pattern_with_slash)
}

# Check if command references a protected path
contains_protected_reference(cmd, protected_path) if {
	# Direct reference (case-insensitive)
	contains(cmd, lower(protected_path))
}

contains_protected_reference(cmd, protected_path) if {
	# Without trailing slash if it's a directory pattern
	endswith(protected_path, "/")
	path_without_slash := substring(lower(protected_path), 0, count(protected_path) - 1)
	contains(cmd, path_without_slash)
}

# Get list of protected paths from builtin config
get_protected_paths := paths if {
	# Direct access to builtin config
	paths := input.builtin_config.rulebook_security_guardrails.protected_paths
} else := paths if {
	# Default protected paths
	paths := [".cupcake/"]
}

