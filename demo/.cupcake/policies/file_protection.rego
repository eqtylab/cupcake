package cupcake.policies.file_protection

import rego.v1

# METADATA
# scope: rule
# title: File Protection Policy
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["Write", "Edit"]

# Block any writes to .txt files
deny contains decision if {
    # Trust routing - we know this is PreToolUse:Write/Edit
    endswith(input.tool_input.file_path, ".txt")
    
    decision := {
        "reason": "Writing to .txt files is blocked by security policy",
        "severity": "HIGH",
        "rule_id": "FILE-001-TXT"
    }
}

# Block dangerous file operations
deny contains decision if {
    # Trust routing - we know this is PreToolUse:Write/Edit
    contains(input.tool_input.file_path, "/etc/")
    
    decision := {
        "reason": "Modifying system files in /etc/ is prohibited",
        "severity": "CRITICAL", 
        "rule_id": "FILE-001-SYSTEM"
    }
}