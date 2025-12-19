# METADATA
# scope: package
# title: Protected Paths - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-PROTECTED-PATHS
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Edit", "Write", "MultiEdit", "NotebookEdit", "Bash"]
package cupcake.policies.builtins.protected_paths

import data.cupcake.system.commands
import data.cupcake.system.paths
import rego.v1

# Block WRITE operations on protected paths (but allow reads)
# For regular tools (Edit, Write, NotebookEdit)
halt contains decision if {
	input.hook_event_name == "PreToolUse"

	# Check for SINGLE-file writing tools only
	single_file_tools := {"Edit", "Write", "NotebookEdit"}
	input.tool_name in single_file_tools

	# Get the file path from tool input
	# TOB-4 fix: Use canonical path (always provided by Rust preprocessing)
	file_path := input.resolved_file_path
	file_path != null

	# Check if path matches any protected path
	is_protected_path(file_path)

	# Get configured message from signals
	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS",
		"reason": concat("", [message, " (", file_path, ")"]),
		"severity": "HIGH",
	}
}

# Block WRITE operations on protected paths - MultiEdit special handling
# MultiEdit has an array of edits, each with their own resolved_file_path
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "MultiEdit"

	# Check each edit in the edits array
	some edit in input.tool_input.edits
	file_path := edit.resolved_file_path
	file_path != null

	# Check if THIS edit's path matches any protected path
	is_protected_path(file_path)

	# Get configured message from signals
	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS",
		"reason": concat("", [message, " (", file_path, ")"]),
		"severity": "HIGH",
	}
}

# Block ALL Bash commands that reference protected paths UNLESS whitelisted
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	# Get the command
	command := input.tool_input.command
	lower_cmd := lower(command)

	# Check if any protected path is mentioned in the command
	some protected_path in get_protected_paths
	contains_protected_reference(lower_cmd, protected_path)

	# ONLY allow if it's a whitelisted read operation
	not is_whitelisted_read_command(lower_cmd)

	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS",
		"reason": concat("", [message, " (only read operations allowed)"]),
		"severity": "HIGH",
	}
}

# Block destructive commands that would affect a parent directory containing protected paths
# This catches cases like `rm -rf /home/user/*` when `/home/user/.cupcake/` is protected
# The `affected_parent_directories` field is populated by Rust preprocessing for destructive commands
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	# Get affected parent directories from preprocessing
	# This is populated for commands like rm -rf, chmod -R, etc.
	affected_dirs := input.affected_parent_directories
	count(affected_dirs) > 0

	# Check if any protected path is a CHILD of an affected directory
	some affected_dir in affected_dirs
	some protected_path in get_protected_paths
	protected_is_child_of_affected(protected_path, affected_dir)

	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS-PARENT",
		"reason": concat("", [message, " (", protected_path, " would be affected by operation on ", affected_dir, ")"]),
		"severity": "HIGH",
	}
}

# Block interpreter inline scripts (-c/-e flags) that mention protected paths
# This catches attacks like: python -c 'pathlib.Path("../my-favorite-file.txt").delete()'
halt contains decision if {
	input.hook_event_name == "PreToolUse"
	input.tool_name == "Bash"

	command := input.tool_input.command
	lower_cmd := lower(command)

	# Detect inline script execution with interpreters
	interpreters := ["python", "python3", "python2", "ruby", "perl", "node", "php"]
	some interp in interpreters
	regex.match(concat("", ["(^|\\s)", interp, "\\s+(-c|-e)\\s"]), lower_cmd)

	# Check if any protected path is mentioned anywhere in the command
	some protected_path in get_protected_paths
	contains(lower_cmd, lower(protected_path))

	message := get_configured_message

	decision := {
		"rule_id": "BUILTIN-PROTECTED-PATHS-SCRIPT",
		"reason": concat("", [message, " (inline script mentions '", protected_path, "')"]),
		"severity": "HIGH",
	}
}

# Extract file path from tool input
get_file_path_from_tool_input := path if {
	path := input.tool_input.file_path
} else := path if {
	path := input.tool_input.path
} else := path if {
	path := input.tool_input.notebook_path
} else := path if {
	# For MultiEdit, check if any edit targets a protected path
	# Return the first protected path found
	some edit in input.tool_input.edits
	path := edit.file_path
} else := ""

# Check if a path is protected
is_protected_path(path) if {
	protected_paths := get_protected_paths
	some protected_path in protected_paths
	path_matches(path, protected_path)
}

# Path matching logic (supports exact, directory prefix, filename, and glob patterns)
path_matches(path, pattern) if {
	# Exact match (case-insensitive)
	lower(path) == lower(pattern)
}

path_matches(path, pattern) if {
	# Filename match - pattern is just a filename (no path separators)
	# Matches if the canonical path ends with the filename
	not contains(pattern, "/")
	not contains(pattern, "\\")
	endswith(lower(path), concat("/", [lower(pattern)]))
}

path_matches(path, pattern) if {
	# Filename match for Windows paths
	not contains(pattern, "/")
	not contains(pattern, "\\")
	endswith(lower(path), concat("\\", [lower(pattern)]))
}

path_matches(path, pattern) if {
	# Directory prefix match - absolute pattern (starts with /)
	# Pattern: "/absolute/path/" matches "/absolute/path/file.txt"
	endswith(pattern, "/")
	startswith(pattern, "/")
	startswith(lower(path), lower(pattern))
}

