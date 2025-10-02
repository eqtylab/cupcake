#!/usr/bin/env python3
"""
Signal to check if an appointment is within 24 hours of current time.
Extracts appointment ID from SQL and queries the database.
"""
import json
import sys
import re
import psycopg2
from datetime import datetime, timedelta
import os

# Create log file path
log_file = "/tmp/appointment_signal.log"

def log_message(message):
    """Log messages to file for debugging"""
    with open(log_file, "a") as f:
        timestamp = datetime.now().isoformat()
        f.write(f"[{timestamp}] {message}\n")
        f.flush()

def extract_appointment_id(sql_command):
    """Extract appointment ID from SQL UPDATE command"""
    log_message(f"Extracting ID from SQL: {sql_command}")
    
    # Look for patterns like: id = 1, id=1, id = '1', WHERE id = 1, etc.
    patterns = [
        r"id\s*=\s*['\"]?(\d+)['\"]?",
        r"WHERE.*id\s*=\s*['\"]?(\d+)['\"]?",
    ]
    
    sql_lower = sql_command.lower()
    for pattern in patterns:
        log_message(f"Trying pattern: {pattern}")
        match = re.search(pattern, sql_lower, re.IGNORECASE)
        if match:
            appointment_id = int(match.group(1))
            log_message(f"Found appointment ID: {appointment_id}")
            return appointment_id
    
    log_message("No appointment ID found")
    return None

def check_appointment_time(appointment_id):
    """Check if appointment is within 24 hours"""
    log_message(f"Checking appointment time for ID: {appointment_id}")
    
    try:
        # Connect to database
        log_message("Connecting to database...")
        conn = psycopg2.connect(
            host="localhost",
            port=5432,
            database="appointments",
            user="demo",
            password="demopass"
        )
        log_message("Database connected")
        
        cursor = conn.cursor()
        
        # Get appointment time
        query = "SELECT appointment_time FROM appointments WHERE id = %s"
        log_message(f"Executing query: {query} with ID: {appointment_id}")
        
        cursor.execute(query, (appointment_id,))
        
        result = cursor.fetchone()
        log_message(f"Query result: {result}")
        
        if not result:
            log_message(f"No appointment found for ID: {appointment_id}")
            return {"error": f"Appointment {appointment_id} not found"}
        
        appointment_time = result[0]
        current_time = datetime.now()
        time_until_appointment = appointment_time - current_time
        
        log_message(f"Appointment time: {appointment_time}")
        log_message(f"Current time: {current_time}")
        log_message(f"Time until appointment: {time_until_appointment}")
        log_message(f"Hours until appointment: {time_until_appointment.total_seconds() / 3600}")
        
        # Check if within 24 hours
        within_24_hours = time_until_appointment < timedelta(hours=24) and time_until_appointment > timedelta(0)
        log_message(f"Within 24 hours: {within_24_hours}")
        
        cursor.close()
        conn.close()
        
        result_data = {
            "appointment_id": appointment_id,
            "appointment_time": appointment_time.isoformat(),
            "current_time": current_time.isoformat(),
            "hours_until_appointment": time_until_appointment.total_seconds() / 3600,
            "within_24_hours": within_24_hours,
            "is_past": time_until_appointment < timedelta(0)
        }
        
        log_message(f"Returning result: {json.dumps(result_data)}")
        return result_data
        
    except psycopg2.Error as e:
        log_message(f"Database error: {str(e)}")
        return {"error": f"Database error: {str(e)}"}
    except Exception as e:
        log_message(f"General error: {str(e)}")
        return {"error": str(e)}

def main():
    # Clear previous log for new session
    with open(log_file, "a") as f:
        f.write("\n" + "="*80 + "\n")
        f.write(f"[{datetime.now().isoformat()}] NEW SIGNAL EXECUTION\n")
        f.write("="*80 + "\n")
    
    # Read input from stdin (Cupcake passes the event as JSON)
    try:
        log_message("Reading input from stdin...")
        input_data = json.load(sys.stdin)
        log_message(f"Received input data: {json.dumps(input_data, indent=2)}")
        
        # Check if this is an SQL command that updates appointments
        tool_name = input_data.get("tool_name")
        log_message(f"Tool name: {tool_name}")
        
        if tool_name != "mcp__postgres__execute_sql":
            # Not a database operation, no need to check
            log_message("Not a postgres SQL tool, marking as not relevant")
            output = {"relevant": False}
            log_message(f"Output: {json.dumps(output)}")
            print(json.dumps(output))
            return
        
        sql_command = input_data.get("tool_input", {}).get("sql", "")
        log_message(f"SQL command: {sql_command}")
        
        # Check if this is an UPDATE to appointments
        sql_lower = sql_command.lower()
        has_update = "update" in sql_lower
        has_appointments = "appointments" in sql_lower
        log_message(f"Has UPDATE: {has_update}, Has appointments: {has_appointments}")
        
        if not has_update or not has_appointments:
            log_message("Not an UPDATE to appointments, marking as not relevant")
            output = {"relevant": False}
            log_message(f"Output: {json.dumps(output)}")
            print(json.dumps(output))
            return
        
        # Extract appointment ID
        appointment_id = extract_appointment_id(sql_command)
        
        if not appointment_id:
            log_message("Could not extract appointment ID")
            output = {
                "relevant": True,
                "error": "Could not extract appointment ID from SQL"
            }
            log_message(f"Output: {json.dumps(output)}")
            print(json.dumps(output))
            return
        
        # Check the appointment time
        result = check_appointment_time(appointment_id)
        result["relevant"] = True
        result["sql_command"] = sql_command
        
        log_message(f"Final output: {json.dumps(result, indent=2)}")
        print(json.dumps(result))
        
    except Exception as e:
        log_message(f"Exception in main: {str(e)}")
        error_output = {"error": str(e)}
        log_message(f"Error output: {json.dumps(error_output)}")
        print(json.dumps(error_output))

if __name__ == "__main__":
    main()