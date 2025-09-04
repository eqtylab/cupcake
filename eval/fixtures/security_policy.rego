# METADATA
# scope: package
# title: Basic Security Policy
# description: Demonstrates blocking dangerous commands and file operations
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Bash", "Edit"]
package cupcake.policies.security

import rego.v1

# Block dangerous commands
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    dangerous_commands := {"rm -rf", "sudo rm", "format", "fdisk", "> /dev/"}
    some cmd in dangerous_commands
    contains(input.tool_input.command, cmd)
    decision := {
        "rule_id": "SECURITY-001",
        "reason": concat(" ", ["Dangerous command blocked:", cmd]),
        "severity": "CRITICAL"
    }
}

# Block directory removal commands on temp directories
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    
    # Check for rmdir or rm -r commands
    cmd := lower(input.tool_input.command)
    directory_removal := contains(cmd, "rmdir") or (contains(cmd, "rm ") and contains(cmd, "/tmp/"))
    directory_removal
    
    # Block if targeting /tmp directories
    contains(cmd, "/tmp/")
    
    decision := {
        "rule_id": "SECURITY-003",
        "reason": "Directory removal in /tmp is not permitted - please use cleanup scripts",
        "severity": "HIGH"
    }
}

# Block editing system files
deny contains decision if {
    input.hook_event_name == "PreToolUse"  
    input.tool_name == "Edit"
    protected_paths := {"/etc/", "/System/", "~/.ssh/"}
    some path in protected_paths
    startswith(input.tool_input.file_path, path)
    decision := {
        "rule_id": "SECURITY-002",
        "reason": concat(" ", ["System file modification blocked:", input.tool_input.file_path]),
        "severity": "HIGH"
    }
}