path_matches(path, pattern) if {
	# Directory prefix match - relative pattern
	# Pattern: "src/legacy/" should match "/tmp/project/src/legacy/file.rs"
	# This handles canonical absolute paths against relative pattern configs
	endswith(pattern, "/")
	not startswith(pattern, "/")

	# Check if the pattern appears in the path as a directory component
	# We need to match "/src/legacy/" not just any "src/legacy/" substring
	contains(lower(path), concat("/", [lower(pattern)]))
}

path_matches(path, pattern) if {
	# Directory match without trailing slash - absolute pattern
	# If pattern is "/absolute/path/src/legacy", match "/absolute/path/src/legacy/file.js"
	not endswith(pattern, "/")
	startswith(pattern, "/")
	prefix := concat("", [lower(pattern), "/"])
	startswith(lower(path), prefix)
}

path_matches(path, pattern) if {
	# Directory match without trailing slash - relative pattern
	# If pattern is "src/legacy", match "/tmp/project/src/legacy/file.js"
	not endswith(pattern, "/")
	not startswith(pattern, "/")
	prefix := concat("/", [lower(pattern), "/"])
	contains(lower(path), prefix)
}

path_matches(path, pattern) if {
	# Glob pattern matching (simplified - just * wildcard for now)
	contains(pattern, "*")
	glob_match(lower(path), lower(pattern))
}

# Simple glob matching (supports * wildcard)
glob_match(path, pattern) if {
	# Convert glob pattern to regex: * becomes .*
	regex_pattern := replace(replace(pattern, ".", "\\."), "*", ".*")
	regex_pattern_anchored := concat("", ["^", regex_pattern, "$"])
	regex.match(regex_pattern_anchored, path)
}

# WHITELIST approach: Only these read operations are allowed on protected paths
is_whitelisted_read_command(cmd) if {
	# Exclude dangerous sed variants FIRST
	startswith(cmd, "sed -i") # In-place edit
	false # Explicitly reject
}

is_whitelisted_read_command(cmd) if {
	# Check if command starts with a safe read-only command
	safe_read_verbs := {
		"cat", # Read file contents
		"less", # Page through file
		"more", # Page through file
		"head", # Read first lines
		"tail", # Read last lines
		"grep", # Search in file
		"egrep", # Extended grep
		"fgrep", # Fixed string grep
		"zgrep", # Grep compressed files
		"wc", # Word/line count
		"file", # Determine file type
		"stat", # File statistics
		"ls", # List files
		"find", # Find files (read-only by default)
		"awk", # Text processing (without output redirect)
		"sed", # Stream editor (safe without -i flag)
		"sort", # Sort lines
		"uniq", # Filter unique lines
		"diff", # Compare files
		"cmp", # Compare files byte by byte
		"md5sum", # Calculate checksum
		"sha256sum", # Calculate checksum
		"hexdump", # Display in hex
		"strings", # Extract strings from binary
		"od", # Octal dump
	}

	some verb in safe_read_verbs
	commands.has_verb(cmd, verb)

	# CRITICAL: Exclude sed -i specifically
	# This check is NOT redundant with lines 188-192. OPA evaluates ALL rule bodies
	# for is_whitelisted_read_command(). Body 1 (lines 188-192) explicitly rejects "sed -i",
	# but OPA continues to evaluate Body 2 (this body). Without this check, "sed -i"
	# would match the "sed" verb above and incorrectly be whitelisted.
	# Whitespace variations (sed  -i, sed\t-i) are normalized by preprocessing.
	not startswith(cmd, "sed -i")

	# Ensure no output redirection
	not commands.has_output_redirect(cmd)
}

is_whitelisted_read_command(cmd) if {
	# Also allow piped commands that start with safe reads
	# e.g., "cat file.txt | grep pattern"
	contains(cmd, "|")
	parts := split(cmd, "|")
	first_part := trim_space(parts[0])

	# Check if first part starts with a safe command (avoid recursion)
	safe_read_verbs := {
		"cat", # Read file contents
		"less", # Page through file
		"more", # Page through file
		"head", # Read first lines
		"tail", # Read last lines
		"grep", # Search in file
		"wc", # Word/line count
		"file", # Determine file type
		"stat", # File statistics
		"ls", # List files
	}

	some verb in safe_read_verbs
	commands.has_verb(first_part, verb)
}

# Check if command references a protected path
contains_protected_reference(cmd, protected_path) if {
	# Direct reference
	contains(cmd, lower(protected_path))
}

contains_protected_reference(cmd, protected_path) if {
	# Without trailing slash if it's a directory pattern
	endswith(protected_path, "/")
	path_without_slash := substring(lower(protected_path), 0, count(protected_path) - 1)
	contains(cmd, path_without_slash)
}

# Get configured message from builtin config
get_configured_message := msg if {
	# Direct access to builtin config (no signal execution needed)
	msg := input.builtin_config.protected_paths.message
} else := msg if {
	# Fallback to default if config not present
	msg := "This path is read-only and cannot be modified"
}

# Get list of protected paths from builtin config
get_protected_paths := paths if {
	# Direct access to builtin config (no signal execution needed)
	paths := input.builtin_config.protected_paths.paths
} else := paths if {
	# No paths configured - policy inactive
	paths := []
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
