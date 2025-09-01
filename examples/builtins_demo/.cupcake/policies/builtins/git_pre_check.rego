package cupcake.policies.builtins.git_pre_check

import rego.v1

# METADATA
# scope: rule
# title: Git Pre-Check - Builtin Policy
# authors: ["Cupcake Builtins"]
# custom:
#   severity: HIGH
#   id: BUILTIN-GIT-CHECK
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

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
    # In production, this would:
    # 1. Execute signals like __builtin_git_check_0, __builtin_git_check_1, etc.
    # 2. Check exit codes and output
    # 3. Return success/failure for each
    
    # For demonstration, simulate some checks
    results := [
        {
            "name": "cargo test",
            "success": true,
            "message": "All tests pass"
        },
        {
            "name": "cargo fmt --check",
            "success": false,
            "message": "Code must be formatted (run 'cargo fmt')"
        }
    ]
    
    # In real implementation:
    # check_signals := [name |
    #     some name, _ in data.signals
    #     startswith(name, "__builtin_git_check_")
    # ]
    # 
    # results := [result |
    #     some signal_name in check_signals
    #     signal_result := data.signals[signal_name]
    #     result := evaluate_check(signal_name, signal_result)
    # ]
}

# Evaluate a check result
evaluate_check(name, result) := check if {
    # Check would examine exit code, stdout, stderr
    # For now, just check if result exists
    result != null
    check := {
        "name": name,
        "success": result.exit_code == 0,
        "message": result.message
    }
}