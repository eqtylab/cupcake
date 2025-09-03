# METADATA
# scope: package
# title: MCP Memory Security Guard
# authors: ["Security Team"]
# custom:
#   severity: HIGH
#   id: MCP-MEMORY-001
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["mcp__memory__create_entities"]
package cupcake.policies.mcp.memory_guard

import rego.v1

# Deny storing sensitive information in MCP memory tools
deny contains decision if {
    # Trust routing - we know this is an MCP memory tool
    sensitive_patterns := ["password", "secret", "key", "token", "api_key", "credential"]
    
    input_content := sprintf("%v", [input.tool_input])
    
    some pattern in sensitive_patterns
    contains(lower(input_content), pattern)
    
    decision := {
        "reason": sprintf("Cannot store sensitive information (%s) in memory system", [pattern]),
        "severity": "HIGH",
        "rule_id": "MCP-MEMORY-001"
    }
}

# Ask for confirmation on bulk memory operations
ask contains decision if {
    # Trust routing - we know this is an MCP memory tool
    contains(input.tool_name, "bulk")
    
    decision := {
        "reason": "Bulk memory operations can affect large amounts of data. Confirm?",
        "severity": "MEDIUM",
        "rule_id": "MCP-MEMORY-002"
    }
}

ask contains decision if {
    # Trust routing - we know this is an MCP memory tool
    contains(input.tool_name, "batch")
    
    decision := {
        "reason": "Batch memory operations can affect large amounts of data. Confirm?",
        "severity": "MEDIUM",
        "rule_id": "MCP-MEMORY-003"
    }
}

# Add context about memory usage
add_context contains "💾 Working with MCP memory system - data persists across sessions" if {
    # Trust routing - we know this is an MCP memory tool
    true
}