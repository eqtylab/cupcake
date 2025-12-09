#!/usr/bin/env python3
"""Signal to check if an appointment is within 24 hours of current time."""
import json
import sys
import re
import psycopg2
from datetime import datetime, timedelta


def extract_identifier(sql_command):
    """Extract appointment ID or patient name from SQL."""
    # Try ID first
    match = re.search(r"id\s*=\s*['\"]?(\d+)['\"]?", sql_command, re.IGNORECASE)
    if match:
        return {"type": "id", "value": int(match.group(1))}

    # Try patient name
    match = re.search(r"patient_name\s*=\s*'([^']+)'", sql_command, re.IGNORECASE)
    if match:
        return {"type": "patient_name", "value": match.group(1)}

    return None


def check_appointment_time(identifier):
    """Check if appointment is within 24 hours."""
    try:
        conn = psycopg2.connect(
            host="localhost",
            port=15432,
            database="appointments",
            user="demo",
            password="demopass"
        )
        cursor = conn.cursor()

        if identifier["type"] == "id":
            query = "SELECT id, patient_name, appointment_time FROM appointments WHERE id = %s"
        else:
            query = "SELECT id, patient_name, appointment_time FROM appointments WHERE patient_name = %s"

        cursor.execute(query, (identifier["value"],))
        result = cursor.fetchone()
        cursor.close()
        conn.close()

        if not result:
            return {"error": f"Appointment not found for {identifier['type']}: {identifier['value']}"}

        appointment_id, patient_name, appointment_time = result
        current_time = datetime.now()
        time_until = appointment_time - current_time
        hours_until = time_until.total_seconds() / 3600

        return {
            "appointment_id": appointment_id,
            "patient_name": patient_name,
            "appointment_time": appointment_time.isoformat(),
            "hours_until_appointment": hours_until,
            "within_24_hours": 0 < hours_until < 24,
            "is_past": hours_until < 0
        }

    except psycopg2.Error as e:
        return {"error": f"Database error: {str(e)}"}


def main():
    try:
        input_data = json.load(sys.stdin)

        if input_data.get("tool_name") != "mcp__postgres__execute_sql":
            print(json.dumps({"relevant": False}))
            return

        sql = input_data.get("tool_input", {}).get("sql", "").lower()

        if "update" not in sql or "appointments" not in sql:
            print(json.dumps({"relevant": False}))
            return

        identifier = extract_identifier(input_data.get("tool_input", {}).get("sql", ""))
        if not identifier:
            print(json.dumps({"relevant": True, "error": "Could not extract appointment identifier from SQL"}))
            return

        result = check_appointment_time(identifier)
        result["relevant"] = True
        print(json.dumps(result))

    except Exception as e:
        print(json.dumps({"error": str(e)}))


if __name__ == "__main__":
    main()
