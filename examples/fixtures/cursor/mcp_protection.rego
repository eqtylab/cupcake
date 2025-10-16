# METADATA
# scope: package
# title: Cursor MCP Tool Protection
# custom:
#   routing:
#     required_events: ["beforeMCPExecution"]
package cupcake.policies.cursor.mcp_protection

import rego.v1

# Block dangerous database operations
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    startswith(input.tool_name, "postgres")
    dangerous_ops := ["DELETE", "DROP", "TRUNCATE"]
    some op in dangerous_ops
    contains(upper(input.tool_input), op)

    decision := {
        "rule_id": "CURSOR-MCP-001",
        "reason": concat(" ", ["Dangerous database operation blocked:", op]),
        "agent_context": concat("", [
            op, " operation detected in SQL. ",
            "Destructive database operations are not allowed. ",
            "Alternatives: 1) Use SELECT to query data, ",
            "2) Use UPDATE to modify specific records, ",
            "3) Create backups before destructive operations."
        ]),
        "severity": "CRITICAL"
    }
}

# Ask for confirmation on data modifications
ask contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    contains(upper(input.tool_input), "UPDATE")

    decision := {
        "rule_id": "CURSOR-MCP-002",
        "reason": "Database update requires confirmation",
        "question": "Allow database update operation?",
        "severity": "MEDIUM"
    }
}
