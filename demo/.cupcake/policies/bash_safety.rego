package cupcake.policies.bash_safety

import rego.v1

# METADATA
# scope: rule  
# title: Bash Safety Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]

# Halt catastrophic commands
halt contains decision if {
    # Trust routing - we know this is PreToolUse:Bash
    contains(input.tool_input.command, "rm -rf /")
    
    decision := {
        "reason": "EMERGENCY HALT: This command would destroy the entire filesystem!",
        "severity": "CRITICAL",
        "rule_id": "BASH-001-DESTROY"
    }
}

# Deny dangerous deletions
deny contains decision if {
    # Trust routing - we know this is PreToolUse:Bash
    regex.match(`\brm\s+.*\*`, input.tool_input.command)
    
    decision := {
        "reason": "Wildcard deletions are dangerous and blocked",
        "severity": "HIGH",
        "rule_id": "BASH-001-WILDCARD" 
    }
}

# Ask for confirmation on sudo
ask contains decision if {
    # Trust routing - we know this is PreToolUse:Bash
    startswith(trim_space(input.tool_input.command), "sudo")
    
    decision := {
        "reason": "Sudo command requires confirmation for security",
        "severity": "MEDIUM",
        "rule_id": "BASH-001-SUDO"
    }
}