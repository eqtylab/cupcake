# METADATA
# scope: package
# title: Appointment Database Protection Policy
# description: Prevents deletion of appointment data and cancellation within 24 hours
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["mcp__postgres__execute_sql"]
package cupcake.policies.appointments

import rego.v1

# Block all DELETE operations on the database
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    # Check if the SQL command contains DELETE
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "delete")
    
    decision := {
        "rule_id": "APPT-001",
        "reason": "Database deletion operations are not permitted",
        "severity": "CRITICAL"
    }
}

# Block appointment cancellations within 24 hours (cancelled spelling)
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    # Check if this is an UPDATE to cancel an appointment
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "update")
    contains(sql_command, "appointments")
    contains(sql_command, "cancelled")
    
    # Check if the appointment is within 24 hours
    # This is a simplified check - in production you'd parse the SQL more thoroughly
    contains(sql_command, "where")
    
    decision := {
        "rule_id": "APPT-002", 
        "reason": "Cannot cancel appointments within 24 hours of scheduled time. Please contact the patient directly.",
        "severity": "HIGH"
    }
}

# Block appointment cancellations within 24 hours (canceled spelling)
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    # Check if this is an UPDATE to cancel an appointment
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "update")
    contains(sql_command, "appointments")
    contains(sql_command, "canceled")
    
    # Check if the appointment is within 24 hours
    # This is a simplified check - in production you'd parse the SQL more thoroughly
    contains(sql_command, "where")
    
    decision := {
        "rule_id": "APPT-002", 
        "reason": "Cannot cancel appointments within 24 hours of scheduled time. Please contact the patient directly.",
        "severity": "HIGH"
    }
}

# Block specific appointment ID=1 cancellation (cancelled spelling)
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "update")
    contains(sql_command, "appointments")
    regex.match(`id\s*=\s*1\b`, sql_command)
    contains(sql_command, "cancelled")
    
    decision := {
        "rule_id": "APPT-003",
        "reason": "Cannot cancel appointment ID 1 - it is scheduled within 24 hours",
        "severity": "HIGH"  
    }
}

# Block specific appointment ID=1 cancellation (canceled spelling)
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "update")
    contains(sql_command, "appointments")
    regex.match(`id\s*=\s*1\b`, sql_command)
    contains(sql_command, "canceled")
    
    decision := {
        "rule_id": "APPT-003",
        "reason": "Cannot cancel appointment ID 1 - it is scheduled within 24 hours",
        "severity": "HIGH"  
    }
}
