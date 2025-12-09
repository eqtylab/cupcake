# METADATA
# scope: package
# title: Appointment Database Protection Policy (Cursor)
# description: Prevents deletion of appointment data and cancellation within 24 hours
# custom:
#   routing:
#     required_events: ["beforeMCPExecution"]
#     required_signals: ["appointment_time_check"]
package cupcake.policies.cursor.appointments

import rego.v1

# Block all DELETE operations on the database
# Note: Cursor uses beforeMCPExecution for MCP tool events
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    input.tool_name == "execute_sql"

    # Parse tool_input JSON string and access sql field
    tool_input := json.unmarshal(input.tool_input)
    sql_command := lower(tool_input.sql)
    contains(sql_command, "delete")

    decision := {
        "rule_id": "APPT-001",
        "reason": "Database deletion operations are not permitted for data retention",
        "severity": "CRITICAL"
    }
}

# Block appointment cancellations within 24 hours using signal data (cancelled spelling)
deny contains decision if {
    input.hook_event_name == "beforeMCPExecution"
    input.tool_name == "execute_sql"

    # Parse tool_input JSON string and access sql field
    tool_input := json.unmarshal(input.tool_input)
    sql_command := lower(tool_input.sql)
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
    input.hook_event_name == "beforeMCPExecution"
    input.tool_name == "execute_sql"

    # Parse tool_input JSON string and access sql field
    tool_input := json.unmarshal(input.tool_input)
    sql_command := lower(tool_input.sql)
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
