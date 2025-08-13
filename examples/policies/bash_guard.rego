package cupcake.policies.bash_guard

import rego.v1

# METADATA
# scope: rule
# title: Bash Security Guard Policy
# authors: ["Security Team"]
# custom:
#   severity: HIGH
#   id: BASH-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

# Decision Verbs - Modern Rego v1.0 format for NEW_GUIDING_FINAL.md

# Halt: Immediate cessation for catastrophic commands
halt contains decision if {
    # Trust routing - we know this is PreToolUse:Bash from metadata
    contains(input.tool_input.command, "rm -rf /")
    
    decision := {
        "reason": "EMERGENCY HALT: 'rm -rf /' command would destroy the entire filesystem",
        "severity": "CRITICAL",
        "rule_id": "BASH-001-HALT"
    }
}

# Deny: High-risk commands requiring explicit approval
deny contains decision if {
    # Trust routing - we know this is PreToolUse:Bash from metadata
    contains(input.tool_input.command, "sudo")
    not contains(input.tool_input.command, "sudo -l")  # Allow listing permissions
    
    decision := {
        "reason": "Sudo commands require explicit approval for security",
        "severity": "HIGH", 
        "rule_id": "BASH-001-SUDO"
    }
}

deny contains decision if {
    # Trust routing - we know this is PreToolUse:Bash from metadata
    regex.match(`\b(rm|rmdir)\s+.*(-r|--recursive).*(-f|--force)`, input.tool_input.command)
    not contains(input.tool_input.command, "rm -rf /")  # Caught by halt above
    
    decision := {
        "reason": "Recursive force deletion commands are high-risk operations",
        "severity": "HIGH",
        "rule_id": "BASH-001-FORCE-DELETE"
    }
}

# Ask: Potentially risky commands requiring user confirmation
ask contains decision if {
    # Trust routing - we know this is PreToolUse:Bash from metadata
    regex.match(`\b(wget|curl)\s+.*\|\s*(bash|sh)`, input.tool_input.command)
    
    decision := {
        "reason": "Downloading and executing scripts can be dangerous. Confirm this is safe?",
        "severity": "MEDIUM",
        "rule_id": "BASH-001-PIPE-EXEC"
    }
}

# Context: Helpful information for users
add_context contains "⚠️ Working in production environment - extra caution advised" if {
    # Trust routing - we know this is PreToolUse:Bash from metadata
    contains(input.cwd, "/prod")
}