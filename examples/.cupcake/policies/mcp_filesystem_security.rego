# METADATA
# scope: package
# title: MCP Filesystem Security Policy
# authors: ["Security Team"]
# custom:
#   severity: HIGH
#   id: MCP-FS-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["mcp__filesystem__.*"]
package cupcake.policies.mcp.filesystem_security

import rego.v1

# Block access to sensitive system directories
deny contains decision if {
    # Trust routing - we know this is an MCP filesystem tool
    file_path := input.tool_input.path
    
    sensitive_paths := [
        "/etc/passwd", "/etc/shadow", "/etc/hosts",
        "/root/", "/var/log/", "/proc/", "/sys/",
        "/.env", "/.aws/", "/.ssh/"
    ]
    
    some path in sensitive_paths
    startswith(file_path, path)
    
    decision := {
        "reason": sprintf("Access to sensitive system path '%s' is blocked", [file_path]),
        "severity": "HIGH",
        "rule_id": "MCP-FS-001"
    }
}

# Require confirmation for write operations outside project directory
ask contains decision if {
    # Trust routing - we know this is an MCP filesystem tool
    contains(input.tool_name, "write")
    
    file_path := input.tool_input.path
    not startswith(file_path, input.cwd)
    
    decision := {
        "reason": sprintf("File write outside project directory (%s). Continue?", [file_path]),
        "severity": "MEDIUM", 
        "rule_id": "MCP-FS-002"
    }
}

ask contains decision if {
    # Trust routing - we know this is an MCP filesystem tool
    contains(input.tool_name, "create")
    
    file_path := input.tool_input.path
    not startswith(file_path, input.cwd)
    
    decision := {
        "reason": sprintf("File creation outside project directory (%s). Continue?", [file_path]),
        "severity": "MEDIUM", 
        "rule_id": "MCP-FS-003"
    }
}

# Add helpful context for filesystem operations
add_context contains sprintf("📁 MCP Filesystem: %s on %s", [action, input.tool_input.path]) if {
    # Trust routing - we know this is an MCP filesystem tool
    action := regex.replace(input.tool_name, "mcp__filesystem__", "")
}