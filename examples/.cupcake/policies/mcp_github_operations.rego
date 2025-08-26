package cupcake.policies.mcp.github_operations

import rego.v1

# METADATA
# scope: rule
# title: MCP GitHub Operations Policy
# authors: ["DevOps Team"]
# custom:
#   severity: MEDIUM
#   id: MCP-GH-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["mcp__github__.*"]

# Block destructive operations on main branch
deny contains decision if {
    # Trust routing - we know this is an MCP GitHub tool
    destructive_actions := ["delete", "force_push", "reset"]
    
    some action in destructive_actions
    contains(input.tool_name, action)
    
    # Check if targeting main branch
    branch := input.tool_input.branch
    branch in ["main", "master", "production"]
    
    decision := {
        "reason": sprintf("Destructive GitHub operation '%s' blocked on protected branch '%s'", [action, branch]),
        "severity": "HIGH",
        "rule_id": "MCP-GH-001"
    }
}

# Require confirmation for public repository operations
ask contains decision if {
    # Trust routing - we know this is an MCP GitHub tool
    input.tool_input.visibility == "public"
    contains(input.tool_name, "create")
    
    decision := {
        "reason": "Creating public repository. This will be visible to everyone. Continue?",
        "severity": "MEDIUM",
        "rule_id": "MCP-GH-002"
    }
}

# Add context about GitHub operations
add_context contains sprintf("🐙 GitHub %s: %s", [operation, details]) if {
    # Trust routing - we know this is an MCP GitHub tool
    operation := regex.replace(input.tool_name, "mcp__github__", "")
    repo := object.get(input.tool_input, "repository", "unknown")
    details := sprintf("repo=%s", [repo])
}