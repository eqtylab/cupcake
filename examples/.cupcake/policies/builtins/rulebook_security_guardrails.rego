# METADATA
# scope: package
# title: Rulebook Security Guardrails - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-RULEBOOK-SECURITY
#   routing:
#     required_events: ["PreToolUse"]
package cupcake.policies.builtins.rulebook_security_guardrails

import rego.v1

# Block ANY tool operations targeting .cupcake/ directory
halt contains decision if {
    input.hook_event_name == "PreToolUse"
    
    # Check for ANY file operation tools (read, write, search, etc.)
    file_operation_tools := {
        "Edit", "Write", "MultiEdit", "NotebookEdit",  # Writing tools
        "Read",                                         # Reading tools
        "Grep", "Glob",                                 # Search/listing tools  
        "WebFetch",                                     # Could use file:// URLs
        "Task"                                          # Could spawn agent to bypass
    }
    input.tool_name in file_operation_tools
    
    # Check if any parameter contains .cupcake/ (case-insensitive)
    file_path := get_file_path_from_tool_input
    file_path != ""
    targets_cupcake_directory(file_path)
    
    # Get configured message from signals (fallback to default)
    message := get_configured_message
    
    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY",
        "reason": concat("", [message, " (blocked file operation on ", file_path, ")"]),
        "severity": "HIGH"
    }
}

# Block Bash commands that could modify .cupcake/ directory
halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    
    # Check if command contains patterns that could modify .cupcake/
    # Bash tool uses tool_input.command, not params.command
    command := lower(input.tool_input.command)
    contains_cupcake_modification_pattern(command)
    
    message := get_configured_message
    
    decision := {
        "rule_id": "BUILTIN-RULEBOOK-SECURITY",
        "reason": concat("", [message, " (detected .cupcake/ modification in bash command)"]),
        "severity": "HIGH"
    }
}

# Check if file path targets .cupcake directory (handles various path formats)
targets_cupcake_directory(file_path) if {
    # Direct .cupcake/ reference (case-insensitive)
    lower_path := lower(file_path)
    contains(lower_path, ".cupcake/")
}

targets_cupcake_directory(file_path) if {
    # Relative path obfuscation: ./././.cupcake/ (case-insensitive)
    lower_path := lower(file_path)
    regex.match(`\.+/+\.cupcake/?`, lower_path)
}

targets_cupcake_directory(file_path) if {
    # Absolute paths ending in .cupcake/ (case-insensitive)
    lower_path := lower(file_path)
    endswith(lower_path, "/.cupcake")
}

targets_cupcake_directory(file_path) if {
    # Handle path normalization cases (case-insensitive)
    lower_path := lower(file_path)
    normalized := regex.replace(lower_path, `/{2,}`, "/")  # Replace multiple slashes
    normalized_clean := regex.replace(normalized, `/\./`, "/")  # Remove /./ segments
    contains(normalized_clean, ".cupcake")
}

# Detect Bash command patterns that could access .cupcake/
# SIMPLE RULE: Block ANY command that mentions .cupcake (case-insensitive)
contains_cupcake_modification_pattern(cmd) if {
    contains(cmd, ".cupcake")
}

# The following rules are kept for documentation and specific pattern detection
# They're redundant with the blanket rule above but show attack patterns we defend against

contains_cupcake_modification_pattern(cmd) if {
    # Dangerous modification operations
    contains(cmd, ".cupcake")
    
    dangerous_patterns := {
        "rm ",           # Remove files/directories
        "rmdir ",        # Remove directories  
        "mv ",           # Move (could overwrite)
        "cp ",           # Copy (could overwrite)
        " > ",           # Redirect output (write)
        " >> ",          # Append output
        "tee ",          # Write to file
        "sed -i",        # In-place edit
        "perl -i",       # In-place edit
        "tar ",          # Archive operations
        "zip ",          # Archive operations
        "unzip ",        # Extraction
        "chmod ",        # Change permissions
        "chown ",        # Change ownership
        "touch ",        # Create/modify files
        "truncate ",     # Truncate files
        "dd ",           # Data duplicator
        "rsync ",        # File synchronization
    }
    
    some pattern in dangerous_patterns
    contains(cmd, pattern)
}

contains_cupcake_modification_pattern(cmd) if {
    # Complex regex patterns that need separate handling
    contains(cmd, ".cupcake")
    
    # Regex-based pattern detection for complex commands
    regex_patterns := [
        `awk.*>`,        # Awk with output redirect
        `find.*-delete`, # Find with delete flag
        `find.*-exec`,   # Find with exec flag
    ]
    
    some pattern in regex_patterns
    regex.match(pattern, cmd)
}

contains_cupcake_modification_pattern(cmd) if {
    # Script execution patterns that might target .cupcake/
    contains(cmd, ".cupcake")
    
    script_patterns := {
        "python ",       # Python scripts
        "python3 ",      # Python 3 scripts
        "ruby ",         # Ruby scripts
        "perl ",         # Perl scripts
        "node ",         # Node.js scripts
        "sh ",           # Shell scripts
        "bash ",         # Bash scripts
        "/bin/sh",       # Direct shell execution
        "/bin/bash",     # Direct bash execution
    }
    
    some pattern in script_patterns
    contains(cmd, pattern)
}

contains_cupcake_modification_pattern(cmd) if {
    # Command substitution or variable expansion targeting .cupcake/
    contains(cmd, ".cupcake")
    
    expansion_patterns := {
        "$(echo",        # Command substitution
        "`echo",         # Backtick expansion
        "${",            # Variable expansion
        "$(",            # Command substitution
        "env ",          # Environment variable access
        "printenv ",     # Print environment
        "eval ",         # Dynamic evaluation
    }
    
    some pattern in expansion_patterns
    contains(cmd, pattern)
}

# Get configured message (would come from signal in real implementation)
get_configured_message := msg if {
    # Check for signal from Rust configuration
    msg_signal := input.signals["__builtin_rulebook_protected_message"]
    is_string(msg_signal)
    msg := msg_signal
} else := msg if {
    # Check if signal has structured format with message
    msg_signal := input.signals["__builtin_rulebook_protected_message"]
    is_object(msg_signal)
    msg_signal.output != ""
    msg := msg_signal.output
} else := msg if {
    # Default message if no signal configured
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

# Helper: Get list of protected paths from signals (future enhancement)
get_protected_paths := paths if {
    paths_signal := input.signals["__builtin_rulebook_protected_paths"]
    is_array(paths_signal)
    paths := paths_signal
} else := paths if {
    # Parse JSON array from signal output
    paths_signal := input.signals["__builtin_rulebook_protected_paths"]
    is_object(paths_signal)
    paths_signal.output != ""
    paths := json.unmarshal(paths_signal.output)
} else := paths if {
    # Default protected paths
    paths := [".cupcake/"]
}