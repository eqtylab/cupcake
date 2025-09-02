package cupcake.policies.builtins.post_edit_check

import rego.v1

# METADATA
# scope: rule
# title: Post Edit Check - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: MEDIUM
#   id: BUILTIN-POST-EDIT
#   routing:
#     required_events: ["PostToolUse"]

# Run validation after file edits
ask contains decision if {
    input.hook_event_name == "PostToolUse"
    
    # Check if this was a file editing operation
    editing_tools := {"Edit", "Write", "MultiEdit", "NotebookEdit"}
    input.tool_name in editing_tools
    
    # Get the file that was edited
    file_path := get_edited_file_path
    file_path != ""
    
    # Get file extension
    extension := get_file_extension(file_path)
    extension != ""
    
    # Run validation for this file type
    validation_result := run_validation_for_extension(extension, file_path)
    
    # If validation failed, ask for user confirmation
    not validation_result.success
    
    question := concat("\n", [
        concat(" ", ["File validation failed for", file_path]),
        validation_result.message,
        "",
        "Do you want to continue anyway?"
    ])
    
    decision := {
        "rule_id": "BUILTIN-POST-EDIT",
        "question": question,
        "severity": "MEDIUM"
    }
}

# Also provide feedback as context when validation succeeds
add_context contains decision if {
    input.hook_event_name == "PostToolUse"
    
    editing_tools := {"Edit", "Write", "MultiEdit", "NotebookEdit"}
    input.tool_name in editing_tools
    
    file_path := get_edited_file_path
    file_path != ""
    
    extension := get_file_extension(file_path)
    extension != ""
    
    validation_result := run_validation_for_extension(extension, file_path)
    
    # If validation succeeded, provide positive feedback
    validation_result.success
    
    decision := {
        "rule_id": "BUILTIN-POST-EDIT",
        "context": concat(" ", ["✓ Validation passed for", file_path]),
        "severity": "LOW"
    }
}

# Extract file path from tool response/params
get_edited_file_path := path if {
    path := input.params.file_path
} else := path if {
    path := input.params.path
} else := ""

# Get file extension from path
get_file_extension(path) := ext if {
    parts := split(path, ".")
    count(parts) > 1
    ext := parts[count(parts) - 1]
} else := ""

# Run validation for a specific file extension
run_validation_for_extension(ext, file_path) := result if {
    # Check if there's a configured validation signal for this extension
    signal_name := concat("", ["__builtin_post_edit_", ext])
    signal_name in input.signals
    
    # Get the validation result from the signal
    signal_result := input.signals[signal_name]
    
    # Parse the result based on its type
    result := parse_validation_result(signal_result, file_path)
} else := result if {
    # No validation configured for this extension
    result := {
        "success": true,
        "message": "No validation configured"
    }
}

# Parse validation result from signal
parse_validation_result(signal_result, file_path) := result if {
    # Handle object results with exit_code and output
    is_object(signal_result)
    result := {
        "success": signal_result.exit_code == 0,
        "message": default_validation_message(signal_result, file_path)
    }
} else := result if {
    # Handle string results (assume success if we got output)
    is_string(signal_result)
    result := {
        "success": true,
        "message": signal_result
    }
}

# Generate appropriate validation message
default_validation_message(signal_result, file_path) := msg if {
    signal_result.output != ""
    msg := signal_result.output
} else := msg if {
    signal_result.exit_code == 0
    msg := "Validation passed"
} else := msg if {
    msg := concat("", ["Validation failed with exit code ", format_int(signal_result.exit_code, 10)])
}