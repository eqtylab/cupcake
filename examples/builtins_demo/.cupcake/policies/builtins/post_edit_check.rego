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
    # In production, this would:
    # 1. Check if there's a configured validation for this extension
    # 2. Execute signal like __builtin_post_edit_rs, __builtin_post_edit_py, etc.
    # 3. Return the validation result
    
    # For demonstration, provide extension-specific feedback
    ext == "rs"
    result := {
        "success": false,
        "message": "Rust compilation error: expected `;` at line 42"
    }
} else := result if {
    ext == "py"
    result := {
        "success": true,
        "message": "Python syntax valid"
    }
} else := result if {
    ext == "tsx"
    result := {
        "success": false,
        "message": "TypeScript error: Property 'name' does not exist on type 'User'"
    }
} else := result if {
    ext == "go"
    result := {
        "success": true,
        "message": "Go format and vet passed"
    }
} else := result if {
    # No validation configured for this extension
    result := {
        "success": true,
        "message": "No validation configured"
    }
}

# In real implementation, would execute validation command:
# execute_validation(ext, file_path) := result if {
#     signal_name := concat("", ["__builtin_post_edit_", ext])
#     signal_name in data.signals
#     
#     signal_result := data.signals[signal_name]
#     result := {
#         "success": signal_result.exit_code == 0,
#         "message": signal_result.output
#     }
# }