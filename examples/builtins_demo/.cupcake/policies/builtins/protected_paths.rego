# METADATA
# scope: package
# title: Protected Paths - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-PROTECTED-PATHS
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.policies.builtins.protected_paths

import rego.v1

# Block WRITE operations on protected paths (but allow reads)
halt contains decision if {
    input.hook_event_name == "PreToolUse"
    
    # Check for file WRITING tools only (not Read, Grep, Glob)
    write_tools := {"Edit", "Write", "MultiEdit", "NotebookEdit"}
    input.tool_name in write_tools
    
    # Get the file path from tool input
    file_path := get_file_path_from_tool_input
    file_path != ""
    
    # Check if path matches any protected path
    is_protected_path(file_path)
    
    # Get configured message from signals
    message := get_configured_message
    
    decision := {
        "rule_id": "BUILTIN-PROTECTED-PATHS",
        "reason": concat("", [message, " (", file_path, ")"]),
        "severity": "HIGH"
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
        "severity": "HIGH"
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

# Path matching logic (supports exact, directory prefix, and glob patterns)
path_matches(path, pattern) if {
    # Exact match (case-insensitive)
    lower(path) == lower(pattern)
}

path_matches(path, pattern) if {
    # Directory prefix match - pattern ends with / means "anything inside"
    endswith(pattern, "/")
    startswith(lower(path), lower(pattern))
}

path_matches(path, pattern) if {
    # Directory match without trailing slash
    # If pattern is "src/legacy", match "src/legacy/file.js"
    not endswith(pattern, "/")
    prefix := concat("", [lower(pattern), "/"])
    startswith(lower(path), prefix)
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
    # Check if command starts with a safe read-only command
    safe_read_commands := {
        "cat ",         # Read file contents
        "less ",        # Page through file
        "more ",        # Page through file
        "head ",        # Read first lines
        "tail ",        # Read last lines
        "grep ",        # Search in file
        "egrep ",       # Extended grep
        "fgrep ",       # Fixed string grep
        "zgrep ",       # Grep compressed files
        "wc ",          # Word/line count
        "file ",        # Determine file type
        "stat ",        # File statistics
        "ls ",          # List files
        "find ",        # Find files (read-only by default)
        "awk ",         # Text processing (without output redirect)
        "sed ",         # Stream editor (without -i flag)
        "sort ",        # Sort lines
        "uniq ",        # Filter unique lines
        "diff ",        # Compare files
        "cmp ",         # Compare files byte by byte
        "md5sum ",      # Calculate checksum
        "sha256sum ",   # Calculate checksum
        "hexdump ",     # Display in hex
        "strings ",     # Extract strings from binary
        "od ",          # Octal dump
    }
    
    some pattern in safe_read_commands
    startswith(cmd, pattern)
}

is_whitelisted_read_command(cmd) if {
    # Also allow piped commands that start with safe reads
    # e.g., "cat file.txt | grep pattern"
    contains(cmd, "|")
    parts := split(cmd, "|")
    first_part := trim_space(parts[0])
    
    # Check if first part starts with a safe command (avoid recursion)
    safe_read_commands := {
        "cat ",         # Read file contents
        "less ",        # Page through file
        "more ",        # Page through file
        "head ",        # Read first lines
        "tail ",        # Read last lines
        "grep ",        # Search in file
        "wc ",          # Word/line count
        "file ",        # Determine file type
        "stat ",        # File statistics
        "ls ",          # List files
    }
    
    some pattern in safe_read_commands
    startswith(first_part, pattern)
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

# Get configured message from signals
get_configured_message := msg if {
    msg_signal := input.signals["__builtin_protected_paths_message"]
    is_string(msg_signal)
    msg := msg_signal
} else := msg if {
    msg_signal := input.signals["__builtin_protected_paths_message"]
    is_object(msg_signal)
    msg_signal.output != ""
    msg := msg_signal.output
} else := msg if {
    msg := "This path is read-only and cannot be modified"
}

# Get list of protected paths from signals
get_protected_paths := paths if {
    paths_signal := input.signals["__builtin_protected_paths_list"]
    is_string(paths_signal)
    # Try to parse as JSON array
    paths := json.unmarshal(paths_signal)
} else := paths if {
    paths_signal := input.signals["__builtin_protected_paths_list"]
    is_object(paths_signal)
    paths_signal.output != ""
    # Parse the output as JSON array
    paths := json.unmarshal(paths_signal.output)
} else := paths if {
    # No paths configured - policy inactive
    paths := []
}