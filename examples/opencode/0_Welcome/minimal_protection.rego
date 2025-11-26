# METADATA
# scope: package
# title: Minimal Protection Policy
# description: A simple example policy that blocks dangerous git commands
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash"]
package cupcake.policies.opencode.minimal_protection

import rego.v1

# Block git commit with --no-verify flag
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    
    # Check if this is a git commit with --no-verify
    contains(command, "git commit")
    contains(command, "--no-verify")
    
    decision := {
        "rule_id": "GIT_NO_VERIFY",
        "reason": "The --no-verify flag bypasses pre-commit hooks and security checks. This is blocked by your organization's security policy.",
        "severity": "HIGH"
    }
}

# Block force push to main branch
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    
    # Check if this is a force push
    contains(command, "git push")
    contains(command, "--force")
    
    decision := {
        "rule_id": "GIT_FORCE_PUSH",
        "reason": "Force pushing is dangerous and can overwrite remote history. Use --force-with-lease if you must force push.",
        "severity": "HIGH"
    }
}

# Block rm -rf on system directories
deny contains decision if {
    input.tool_name == "Bash"
    command := input.tool_input.command
    
    # Check for dangerous rm -rf patterns
    contains(command, "rm -rf")
    
    # Check for system directories
    dangerous_paths := ["/", "/usr", "/etc", "/var", "/home", "/root"]
    some path in dangerous_paths
    contains(command, path)
    
    decision := {
        "rule_id": "DANGEROUS_RM",
        "reason": concat("", ["Attempted to run 'rm -rf' on system directory: ", path, ". This is extremely dangerous and is blocked."]),
        "severity": "CRITICAL"
    }
}
