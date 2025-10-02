# METADATA
# scope: package
# title: Appointment Database Protection Policy
# description: Prevents deletion of appointment data and cancellation within 24 hours
# custom:
#   routing:
#     required_events: ["PreToolUse"]
#     required_tools: ["mcp__postgres__execute_sql"]
#     required_signals: ["appointment_time_check"]
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
        "reason": "Database deletion operations are not permitted for data retention",
        "severity": "CRITICAL"
    }
}

# Block appointment cancellations within 24 hours using signal data (cancelled spelling)
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    # Check if this is an UPDATE to cancel an appointment
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "update")
    contains(sql_command, "appointments")
    contains(sql_command, "cancelled")
    
    # Check signal data for appointment time
    signal_data := input.signals.appointment_time_check
    signal_data.relevant == true
    signal_data.within_24_hours == true
    
    decision := {
        "rule_id": "APPT-002",
        "reason": "Cannot cancel this appointment - it is scheduled within 24 hours. Please contact the patient directly.",
        "severity": "HIGH"
    }
}

# Block appointment cancellations within 24 hours using signal data (canceled spelling)
deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "mcp__postgres__execute_sql"
    
    # Check if this is an UPDATE to cancel an appointment
    sql_command := lower(input.tool_input.sql)
    contains(sql_command, "update")
    contains(sql_command, "appointments")
    contains(sql_command, "canceled")
    
    # Check signal data for appointment time
    signal_data := input.signals.appointment_time_check
    signal_data.relevant == true
    signal_data.within_24_hours == true
    
    decision := {
        "rule_id": "APPT-002",
        "reason": "Cannot cancel this appointment - it is scheduled within 24 hours. Please contact the patient directly.",
        "severity": "HIGH"
    }
}
