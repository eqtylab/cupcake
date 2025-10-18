# METADATA
# scope: package
# title: Git Pre-Check - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-GIT-CHECK
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.builtins.git_pre_check

import rego.v1

# Check git operations and run validation before allowing
halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    
    # Check if this is a git operation that needs validation
    command := lower(input.params.command)
    is_git_operation(command)
    
    # Run all configured checks
    check_results := run_all_checks
    
    # Find any failed checks
    failed_checks := [check | 
        some check in check_results
        not check.success
    ]
    
    # If any checks failed, halt the operation
    count(failed_checks) > 0
    
    # Build failure message
    failure_messages := [msg |
        some check in failed_checks
        msg := concat("", ["- ", check.message])
    ]
    
    failure_list := concat("\n", failure_messages)
    reason := concat("\n", ["Git pre-checks failed:", failure_list])
    
    decision := {
        "rule_id": "BUILTIN-GIT-CHECK",
        "reason": reason,
        "severity": "HIGH"
    }
}

# Check if command is a git operation that needs validation
is_git_operation(cmd) if {
    git_patterns := {
        "git commit",
        "git push",
        "git merge"
    }
    
    some pattern in git_patterns
    contains(cmd, pattern)
}

# Run all configured pre-checks
run_all_checks := results if {
    # Collect all git check signals
    check_signals := [name |
        some name, _ in input.signals
        startswith(name, "__builtin_git_check_")
    ]
    
    # Evaluate each check
    results := [result |
        some signal_name in check_signals
        signal_result := input.signals[signal_name]
        result := evaluate_check(signal_name, signal_result)
    ]
    
    # Return results if we have any
    count(results) > 0
} else := [] if {
    # No checks configured
    true
}

# Evaluate a check result
evaluate_check(name, result) := check if {
    # Parse the signal result which should contain exit_code and output
    is_object(result)
    check := {
        "name": clean_signal_name(name),
        "success": result.exit_code == 0,
        "message": default_message(result)
    }
} else := check if {
    # Handle string results (command output)
    is_string(result)
    check := {
        "name": clean_signal_name(name),
        "success": true,  # Assume success if we got output
        "message": result
    }
}

# Extract readable name from signal name
clean_signal_name(signal_name) := name if {
    # Remove __builtin_git_check_ prefix and return the index
    parts := split(signal_name, "__builtin_git_check_")
    count(parts) > 1
    name := concat("Check ", [parts[1]])
} else := signal_name

# Get appropriate message from result
default_message(result) := msg if {
    result.output != ""
    msg := result.output
} else := msg if {
    result.exit_code == 0
    msg := "Check passed"
} else := msg if {
    msg := concat("", ["Check failed with exit code ", format_int(result.exit_code, 10)])
}