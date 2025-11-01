# METADATA
# scope: package
# title: Rulebook Security Guardrails - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-RULEBOOK-SECURITY
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Read", "Grep", "Glob", "Bash", "Task", "WebFetch"]
package cupcake.policies.builtins.rulebook_security_guardrails

import rego.v1

import data.cupcake.helpers.commands

# Block ANY tool operations targeting protected paths
halt contains decision if {
	input.hook_event_name == "PreToolUse"

	# Check for ANY file operation tools (read, write, search, etc.)
	file_operation_tools := {
		"Edit", "Write", "MultiEdit", "NotebookEdit", # Writing tools
		"Read", # Reading tools
		"Grep", "Glob", # Search/listing tools
		"WebFetch", # Could use file:// URLs
		"Task", # Could spawn agent to bypass
	}
	input.tool_name in file_operation_tools

	# Check if any parameter contains a protected path (case-insensitive)
	# TOB-4 fix: Prefer canonical path (input.resolved_file_path) when available,
	# but fall back to raw tool_input fields for pattern-based tools (Glob/Grep)
	# that don't have file paths that can be canonicalized
	file_path := get_file_path_with_preprocessing_fallback
	file_path != ""
	is_protected_path(file_path)

	# Get configured message from signals (fallback to default)
	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY",
		"reason": concat("", [message, " (blocked file operation on ", file_path, ")"]),
		"severity": "HIGH",
	}
}

# Block Bash commands that reference any protected path
# Total lockdown - NO whitelist (unlike protected_paths builtin)
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	# Check if command references any protected path
	# Bash tool uses tool_input.command, not params.command
	command := lower(input.tool_input.command)

	# Iterate over all protected paths
	some protected_path in get_protected_paths
	contains_protected_reference(command, protected_path)

	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY",
		"reason": concat("", [message, " (detected protected path reference in bash command)"]),
		"severity": "HIGH",
	}
}

# Block symlink creation involving any protected path (TOB-EQTY-LAB-CUPCAKE-4)
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	command := lower(input.tool_input.command)

	# Check if command creates symlink involving ANY protected path (source OR target)
	some protected_path in get_protected_paths
	commands.symlink_involves_path(command, protected_path)

	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-RULEBOOK-SECURITY",
		"reason": concat("", [message, " (symlink creation involving protected path is not permitted)"]),
		"severity": "HIGH",
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
	# "/full/path/.cupcake/file" matches ".cupcake"
	# "/full/path/secrets/api.key" matches "secrets/"
	lower_path := lower(path)
	lower_pattern := lower(pattern)
	contains(lower_path, lower_pattern)
}

path_matches(path, pattern) if {
	# Directory match without trailing slash
	# Pattern "secrets" should match "/full/path/secrets/file"
	not endswith(pattern, "/")
	lower_path := lower(path)
	lower_pattern := lower(pattern)
	# Add slash to ensure directory boundary
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
	# "secrets/" pattern should also match "secrets" in command
	endswith(protected_path, "/")
	path_without_slash := substring(lower(protected_path), 0, count(protected_path) - 1)
	contains(cmd, path_without_slash)
}

# Get configured message from builtin config
get_configured_message := msg if {
	# Direct access to builtin config (no signal execution needed)
	msg := input.builtin_config.rulebook_security_guardrails.message
} else := msg if {
	# Fallback to default if config not present
	msg := "Cupcake configuration files are protected from modification"
}

# Extract file path from tool input based on tool type
get_file_path_from_tool_input := path if {
	# Standard file_path parameter (Edit, Write, MultiEdit, NotebookEdit, Read)
	path := input.tool_input.file_path
} else := path if {
	# Path parameter (Grep, Glob)
	path := input.tool_input.path
} else := path if {
	# Pattern parameter might contain path (Glob)
	path := input.tool_input.pattern
} else := path if {
	# URL parameter for WebFetch (could be file:// URL)
	path := input.tool_input.url
} else := path if {
	# Task prompt might contain .cupcake references
	path := input.tool_input.prompt
} else := path if {
	# Notebook path for NotebookEdit
	path := input.tool_input.notebook_path
} else := path if {
	# Some tools use params instead of tool_input
	path := input.params.file_path
} else := path if {
	path := input.params.path
} else := path if {
	path := input.params.pattern
} else := ""

# TOB-4 aware path extraction: Prefer canonical path from preprocessing,
# fall back to raw tool_input for tools that can't be canonicalized (Glob/Grep patterns)
get_file_path_with_preprocessing_fallback := path if {
	# For Grep/Glob, always use raw paths since they work with patterns/directories
	input.tool_name in {"Grep", "Glob"}
	path := get_file_path_from_tool_input
} else := input.resolved_file_path if {
	# For other tools, use canonical path from Rust preprocessing (TOB-4 defense)
	input.resolved_file_path != ""
} else := path if {
	# Final fallback
	path := get_file_path_from_tool_input
}

# Helper: Get list of protected paths from builtin config
get_protected_paths := paths if {
	# Direct access to builtin config (no signal execution needed)
	paths := input.builtin_config.rulebook_security_guardrails.protected_paths
} else := paths if {
	# Default protected paths
	paths := [".cupcake/"]
}
